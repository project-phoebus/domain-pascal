//! Instruction selection: IR → m68k instructions (CLAUDE.md 4.4).
//!
//! Three steps per routine:
//!
//! 1. **Body.**  Pattern-directed, one IR node at a time, over virtual
//!    registers (`regs.rs`): every value a pattern computes gets a new
//!    one.  A pattern that loads A5 records it in the selector's
//!    `clobbered` set.
//! 2. **Registers.**  [`crate::regs::allocate`] computes the live ranges
//!    of the body and gives each value the lowest register of its class
//!    free over its whole range, a callee-saved one if it lives across a
//!    call (codegen-patterns 12.4, R1-R3), and rewrites the body.
//!    [`crate::sched::schedule`] then orders the instructions inside
//!    each basic block (codegen-patterns 12.4 R9-R12; R11 with a window
//!    of 3 by default).
//! 3. **Frame.**  The callee-saved registers the body wrote decide the
//!    save set; [`crate::frame::Frame`] then gives the prologue (`link.w`,
//!    the save) and the epilogue (the restore from the frame, `unlk`,
//!    `rts`) around the body.
//!
//! Each pattern cites the image address(es) it reproduces.  Patterns the
//! image has not shown are rejected, not guessed.

use dpc_ir::{
    Addr, Arg, BinOp, Call, Callee, CaseArm, Cond as IrCond, Expr, ForLoop, Function, LinkArg, Rel,
    ResultClass, ResultSpec, Scalar, Sign, StaticLink, Stmt, StmtKind, UnOp, Width,
};
use dpc_syntax::{Diagnostic, Span};

use crate::emit::DATA_BASE_LABEL;
use crate::frame::{frame_for, FP, SP};
use crate::insn::{AluOp, Cond, Ea, Insn, Reg, RegSet, ShiftCount, ShiftOp, Size, UnaryOp};
use crate::regs::{allocate_apart, RegClass, VRegs, DATA_BASE};
use crate::sched::Schedule;

/// The register a nested routine's static link is passed in
/// (codegen-patterns 6: 0xE13928 `movea.l A6,A1; bsr.w 0xE1359C`).
const STATIC_LINK_A: u8 = 1;

/// A5, the data base register, as an address register number.
const DATA_BASE_A: u8 = 5;

type Res<T> = Result<T, Diagnostic>;

fn unsupported(span: Span, what: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(span, format!("{what} not supported yet"))
}

/// A constant as the immediate of a `w`-sized instruction: a byte as
/// 0..=255 (see [`Ea::Imm`]), a word or longword as the signed number.
fn imm(value: i64, w: Width) -> i64 {
    match w {
        Width::Byte => value & 0xff,
        Width::Word => value as i16 as i64,
        Width::Long => value as i32 as i64,
    }
}

/// The condition code under which `a op b` holds after `cmp b,a` (or
/// `cmpi #b,a`): signed or unsigned (MC68000 PRM, Table 3-19).
fn rel_cond(op: Rel, signed: bool) -> Cond {
    match (op, signed) {
        (Rel::Eq, _) => Cond::Eq,
        (Rel::Ne, _) => Cond::Ne,
        (Rel::Lt, true) => Cond::Lt,
        (Rel::Le, true) => Cond::Le,
        (Rel::Gt, true) => Cond::Gt,
        (Rel::Ge, true) => Cond::Ge,
        (Rel::Lt, false) => Cond::Cs,
        (Rel::Le, false) => Cond::Ls,
        (Rel::Gt, false) => Cond::Hi,
        (Rel::Ge, false) => Cond::Cc,
    }
}

/// Whether a multiplication by the constant `c` is done with shifts and
/// adds ([`Selector::mul_const`]): `c` = 2^s × m with m = 1 or 2^k ± 1
/// (2^k - 1 for k >= 3), `c` in 1..=0xffff.
fn by_shifts(c: i64) -> bool {
    if !(1..=0xffff).contains(&c) {
        return false;
    }
    let m = (c >> c.trailing_zeros()) as u32;
    m == 1 || (m - 1).is_power_of_two() || (m > 3 && (m + 1).is_power_of_two())
}

fn size_of(w: Width) -> Size {
    match w {
        Width::Byte => Size::B,
        Width::Word => Size::W,
        Width::Long => Size::L,
    }
}

/// Select the instructions of one routine, prologue to `rts`.
///
/// An empty procedure is a bare `rts`, without `link` (codegen-patterns
/// 2): TIME_$Q_INIT 0xE16C5C, IIC_$INIT 0xE70A54, SMD_$PUTC 0xE1D89C, and
/// MT_$RELEASE 0xE70A4C with its three by-address parameters (the image
/// has no `link; unlk; rts` at all).  One with locals is refused: no such
/// frame is seen, and whether it would be kept is not known.
pub fn select(f: &Function) -> Result<Vec<Insn>, Diagnostic> {
    select_with(f, Schedule::default())
}

/// [`select`] with the body scheduled under `sched` (`sched.rs`).
pub fn select_with(f: &Function, sched: Schedule) -> Result<Vec<Insn>, Diagnostic> {
    if f.is_empty_procedure() {
        if f.frame_size > 0 {
            return Err(unsupported(
                f.span,
                "an empty procedure with locals (a bare `rts` or a frame: not seen, \
                 codegen-patterns 2) is",
            ));
        }
        return Ok(vec![Insn::Rts]);
    }
    let body = select_body_scheduled(f, sched)?;
    wrap(f, body)
}

/// Phase 2: the frame of `f` around `body`.
pub fn wrap(f: &Function, body: Body) -> Result<Vec<Insn>, Diagnostic> {
    let frame = frame_for(f.frame_size, body.saves, f.span)?;
    let mut out = frame.prologue();
    out.extend(body.insns);
    out.extend(frame.epilogue());
    Ok(out)
}

/// Phase 1's result: the body's instructions and the callee-saved
/// registers they write.
#[derive(Debug)]
pub struct Body {
    pub insns: Vec<Insn>,
    pub saves: RegSet,
}

/// Phase 1: the body of `f`, without prologue and epilogue.
///
/// A body of one simple statement is selected in the statement's own
/// evaluation order (PROC1_$TST_LOCK 0xE148CA: the parameter is read where
/// the expression needs it, after the set's address).  A body of two or
/// more statements first loads the by-value parameters it reads into
/// registers ([`Selector::entry_loads`]; TIME_$Q_INIT_QUEUE
/// 0xE16C62..0xE16C6A).  Those loads, and the values the statements
/// compute, are ordinary values for the allocator (codegen-patterns 12.4
/// R1-R3): a parameter's register is free after its last use, a
/// statement's value takes the lowest register free over its live range,
/// and a value live across a call a callee-saved one (module-local
/// 0xE32B76, A2 and D2 around three calls).  Registers held longer than
/// the live range (12.4 R4) are modelled for a parameter the arguments of
/// a call read last ([`held_through_calls`]; PROC1_$SET_ASID 0xE148FE)
/// and for by-address slots read once, which avoid the registers of
/// earlier statements ([`fresh_slot_loads`]; IIC_$RECEIVE 0xE70ABA), in
/// bodies without branches or calls; not to the end of a statement
/// (R4(a)), nor the scheduling of entry loads with other work (0xE758C4,
/// R9).
///
/// A function body of several statements is selected the same way, its
/// result variable held in a register up to the epilogue
/// ([`Selector::set_result_web`]; codegen-patterns 3, 12.4 R5).
///
/// A body that is one `if` or `repeat` statement is treated like a body of
/// several statements (NAME_CONVERT_ACL_STATUS 0xE58620: one `if ... else
/// if` chain, its by-address status loaded into A0 once, at the top), but
/// may not read by-value parameters: whether the original loaded them at
/// entry there is not known (codegen-patterns 12.2; the sites that load
/// them at entry have several statements).
///
/// Computed values used again later are reused from their register
/// (codegen-patterns 12.4 R7, [`Cse`]): the body is selected once to find
/// which computations a later one can reuse ([`Selector::cse_value`]),
/// then again holding exactly those.
pub fn select_body(f: &Function) -> Result<Body, Diagnostic> {
    select_body_scheduled(f, Schedule::default())
}

/// [`select_body`] with the body scheduled under `sched` after register
/// allocation (`sched.rs`, codegen-patterns 12.4 R9-R12).
pub fn select_body_scheduled(f: &Function, sched: Schedule) -> Result<Body, Diagnostic> {
    let (body, hits) = select_body_with(f, CsePlan::Probe, sched)?;
    if hits.is_empty() {
        return Ok(body);
    }
    let (body, again) = select_body_with(f, CsePlan::Hold(hits.clone()), sched)?;
    if again != hits {
        // The second selection must find the reuses the first one did:
        // the validity of a reuse depends only on labels, calls and
        // stores, which holding a value does not change.
        return Err(unsupported(
            f.span,
            "a body whose reusable computed values change when they are held (codegen-patterns              12.4 R7) is",
        ));
    }
    Ok(body)
}

fn select_body_with(
    f: &Function,
    plan: CsePlan,
    sched: Schedule,
) -> Result<(Body, Vec<(u32, u32)>), Diagnostic> {
    let mut s = Selector::new(f.span);
    s.cse_plan = plan;
    if f.loads_data_base {
        s.load_data_base();
    }
    if f.data_absolute {
        // codegen-patterns 4: the only sites are straight-line stores
        // (REM_NAME_$REGISTER_SERVER 0xE4A4AE, AUDIT_$SUSPEND 0xE70DB6).
        if let Some(st) = f
            .body
            .iter()
            .find(|st| !matches!(st.kind, StmtKind::Store { .. }))
        {
            return Err(unsupported(
                st.span,
                "in a routine that addresses its unit's data absolutely (--a5-absolute), a \
                 statement other than an assignment (only straight-line stores are seen, \
                 codegen-patterns 4) is",
            ));
        }
        s.data_absolute = true;
    }
    s.uplevel = f.uplevel_refs.clone();
    s.escaped = escaped_frame_objects(f);
    let control_only = f.body.len() == 1 && !is_simple(&f.body[0]);
    s.multi_body = f.body.len() >= 2 || control_only;
    if s.multi_body {
        let reads_value_param = param_uses(f)
            .iter()
            .any(|(o, _)| f.value_params.iter().any(|p| p.offset == *o));
        if control_only && reads_value_param {
            return Err(unsupported(
                f.body[0].span,
                "a body of one `if`, `case` or loop statement that reads by-value parameters \
                 (whether they are loaded at entry is not settled, codegen-patterns 12.2) is",
            ));
        }
        reject_mergeable_stores(&f.body)?;
        // A parameter loaded at entry and live across a call is held in a
        // callee-saved register by the allocator (codegen-patterns 12.4
        // R3: PROC1_$SET_TYPE 0xE152F2 D2, D3 around a `jsr`;
        // module-local 0xE32B76, 0xE32B7E A2, D2 around three).
        s.entry_loads(f)?;
        s.entry_regs = s.pinned.iter().map(|&(_, r)| r).collect();
        // A function body of several statements holds its result variable
        // in a register up to the epilogue (codegen-patterns 3, "Result
        // variable"; [`Selector::set_result_web`]).
        s.result_web = f.result.is_some();
    }
    s.load_link(f)?;
    s.reg_locals = register_locals(f);
    s.ptr_locals = loop_pointer_locals(f, &s.reg_locals)
        .into_iter()
        .map(|o| (o, None))
        .collect();
    let mut taken = s.reg_locals.clone();
    taken.extend(s.ptr_locals.iter().map(|&(o, _)| o));
    let call_ptrs = call_pointer_locals(f, &taken);
    s.ptr_locals.extend(call_ptrs.iter().map(|&o| (o, None)));
    s.seq(f, &f.body, Tail::END)?;
    if f.data_absolute
        && s.out.iter().any(|i| {
            matches!(
                i,
                Insn::Branch { .. } | Insn::Dbf { .. } | Insn::Jsr(_) | Insn::Bsr { .. }
            )
        })
    {
        // Where the base register would live across a call, and how a
        // branch would place its `lea`, is not seen (codegen-patterns 4).
        return Err(unsupported(
            f.span,
            "a routine that addresses its unit's data absolutely (--a5-absolute) and calls or \
             branches is",
        ));
    }
    // A0 given as a pointer local's base must reach its use: nothing of
    // the statement may write A0 first (an M$ARITH call among the
    // operands, say).
    for &(at, span) in &s.a0_bases {
        let a0 = Reg::A(0);
        let used = s.out[at..]
            .iter()
            .position(|i| crate::regs::access(i).uses.contains(&a0));
        let written = s.out[at..]
            .iter()
            .position(|i| crate::regs::access(i).defs.contains(&a0));
        if used.is_none() || matches!((used, written), (Some(u), Some(w)) if w < u) {
            return Err(unsupported(
                span,
                "an expression that writes A0 after reading a pointer local through it is",
            ));
        }
    }
    if let Some(l) = s.epilogue {
        s.place(l);
    }
    if let (Some(web), Some(res)) = (s.web, f.result) {
        // The result variable's register into the result register, at
        // the start of the epilogue, where a `return` branches to
        // (PKT_$NEXT_ID 0xE124D0 `move.w D2,D0`; NETBUF_$RTNVA 0xE0ED60
        // `move.l D2,D0`; 62 of the 73 such moves of `tools/resultsurvey.py
        // list reg` are from a callee-saved register).  Dropped when the
        // allocator gives the variable D0 itself (PARITY_$CHK_IO 0xE0B19C
        // `moveq #1,D0; bra.s`; PROC1_$SUSPENDP 0xE14886 `st D0`).
        s.result_moves.push(s.out.len());
        s.out.push(Insn::Move {
            size: size_of(res.ty.width),
            src: Ea::Reg(Reg::D(web)),
            dst: Ea::Reg(Reg::D(0)),
        });
    }
    let mut web_a_move = None;
    if let Some(web) = s.web_a {
        web_a_move = Some(s.out.len());
        // A pointer result variable into A0 at the start of the epilogue
        // (DISK_$GET_BLOCK 0xE3BBCA `movea.l A2,A0`; codegen-patterns 3,
        // "Pointer results": 12 of the 16 such moves are from A2).
        s.result_moves.push(s.out.len());
        s.out.push(Insn::Movea {
            size: Size::L,
            src: Ea::Reg(Reg::A(web)),
            dst: 0,
        });
    }
    // Labels nothing branches to (the join after a `then` part that ends
    // in `return`, ...) are dropped; they emit no bytes either way.
    // A `case` table's entries name their targets like branches (a
    // target before a `bra` is refused as for a branch); its own label is
    // named by the dispatch.
    let used: Vec<u32> = s
        .out
        .iter()
        .flat_map(|i| match i {
            Insn::Branch { label, .. } | Insn::Dbf { label, .. } => vec![*label],
            Insn::TableIndex { table, .. } => vec![*table],
            Insn::TableEntry { target, .. } => vec![*target],
            _ => vec![],
        })
        .collect();
    if let Some(&(_, span)) = s.before_bra.iter().find(|(l, _)| used.contains(l)) {
        // `if a then begin if b then ... else ... end else ...`: the inner
        // `then` part's `bra` to its join reaches the outer `bra`; `if a
        // then begin if b then ... end else ...` and `if a then begin if b
        // then ...; return end` likewise with the inner test's `b<cc>`;
        // `if a or else b then return` with the `bmi` past the test of `b`.
        return Err(unsupported(
            span,
            "a branch to a `bra` (an `if` part ending in an `if`, then a `bra` to a join or \
             the epilogue; compiled code has no branch to a `bra`, codegen-patterns 7.1) is",
        ));
    }
    // An `exit` or `next` whose target is the very next instruction (a
    // `next` ending a loop body, an `exit` before an inner loop's end):
    // whether the original emitted the branch is not known.
    for &(at, span) in &s.loop_jumps {
        let Insn::Branch { label, .. } = s.out[at] else {
            unreachable!("loop_jumps records branches")
        };
        let next = s.out[at + 1..]
            .iter()
            .take_while(|i| matches!(i, Insn::Label(_) | Insn::Align4))
            .any(|i| matches!(i, Insn::Label(l) if *l == label));
        if next {
            return Err(unsupported(
                span,
                "an `exit` or `next` that would branch to the next instruction is",
            ));
        }
    }
    // Phase 1b: physical registers (`regs.rs`, codegen-patterns 12.4).
    let exit: &[Reg] = match f.result {
        Some(ResultSpec {
            class: ResultClass::Data,
            ..
        }) => &[Reg::D(0)],
        Some(ResultSpec {
            class: ResultClass::Address,
            ..
        }) => &[Reg::A(0)],
        _ => &[],
    };
    // The registers of the pointer locals assigned from calls: held
    // through a call whose arguments use them last, in a body with
    // branches too ([`call_pointer_locals`]).
    let held_ptrs: Vec<Reg> = s
        .ptr_locals
        .iter()
        .filter(|(o, _)| call_ptrs.contains(o))
        .filter_map(|&(_, a)| a.map(Reg::A))
        .collect();
    let holds = held_through_calls(&s.out, &s.call_stmts, exit, s.link.map(Reg::A), &held_ptrs)?;
    let mut apart = fresh_slot_loads(&s.out, &s.fresh)?;
    apart.extend(argument_list_loads(&s.out, &s.call_stmts)?);
    let first = s.web.map(|w| (Reg::D(w), s.entry_regs.clone()));
    let saves = allocate_apart(&mut s.out, exit, &holds, &apart, first)
        .map_err(|what| unsupported(f.span, what))?;
    for &(o, k, span) in &s.ptr_defs {
        // A pointer local assigned from calls that no call follows: the
        // image keeps such locals in D0 (`move.l A0,D0` after the call,
        // `movea.l D0,A0` where it is dereferenced: 28 sites, TTY_$K_*
        // and SIO_$K_* after TTY_$I_GET_DESC / SIO_$I_GET_DESC, e.g.
        // TTY_$K_INQ_FUNC_ENABLED 0xE6763C / 0xE67642) or in A1
        // (0xE4D7FC, 0xE528CE); which, is not known (bead
        // domain-pascal-500).  Only one in a callee-saved register (live
        // across a call, or held through one) is kept.
        if let Insn::Movea { dst, .. } = s.out[k] {
            if call_ptrs.contains(&o) && dst < 2 {
                return Err(unsupported(
                    span,
                    "a pointer local assigned from a call that no call follows (D0, A1 or a \
                     callee-saved register in the image; not settled) is",
                ));
            }
        }
    }
    if let Some(Insn::Movea {
        src: Ea::Reg(Reg::A(a)),
        ..
    }) = web_a_move.map(|k| &s.out[k])
    {
        // A pointer result variable that no call follows: of the 16
        // register-held ones (`tools/resultsurvey.py pointer`), the 12 in
        // A2 and the one in A3 all have a call after their first
        // assignment (codegen-patterns 3, "Pointer results";
        // DISK_$GET_BLOCK 0xE3BBBC); the one that none follows is in D0
        // (0xE6C21A: `move.l
        // A0,D0` after DISK_$GET_BLOCK at 0xE6C244, `clr.l D0` on another
        // path, `movea.l D0,A0` at 0xE6C2AA), others in their frame slot
        // (TTY_$I_GET_DESC 0xE667BA, the NET_ module-local 0xE5A188):
        // never in A1, which the allocator gives it when it is assigned a
        // call's A0 (R2 keeps it out of A0) or A0 is busy (bead
        // domain-pascal-500).  A0 itself (only memory loads assigned, no
        // call after) is kept as before.
        if *a == 1 {
            return Err(unsupported(
                f.span,
                "a pointer result variable held in a register that no call follows (D0 at \
                 0xE6C21A, the frame elsewhere; not settled) is",
            ));
        }
    }
    // A result computed in D0 already is not moved (0xE148E0 moves it
    // only because the `sne` left it in D1).
    let keep: Vec<bool> = s
        .out
        .iter()
        .enumerate()
        .map(|(k, i)| {
            !(s.result_moves.contains(&k)
                && (matches!(i, Insn::Move { src: Ea::Reg(a), dst: Ea::Reg(b), .. } if a == b)
                    || matches!(i, Insn::Movea { src: Ea::Reg(Reg::A(a)), dst, .. } if a == dst)))
                && !matches!(i, Insn::Label(l) if !used.contains(l))
        })
        .collect();
    // The units in the kept instructions' indices.
    let mut new_index = Vec::with_capacity(keep.len() + 1);
    let mut n = 0;
    for &k in &keep {
        new_index.push(n);
        n += usize::from(k);
    }
    new_index.push(n);
    let units: Vec<crate::sched::Unit> = s
        .units
        .iter()
        .map(|&(a, b)| (new_index[a], new_index[b]))
        .filter(|(a, b)| b - a > 1)
        .collect();
    let mut out: Vec<Insn> = s
        .out
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(i, _)| i)
        .collect();
    // Phase 1c: the order inside basic blocks (`sched.rs`).
    crate::sched::schedule(&mut out, &units, sched);
    Ok((
        Body {
            insns: out,
            saves: saves.union(s.clobbered),
        },
        s.cse_hits,
    ))
}

/// The values to hold live through a call (codegen-patterns 12.4 R4(c)),
/// as `(index of the jsr/bsr, register)` for [`allocate`].
///
/// A value defined before a call statement (in `dpc`, a parameter loaded at
/// entry) whose last use is in that statement's arguments:
///
/// - in a body without branches it is held through the call, so it takes
///   a callee-saved register: PROC1_$SET_ASID 0xE148FE D2 (`move.w
///   D2,-(SP)` at 0xE1490E); TTY_$I_FLUSH_OUTPUT 0xE1B80C A2,
///   TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39A A2, module-local 0xE1D592 at
///   0xE1D598 A2, SIO_$I_INIT 0xE67E64 A2.  These are
///   all five such parameter loads in the image's routines without
///   branches where liveness alone leaves a scratch register free
///   (`tools/regalloc.py callargs`);
/// - in a body with branches the image does both (26 parameter loads
///   callee-saved, 49 scratch: EC2_$ADVANCE 0xE42CB8 D0, `move.l
///   D0,-(SP)` at 0xE42CDA; DIR_$FIND_NET 0xE4FE36 A0, `pea (A0)` at
///   0xE4FE76), by a rule not known: refused.
///
/// A value already live across an earlier call (so in a callee-saved
/// register by R3) is held with branches too: CAL_$SHUTDOWN 0xE3E6E4
/// keeps `status` in A2 across DBUF_$GET_BLOCK and passes it last to
/// DBUF_$SET_BUFF (`pea (A2)` at 0xE3E72A); `tools/regalloc.py callargs`
/// counts only values that cross no call.
///
/// `ptrs`: the registers of pointer locals assigned from calls
/// ([`call_pointer_locals`]), held through such a call with branches too:
/// of the 108 `movea.l A0,An` copies of a call's result, the 13 whose
/// register is next read only inside a later call's arguments, all in
/// bodies with branches, are all in callee-saved registers (AST_
/// module-local 0xE070CA `movea.l A0,A3`, `pea (A3)` at 0xE070DC;
/// VTOC_$ALLOCATE 0xE38BB6; KBD_$SET_KBD_TYPE 0xE7253E A2, `pea (A2)`
/// at 0xE72550), against none in A1 or D0/D1 (bead domain-pascal-500,
/// ad-hoc script over `tools/resultsurvey.py`'s `a0_reads`).
///
/// `link`: the register of a nested routine's static link (codegen-patterns
/// 6), loaded at entry like a parameter; whether the original held it
/// through a call whose arguments use it last (reaching the enclosing
/// frame, or passing it on to a sibling) is not seen either.
fn held_through_calls(
    out: &[Insn],
    calls: &[(usize, usize, Span)],
    exit: &[Reg],
    link: Option<Reg>,
    ptrs: &[Reg],
) -> Res<Vec<(usize, Reg)>> {
    let straight = !out
        .iter()
        .any(|i| matches!(i, Insn::Branch { .. } | Insn::Dbf { .. }));
    let live_out = crate::regs::liveness(out, exit, &[]);
    let mut holds = Vec::new();
    for &(start, call, span) in calls {
        let defined_before: Vec<Reg> = out[..start]
            .iter()
            .flat_map(|i| crate::regs::access(i).defs)
            .filter(|&r| crate::regs::is_virtual(r))
            .collect();
        for i in &out[start..call] {
            for r in crate::regs::access(i).uses {
                if !defined_before.contains(&r)
                    || live_out[call].contains(&crate::regs::RegKey(r))
                    || holds.contains(&(call, r))
                {
                    continue;
                }
                if !straight && Some(r) == link {
                    return Err(unsupported(
                        span,
                        "in a body with branches, a call whose arguments are the last use of the \
                         static link (held through the call or not, codegen-patterns 6, 12.4 \
                         R4(c)) is",
                    ));
                }
                // A value live across an earlier call is in a callee-saved
                // register whatever this call does (12.4 R3); the survey
                // of R4(c) leaves such values out (`tools/regalloc.py
                // callargs`, `crosses_call`): held, as in a body without
                // branches.
                let crossed = calls
                    .iter()
                    .any(|&(_, c, _)| c < start && live_out[c].contains(&crate::regs::RegKey(r)));
                if !straight && !ptrs.contains(&r) && !crossed {
                    return Err(unsupported(
                        span,
                        "in a body with branches, a call whose arguments are the last use of a \
                         parameter loaded at entry (held through the call or not, \
                         codegen-patterns 12.4 R4(c)) is",
                    ));
                }
                holds.push((call, r));
            }
        }
    }
    Ok(holds)
}

/// The register pairs to keep apart for by-address slots loaded for one
/// use in a body of several statements (codegen-patterns 12.4 R4(b)), as
/// `(slot value, earlier value)` for [`allocate_apart`].
///
/// Such a slot is loaded where its statement needs it, into the lowest
/// address register that is free over its range *and* does not hold a
/// value an earlier statement left there, even when that value is dead:
///
/// - IIC_$RECEIVE 0xE70AC0..0xE70AD0: three slots, each read once, in
///   A0, A1 and A2 (callee-saved, so saved with `pea (A2)`) although A0
///   and A1 are dead by then;
/// - PROC2_$ALIGN_CTL 0xE41D82..0xE41D8E likewise (A0, A1, A2);
/// - XNS_IDP_$GET_STATS 0xE18FF4: the status slot in A1 after the stats
///   slot, held in A0 for three stores and dead (12.2);
/// - MMAP_$ALLOC_CONTIG 0xE0D902 A1 after A0; module-local 0xE32BB8 A2
///   after A0 and A1.
///
/// A register whose value was used up by `(An)+` / `-(An)` (a block copy)
/// does count as free: DIR_$ADD_MOUNT 0xE534DC loads its second slot
/// into A0 after the first one's `(A0)+` copy; ACL_ 0xE47944 into A2 after
/// a `(A2)+` copy, while the dead A0 of an earlier statement is passed
/// over.  Within one statement a dead register is reused (0xE41C2E
/// `movea.l (8,A6),A0` after A0's base was last read by the `lea` before).
///
/// Survey (ad-hoc script over the routines `tools/regalloc.py` analyses,
/// `movea.l (d,A6),An` loads of a slot): in the 105 such loads in
/// routines without branches and calls, this rule and plain lowest-free
/// (12.4 R1) agree 86 times; of the other 19 this rule picks the
/// original's register 17 times, lowest-free once (0xE41C2E, the same
/// statement), neither once (0xE3353A).  Where a branch or a call comes
/// before the load the original's register fits this rule only, or
/// lowest-free only, 85/28 times (branches) and 102/81 (branches and
/// calls), a reload of the same slot counted as the same value: the rule
/// there is not known, and such bodies are refused.
fn fresh_slot_loads(out: &[Insn], fresh: &[(Reg, usize, Span)]) -> Res<Vec<(Reg, Reg)>> {
    let Some(&(_, _, span)) = fresh.first() else {
        return Ok(Vec::new());
    };
    if out.iter().any(|i| {
        matches!(
            i,
            Insn::Branch { .. } | Insn::Dbf { .. } | Insn::Jsr(_) | Insn::Bsr { .. }
        )
    }) {
        return Err(unsupported(
            span,
            "in a body of several statements with branches or calls, a pointer parameter or \
             by-address parameter that is used once (loaded where needed into a fresh register: \
             IIC_$RECEIVE 0xE70AC0, codegen-patterns 12.4 R4(b)) is",
        ));
    }
    // Registers used up by `(An)+` / `-(An)`.
    let consumed: Vec<u8> = out
        .iter()
        .flat_map(|i| {
            let mut v = Vec::new();
            let mut note = |ea: &Ea| {
                if let Ea::PostInc(a) | Ea::PreDec(a) = ea {
                    v.push(*a);
                }
            };
            if let Insn::Move { src, dst, .. } = i {
                note(src);
                note(dst);
            }
            if let Insn::Clr { dst, .. } = i {
                note(dst);
            }
            v
        })
        .collect();
    let mut apart = Vec::new();
    for &(v, start, _) in fresh {
        for i in &out[..start] {
            for d in crate::regs::access(i).defs {
                if let Reg::A(n) = d {
                    if crate::regs::is_virtual(d) && !consumed.contains(&n) {
                        apart.push((v, d));
                    }
                }
            }
        }
    }
    Ok(apart)
}

/// Address values loaded inside one call's argument list are kept apart
/// (codegen-patterns 12.4 R4(a), "Inside one argument list"): each takes a
/// register no earlier load of the list took, although the earlier one
/// is dead.  MST_$MAP 0xE43886..0xE438AA `movea.l (0x1c,A6),A0; move.b
/// (A0),-(SP)`, then A1, A2, A3, A4 for four more slots read once (A2-A4
/// saved by the `movem` for this alone); the same at MST_$MAP_TOP
/// 0xE438E6 and MST_$MAP_GLOBAL 0xE43946; the nested 0xE1359C (A0, A1,
/// A3).  Survey (ad-hoc script over `tools/nestsurvey.py`'s routines, not
/// kept): of the argument lists with two or more slot loads read once by
/// the next push, 39 use a new register for each and 4 reuse one, all four
/// when the address registers run out (MST_$MAP_AT 0xE42F9C reuses A0
/// for the sixth slot after A0-A4; 0xE59AB6, 0xE59B50, 0xE59E4E with A2-A4
/// or A3 busy).  Such a list is refused here: which register the original
/// reused then is not modelled.
fn argument_list_loads(out: &[Insn], calls: &[(usize, usize, Span)]) -> Res<Vec<(Reg, Reg)>> {
    let mut apart = Vec::new();
    for &(start, jsr, span) in calls {
        let mut loads: Vec<Reg> = Vec::new();
        for i in &out[start..jsr] {
            for d in crate::regs::access(i).defs {
                if matches!(d, Reg::A(_)) && crate::regs::is_virtual(d) && !loads.contains(&d) {
                    loads.push(d);
                }
            }
        }
        if loads.len() > crate::regs::ALLOC_A.len() {
            return Err(unsupported(
                span,
                "more address values in one argument list than address registers (the \
                 original reuses one, MST_$MAP_AT 0xE42F9C; codegen-patterns 12.4 R4(a)) is",
            ));
        }
        for (k, &a) in loads.iter().enumerate() {
            for &b in &loads[k + 1..] {
                apart.push((a, b));
            }
        }
    }
    Ok(apart)
}

/// The scalar locals of `f` held in a data register rather than in their
/// frame slot (codegen-patterns 12.5; ref 6.4.24, p. 6-29: at `-opt 3`
/// "Global register allocation allows local variables to have their values
/// placed in machine registers").  The slot stays allocated: TIME_$GET_ADJUST
/// 0xE16AA8 `link.w A6,#-4` never touches its frame, its local is in D1
/// (0xE16AB4..0xE16AD6).
///
/// The rule (**Uncertain**, the narrowest one that fits the sites):
/// a local whose only references are whole reads and assignments of the
/// variable, assigned exactly once, by a statement of the routine's own
/// statement part (not inside an `if`, loop or `case`), with a value that
/// is not a constant, read only by later statements of that part, and
/// never read inside an argument of a call.
/// Fits TIME_$GET_ADJUST (D1) and PROC1_$SET_VT 0xE149B6 / PROC1_$TS_END_CALLBACK
/// 0xE14A90 (the saved status word in D3; those assignments are inside an
/// `if` there, which `dpc` does not yet take).  Locals that do not fit
/// stay in the frame, as before.  The image keeps a local passed by value
/// in its frame even when it is assigned once at the top level
/// (codegen-patterns 12.5 survey: MMAP_$FREE 0xE0CAEC, PROC2_$DELIVER_PENDING
/// 0xE3F53E, LOG_$SHUTDN 0xE175AE, TTY_$K_FLUSH_INPUT 0xE1C0BE,
/// SIO2681_$SET_BREAK 0xE1D134), and PROC1_$TS_END_CALLBACK's
/// `timeslice` (0xE14AFE `move.w (-8,A6),-(SP)`).  A constant assigned to a register variable is
/// `move.w #-1,D2` at 0xE149A4, not the `moveq` of [`Selector::load_const`]:
/// such locals stay in the frame too.
fn register_locals(f: &Function) -> Vec<i32> {
    #[derive(Default)]
    struct Refs {
        /// (top-level statement index, nested, value is a constant)
        stores: Vec<(usize, bool, bool)>,
        loads: Vec<usize>,
        other: bool,
        /// Read inside an argument of a call.
        arg: bool,
    }
    struct Walk<'a> {
        f: &'a Function,
        refs: Vec<Refs>,
        top: usize,
        /// Walking an argument of a call.
        in_arg: bool,
    }
    impl Walk<'_> {
        /// The local whose object covers frame offset `o`.
        fn local(&self, o: i32) -> Option<usize> {
            self.f
                .scalar_locals
                .iter()
                .position(|l| o >= l.offset && o < l.offset + l.ty.bytes() as i32)
        }
        fn exact(&self, o: i32, w: Width) -> Option<usize> {
            self.f
                .scalar_locals
                .iter()
                .position(|l| l.offset == o && l.ty.width == w)
        }
        fn other(&mut self, a: &Addr) {
            match a {
                Addr::Frame(o) => {
                    if let Some(k) = self.local(*o) {
                        self.refs[k].other = true;
                    }
                }
                Addr::Enclosing(_) | Addr::Static(_) | Addr::Symbol { .. } | Addr::Const(_) => {}
                Addr::Indirect { base, .. } => match &**base {
                    // A local dereferenced as a pointer: not a data value.
                    Expr::Load { addr, .. } => self.other(addr),
                    b => self.expr(b),
                },
                Addr::Index { base, index, .. } => {
                    self.other(base);
                    self.expr(index);
                }
            }
        }
        fn expr(&mut self, e: &Expr) {
            match e {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ty,
                } if self.local(*o).is_some() => match self.exact(*o, ty.width) {
                    Some(k) => {
                        self.refs[k].loads.push(self.top);
                        self.refs[k].arg |= self.in_arg;
                    }
                    None => self.other(&Addr::Frame(*o)),
                },
                Expr::Load { addr, .. } | Expr::AddrOf(addr) => self.other(addr),
                Expr::Const { .. } => {}
                Expr::SetMember { elem, set, .. } => {
                    self.expr(elem);
                    self.expr(set);
                }
                Expr::Binary { lhs, rhs, .. } => {
                    self.expr(lhs);
                    self.expr(rhs);
                }
                Expr::Unary { operand: x, .. }
                | Expr::Convert { value: x, .. }
                | Expr::SetSr(x) => self.expr(x),
                Expr::Call { call, .. } => self.call(call),
                Expr::Test(c) => self.cond(c),
            }
        }
        fn call(&mut self, c: &Call) {
            let outer = self.in_arg;
            self.in_arg = true;
            for a in &c.args {
                match a {
                    Arg::Value { value, .. } | Arg::Temp { value, .. } => self.expr(value),
                    Arg::Address { addr, .. } => self.other(addr),
                }
            }
            self.in_arg = outer;
        }
        fn cond(&mut self, c: &IrCond) {
            match c {
                IrCond::Compare { lhs, rhs, .. } => {
                    self.expr(lhs);
                    self.expr(rhs);
                }
                IrCond::Test(e) => self.expr(e),
                IrCond::Not(x) => self.cond(x),
                IrCond::AndThen(a, b) | IrCond::OrElse(a, b) => {
                    self.cond(a);
                    self.cond(b);
                }
            }
        }
        fn stmts(&mut self, body: &[Stmt], nested: bool) {
            for (i, st) in body.iter().enumerate() {
                if !nested {
                    self.top = i;
                }
                match &st.kind {
                    StmtKind::Store {
                        addr: Addr::Frame(o),
                        value,
                    } if self.exact(*o, value.ty().width).is_some() => {
                        self.expr(value);
                        let k = self.exact(*o, value.ty().width).unwrap();
                        let konst = matches!(value, Expr::Const { .. });
                        self.refs[k].stores.push((self.top, nested, konst));
                    }
                    StmtKind::Store { addr, value } => {
                        self.other(addr);
                        self.expr(value);
                    }
                    StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e),
                    StmtKind::Copy { dst, src, .. } => {
                        self.other(src);
                        self.other(dst);
                    }
                    StmtKind::Call(c) => self.call(c),
                    StmtKind::Fill { start, value, .. } => {
                        self.other(start);
                        self.expr(value);
                    }
                    StmtKind::If { cond, then_, else_ } => {
                        self.cond(cond);
                        self.stmts(then_, true);
                        self.stmts(else_, true);
                    }
                    StmtKind::Repeat { body, until } => {
                        self.stmts(body, true);
                        self.cond(until);
                    }
                    StmtKind::While { cond, body } => {
                        self.cond(cond);
                        self.stmts(body, true);
                    }
                    // The index is the loop's own register (`Selector::for_`);
                    // its reads count as reads of a local with no store, so
                    // it is never a register local of this rule.
                    StmtKind::For(l) => {
                        self.expr(&l.from);
                        self.expr(&l.to);
                        self.stmts(&l.body, true);
                    }
                    StmtKind::Case {
                        selector,
                        arms,
                        otherwise,
                    } => {
                        self.expr(selector);
                        for a in arms {
                            self.stmts(&a.body, true);
                        }
                        if let Some(o) = otherwise {
                            self.stmts(o, true);
                        }
                    }
                    StmtKind::Return
                    | StmtKind::Disable
                    | StmtKind::Enable
                    | StmtKind::Exit
                    | StmtKind::Next => {}
                }
            }
        }
    }
    let mut w = Walk {
        f,
        refs: f.scalar_locals.iter().map(|_| Refs::default()).collect(),
        top: 0,
        in_arg: false,
    };
    w.stmts(&f.body, false);
    f.scalar_locals
        .iter()
        .zip(&w.refs)
        .filter(|(_, r)| {
            !r.other
                && !r.arg
                && matches!(r.stores[..], [(at, false, false)] if r.loads.iter().all(|&l| l > at))
        })
        .map(|(l, _)| l.offset)
        .collect()
}

/// A node [`walk`] visits.
#[derive(Clone, Copy)]
enum Node<'a> {
    Expr(&'a Expr),
    Addr(&'a Addr),
    Call(&'a Call),
}

/// Visit `e` and every expression, address and call inside it.
fn walk_expr<'a>(e: &'a Expr, v: &mut dyn FnMut(Node<'a>)) {
    v(Node::Expr(e));
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr, .. } | Expr::AddrOf(addr) => walk_addr(addr, v),
        Expr::SetMember { elem, set, .. } => {
            walk_expr(elem, v);
            walk_expr(set, v);
        }
        Expr::Binary { lhs, rhs, .. } => {
            walk_expr(lhs, v);
            walk_expr(rhs, v);
        }
        Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
            walk_expr(x, v)
        }
        Expr::Call { call, .. } => walk_call(call, v),
        Expr::Test(c) => walk_cond(c, v),
    }
}

fn walk_addr<'a>(a: &'a Addr, v: &mut dyn FnMut(Node<'a>)) {
    v(Node::Addr(a));
    match a {
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::Static(_)
        | Addr::Symbol { .. }
        | Addr::Const(_) => {}
        Addr::Indirect { base, .. } => walk_expr(base, v),
        Addr::Index { base, index, .. } => {
            walk_addr(base, v);
            walk_expr(index, v);
        }
    }
}

fn walk_call<'a>(c: &'a Call, v: &mut dyn FnMut(Node<'a>)) {
    v(Node::Call(c));
    for a in &c.args {
        match a {
            Arg::Value { value, .. } | Arg::Temp { value, .. } => walk_expr(value, v),
            Arg::Address { addr, .. } => walk_addr(addr, v),
        }
    }
}

fn walk_cond<'a>(c: &'a IrCond, v: &mut dyn FnMut(Node<'a>)) {
    match c {
        IrCond::Compare { lhs, rhs, .. } => {
            walk_expr(lhs, v);
            walk_expr(rhs, v);
        }
        IrCond::Test(e) => walk_expr(e, v),
        IrCond::Not(x) => walk_cond(x, v),
        IrCond::AndThen(a, b) | IrCond::OrElse(a, b) => {
            walk_cond(a, v);
            walk_cond(b, v);
        }
    }
}

/// Visit every expression, address and call of `body`, nested statements
/// included.
fn walk<'a>(body: &'a [Stmt], v: &mut dyn FnMut(Node<'a>)) {
    for st in body {
        match &st.kind {
            StmtKind::SetResult(e) | StmtKind::Discard(e) => walk_expr(e, v),
            StmtKind::Store { addr, value } => {
                walk_addr(addr, v);
                walk_expr(value, v);
            }
            StmtKind::Copy { dst, src, .. } => {
                walk_addr(dst, v);
                walk_addr(src, v);
            }
            StmtKind::Call(c) => walk_call(c, v),
            StmtKind::Fill { start, value, .. } => {
                walk_addr(start, v);
                walk_expr(value, v);
            }
            StmtKind::If { cond, then_, else_ } => {
                walk_cond(cond, v);
                walk(then_, v);
                walk(else_, v);
            }
            StmtKind::Repeat { body, until } => {
                walk(body, v);
                walk_cond(until, v);
            }
            StmtKind::While { cond, body } => {
                walk_cond(cond, v);
                walk(body, v);
            }
            StmtKind::For(l) => {
                walk_expr(&l.from, v);
                walk_expr(&l.to, v);
                walk(&l.body, v);
            }
            StmtKind::Case {
                selector,
                arms,
                otherwise,
            } => {
                walk_expr(selector, v);
                for a in arms {
                    walk(&a.body, v);
                }
                if let Some(o) = otherwise {
                    walk(o, v);
                }
            }
            StmtKind::Return
            | StmtKind::Disable
            | StmtKind::Enable
            | StmtKind::Exit
            | StmtKind::Next => {}
        }
    }
}

/// The memory operand an instruction writes, if any.
/// The call of `e` when `e` is the value of a function whose result is in
/// A0 (codegen-patterns 3, "Callers of pointer functions").
fn a0_call(e: &Expr) -> Option<&Call> {
    match e {
        Expr::Call {
            call,
            reg: ResultClass::Address,
            ..
        } => Some(call),
        _ => None,
    }
}

fn mem_dst(i: &Insn) -> Option<&Ea> {
    let dst = match i {
        Insn::Move { dst, .. }
        | Insn::Clr { dst, .. }
        | Insn::Scc { dst, .. }
        | Insn::Unary { dst, .. }
        | Insn::Addq { dst, .. }
        | Insn::Subq { dst, .. }
        | Insn::AluImm { dst, .. } => dst,
        // `Alu` writes a register; `Tst`, `Cmpi`, `Btst`, `BtstImm` read.
        _ => return None,
    };
    (!matches!(dst, Ea::Reg(_))).then_some(dst)
}

/// Whether an instruction may write the bytes `o..o + 4` of the frame
/// ([`Selector::forwarded`]): a plain `(d,A6)` destination that overlaps
/// them, any other A6-based one (an element of a frame array at a
/// run-time index), or, when `escaped` (some frame object's address was
/// passed to a writable parameter or taken), a destination through an
/// address register other than A5 and SP (a pointer that may point into
/// the frame).
fn may_write_frame(i: &Insn, o: i32, escaped: bool) -> bool {
    let base = match mem_dst(i) {
        Some(Ea::Disp(d, FP)) => return *d < o + 4 && o < *d + 4,
        Some(Ea::Ind(a) | Ea::Disp(_, a) | Ea::Idx(_, a, ..) | Ea::PostInc(a) | Ea::PreDec(a)) => {
            *a
        }
        _ => return false,
    };
    base == FP || (escaped && base != 5 && base != 7)
}

/// Whether a store of `wbytes` bytes at `w` may write the plain variable
/// `l` of `lbytes` bytes ([`Selector::cse_valid`]; codegen-patterns 12.4
/// R7).  `escaped`: some frame object's address escaped
/// ([`escaped_frame_objects`]).
fn store_may_write(w: &Addr, wbytes: u32, l: &Addr, lbytes: u32, escaped: bool) -> bool {
    /// What a write's address is rooted at, and whether its offset is
    /// exact (not an element at a run-time index).
    enum Root<'a> {
        Frame(i32),
        Static(i32),
        Symbol(&'a str, i32),
        Enclosing,
        Pointer,
        Const,
    }
    fn root(a: &Addr) -> (Root<'_>, bool) {
        match a {
            Addr::Frame(o) => (Root::Frame(*o), true),
            Addr::Static(o) => (Root::Static(*o), true),
            Addr::Symbol { name, offset } => (Root::Symbol(name, *offset), true),
            Addr::Enclosing(_) => (Root::Enclosing, true),
            Addr::Indirect { .. } => (Root::Pointer, true),
            Addr::Const(_) => (Root::Const, true),
            Addr::Index { base, .. } => (root(base).0, false),
        }
    }
    let overlap = |a: i32, b: i32| a < b + lbytes as i32 && b < a + wbytes as i32;
    match (root(w), l) {
        ((Root::Frame(p), exact), Addr::Frame(o)) => !exact || overlap(p, *o),
        ((Root::Static(p), exact), Addr::Static(o)) => !exact || overlap(p, *o),
        ((Root::Symbol(n, p), exact), Addr::Symbol { name, offset }) => {
            n == name && (!exact || overlap(p, *offset))
        }
        // Distinct variables: the unit's own data and another unit's
        // object (`extern`, ref 7.2, p. 7-3: allocated elsewhere) do not
        // overlap; nor do two named objects of other units (0xE0A9A6, a
        // store to FIM_$QUIT_INH, does not end the reuse of `PROC1_$AS_ID
        // * 12` at 0xE0A9B0).
        ((Root::Pointer, _), Addr::Frame(_)) => escaped,
        ((Root::Pointer, _), _) => true,
        _ => false,
    }
}

/// The frame offsets at which `f`'s body lets an object's address escape
/// so that it may be written behind the selector's back (bead
/// domain-pascal-pe9; codegen-patterns 12.2): passed by address to a
/// `var`, `out` or `in out` parameter ([`Arg::Address::writable`]), or
/// taken with `addr` ([`Expr::AddrOf`], ref p. 4-13: the pointer may be
/// stored and written through at any later point).  The offset is that of
/// the addressed object's frame root (a field or element of a frame
/// object counts as its object's base offset plus the displacement).
///
/// A value or `in` argument passed by address (the default convention)
/// does not escape: the callee copies a value parameter (ref 5.2, p.
/// 5-4) and may not assign an `in` one (5.3.2, p. 5-7), and the original
/// holds such a parameter in its register across the call (MST_$TOUCH
/// 0xE0DD40: `(8,A6)` in D3 from 0xE0DD4E, `pea (8,A6)` to PEB_$TOUCH at
/// 0xE0DEFC, D3 pushed at 0xE0E07A; AST_$PMAP_ASSOC 0xE0440A `pea
/// (0xe,A6)` to MMAP_$INSTALL_LIST, D3 stored at 0xE04414; PROC1_$GET_INFO
/// from 0xE40972, ASKNODE_$INTERNET_INFO from 0xE33688, ...).
fn escaped_frame_objects(f: &Function) -> Vec<i32> {
    fn root(a: &Addr) -> Option<i32> {
        match a {
            Addr::Frame(o) => Some(*o),
            Addr::Index { base, .. } => root(base),
            // Another frame (the enclosing routine's objects are never
            // held in a register, [`Function::uplevel_refs`]), the unit's
            // data, or the object a pointer points to.
            Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::Symbol { .. }
            | Addr::Indirect { .. }
            | Addr::Const(_) => None,
        }
    }
    let mut out = Vec::new();
    walk(&f.body, &mut |n| match n {
        Node::Call(c) => {
            for a in &c.args {
                if let Arg::Address {
                    addr,
                    writable: true,
                    ..
                } = a
                {
                    out.extend(root(addr));
                }
            }
        }
        Node::Expr(Expr::AddrOf(a)) => out.extend(root(a)),
        Node::Expr(_) | Node::Addr(_) => {}
    });
    out
}

/// Whether `e` reads the frame object at `off` as a value.
fn expr_reads_frame(e: &Expr, off: i32) -> bool {
    let mut hit = false;
    walk_expr(e, &mut |n| {
        if let Node::Expr(Expr::Load {
            addr: Addr::Frame(o),
            ..
        }) = n
        {
            hit |= *o == off;
        }
    });
    hit
}

/// Whether `body` calls a routine (a statement or a function call, or an
/// M$ARITH entry: a `jsr` either way).
fn stmts_call(body: &[Stmt]) -> bool {
    let mut hit = false;
    walk(body, &mut |n| match n {
        Node::Call(_) => hit = true,
        Node::Expr(e) => hit |= expr_has_call(e),
        Node::Addr(_) => {}
    });
    hit
}

/// An array a `for` loop reaches through a pointer stepped by the element
/// size (codegen-patterns 7.4, "Array indexing by the index"): the
/// array's address (`base`, an [`Addr::Indirect`] at offset 0) and the
/// element size.
#[derive(Clone, Debug, PartialEq)]
struct SteppedArray {
    base: Addr,
    scale: u32,
}

/// Visit the expressions, addresses and calls of `body` in evaluation
/// order: an assignment's value before its destination, a copy's source
/// before its destination ([`walk`] visits destinations first).
fn walk_eval<'a>(body: &'a [Stmt], v: &mut dyn FnMut(Node<'a>)) {
    for st in body {
        match &st.kind {
            StmtKind::Store { addr, value } => {
                walk_expr(value, v);
                walk_addr(addr, v);
            }
            StmtKind::Copy { dst, src, .. } => {
                walk_addr(src, v);
                walk_addr(dst, v);
            }
            StmtKind::If { cond, then_, else_ } => {
                walk_cond(cond, v);
                walk_eval(then_, v);
                walk_eval(else_, v);
            }
            StmtKind::Repeat { body, until } => {
                walk_eval(body, v);
                walk_cond(until, v);
            }
            StmtKind::While { cond, body } => {
                walk_cond(cond, v);
                walk_eval(body, v);
            }
            StmtKind::For(l) => {
                walk_expr(&l.from, v);
                walk_expr(&l.to, v);
                walk_eval(&l.body, v);
            }
            StmtKind::Case {
                selector,
                arms,
                otherwise,
            } => {
                walk_expr(selector, v);
                for a in arms {
                    walk_eval(&a.body, v);
                }
                if let Some(o) = otherwise {
                    walk_eval(o, v);
                }
            }
            _ => walk(std::slice::from_ref(st), v),
        }
    }
}

/// Whether `index` is the plain `for` index at frame offset `off`.
fn is_index(index: &Expr, off: i32) -> bool {
    matches!(index, Expr::Load { addr: Addr::Frame(o), ty } if *o == off && ty.width == Width::Word)
}

/// The arrays of elements wider than a byte that `body` indexes by the
/// `for` index at frame offset `off`, in the order the pointers are formed
/// (codegen-patterns 7.4): the order of first use in evaluation order, so
/// the source of `a[i] := b[i]` first (LOG_$READ's worker 0xE177E0 `lea
/// (2,A1),A0` for the source, then `lea (2,A2),A1`; 0xE4F010).  Byte
/// arrays are indexed with the index register itself (83 loops: `move.b`,
/// `cmpi.b`, `lea` with `(d,An,Di.w)`; KBD's module-local 0xE1CA70 `cmp.b
/// (0,A5,D1.w),D2`) and are not listed.
///
/// `Err` (a message for "... is not supported yet") when an element is
/// reached in a way not implemented:
///
/// - an index that is an expression of the loop index (`a[i + 1]`), or an
///   array whose address depends on it;
/// - an array in the unit's data or in the frame: the pointer is formed
///   with `lea (d,A5)` (PKT_$RECENTLY_MISSING 0xE128D6 `lea (8,A5),A0`)
///   or `movea.l A5,An; addq.l #d,An` (0xE4296E, 0xE5142E, 0xE4E478), with
///   `lea (d,A6)` (0xE4EB06) or `movea.l A6,An; addq.l #d,An` (0xE427DE),
///   both in one preheader at 0xE134DE: no rule found; another unit's
///   array: `movea.l #abs,A0; lea (12,A0),A0` (HINT_$INIT_CACHE 0xE313DA),
///   the address split between the immediate and the displacements not
///   catalogued (section 5.2);
/// - an array at a non-zero offset from its pointer (`p^.f[i]`: whether
///   the offset goes into the pointer or the displacements is not seen;
///   the sites looked at all have offset 0);
/// - an array whose pointer is not a variable that the body leaves
///   unchanged (a frame object never assigned or addressed in the body;
///   a variable of the unit or of another unit when the body neither
///   calls nor assigns it; a frame object a nested routine reaches,
///   `uplevel` ([`Function::uplevel_refs`]), only when the body makes no
///   call: a nested routine called in the body may assign it).
fn stepped_arrays(
    body: &[Stmt],
    off: i32,
    uplevel: &[(i32, u32)],
) -> Result<Vec<SteppedArray>, &'static str> {
    let mut out: Vec<SteppedArray> = Vec::new();
    let mut err = None;
    walk_eval(body, &mut |n| {
        let Node::Addr(Addr::Index {
            base, index, scale, ..
        }) = n
        else {
            return;
        };
        if !expr_reads_frame(index, off) && !addr_reads_frame(base, off) {
            return;
        }
        if !is_index(index, off) || addr_reads_frame(base, off) {
            err.get_or_insert(
                "an element at an index computed from a `for` index (strength reduction, \
                 codegen-patterns 7.4) is",
            );
            return;
        }
        if *scale == 1 {
            return;
        }
        let a = SteppedArray {
            base: (**base).clone(),
            scale: *scale,
        };
        if !out.contains(&a) {
            out.push(a);
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    for a in &out {
        let Addr::Indirect { base: p, offset } = &a.base else {
            return Err(
                "a `for` loop stepping a pointer through an array not reached through a \
                 pointer (in the unit's data or the frame: `lea (d,A5)` or `movea.l A5,An; \
                 addq.l #d,An`, no rule found; another unit's: `movea.l #abs,An`, address \
                 split not catalogued; codegen-patterns 7.4) is",
            );
        };
        if *offset != 0 {
            return Err(
                "a `for` loop stepping a pointer through an array at an offset from its \
                 pointer (codegen-patterns 7.4) is",
            );
        }
        if !loop_invariant_pointer(p, body, uplevel) {
            return Err(
                "a `for` loop stepping a pointer through an array whose pointer is not a \
                 variable the body leaves unchanged (codegen-patterns 7.4) is",
            );
        }
    }
    Ok(out)
}

/// Whether `a` reads the frame object at `off` as a value.
fn addr_reads_frame(a: &Addr, off: i32) -> bool {
    let mut hit = false;
    walk_addr(a, &mut |n| {
        if let Node::Expr(Expr::Load {
            addr: Addr::Frame(o),
            ..
        }) = n
        {
            hit |= *o == off;
        }
    });
    hit
}

/// Whether pointer `p` is a variable `body` leaves unchanged
/// ([`stepped_arrays`]).
fn loop_invariant_pointer(p: &Expr, body: &[Stmt], uplevel: &[(i32, u32)]) -> bool {
    let Expr::Load { addr, ty } = p else {
        return false;
    };
    let size = ty.bytes() as i32;
    // Whether an object at `a` of `len` bytes (`None`: not known, an
    // address taken) may overlap the pointer: a record holding it assigned
    // or addressed as a whole counts.
    let hits = |a: &Addr, len: Option<i32>| match (a, addr) {
        (Addr::Frame(x), Addr::Frame(o)) | (Addr::Static(x), Addr::Static(o)) => {
            *x < o + size && len.is_none_or(|n| *o < x + n)
        }
        (Addr::Symbol { name: n, .. }, Addr::Symbol { name, .. }) => n == name,
        _ => false,
    };
    if !matches!(addr, Addr::Frame(_) | Addr::Static(_) | Addr::Symbol { .. }) {
        return false;
    }
    let reached = |o: i32| {
        uplevel
            .iter()
            .any(|&(u, n)| u < o + size && o < u + n as i32)
    };
    if !matches!(addr, Addr::Frame(o) if !reached(*o)) && stmts_call(body) {
        return false;
    }
    let mut ok = true;
    walk(body, &mut |n| match n {
        Node::Expr(Expr::AddrOf(a)) => ok &= !hits(a, None),
        Node::Call(c) => {
            for a in &c.args {
                if let Arg::Address { addr: a, .. } = a {
                    ok &= !hits(a, None);
                }
            }
        }
        _ => {}
    });
    dpc_ir::visit_stmts(body, &mut |st| match &st.kind {
        StmtKind::Store { addr: a, value } => ok &= !hits(a, Some(value.ty().bytes() as i32)),
        StmtKind::Copy { dst: a, bytes, .. } => ok &= !hits(a, Some(*bytes as i32)),
        StmtKind::Fill {
            start: a,
            count,
            value,
        } => ok &= !hits(a, Some((*count * value.ty().bytes()) as i32)),
        _ => {}
    });
    ok
}

/// Whether `body` reads the `for` index at `off` other than as the index
/// of the arrays its loop steps pointers through ([`stepped_arrays`]).
fn index_read_unstepped(body: &[Stmt], off: i32) -> bool {
    let (mut reads, mut stepped) = (0, 0);
    walk(body, &mut |n| match n {
        Node::Expr(Expr::Load {
            addr: Addr::Frame(o),
            ..
        }) if *o == off => reads += 1,
        Node::Addr(Addr::Index { index, scale, .. }) if *scale > 1 && is_index(index, off) => {
            stepped += 1
        }
        _ => {}
    });
    reads > stepped
}

/// The pointer locals of `f` held in an address register for the whole
/// body, assigned inside a loop (codegen-patterns 7.4, 12.5): a list walk's
/// pointer.
///
/// The DISK_ module-local 0xE3CCCE (`disk/write_multi.c`,
/// `disk_$set_chain_status`): `movea.l D0,A1` (the walk pointer from the
/// parameter loaded at entry), then in the loop `move.l (A0),(0xc,A1);
/// movea.l (A1),A1` and the test `cmpa.w #0,A1; bne.s`; its frame
/// (`link #-8`) is never addressed.  Of the `while` loops that test a
/// pointer against `nil` at the bottom, 45 test an address register
/// (`cmpa.w #0,An`) and 12 a frame slot (`tst.l (d,A6)`; survey 7.4).
///
/// The rule (**Uncertain**, fitted to 0xE3CCCE): a 4-byte unsigned local
/// (`Scalar::POINTER`) that the body dereferences, whose only references
/// are whole reads, dereferences and whole assignments, whose first
/// reference is an assignment of a non-constant value by a statement of
/// the routine's own statement part, and which is assigned again inside a
/// loop, in a body without calls (where it would have to live in A2..A4
/// is not looked at).  `exclude`: locals already held by the other rule.
fn loop_pointer_locals(f: &Function, exclude: &[i32]) -> Vec<i32> {
    if stmts_call(&f.body) {
        return Vec::new();
    }
    #[derive(Default)]
    struct St {
        /// The first reference was a suitable assignment (`Some(true)`),
        /// or something else.
        first: Option<bool>,
        deref: bool,
        loop_store: bool,
        bad: bool,
    }
    fn exprs(e: &Expr, off: i32, st: &mut St) {
        let (mut loads, mut addrs) = (0, 0);
        walk_expr(e, &mut |n| match n {
            Node::Expr(Expr::Load {
                addr: Addr::Frame(o),
                ty,
            }) if *o == off => {
                loads += 1;
                if *ty != Scalar::POINTER {
                    st.bad = true;
                }
            }
            Node::Addr(Addr::Indirect { base, .. }) => {
                if matches!(&**base, Expr::Load { addr: Addr::Frame(o), .. } if *o == off) {
                    st.deref = true;
                }
            }
            Node::Addr(Addr::Frame(o)) if *o < off + 4 && off < *o + 4 => addrs += 1,
            _ => {}
        });
        if addrs != loads {
            // Addressed, or read in part.
            st.bad = true;
        }
        if loads > 0 && st.first.is_none() {
            st.first = Some(false);
        }
    }
    fn cond(c: &IrCond, off: i32, st: &mut St) {
        match c {
            IrCond::Compare { lhs, rhs, .. } => {
                exprs(lhs, off, st);
                exprs(rhs, off, st);
            }
            IrCond::Test(e) => exprs(e, off, st),
            IrCond::Not(x) => cond(x, off, st),
            IrCond::AndThen(a, b) | IrCond::OrElse(a, b) => {
                cond(a, off, st);
                cond(b, off, st);
            }
        }
    }
    fn addr(a: &Addr, off: i32, st: &mut St) {
        exprs(&Expr::AddrOf(a.clone()), off, st);
    }
    fn stmts(body: &[Stmt], off: i32, top: bool, in_loop: bool, st: &mut St) {
        for s in body {
            match &s.kind {
                StmtKind::Store {
                    addr: Addr::Frame(o),
                    value,
                } if *o == off => {
                    exprs(value, off, st);
                    if value.ty() != Scalar::POINTER {
                        st.bad = true;
                    }
                    if st.first.is_none() {
                        st.first = Some(top && !matches!(value, Expr::Const { .. }));
                    }
                    st.loop_store |= in_loop;
                }
                StmtKind::Store { addr: a, value } => {
                    addr(a, off, st);
                    exprs(value, off, st);
                }
                StmtKind::SetResult(e) | StmtKind::Discard(e) => exprs(e, off, st),
                StmtKind::Copy { dst, src, .. } => {
                    addr(src, off, st);
                    addr(dst, off, st);
                }
                StmtKind::Call(_) => st.bad = true,
                StmtKind::Fill { start, value, .. } => {
                    addr(start, off, st);
                    exprs(value, off, st);
                }
                StmtKind::If {
                    cond: c,
                    then_,
                    else_,
                } => {
                    cond(c, off, st);
                    stmts(then_, off, false, in_loop, st);
                    stmts(else_, off, false, in_loop, st);
                }
                StmtKind::Repeat { body, until } => {
                    stmts(body, off, false, true, st);
                    cond(until, off, st);
                }
                StmtKind::While { cond: c, body } => {
                    cond(c, off, st);
                    stmts(body, off, false, true, st);
                }
                StmtKind::For(l) => {
                    exprs(&l.from, off, st);
                    exprs(&l.to, off, st);
                    stmts(&l.body, off, false, true, st);
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => {
                    exprs(selector, off, st);
                    for a in arms {
                        stmts(&a.body, off, false, in_loop, st);
                    }
                    if let Some(o) = otherwise {
                        stmts(o, off, false, in_loop, st);
                    }
                }
                StmtKind::Return
                | StmtKind::Disable
                | StmtKind::Enable
                | StmtKind::Exit
                | StmtKind::Next => {}
            }
        }
    }
    f.scalar_locals
        .iter()
        .filter(|l| l.ty == Scalar::POINTER && !exclude.contains(&l.offset))
        .filter(|l| {
            let mut st = St::default();
            stmts(&f.body, l.offset, true, false, &mut st);
            !st.bad && st.first == Some(true) && st.deref && st.loop_store
        })
        .map(|l| l.offset)
        .collect()
}

/// The pointer locals of `f` assigned from pointer functions' results and
/// held in an address register for the whole body, like the loop pointer
/// locals of [`loop_pointer_locals`] (codegen-patterns 3, "Callers of
/// pointer functions", and 12.5, "Pointer locals assigned from calls";
/// bead domain-pascal-500).
///
/// Of the 234 calls of A0 functions whose caller reads A0 next
/// (`tools/resultsurvey.py callers`, `ptrlocals`), 108 copy it to an address register
/// with `movea.l A0,An` (A2 57, A3 41, A4 7, A1 3): KBD_$SET_KBD_TYPE
/// 0xE7253E `movea.l A0,A2`, then `pea (A2)` for the descriptor passed by
/// address (0xE72550); AST_$LOAD_AOTE's family 0xE02F56 `movea.l A0,A3;
/// cmpa.w #0,A3`.  None of those registers is ever pushed by value
/// (`move.l An,-(SP)`), while 8 of the 28 locals stored to a frame slot
/// instead (12 of the 34 stores) are pushed by value later (AST_$RESERVE
/// 0xE067BA, whose `aote` is passed to `ast_$lookup_aste`; AST_$TRUNCATE
/// 0xE05CC8, AST_$PURIFY 0xE056D8, VTOC_$LOOKUP 0xE39056), 2 (4 stores)
/// are reached by a nested routine (AST_$INVALIDATE 0xE0666A, the AST_
/// module-local 0xE05214 at 0xE0528C; `ast/invalidate.c`,
/// `ast/set_attribute_internal.c`), 5 hold a routine pointer that is
/// called (NET_$OPEN 0xE5A1D8 and its siblings), the rest
/// are result variables (MST_$MAPS 0xE439C8) or not explained (DIR_
/// module-locals 0xE4DC74, 0xE53B02).
///
/// The rule (**Uncertain** beyond those sites; the narrowest that fits
/// them): a 4-byte pointer local whose only references are whole reads,
/// dereferences (a field read or written, the object passed by address:
/// `pea (An)`) and whole assignments; every assignment's value a call of a
/// pointer function; the first reference an assignment; never read whole
/// inside an argument of a call (pushed by value), never addressed (passed
/// by address, `addr`), never reached by a nested routine
/// ([`Function::uplevel_refs`]; bead domain-pascal-pe9).  Every assignment
/// writes the register (`movea.l A0,An`), the allocator gives it a
/// callee-saved register when it lives across a call (12.4 R3), and a
/// call whose arguments use it last holds it through the call
/// ([`held_through_calls`]).  One that no call follows is refused after
/// allocation (D0 or A1 in the image, [`select_body_with`]).  Pointer
/// locals assigned otherwise (`nil`, a memory load) stay in the frame as
/// before (not surveyed).
fn call_pointer_locals(f: &Function, exclude: &[i32]) -> Vec<i32> {
    #[derive(Default)]
    struct St {
        /// The first reference was an assignment (`Some(true)`), or
        /// something else.
        first: Option<bool>,
        bad: bool,
    }
    /// Whether `e` reads the local at `off` whole (not as a pointer it
    /// dereferences).
    fn whole_read(e: &Expr, off: i32) -> bool {
        let (mut loads, mut derefs) = (0, 0);
        walk_expr(e, &mut |n| match n {
            Node::Expr(Expr::Load {
                addr: Addr::Frame(o),
                ..
            }) if *o == off => loads += 1,
            Node::Addr(Addr::Indirect { base, .. }) if matches!(&**base, Expr::Load { addr: Addr::Frame(o), .. } if *o == off) => {
                derefs += 1
            }
            _ => {}
        });
        loads > derefs
    }
    /// The references to the local at `off` among the nodes `go` visits.
    fn scan<'a>(off: i32, st: &mut St, go: impl FnOnce(&mut dyn FnMut(Node<'a>))) {
        let (mut loads, mut addrs) = (0, 0);
        let mut bad = false;
        go(&mut |n| match n {
            Node::Expr(Expr::Load {
                addr: Addr::Frame(o),
                ty,
            }) if *o == off => {
                loads += 1;
                bad |= *ty != Scalar::POINTER;
            }
            Node::Addr(Addr::Frame(o)) if *o < off + 4 && off < *o + 4 => addrs += 1,
            Node::Call(c) => {
                for a in &c.args {
                    if let Arg::Value { value, .. } | Arg::Temp { value, .. } = a {
                        bad |= whole_read(value, off);
                    }
                }
            }
            _ => {}
        });
        // Addressed, or read in part.
        st.bad |= bad || addrs != loads;
        if loads > 0 && st.first.is_none() {
            st.first = Some(false);
        }
    }
    fn cond(c: &IrCond, off: i32, st: &mut St) {
        scan(off, st, |v| walk_cond(c, v));
    }
    fn stmts(body: &[Stmt], off: i32, st: &mut St) {
        for s in body {
            match &s.kind {
                StmtKind::Store {
                    addr: Addr::Frame(o),
                    value,
                } if *o == off => {
                    scan(off, st, |v| walk_expr(value, v));
                    if value.ty() != Scalar::POINTER || a0_call(value).is_none() {
                        st.bad = true;
                    }
                    if st.first.is_none() {
                        st.first = Some(true);
                    }
                }
                StmtKind::If {
                    cond: c,
                    then_,
                    else_,
                } => {
                    cond(c, off, st);
                    stmts(then_, off, st);
                    stmts(else_, off, st);
                }
                StmtKind::Repeat { body, until } => {
                    stmts(body, off, st);
                    cond(until, off, st);
                }
                StmtKind::While { cond: c, body } => {
                    cond(c, off, st);
                    stmts(body, off, st);
                }
                StmtKind::For(l) => {
                    scan(off, st, |v| {
                        walk_expr(&l.from, v);
                        walk_expr(&l.to, v);
                    });
                    stmts(&l.body, off, st);
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => {
                    scan(off, st, |v| walk_expr(selector, v));
                    for a in arms {
                        stmts(&a.body, off, st);
                    }
                    if let Some(o) = otherwise {
                        stmts(o, off, st);
                    }
                }
                _ => scan(off, st, |v| walk(std::slice::from_ref(s), v)),
            }
        }
    }
    f.scalar_locals
        .iter()
        .filter(|l| l.ty == Scalar::POINTER && !exclude.contains(&l.offset))
        .filter(|l| {
            !f.uplevel_refs
                .iter()
                .any(|&(p, n)| p < l.offset + 4 && l.offset < p + n as i32)
        })
        .filter(|l| {
            let mut st = St::default();
            stmts(&f.body, l.offset, &mut st);
            !st.bad && st.first == Some(true)
        })
        .map(|l| l.offset)
        .collect()
}

/// Whether `stmts` ends in an unconditional jump (`return`, `exit`,
/// `next`), so that no `bra` to a join follows it: the `bra` would never
/// be reached.
fn ends_in_jump(stmts: &[Stmt]) -> bool {
    matches!(
        stmts.last().map(|s| &s.kind),
        Some(StmtKind::Return | StmtKind::Exit | StmtKind::Next)
    )
}

/// A statement that is not control flow.
fn is_simple(st: &Stmt) -> bool {
    !matches!(
        st.kind,
        StmtKind::If { .. }
            | StmtKind::Repeat { .. }
            | StmtKind::While { .. }
            | StmtKind::For(_)
            | StmtKind::Case { .. }
            | StmtKind::Return
            | StmtKind::Exit
            | StmtKind::Next
    )
}

/// What follows a statement sequence up to the epilogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tail {
    /// Nothing is emitted between the end of the sequence and the
    /// epilogue (the routine's own statement part, or an `if` arm that
    /// falls through to the end of the routine).
    end: bool,
    /// Every path from the end of the sequence to the epilogue is free of
    /// pushes, calls and conditional branches (`dbf` included); an
    /// unconditional `bra` to a join may occur.  The pop rule depends on it
    /// ([`Selector::call`]).
    clean: bool,
}

impl Tail {
    const END: Tail = Tail {
        end: true,
        clean: true,
    };
}

/// Whether the code of `stmts` and then a continuation that is clean when
/// `tail_clean` is free of pushes, calls and conditional branches up to the
/// epilogue (see [`Tail::clean`]).  A `return` ends the path: what follows
/// it, the continuation included, is not reached.
fn seq_clean(stmts: &[Stmt], tail_clean: bool) -> bool {
    for st in stmts {
        match &st.kind {
            StmtKind::Return => return true,
            // A function call inside the value pushes and calls.
            StmtKind::SetResult(e) | StmtKind::Store { value: e, .. } if expr_has_call(e) => {
                return false
            }
            StmtKind::SetResult(_) | StmtKind::Store { .. } => {}
            StmtKind::Discard(e) if expr_has_call(e) => return false,
            StmtKind::Discard(_) | StmtKind::Disable | StmtKind::Enable => {}
            // Up to 18 bytes the copy is unrolled; from 20 it is a `dbf`
            // loop (codegen-patterns 5.2).
            StmtKind::Copy { bytes, .. } if *bytes <= 18 => {}
            // `exit` and `next` go on inside or after a loop, whose test
            // or `dbf` branches conditionally (conservatively not clean).
            StmtKind::Copy { .. }
            | StmtKind::Fill { .. }
            | StmtKind::Call(_)
            | StmtKind::If { .. }
            | StmtKind::Repeat { .. }
            | StmtKind::While { .. }
            | StmtKind::For(_)
            | StmtKind::Exit
            | StmtKind::Next
            | StmtKind::Case { .. } => return false,
        }
    }
    tail_clean
}

/// Whether evaluating `e` calls a function, an M$ARITH entry included, or
/// branches conditionally (a signed `div` by a power of two,
/// [`Selector::div_pow2`]).
fn expr_has_call(e: &Expr) -> bool {
    match e {
        Expr::Const { .. } | Expr::AddrOf(_) => false,
        Expr::Load { addr, .. } => addr_has_call(addr),
        Expr::SetMember { elem, set, .. } => expr_has_call(elem) || expr_has_call(set),
        Expr::Binary { op, lhs, rhs, ty } => {
            arith_calls(*op, lhs, rhs, *ty, false) || expr_has_call(lhs) || expr_has_call(rhs)
        }
        Expr::Convert {
            value: x,
            ty: Scalar {
                width: Width::Word, ..
            },
        } if matches!(
            **x,
            Expr::Binary {
                op: BinOp::Div | BinOp::Mod,
                ty: Scalar {
                    width: Width::Long,
                    ..
                },
                ..
            }
        ) =>
        {
            let Expr::Binary { op, lhs, rhs, ty } = &**x else {
                unreachable!()
            };
            arith_calls(*op, lhs, rhs, *ty, true) || expr_has_call(lhs) || expr_has_call(rhs)
        }
        Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
            expr_has_call(x)
        }
        Expr::Call { .. } => true,
        Expr::Test(c) => cond_has_call(c),
    }
}

/// Whether the selector computes `lhs op rhs` of type `ty` with an
/// M$ARITH call or a conditional branch (the choices of [`Selector::mul`]
/// and [`Selector::div`]); `narrowed`: a longword quotient used as a word.
fn arith_calls(op: BinOp, lhs: &Expr, rhs: &Expr, ty: Scalar, narrowed: bool) -> bool {
    let konst = |e: &Expr| match e {
        Expr::Const { value, .. } => Some(*value),
        _ => None,
    };
    let word = |e: &Expr| Selector::word_part(e, ty.sign).is_some();
    let long = ty.width == Width::Long;
    match op {
        BinOp::Mul if long => match (konst(lhs), konst(rhs)) {
            (_, Some(c)) | (Some(c), None) => {
                let x = if konst(rhs).is_some() { lhs } else { rhs };
                !(by_shifts(c) || (word(x) && word(&Expr::Const { value: c, ty })))
            }
            (None, None) => !(word(lhs) && word(rhs)),
        },
        BinOp::Div => {
            let pow2 = konst(rhs).is_some_and(|c| c > 1 && c & (c - 1) == 0);
            if pow2 {
                // `bpl` when signed; `lsr.l` (or refused) when not.
                return ty.is_signed();
            }
            long && !(word(lhs) && word(rhs)) && !(narrowed && word(rhs))
        }
        BinOp::Mod => long && !(word(lhs) && word(rhs)),
        _ => false,
    }
}

fn addr_has_call(a: &Addr) -> bool {
    match a {
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::Static(_)
        | Addr::Symbol { .. }
        | Addr::Const(_) => false,
        Addr::Indirect { base, .. } => expr_has_call(base),
        Addr::Index { base, index, .. } => addr_has_call(base) || expr_has_call(index),
    }
}

fn cond_has_call(c: &IrCond) -> bool {
    match c {
        IrCond::Compare { lhs, rhs, .. } => expr_has_call(lhs) || expr_has_call(rhs),
        IrCond::Test(e) => expr_has_call(e),
        IrCond::Not(x) => cond_has_call(x),
        IrCond::AndThen(a, b) | IrCond::OrElse(a, b) => cond_has_call(a) || cond_has_call(b),
    }
}

/// The argument bytes the caller pops after `st`, when `st` is a call
/// that pushes any.
fn call_bytes(st: Option<&Stmt>) -> Option<u32> {
    match st.map(|s| &s.kind) {
        Some(StmtKind::Call(c)) if c.bytes() > 0 => Some(c.bytes()),
        _ => None,
    }
}

/// Ref 3.15.3, p. 3-57 ("Device"): "By default, the compiler optimizes
/// certain adjacent references by merging them into one large reference"
/// (`a := 0; b := 0` on two `integer16`s becomes one 32-bit store).  Which
/// references it merged has not been catalogued: the `clr.b` of the
/// adjacent bytes 12 and 13 are *not* merged in XPD_$RESET_PTRACE_OPTS
/// (0xE5B15E, 0xE5B164), but they are not consecutive in the image
/// (offsets 12, 0, 13, 4, 8) and the source order is not known.  So two consecutive
/// stores to adjacent objects of the same base that fit together in a
/// word or longword are rejected rather than guessed at; a byte and a word
/// (3 bytes) cannot be one 68000 reference and are kept apart, as at
/// module-local 0xE32B76, 0xE32BA6/0xE32BAA (`clr.b (0x36,A2); move.w
/// D2,(0x34,A2)`, a byte and the word below it).  TIME_$Q_INIT_QUEUE's
/// stores (0..4, 4..8, 8, 10..12) never fit: 0xE16C6E..0xE16C78.  Each
/// statement sequence (a body, an `if` arm, a loop body) is checked.
fn reject_mergeable_stores(body: &[Stmt]) -> Res<()> {
    let mut prev: Option<(&Addr, u32)> = None;
    for st in body {
        match &st.kind {
            StmtKind::If { then_, else_, .. } => {
                reject_mergeable_stores(then_)?;
                reject_mergeable_stores(else_)?;
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                reject_mergeable_stores(body)?
            }
            StmtKind::For(l) => reject_mergeable_stores(&l.body)?,
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms {
                    reject_mergeable_stores(&a.body)?;
                }
                if let Some(o) = otherwise {
                    reject_mergeable_stores(o)?;
                }
            }
            _ => {}
        }
        let StmtKind::Store { addr, value } = &st.kind else {
            prev = None;
            continue;
        };
        let bytes = value.ty().bytes();
        if let Some((pa, pb)) = prev {
            if let (Some((k1, o1)), Some((k2, o2))) = (split_addr(pa), split_addr(addr)) {
                let adjacent = o1 + pb as i32 == o2 || o2 + bytes as i32 == o1;
                if k1 == k2 && adjacent && matches!(pb + bytes, 2 | 4) {
                    return Err(unsupported(
                        st.span,
                        "a store adjacent to the previous statement's store (the original \
                         compiler may merge them, ref 3.15.3 p. 3-57) is",
                    ));
                }
            }
        }
        prev = Some((addr, bytes));
    }
    Ok(())
}

/// The base of `addr` (with offset 0) and the offset.
fn split_addr(addr: &Addr) -> Option<(Addr, i32)> {
    match addr {
        Addr::Frame(o) => Some((Addr::Frame(0), *o)),
        Addr::Enclosing(o) => Some((Addr::Enclosing(0), *o)),
        Addr::Static(o) => Some((Addr::Static(0), *o)),
        Addr::Symbol { name, offset } => Some((
            Addr::Symbol {
                name: name.clone(),
                offset: 0,
            },
            *offset,
        )),
        Addr::Indirect { base, offset } => Some((
            Addr::Indirect {
                base: base.clone(),
                offset: 0,
            },
            *offset,
        )),
        Addr::Const(_) => None,
        // Two elements of one array at run-time indices: adjacency is not
        // known at compile time.
        Addr::Index { .. } => None,
    }
}

/// How the body uses a by-value parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamUse {
    /// Its value is read as data.
    Data,
    /// It is a pointer the body dereferences (`p^`, `p^.f`).
    Base,
}

/// Every use of a by-value parameter in `f`'s body, as (frame offset,
/// use), in body order.
fn param_uses(f: &Function) -> Vec<(i32, ParamUse)> {
    let mut u = Uses {
        f,
        out: Vec::new(),
        stepped: Vec::new(),
    };
    u.stmts(&f.body);
    u.out
}

/// The walk of [`param_uses`].
struct Uses<'a> {
    f: &'a Function,
    out: Vec<(i32, ParamUse)>,
    /// The indices of the `for` loops being walked that step pointers
    /// through their arrays ([`stepped_arrays`]).
    stepped: Vec<i32>,
}

impl Uses<'_> {
    fn is_param(&self, o: i32) -> bool {
        self.f.value_params.iter().any(|p| p.offset == o)
    }

    fn is_slot(&self, o: i32) -> bool {
        self.f.address_params.contains(&o)
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if self.is_param(*o) => self.out.push((*o, ParamUse::Data)),
            Expr::Load { addr, .. } | Expr::AddrOf(addr) => self.address(addr),
            Expr::SetMember { elem, set, .. } => {
                self.expr(elem);
                self.expr(set);
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
                self.expr(x)
            }
            Expr::Call { call, .. } => self.call_args(call),
            Expr::Test(c) => self.cond(c),
            Expr::Const { .. } => {}
        }
    }

    fn address(&mut self, a: &Addr) {
        match a {
            Addr::Indirect { base, .. } => match &**base {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } if self.is_param(*o) || self.is_slot(*o) => self.out.push((*o, ParamUse::Base)),
                b => self.expr(b),
            },
            // An element reached through a stepped pointer: the array's
            // pointer is used once, before the loop (below, `For`).
            Addr::Index { index, scale, .. }
                if *scale > 1 && self.stepped.iter().any(|&o| is_index(index, o)) => {}
            Addr::Index { base, index, .. } => {
                self.address(base);
                self.expr(index);
            }
            Addr::Frame(_)
            | Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::Symbol { .. }
            | Addr::Const(_) => {}
        }
    }

    fn call_args(&mut self, c: &Call) {
        for a in &c.args {
            match a {
                // A by-value parameter that is only pushed as an argument is
                // pushed from the frame, not loaded at entry:
                // AST_$SET_ATTRIBUTE 0xE053DA `move.l (0xe,A6),-(SP)`;
                // 0xE00EBE and 0xE00EDE push (8,A6) twice (codegen-patterns
                // 12.2).
                Arg::Value {
                    value:
                        Expr::Load {
                            addr: Addr::Frame(_),
                            ..
                        },
                    ..
                } => {}
                Arg::Value { value, .. } | Arg::Temp { value, .. } => self.expr(value),
                Arg::Address { addr, .. } => self.address(addr),
            }
        }
    }

    fn cond(&mut self, c: &IrCond) {
        match c {
            IrCond::Compare { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            IrCond::Test(e) => self.expr(e),
            IrCond::Not(x) => self.cond(x),
            IrCond::AndThen(a, b) | IrCond::OrElse(a, b) => {
                self.cond(a);
                self.cond(b);
            }
        }
    }

    fn stmts(&mut self, body: &[Stmt]) {
        for st in body {
            match &st.kind {
                StmtKind::If { cond, then_, else_ } => {
                    self.cond(cond);
                    self.stmts(then_);
                    self.stmts(else_);
                }
                // A use inside a loop body is a use on every iteration: a
                // by-address slot used once there is loaded at entry like
                // one used more than once (0xE3CCD6 `movea.l (0xc,A6),A0`,
                // the status slot of a `while` loop's one store;
                // NAME_$VALIDATE's module-local 0xE4A004: the slots (0x10)
                // and (0x18), used in its loops, at 0xE4A018/0xE4A01C, but
                // (0x14), used once outside them, at 0xE4A03C; 0xE49E5C,
                // 0xE49E60), so the body is walked twice.
                StmtKind::Repeat { body, until } => {
                    for _ in 0..2 {
                        self.stmts(body);
                        self.cond(until);
                    }
                }
                StmtKind::While { cond, body } => {
                    for _ in 0..2 {
                        self.cond(cond);
                        self.stmts(body);
                    }
                }
                // The pointers of the arrays a `for` loop steps through
                // are loaded once, before it (codegen-patterns 7.4;
                // LOG_$READ's worker loads `buffer`, a by-address slot,
                // at 0xE177CC, not at entry).
                StmtKind::For(l) => {
                    self.expr(&l.from);
                    self.expr(&l.to);
                    let arrays = stepped_arrays(&l.body, l.index.offset, &self.f.uplevel_refs);
                    if let Ok(arrays) = &arrays {
                        for a in arrays {
                            self.address(&a.base);
                        }
                        self.stepped.push(l.index.offset);
                    }
                    for _ in 0..2 {
                        self.stmts(&l.body);
                    }
                    if arrays.is_ok() {
                        self.stepped.pop();
                    }
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => {
                    self.expr(selector);
                    for a in arms {
                        self.stmts(&a.body);
                    }
                    if let Some(o) = otherwise {
                        self.stmts(o);
                    }
                }
                StmtKind::Return
                | StmtKind::Disable
                | StmtKind::Enable
                | StmtKind::Exit
                | StmtKind::Next => {}
                StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e),
                StmtKind::Store { addr, value } => {
                    self.address(addr);
                    self.expr(value);
                }
                StmtKind::Copy { dst, src, .. } => {
                    self.address(src);
                    self.address(dst);
                }
                StmtKind::Call(c) => self.call_args(c),
                StmtKind::Fill { start, value, .. } => {
                    self.address(start);
                    self.expr(value);
                }
            }
        }
    }
}

/// Selection state of one routine body.
pub struct Selector {
    out: Vec<Insn>,
    /// Virtual registers handed out so far (`regs.rs`).
    vregs: VRegs,
    /// Callee-saved registers the body writes outside the allocator (A5,
    /// [`Self::load_data_base`]).
    clobbered: RegSet,
    /// Indices in `out` of the moves of a function result into D0
    /// ([`Self::set_result`]): dropped when the value is already there.
    result_moves: Vec<usize>,
    /// Ranges of `out` the scheduler moves as one node (`sched.rs`,
    /// codegen-patterns 12.4 R10): unrolled `(An)+` block copies.
    units: Vec<crate::sched::Unit>,
    /// Call statements: the index in `out` of their first instruction and
    /// of the `jsr`/`bsr`, and the statement ([`held_through_calls`]).
    call_stmts: Vec<(usize, usize, Span)>,
    /// The statement being selected, for diagnostics.
    span: Span,
    /// The index in `out` of the first instruction of the statement being
    /// selected.
    stmt_start: usize,
    /// By-address slots loaded for one use in a body of several
    /// statements: the register, the start of its statement in `out`, and
    /// the statement ([`fresh_slot_loads`]).
    fresh: Vec<(Reg, usize, Span)>,
    /// Parameters loaded at entry, and locals held in a register once
    /// assigned ([`register_locals`]): frame offset and the (virtual)
    /// register the value is in.
    pinned: Vec<(i32, Reg)>,
    /// The frame offsets of the locals held in a register
    /// ([`register_locals`]).
    reg_locals: Vec<i32>,
    /// The statement being selected is in a body of several statements
    /// and does not directly follow a call there: a slot it uses once is
    /// loaded into a fresh register ([`Self::eval_a`],
    /// [`fresh_slot_loads`]; R4(b)).
    multi: bool,
    /// The body has several statements, or is one `if`/`repeat`
    /// ([`select_body`]).
    multi_body: bool,
    /// The next local label number (unique within the routine).
    next_label: u32,
    /// Labels already defined (a branch to one of them goes backward).
    placed: Vec<u32>,
    /// The label at the start of the epilogue, once a `return` needs it.
    epilogue: Option<u32>,
    /// Labels that a `bra` directly follows, with the statement that
    /// emitted the `bra` (see [`Self::jump`]).
    before_bra: Vec<(u32, Span)>,
    /// A function call inside the statement being selected may skip its
    /// pop (codegen-patterns 1.1): the statement is a store or result
    /// assignment whose continuation is clean ([`Tail::clean`]).
    call_no_pop: bool,
    /// Arguments of a call are being pushed (a function call there is
    /// refused: its pop is merged with the outer call's, codegen-patterns
    /// 1.1, 0xE69FF6).
    in_args: bool,
    /// The statement already called a function whose result (in D0) may
    /// still be needed.
    called: bool,
    /// The operands of an M$ARITH call are being evaluated
    /// ([`Self::marith`]): a call there is followed by the outer one's
    /// pushes, so it is popped.
    marith_depth: u32,
    /// A call of the statement being selected skipped its pop
    /// (codegen-patterns 1.1): nothing after it in the statement may
    /// branch conditionally.
    unpopped: bool,
    /// The routine addresses the unit's data from its absolute address
    /// ([`Function::data_absolute`]; [`Self::static_base`]).
    data_absolute: bool,
    /// With `data_absolute`: the address register the area's address was
    /// loaded into, once the first static operand needed it.
    abs_base: Option<u8>,
    /// The loops being selected, innermost last: the label `next` goes to
    /// (a `while`'s or `repeat`'s test, a `for`'s step) and the label
    /// `exit` goes to (after the loop), once made ([`Self::loop_target`]).
    loops: Vec<(Option<u32>, Option<u32>)>,
    /// The pointers of the `for` loops being selected, stepped through
    /// arrays by the element size ([`Self::for_`]): the index's frame
    /// offset, the array, and the (virtual) address register.
    stepped: Vec<(i32, SteppedArray, u8)>,
    /// The indices in `out` of the branches `exit` and `next` emitted
    /// ([`select_body`] refuses one to the very next instruction).
    loop_jumps: Vec<(usize, Span)>,
    /// Pointer locals held in an address register for the whole body
    /// ([`loop_pointer_locals`]): frame offset, and the virtual register
    /// once the first assignment has defined it.
    ptr_locals: Vec<(i32, Option<u8>)>,
    /// The assignments to those locals: frame offset, index in `out` of
    /// the `movea.l` and its statement's span.
    ptr_defs: Vec<(i32, usize, Span)>,
    /// The register holding the static link of a nested routine, loaded
    /// at entry when the body needs it ([`Selector::load_link`]).
    link: Option<u8>,
    /// The function's result variable is held in a register
    /// ([`Self::set_result_web`]): a function body of several statements.
    result_web: bool,
    /// That register, once the first assignment to the result defined it.
    web: Option<u8>,
    /// The same for a pointer result, in an address register
    /// ([`Self::set_pointer_result_web`]).
    web_a: Option<u8>,
    /// Stores of a register into a local's frame slot: offset, size,
    /// register, and the index in `out` after the store
    /// ([`Self::forwarded`]).
    stored: Vec<(i32, Size, Reg, usize)>,
    /// [`escaped_frame_objects`] of the routine.
    escaped: Vec<i32>,
    /// Where [`Self::eval_a`] gave A0 as the base of a pointer local
    /// ([`Self::a0_holds`]): the index in `out` then, checked once the
    /// body is selected (no write of A0 before the base's use).
    a0_bases: Vec<(usize, Span)>,
    /// The registers of the parameters loaded at entry
    /// ([`Self::entry_loads`]), in load order.
    entry_regs: Vec<Reg>,
    /// The frame objects nested routines reach
    /// ([`Function::uplevel_refs`]): never held in a register.
    uplevel: Vec<(i32, u32)>,
    /// Which computed values to hold for reuse ([`select_body`]).
    cse_plan: CsePlan,
    /// The computed values that may be reused ([`Self::cse_value`]).
    cse: Vec<Cse>,
    /// The number of the next computation [`Self::cse_value`] sees.
    cse_next: u32,
    /// Reuses found: (the reusing computation, the computation reused).
    cse_hits: Vec<(u32, u32)>,
    /// The (virtual) data registers of held computed values.
    cse_regs: Vec<u8>,
    /// The statements' memory writes: the range of `out` they emitted,
    /// the object written and its size in bytes ([`Self::cse_valid`]).
    writes: Vec<(usize, usize, Addr, u32)>,
}

/// Which computed values a selection holds for reuse (codegen-patterns
/// 12.4 R7; [`select_body`]).
#[derive(Clone, Debug)]
enum CsePlan {
    /// None: every computation is made as before, and the reuses that
    /// would be valid are recorded.
    Probe,
    /// The reuses found by the probe, (reusing, reused) by number: the
    /// reused computations are held in their register, the reusing ones
    /// read it.
    Hold(Vec<(u32, u32)>),
}

/// What a reusable computation computes: always of a plain variable
/// ([`Selector::cse_leaf`]).
#[derive(Clone, Debug, PartialEq)]
enum CseKey {
    /// The variable widened to the width (zero- or sign-extended by its
    /// own sign; the result's sign does not change the bits).
    Widen(Expr, Width),
    /// The variable times a constant, at its own width: a scaled array
    /// index ([`Selector::index_reg`]) or a product by a constant
    /// ([`Selector::mul_const`]).
    Scaled(Expr, i64),
}

/// A computation that may be reused (codegen-patterns 12.4 R7).
#[derive(Clone, Debug)]
struct Cse {
    key: CseKey,
    /// The variable it reads, and the variable's size.
    leaf: (Addr, u32),
    /// Its number ([`Selector::cse_next`]).
    id: u32,
    /// The data register holding the value.
    reg: u8,
    /// The index in `out` after the computation.
    at: usize,
}

/// An operand as [`Selector::eval`] leaves it.
#[derive(Clone, Debug)]
enum Val {
    /// A new virtual data register that the selector may modify in place.
    Tmp(u8),
    /// A register that holds a value the selector must not modify: a
    /// parameter loaded at entry, a function result in D0.
    Reg(Reg),
    /// A memory operand (any address register it needs is loaded).
    Mem(Ea),
    /// An immediate, as the operation's size reads it ([`imm`]).
    Imm(i64),
}

impl Selector {
    fn new(span: Span) -> Selector {
        Selector {
            out: Vec::new(),
            vregs: VRegs::new(),
            clobbered: RegSet::EMPTY,
            result_moves: Vec::new(),
            units: Vec::new(),
            call_stmts: Vec::new(),
            span,
            stmt_start: 0,
            fresh: Vec::new(),
            pinned: Vec::new(),
            reg_locals: Vec::new(),
            multi: false,
            multi_body: false,
            next_label: 1,
            placed: Vec::new(),
            epilogue: None,
            before_bra: Vec::new(),
            call_no_pop: false,
            in_args: false,
            called: false,
            marith_depth: 0,
            unpopped: false,
            data_absolute: false,
            abs_base: None,
            loops: Vec::new(),
            stepped: Vec::new(),
            loop_jumps: Vec::new(),
            ptr_locals: Vec::new(),
            ptr_defs: Vec::new(),
            link: None,
            uplevel: Vec::new(),
            result_web: false,
            web: None,
            web_a: None,
            entry_regs: Vec::new(),
            stored: Vec::new(),
            escaped: Vec::new(),
            a0_bases: Vec::new(),
            cse_plan: CsePlan::Probe,
            cse: Vec::new(),
            cse_next: 0,
            cse_hits: Vec::new(),
            cse_regs: Vec::new(),
            writes: Vec::new(),
        }
    }

    /// Whether frame offset `o` lies in an object a nested routine reaches.
    fn is_uplevel(&self, o: i32) -> bool {
        self.uplevel
            .iter()
            .any(|&(p, n)| p <= o && o < p + n as i32)
    }

    /// Load the static link of nested routine `f` into a register for the
    /// body (codegen-patterns 6), when the body reaches the enclosing
    /// frame or passes the link on.
    ///
    /// - [`StaticLink::Passed`]: copied from A1, 0xE135A4 `movea.l
    ///   A1,A2`;
    /// - [`StaticLink::CallerFrame`]: the dynamic link, 0xE13616 `movea.l
    ///   (A6),A4`.
    ///
    /// Placed after the parameters loaded at entry (`tools/nestsurvey.py`:
    /// every nested routine that loads parameters at entry loads them
    /// first, e.g. 0xE00CAC, 0xE0305C, 0xE082A8 `move.l (8,A6),D7; move.w
    /// (0xc,A6),D3; movea.l (A6),A4`, 0xE397D0, 0xE62CEA; the one exception
    /// is 0xE1359C, `movea.l A1,A2; move.l (0xc,A6),D2`, where the first
    /// statement reads the link: a scheduler's choice, not modelled).  The
    /// register is a virtual one: the allocator gives it A2..A4 when it
    /// lives across a call (0xE1359C A2; 0xE1360C A4, the order rule of
    /// codegen-patterns 12.4 not settled there), else the lowest free
    /// (0xE15888 `movea.l (A6),A1` beside the parameter in A0).
    fn load_link(&mut self, f: &Function) -> Res<()> {
        let Some(n) = &f.nesting else {
            return Ok(());
        };
        let mut needed = false;
        walk(&f.body, &mut |node| match node {
            Node::Addr(Addr::Enclosing(_)) => needed = true,
            Node::Call(c) if c.link == Some(LinkArg::OwnLink) => needed = true,
            _ => {}
        });
        if !needed {
            return Ok(());
        }
        let a = self.alloc_a()?;
        let src = match n.link {
            StaticLink::Passed => Ea::Reg(Reg::A(STATIC_LINK_A)),
            StaticLink::CallerFrame => Ea::Ind(FP),
        };
        self.out.push(Insn::Movea {
            size: Size::L,
            src,
            dst: a,
        });
        self.link = Some(a);
        Ok(())
    }

    /// The address register that holds the unit's data area: A5
    /// (codegen-patterns 4), or for a routine that addresses it
    /// absolutely (`--a5-absolute`) a register loaded with `lea
    /// <area>.l,An` where the first static operand needs it and kept for
    /// the rest of the body (REM_NAME_$REGISTER_SERVER 0xE4A4B2 `lea
    /// 0xE7DBB8,A0`, then `(0x28,A0)` and `(0x3c,A0)` in two statements;
    /// AUDIT_$SUSPEND 0xE70DC0, after the index load, once for the read
    /// and the write of one element).  The register is a virtual one, so
    /// the allocator gives it the lowest free (A0 at both sites).
    fn static_base(&mut self) -> Res<u8> {
        if !self.data_absolute {
            return Ok(DATA_BASE_A);
        }
        if let Some(a) = self.abs_base {
            return Ok(a);
        }
        let a = self.alloc_a()?;
        self.out.push(Insn::Lea {
            src: Ea::AbsL(DATA_BASE_LABEL.into(), 0),
            dst: a,
        });
        self.abs_base = Some(a);
        Ok(a)
    }

    fn new_label(&mut self) -> u32 {
        let l = self.next_label;
        self.next_label += 1;
        l
    }

    /// Define local label `l` here.
    fn place(&mut self, l: u32) {
        self.out.push(Insn::Label(l));
        self.placed.push(l);
    }

    /// Branch to label `l` when `cond` holds (`bra` for [`Cond::T`]).  The
    /// size is the emitter's (`emit.rs`, `size_branches`).
    ///
    /// A `bra` placed right after labels is recorded: a branch to one of
    /// them would be a branch to a `bra`, which compiled code never has
    /// (survey of 0xE00800..0xE88834: of the forward `bra`s none targets a
    /// `bra`, nor does any `b<cc>`), so the original threaded such branches
    /// or laid the code out otherwise; which, is not known.  [`select_body`]
    /// rejects them.
    fn jump(&mut self, cond: Cond, l: u32) {
        if cond == Cond::T {
            for i in self.out.iter().rev() {
                match i {
                    Insn::Label(m) => self.before_bra.push((*m, self.span)),
                    _ => break,
                }
            }
        }
        self.out.push(Insn::Branch {
            cond,
            label: l,
            short: true,
            back: self.placed.contains(&l),
        });
    }

    /// A statement sequence whose continuation is `tail`.
    fn seq(&mut self, f: &Function, body: &[Stmt], tail: Tail) -> Res<()> {
        let mut after_call = false;
        for (i, st) in body.iter().enumerate() {
            self.span = st.span;
            self.stmt_start = self.out.len();
            let rest = &body[i + 1..];
            // A slot used once is loaded where it is needed, into a
            // register no earlier statement left a value in (codegen-patterns
            // 12.4 R4(b), [`fresh_slot_loads`]) in a body of several
            // statements, except in a simple statement right after a call
            // in a body that loaded nothing at entry, where the lowest free
            // registers are taken (0xE00ECC..0xE00ED2, A0 and D0 after the
            // `jsr` at 0xE00EC2).
            self.multi =
                self.multi_body && !(after_call && self.pinned.is_empty() && is_simple(st));
            // What follows this statement up to the epilogue.
            let next = Tail {
                end: rest.is_empty() && tail.end,
                clean: seq_clean(rest, tail.clean),
            };
            // A function call in a store's value or a result assignment is
            // followed only by the move of its result: popped unless the
            // statement's continuation is clean (codegen-patterns 1.1;
            // 0xE04700 `bsr.w; move.l D0,(A3); bra.w` with no pop).
            self.call_no_pop =
                matches!(st.kind, StmtKind::SetResult(_) | StmtKind::Store { .. }) && next.clean;
            self.called = false;
            self.unpopped = false;
            match &st.kind {
                StmtKind::SetResult(e) => self.set_result(f, e)?,
                StmtKind::Store { addr, value } => {
                    self.store(addr, value)?;
                    self.wrote(addr, value.ty().bytes());
                }
                StmtKind::Copy { dst, src, bytes } => {
                    self.copy(dst, src, *bytes)?;
                    self.wrote(dst, *bytes);
                }
                StmtKind::Call(c) => self.call(c, next.clean)?,
                StmtKind::Fill {
                    start,
                    count,
                    value,
                } => {
                    self.fill(start, *count, value)?;
                    self.wrote(start, count * value.ty().bytes());
                }
                StmtKind::If { cond, then_, else_ } => self.if_(f, cond, then_, else_, next)?,
                StmtKind::Repeat { body, until } => self.repeat(f, body, until)?,
                StmtKind::While { cond, body } => self.while_(f, cond, body)?,
                StmtKind::For(l) => self.for_(f, l)?,
                StmtKind::Exit | StmtKind::Next => {
                    let exit = matches!(st.kind, StmtKind::Exit);
                    if !rest.is_empty() {
                        return Err(unsupported(
                            rest[0].span,
                            "a statement after `exit` or `next` (not reached) is",
                        ));
                    }
                    let l = self.loop_target(exit);
                    let at = self.out.len();
                    self.jump(Cond::T, l);
                    self.loop_jumps.push((at, st.span));
                }
                StmtKind::Case {
                    selector,
                    arms,
                    otherwise,
                } => self.case(f, selector, arms, otherwise.as_deref(), next)?,
                // codegen-patterns 10: `disable` is `ori.w #0x700,SR`,
                // `enable` `andi.w #0xF8FF,SR` (TIME_$GET_ADJUST 0xE16AB0,
                // 0xE16ABA; PEB 0xE70968, 0xE70998; 10 and 7 sites in
                // compiled code).  Statements around them keep their order:
                // no instruction of a neighbouring statement is moved across
                // one at the sites (the entry load 0xE16AAC stays before
                // 0xE16AB0; TIME_$SET_TIME_OF_DAY 0xE16864 before 0xE1686A).
                StmtKind::Disable => self.out.push(Insn::SrImm {
                    op: AluOp::Or,
                    imm: 0x700,
                }),
                StmtKind::Enable => self.out.push(Insn::SrImm {
                    op: AluOp::And,
                    imm: 0xf8ff,
                }),
                // `discard(set_sr(x))`: the old status word is read into a
                // register nothing reads (PROC1_$TS_END_CALLBACK 0xE14AF8
                // `move.w SR,D0; move.w D3,SR`).
                StmtKind::Discard(e) => {
                    self.eval(e)?;
                }
                StmtKind::Return => {
                    if !rest.is_empty() {
                        return Err(unsupported(
                            rest[0].span,
                            "a statement after `return` (not reached) is",
                        ));
                    }
                    if tail.end {
                        // The routine's last statement: the epilogue follows
                        // anyway.  In a nested sequence that ends the
                        // routine the original's choice (a `bra` to the
                        // next instruction, or nothing) is not known.
                        if !std::ptr::eq(body, f.body.as_slice()) {
                            return Err(unsupported(
                                st.span,
                                "a `return` at the end of an `if` part that ends the routine is",
                            ));
                        }
                    } else {
                        // LOG_$READ's worker 0xE17794: `bra.s` to the
                        // register restore that starts the epilogue
                        // (0xE177F6), as the error exits of codegen-patterns
                        // 9 branch to a shared epilogue.
                        let l = self.epilogue_label();
                        self.jump(Cond::T, l);
                    }
                }
            }
            after_call = matches!(st.kind, StmtKind::Call(_));
        }
        Ok(())
    }

    /// The label at the start of the epilogue (the register restore, or
    /// `unlk`), placed after the body.
    fn epilogue_label(&mut self) -> u32 {
        match self.epilogue {
            Some(l) => l,
            None => {
                let l = self.new_label();
                self.epilogue = Some(l);
                l
            }
        }
    }

    /// `if cond then then_ else else_` (ref "If", p. 4-99).
    ///
    /// ```text
    ///         <branch to ELSE when cond is false>
    ///         then_
    ///         bra     JOIN            ; only with an else part
    /// ELSE:   else_
    /// JOIN:
    /// ```
    ///
    /// LOG_$READ's worker: 0xE1778C..0xE17796 `tst.l (0x14,A5); bne.s
    /// 0xE17796; clr.w (A0); bra.s 0xE177F6` (`if ptr = nil then begin
    /// actual_len^ := 0; return end`, the `then` part ending in a
    /// `return`, so no `bra` to a join); 0xE177BA..0xE177CC `cmpi.w
    /// #0x400,D0; bls.s 0xE177C4; clr.w (A0); bra.s 0xE177CC; <else part>`.
    /// Nested: the outer `if` at 0xE177B8 (`bls.s 0xE177CC`) and the inner
    /// `if ... else` share the join 0xE177CC.  Joins are not aligned
    /// (codegen-patterns 7: forward branch targets).
    ///
    /// When both parts end in a call that pushes the same number of bytes
    /// and the caller must pop them, the original pops once, at the join
    /// (survey of compiled code: 56 such pairs share the pop, e.g. 0xE06156
    /// `bsr.w; bra.s 0xE06194` with `addq.w #4,SP` at 0xE06194; 45 pairs
    /// of different sizes pop separately, none of the same size does): not
    /// supported yet.
    fn if_(
        &mut self,
        f: &Function,
        cond: &IrCond,
        then_: &[Stmt],
        else_: &[Stmt],
        next: Tail,
    ) -> Res<()> {
        let span = self.span;
        if !else_.is_empty() && !next.clean {
            if let (Some(a), Some(b)) = (call_bytes(then_.last()), call_bytes(else_.last())) {
                if a == b {
                    return Err(unsupported(
                        self.span,
                        "an `if` whose two parts end in calls with the same argument bytes \
                         (the original shares the pop at the join, codegen-patterns 1.1) is",
                    ));
                }
            }
        }
        // `if c then return`: one branch to the epilogue on `c` itself, not
        // a branch past a `bra` (codegen-patterns 9).  TERM_$CONTROL
        // 0xE66B3A..0xE66B3C `tst.l (A3); bne.w 0xE66D78` (`if
        // status.all <> status_$ok then return`, the status held in A3,
        // per `term/control.c`; 0xE66D78 is the register restore); 0xE62E88
        // `bne.w`, 0xE68C08 `beq.w` likewise.  Compiled code has no
        // conditional branch over a `bra` at all (survey of every routine
        // `tools/regalloc.py` splits, ad-hoc script), against 888
        // conditional branches to an epilogue.  A `return` that ends the
        // routine anyway is refused below (in [`Self::seq`]).
        if matches!(
            then_,
            [Stmt {
                kind: StmtKind::Return,
                ..
            }]
        ) && (!next.end || !else_.is_empty())
        {
            let l = self.epilogue_label();
            self.branch(cond, true, l)?;
            return self.seq(f, else_, next);
        }
        // `if c then exit` / `if c then next` likewise: one conditional
        // branch on `c` to the loop's exit or continuation (no conditional
        // branch over a `bra` in compiled code, above; PKT_$DUMP_DATA
        // 0xE12814 `ble.s` out of its `dbf` loop).
        if let [Stmt {
            kind: k @ (StmtKind::Exit | StmtKind::Next),
            span: jspan,
        }] = then_
        {
            let l = self.loop_target(matches!(k, StmtKind::Exit));
            let at = self.out.len();
            self.branch(cond, true, l)?;
            for i in at..self.out.len() {
                if matches!(self.out[i], Insn::Branch { label, .. } if label == l) {
                    self.loop_jumps.push((i, *jspan));
                }
            }
            return self.seq(f, else_, next);
        }
        let l_else = self.new_label();
        self.branch(cond, false, l_else)?;
        if else_.is_empty() {
            self.seq(f, then_, next)?;
            self.place(l_else);
            return Ok(());
        }
        // The `then` part continues with the `bra` to the join, then
        // whatever follows the `if`.
        self.seq(
            f,
            then_,
            Tail {
                end: false,
                clean: next.clean,
            },
        )?;
        let l_join = self.new_label();
        if !ends_in_jump(then_) {
            self.span = span;
            self.jump(Cond::T, l_join);
        }
        self.place(l_else);
        self.seq(f, else_, next)?;
        self.place(l_join);
        Ok(())
    }

    /// `repeat body until until` (ref "Repeat/Until", p. 4-161): the body,
    /// then the test, branching back to the head while it is false.  The
    /// head is entered by falling through, and aligned to 4 bytes with the
    /// 0x2048 filler (codegen-patterns 7: in aligning routines, loop heads
    /// entered by fall-through have the filler 312 times, are 4-aligned
    /// already 158 times and at 2 mod 4 25 times).  A loop with the test at
    /// the bottom entered by falling through: 0xE03192..0xE031B6 (an `or
    /// else` chain, the last test branching back), 0xE03208..0xE03224.
    fn repeat(&mut self, f: &Function, body: &[Stmt], until: &IrCond) -> Res<()> {
        if body.is_empty() {
            return Err(unsupported(self.span, "an empty `repeat` loop is"));
        }
        // The test is selected under the `repeat` statement's own rules
        // (span, register policy), not those of the body's last statement.
        let (span, multi) = (self.span, self.multi);
        let head = self.new_label();
        self.out.push(Insn::Align4);
        self.place(head);
        // The test follows the body: a conditional branch, so a call in
        // the body is popped (codegen-patterns 1.1).  `next` goes to the
        // test, `exit` past it (codegen-patterns 7.4); their labels are
        // made when one is used.
        self.loops.push((None, None));
        self.seq(
            f,
            body,
            Tail {
                end: false,
                clean: false,
            },
        )?;
        let (test, exit) = self.loops.pop().expect("pushed above");
        (self.span, self.multi) = (span, multi);
        // A call in the test is followed by the conditional branch: popped.
        (self.call_no_pop, self.called) = (false, false);
        if let Some(t) = test {
            self.place(t);
        }
        self.branch(until, false, head)?;
        if let Some(x) = exit {
            self.place(x);
        }
        Ok(())
    }

    /// The label `exit` (`true`) or `next` goes to in the innermost loop,
    /// made on first use.
    fn loop_target(&mut self, exit: bool) -> u32 {
        let fresh = self.next_label;
        let l = self
            .loops
            .last_mut()
            .expect("`exit` and `next` only inside loops (dpc-sema)");
        let slot = if exit { &mut l.1 } else { &mut l.0 };
        match *slot {
            Some(x) => x,
            None => {
                *slot = Some(fresh);
                self.next_label += 1;
                fresh
            }
        }
    }

    /// `while cond do body` (ref "While", p. 4-208): the test at the
    /// bottom, entered by a `bra` to it, branching back to the head while
    /// the condition holds (codegen-patterns 7.2, 7.4: 213 such loops;
    /// 0xE01A8E..0xE01A98; the DISK_ module-local 0xE3CCCE `bra.s
    /// 0xE3CCE4; ...; cmpa.w #0,A1; bne.s 0xE3CCDE`):
    ///
    /// ```text
    ///         bra     TEST
    ///         <filler>        ; aligning routines, head at 2 mod 4
    /// HEAD:   body
    /// TEST:   <branch to HEAD when cond holds>
    /// EXIT:
    /// ```
    ///
    /// The head after the `bra` is aligned like every loop head (7.4: in
    /// aligning routines 29 such heads have the filler, 27 are 4-aligned
    /// already and 2, of loops over 0x100 bytes, are at 2 mod 4;
    /// MAC_OS_$CLOSE 0xE0B4CC `bra.s; movea.l A0,A0` before the head
    /// 0xE0B4D0).  The test is a condition's jumping code with the sense
    /// of the last branch reversed: `and then` leaves the loop on its
    /// first operand's failure and branches back on its second's success
    /// (NAME_$VALIDATE's module-local 0xE4A028..0xE4A034 `cmp.w (A1),D0;
    /// bcs.s <exit>; ...; cmpi.b #47,(-1,A0,D1.w); beq.s <head>`).
    /// `next` branches to the test, `exit` past it.
    fn while_(&mut self, f: &Function, cond: &IrCond, body: &[Stmt]) -> Res<()> {
        let (span, multi) = (self.span, self.multi);
        let head = self.new_label();
        let test = self.new_label();
        let exit = self.new_label();
        self.jump(Cond::T, test);
        self.out.push(Insn::Align4);
        self.place(head);
        self.loops.push((Some(test), Some(exit)));
        // The test follows the body: a call in it is popped.
        self.seq(
            f,
            body,
            Tail {
                end: false,
                clean: false,
            },
        )?;
        self.loops.pop();
        (self.span, self.multi) = (span, multi);
        (self.call_no_pop, self.called) = (false, false);
        self.place(test);
        self.branch(cond, true, head)?;
        self.place(exit);
        Ok(())
    }

    /// A `for` loop ([`ForLoop`]; ref "For", pp. 4-88..4-90) as a `dbf`
    /// loop (codegen-patterns 7, 7.4).
    ///
    /// **Constant bounds** (`n` iterations, 1..=65536):
    ///
    /// ```text
    ///         moveq   #n-1,Dc         ; move.w #n-1,Dc above 127
    ///         moveq   #from,Di        ; clr.w Di for 0; only when the
    ///                                 ;   body reads the index
    ///         <filler>
    /// HEAD:   body
    /// STEP:   addq.w  #1,Di           ; subq.w for downto
    ///         dbf     Dc,HEAD
    /// EXIT:
    /// ```
    ///
    /// The count is loaded before the index (survey 7.4: 116 of 118 such
    /// loops with the index's initial value in the preheader; MST_$ALLOC_ASID
    /// 0xE42D50 `moveq #57,D0; ...; clr.w D2`; DISK_$UNASSIGN_ALL 0xE6BE24
    /// `moveq #9,D2; moveq #1,D3`), the index stepped at the bottom, before
    /// `dbf` (0xE6BE3A; `subq.w #1` for `downto`, 0xE3C99A, 0xE6CAAC).  The
    /// index lives in a register for the loop only: its frame slot is never
    /// written (DISK_$UNASSIGN_ALL, `link #-8` for its two other locals'
    /// 6 bytes and the index's 2; 307 of 320 `dbf` loops with an index step
    /// it in a register).  A body that does not read the index gets no
    /// index register (0xE00EB0; 7, `fill`).  The count and the index are
    /// ordinary values for the allocator: across a call in the body they
    /// take callee-saved registers (DISK_$UNASSIGN_ALL D2, D3).
    ///
    /// **A run-time upper bound** (`to`, from 1, ascending, a signed
    /// index), in a body that calls:
    ///
    /// ```text
    ///         move.w  to,T            ; the bound into a scratch value
    ///         subq.w  #1,T
    ///         bmi     EXIT
    ///         move.w  T,Dc            ; the counter, callee-saved
    ///         moveq   #1,Di
    /// ```
    ///
    /// (7.4: 45 of the 46 such guarded loops whose body calls copy the
    /// count into a counter of its own; DTTY_$WRITE_STRING 0xE1D6AA..0xE1D6B2
    /// `move.w (A0),D0; subq.w #1,D0; bmi.s; move.w D0,D3; moveq #1,D2`.)
    /// Without a call in the body the count is copied 45 times and counted
    /// down in place 64 times, with no rule found: refused, as are other
    /// lower bounds and run-time `downto`.
    ///
    /// Refused besides: a loop that never runs (no site looked at); an
    /// index read as the index of an array of elements wider than a byte
    /// (the original strength-reduced those into a stepping pointer: 7.4,
    /// PKT_$DUMP_DATA 0xE12800, LOG_$READ's worker 0xE177E0); an index
    /// whose initial value is the count's (0xE6A8BC `moveq #19,D2; move.w
    /// D2,D1`: the constant is copied, not reloaded).
    fn for_(&mut self, f: &Function, l: &ForLoop) -> Res<()> {
        let span = self.span;
        let off = l.index.offset;
        let arrays =
            stepped_arrays(&l.body, off, &f.uplevel_refs).map_err(|m| unsupported(span, m))?;
        let uses_index = index_read_unstepped(&l.body, off);
        let konst = |e: &Expr| match e {
            Expr::Const { value, .. } => Some(*value),
            _ => None,
        };
        let exit = self.new_label();
        let (count, init) = match (konst(&l.from), konst(&l.to)) {
            (Some(a), Some(b)) => {
                let n = if l.down { a - b + 1 } else { b - a + 1 };
                if n < 1 {
                    return Err(unsupported(span, "a `for` loop that never runs is"));
                }
                if n > 0x10000 {
                    return Err(unsupported(
                        span,
                        "a `for` loop of more than 65536 iterations is",
                    ));
                }
                if uses_index && a == n - 1 {
                    return Err(unsupported(
                        span,
                        "a `for` index whose initial value equals the `dbf` count (the original \
                         copies the count's register, 0xE6A8BC; codegen-patterns 7.4) is",
                    ));
                }
                let c = self.alloc_d()?;
                let n = n - 1;
                self.out.push(if n <= 127 {
                    Insn::Moveq {
                        value: n as i8,
                        dst: c,
                    }
                } else {
                    Insn::Move {
                        size: Size::W,
                        src: Ea::Imm(imm(n, Width::Word)),
                        dst: Ea::Reg(Reg::D(c)),
                    }
                });
                (c, a)
            }
            (Some(1), None) if !l.down && l.index.ty.is_signed() => {
                if !stmts_call(&l.body) {
                    return Err(unsupported(
                        span,
                        "a `for` loop with a run-time bound and no call in its body (the count \
                         is copied into a counter of its own or not, no rule found, \
                         codegen-patterns 7.4) is",
                    ));
                }
                let t = match self.eval(&l.to)? {
                    Val::Tmp(d) => d,
                    v => self.in_tmp(v, Width::Word)?,
                };
                self.out.push(Insn::Subq {
                    size: Size::W,
                    value: 1,
                    dst: Ea::Reg(Reg::D(t)),
                });
                self.jump(Cond::Mi, exit);
                let c = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: Size::W,
                    src: Ea::Reg(Reg::D(t)),
                    dst: Ea::Reg(Reg::D(c)),
                });
                (c, 1)
            }
            _ => {
                return Err(unsupported(
                    span,
                    "a `for` loop with these bounds (a run-time bound other than an ascending \
                     loop's upper bound from 1, codegen-patterns 7.4) is",
                ))
            }
        };
        let index = if uses_index {
            let d = self.alloc_d()?;
            self.out.push(match init {
                0 => Insn::Clr {
                    size: Size::W,
                    dst: Ea::Reg(Reg::D(d)),
                },
                -128..=127 => Insn::Moveq {
                    value: init as i8,
                    dst: d,
                },
                _ => {
                    return Err(unsupported(
                        span,
                        "a `for` index whose initial value is outside -128..127 (not \
                         catalogued, codegen-patterns 7.4) is",
                    ))
                }
            });
            self.pinned.push((off, Reg::D(d)));
            Some(d)
        } else {
            None
        };
        let ptrs = self.stepped_pointers(&arrays, init)?;
        let depth = self.stepped.len();
        self.stepped
            .extend(arrays.iter().zip(&ptrs).map(|(a, &r)| (off, a.clone(), r)));
        let multi = self.multi;
        let head = self.new_label();
        let step = self.new_label();
        self.out.push(Insn::Align4);
        self.place(head);
        self.loops.push((Some(step), Some(exit)));
        self.seq(
            f,
            &l.body,
            Tail {
                end: false,
                clean: false,
            },
        )?;
        self.loops.pop();
        (self.span, self.multi) = (span, multi);
        (self.call_no_pop, self.called) = (false, false);
        self.place(step);
        if let Some(d) = index {
            let step = Insn::Addq {
                size: Size::W,
                value: 1,
                dst: Ea::Reg(Reg::D(d)),
            };
            self.out.push(if l.down {
                Insn::Subq {
                    size: Size::W,
                    value: 1,
                    dst: Ea::Reg(Reg::D(d)),
                }
            } else {
                step
            });
            self.pinned.retain(|&(o, _)| o != off);
        }
        // The pointers after the index, in the reverse order of their
        // forming (codegen-patterns 7.4: 29 of 30 loops; 0xE177EE `addq.l
        // #2,A1`, the destination, before `addq.l #2,A0`; 0xE2F3F8 forms
        // A2, A1, A0 and steps A0, A1, A2): `addq.l #s` (`subq.l` for
        // `downto`, 0xE3C99A `subq.l #4,A1`) up to 8, `lea (s,An),An` above
        // (HINT_$INIT_CACHE 0xE313F4 `lea (12,A0),A0`).
        for (a, &r) in arrays.iter().zip(&ptrs).rev() {
            let s = a.scale;
            self.out.push(match (s, l.down) {
                (1..=8, false) => Insn::Addq {
                    size: Size::L,
                    value: s as u8,
                    dst: Ea::Reg(Reg::A(r)),
                },
                (1..=8, true) => Insn::Subq {
                    size: Size::L,
                    value: s as u8,
                    dst: Ea::Reg(Reg::A(r)),
                },
                _ => Insn::Lea {
                    src: Ea::Disp(if l.down { -(s as i32) } else { s as i32 }, r),
                    dst: r,
                },
            });
        }
        self.stepped.truncate(depth);
        self.out.push(Insn::Dbf {
            reg: count,
            label: head,
        });
        self.place(exit);
        Ok(())
    }

    /// Form the pointers of a `for` loop's stepped arrays
    /// ([`stepped_arrays`], codegen-patterns 7.4) after its count and
    /// index: each array's pointer is loaded into an address register (or
    /// taken from the one it was loaded into at entry), all of them first,
    /// then each stepping pointer is formed from it with `lea (from ×
    /// size,An),Ap`, `lea (An),Ap` for 0, in the order of first use
    /// (0xE4F010 `movea.l D6,A4; lea (2,A3),A1; lea (2,A4),A0`; 0xE6FCC6
    /// loads its second base before forming the first pointer; LOG_$READ's
    /// worker 0xE177E0 `lea (2,A1),A0; lea (2,A2),A1`; PKT_$DUMP_DATA
    /// 0xE12800 `lea (4,A0),A2` from a pointer loaded at entry; 0xE3E4C6
    /// `lea (A3),A2`).  The pointer is the address of the element at the
    /// index *plus the lower bound times the size*, so the elements are
    /// reached at the displacements the selector forms anyway (the lower
    /// bound and field offsets folded in, section 5.2): 0xE177E8 `move.w
    /// (-2,A0),(-2,A1)`.  A new register for each pointer (R2, no copy
    /// coalescing; 0xE177E0 although A1 dies there).  **Not reproduced:**
    /// the forms in place (`lea (12,A0),A0` after `movea.l (12,A6),A0`,
    /// 0xE7632C, 0xE76378, 0xE76448; `movea.l (32,A6),A1; ...; addq.l
    /// #2,A1`, 0xE6FCC6), and LOG_$READ's worker's `movea.l (8,A6),A2`
    /// hoisted above the count (0xE177CC; 12.4 R11/R12).
    fn stepped_pointers(&mut self, arrays: &[SteppedArray], from: i64) -> Res<Vec<u8>> {
        let mut bases = Vec::new();
        for a in arrays {
            let Addr::Indirect { base, .. } = &a.base else {
                unreachable!("stepped_arrays takes arrays through pointers only")
            };
            bases.push(self.eval_a(base)?);
        }
        let mut ptrs = Vec::new();
        for (a, &b) in arrays.iter().zip(&bases) {
            let d = from * a.scale as i64;
            if !(-0x8000..0x8000).contains(&d) {
                return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
            }
            let r = self.alloc_a()?;
            self.out.push(Insn::Lea {
                src: if d == 0 {
                    Ea::Ind(b)
                } else {
                    Ea::Disp(d as i32, b)
                },
                dst: r,
            });
            ptrs.push(r);
        }
        Ok(ptrs)
    }

    /// `case selector of ... otherwise ... end` (ref "Case", pp.
    /// 4-36..4-37; codegen-patterns 7.3).
    ///
    /// The selector is evaluated into a new data register (a value held
    /// in a register is copied: NETWORK_$SET_SERVICE 0xE0F48E `move.w
    /// D2,D0`; TTY_$I_SIGNAL 0xE1B830 `move.w D0,D1`), then dispatched by
    /// one of two forms, chosen from the number of labels `n` and their
    /// range `r` (`max - min + 1`; survey of compiled code, 50 tables and
    /// 61 compare chains, `tools/casesurvey.py`, codegen-patterns 7.3):
    ///
    /// - **compare chain** when `n <= 3` (all 49 chains of 2 or 3 labels,
    ///   32 of them dense, e.g. SYSBUS_$INIT 0xE0AB64 labels 0..2; no
    ///   table has fewer than 4 labels), or when `3r >= 13n` (every chain
    ///   of 4 or more labels but one has `r/n >= 4.33`: TTY_$I_SIGNAL
    ///   0xE1B832 26/6, module-local 0xE1CC70 48/8):
    ///
    ///   ```text
    ///           cmpi.w  #k1,Dn          ; label 0: tst.w Dn, or nothing
    ///           beq     ARM(k1)         ;   right after the load of Dn
    ///           ...                     ; labels in source order
    ///           bra     OTHERWISE       ; or JOIN
    ///   ```
    ///
    /// - **jump table** when `n >= 4` and `7r <= 19n` (every table has
    ///   `r/n <= 2.71`, module-local 0xE63376 19/7, 0xE6A766 59/22):
    ///
    ///   ```text
    ///           subq.w  #min,Dn         ; subi.w for min > 8, addq.w
    ///           cmpi.w  #r,Dn           ;   for min < 0; none for 0
    ///           bcc     OTHERWISE       ; or JOIN; unsigned, so below
    ///           add.w   Dn,Dn           ;   min is caught too
    ///           move.w  (T,PC,Dn.w),Dn
    ///           jmp     (T,PC,Dn.w)
    ///   T:      .short  ARM(min)-T      ; a missing label: OTHERWISE
    ///           ...                     ;   or JOIN
    ///   ```
    ///
    ///   OSINFO_$GET_MMAP 0xE5C6C0..0xE5C6D6; PROC1_$CREATE_P 0xE15176
    ///   (`subq.w #3`); ACL_$SERVER 0xE495CE (`subi.w #100`);
    ///   MST_$PRIV_SET_TOUCH_AHEAD_CNT 0xE44534 (`addq.w #4`).
    ///
    /// Between the two (2.71 < r/n < 4.33 with `n >= 4`) nothing decides;
    /// refused.  The one exception, the 41-label chain of
    /// ASKNODE_$INTERNET_INFO 0xE6463E (r/n 2.20, where a table is
    /// predicted), is not explained (`tools/casesurvey.py`).  The arms follow in source order; each but the last ends
    /// with a `bra` to the join, the last falls through to it when there is
    /// no `otherwise` (CAL_$WEEKDAY 0xE68716..0xE68730); an `otherwise`
    /// part comes after all the arms and falls through to the join, the
    /// last arm branching over it (MAC_OS_$OPEN 0xE0B3BA..0xE0B3DC;
    /// module-local 0xE1CCA2..0xE1CCBC).  An empty arm's labels go to the
    /// join directly (KBD_$RCV 0xE1CCFE; ASKNODE_$INTERNET_INFO 0xE65614).
    /// No label matching and no `otherwise`: on to the join (ref Appendix
    /// D, ISO 6.8.3.5, p. D-4: not detected).
    fn case(
        &mut self,
        f: &Function,
        selector: &Expr,
        arms: &[CaseArm],
        otherwise: Option<&[Stmt]>,
        next: Tail,
    ) -> Res<()> {
        let span = self.span;
        if expr_has_call(selector) {
            // CAL_$WEEKDAY 0xE686F6 dispatches on D0 after `move.w D0,D1`:
            // what the copy is for is not known.
            return Err(unsupported(span, "a function call in a `case` selector is"));
        }
        let labels: Vec<(i64, usize)> = arms
            .iter()
            .enumerate()
            .flat_map(|(i, a)| a.labels.iter().map(move |&l| (l, i)))
            .collect();
        let n = labels.len() as i64;
        if n < 2 {
            // A single label is not told apart from an `if` in the image.
            return Err(unsupported(span, "a `case` with one label is"));
        }
        let nonempty: Vec<usize> = (0..arms.len())
            .filter(|&i| !arms[i].body.is_empty())
            .collect();
        if nonempty.is_empty() && otherwise.is_none() {
            return Err(unsupported(span, "a `case` without statements is"));
        }
        let lo = labels.iter().map(|l| l.0).min().unwrap();
        let hi = labels.iter().map(|l| l.0).max().unwrap();
        let range = hi - lo + 1;
        let table = if n <= 3 {
            false
        } else if 7 * range <= 19 * n {
            true
        } else if 3 * range >= 13 * n {
            false
        } else {
            return Err(unsupported(
                span,
                format!(
                    "a `case` of {n} labels over a range of {range} (between the sparsest jump \
                     table and the densest compare chain seen, codegen-patterns 7.3) is"
                ),
            ));
        };
        if !next.clean {
            // The original shares a pop at the join when both parts of an
            // `if` end in calls of the same argument bytes (codegen-patterns
            // 1.1); arms are taken to behave alike.
            let ends: Vec<u32> = nonempty
                .iter()
                .map(|&i| arms[i].body.as_slice())
                .chain(otherwise)
                .filter_map(|b| call_bytes(b.last()))
                .collect();
            if (1..ends.len()).any(|k| ends[..k].contains(&ends[k])) {
                return Err(unsupported(
                    span,
                    "a `case` whose parts end in calls with the same argument bytes (the \
                     original may share the pop at the join, codegen-patterns 1.1) is",
                ));
            }
        }
        let w = selector.ty().width;
        let size = size_of(w);
        // A `char` selector widened with `clr.w Dn; move.b x,Dn`
        // (module-local 0xE1CC6C, ACL_$SERVER 0xE495C8).
        let zext =
            matches!(selector, Expr::Convert { value, .. } if value.ty().width == Width::Byte);
        let start = self.out.len();
        let v = self.eval(selector)?;
        let d = self.in_tmp(v, w)?;
        let moved = self.out.len() > start
            && matches!(self.out.last(), Some(Insn::Move { size: s, dst: Ea::Reg(Reg::D(x)), .. }) if *x == d && *s == size);
        let l_join = self.new_label();
        let arm_label: Vec<u32> = arms
            .iter()
            .map(|a| {
                if a.body.is_empty() {
                    l_join
                } else {
                    self.new_label()
                }
            })
            .collect();
        let l_other = otherwise.map(|_| self.new_label());
        let l_default = l_other.unwrap_or(l_join);
        let dreg = Ea::Reg(Reg::D(d));
        if table {
            if w == Width::Long && lo != 0 {
                return Err(unsupported(
                    span,
                    "a jump table on a longword selector whose labels do not start at 0 is",
                ));
            }
            match lo {
                0 => {}
                1..=8 => self.out.push(Insn::Subq {
                    size: Size::W,
                    value: lo as u8,
                    dst: dreg.clone(),
                }),
                -8..=-1 => self.out.push(Insn::Addq {
                    size: Size::W,
                    value: (-lo) as u8,
                    dst: dreg.clone(),
                }),
                9.. => self.out.push(Insn::AluImm {
                    op: AluOp::Sub,
                    size: Size::W,
                    imm: lo,
                    dst: dreg.clone(),
                }),
                _ => {
                    return Err(unsupported(
                        span,
                        "a jump table whose lowest label is below -8 (`addi.w`? not seen) is",
                    ))
                }
            }
            self.out.push(Insn::Cmpi {
                size,
                imm: range,
                dst: dreg.clone(),
            });
            self.jump(Cond::Cc, l_default);
            self.out.push(Insn::Alu {
                op: AluOp::Add,
                size: Size::W,
                src: dreg,
                dst: d,
            });
            let tbl = self.new_label();
            let entries: Vec<u32> = (lo..=hi)
                .map(|v| {
                    labels
                        .iter()
                        .find(|l| l.0 == v)
                        .map_or(l_default, |l| arm_label[l.1])
                })
                .collect();
            let mut targets = Vec::new();
            for &t in &entries {
                if !targets.contains(&t) {
                    targets.push(t);
                }
            }
            self.out.push(Insn::TableIndex { reg: d, table: tbl });
            self.out.push(Insn::TableJump {
                reg: d,
                table: tbl,
                targets,
            });
            self.place(tbl);
            for t in entries {
                let back = self.placed.contains(&t);
                self.out.push(Insn::TableEntry {
                    table: tbl,
                    target: t,
                    back,
                });
            }
        } else {
            // A conditional branch over a `bra` does not occur in compiled
            // code (codegen-patterns 7.1): the last test would skip the
            // final `bra` when its arm is the first code after it.
            if nonempty.is_empty() {
                // Every label to the join and the final `bra` to the
                // `otherwise` part right after it (a `bra.w` of
                // displacement 2): not seen.
                return Err(unsupported(
                    span,
                    "a compare-chain `case` whose arms are all empty (its `bra` to the \
                     `otherwise` part right after it, not seen) is",
                ));
            }
            let first_code = nonempty.first().map(|&i| arm_label[i]).or(l_other);
            if Some(arm_label[labels[labels.len() - 1].1]) == first_code {
                return Err(unsupported(
                    span,
                    "a compare-chain `case` whose last label belongs to its first arm (a `beq` \
                     over the `bra`, not seen) is",
                ));
            }
            for (k, &(l, i)) in labels.iter().enumerate() {
                if l == 0 {
                    // Label 0 is a test: none right after the load of the
                    // selector, whose flags it uses (SYSBUS_$INIT 0xE0AB64
                    // `move.w (4,A0),D0; beq.s`; XPD_$PUT_REGISTERS
                    // 0xE5C3A0; TERM_$GET_EC 0xE723F6; 0xE01B58 `move.l
                    // (A2),D0; beq`), `tst` otherwise (TIME_$GET_EC 0xE16762
                    // after a compare; 0xE556E6 `clr.w D0; move.b
                    // (39,A0),D0; tst.w D0`).
                    if k > 0 || !moved {
                        if k == 0 && !zext {
                            return Err(unsupported(
                                span,
                                "label 0 tested first on a computed `case` selector (`tst` or \
                                 the flags of the computation?) is",
                            ));
                        }
                        self.out.push(Insn::Tst {
                            size,
                            dst: dreg.clone(),
                        });
                    }
                } else {
                    self.out.push(Insn::Cmpi {
                        size,
                        imm: imm(l, w),
                        dst: dreg.clone(),
                    });
                }
                self.jump(Cond::Eq, arm_label[i]);
            }
            self.jump(Cond::T, l_default);
        }
        let last = nonempty.last().copied();
        for &i in &nonempty {
            self.place(arm_label[i]);
            let body = &arms[i].body;
            let falls = otherwise.is_none() && Some(i) == last;
            let tail = if falls {
                next
            } else {
                Tail {
                    end: false,
                    clean: next.clean,
                }
            };
            self.seq(f, body, tail)?;
            if !falls && !ends_in_jump(body) {
                self.span = span;
                self.jump(Cond::T, l_join);
            }
        }
        if let (Some(o), Some(l)) = (otherwise, l_other) {
            self.place(l);
            self.seq(f, o, next)?;
        }
        self.place(l_join);
        Ok(())
    }

    /// Branch to `l` when condition `c` is `when`; fall through otherwise.
    ///
    /// Jumping code: `not` swaps `when`; `a and then b` / `a or else b`
    /// test `a` first and `b` only when needed (ref pp. 4-18..4-19,
    /// 4-134), each with its own branch.  Image: 0xE031A2..0xE031B6, a
    /// loop test `c1 or else c2 or else c3 or else c4` as `cmp.w D4,D2;
    /// bcc.s <exit>; ...; bne.s <exit>; tst.w D5; bmi.s <exit>; btst
    /// #14,D5; bne.s <head>`: every operand but the last branches out when
    /// it is true, the last one back to the head.
    fn branch(&mut self, c: &IrCond, when: bool, l: u32) -> Res<()> {
        match c {
            IrCond::Not(x) => self.branch(x, !when, l),
            IrCond::AndThen(a, b) | IrCond::OrElse(a, b) => {
                // `and then` short-circuits on false, `or else` on true.
                let short = matches!(c, IrCond::OrElse(..));
                if when == short {
                    self.branch(a, when, l)?;
                    self.branch(b, when, l)
                } else {
                    let skip = self.new_label();
                    self.branch(a, short, skip)?;
                    self.branch(b, when, l)?;
                    self.place(skip);
                    Ok(())
                }
            }
            IrCond::Test(e) => {
                // A boolean is 0xFF or 0: `tst.b` and the sign
                // (codegen-patterns 5, 7: after `tst.b` the compiled code
                // branches with `bpl` 919 times, `bmi` 425, `beq`/`bne`
                // 37).  A function's boolean result is tested in D0 after
                // the pop (0xE0082C..0xE00830 `addq.w #8,SP; tst.b D0;
                // bmi.s`).
                let ea = match e {
                    Expr::Load { addr, .. } => self.operand(addr)?,
                    Expr::Call { .. } => match self.eval(e)? {
                        Val::Reg(r) => Ea::Reg(r),
                        _ => unreachable!("a call's value is its result register"),
                    },
                    // `k in s` for a constant k and a set of one word or one
                    // longword: `btst #k` on the set in a register, the
                    // branch on Z (0xE013BA..0xE013C2 `move.w (190,A2),D0;
                    // btst #4,D0; beq.w`; 0xE031B2 `btst #14,D5; bne.s`).
                    Expr::SetMember {
                        elem,
                        set,
                        set_width,
                    } if matches!(**elem, Expr::Const { .. }) => {
                        self.set_bit_test(elem, set, *set_width, false)?;
                        self.jump(if when { Cond::Ne } else { Cond::Eq }, l);
                        return Ok(());
                    }
                    _ => {
                        return Err(unsupported(
                            self.span,
                            "a condition that is a computed boolean value is",
                        ))
                    }
                };
                self.tst(Size::B, ea)?;
                self.jump(if when { Cond::Mi } else { Cond::Pl }, l);
                Ok(())
            }
            IrCond::Compare { op, lhs, rhs } => {
                let cc = self.compare(*op, lhs, rhs)?;
                self.jump(if when { cc } else { cc.negate() }, l);
                Ok(())
            }
        }
    }

    /// The effective address of the scalar at `addr` as an operand, without
    /// loading the value: the register a by-value parameter was loaded
    /// into at entry, else the memory operand.
    fn operand(&mut self, addr: &Addr) -> Res<Ea> {
        if let Addr::Frame(o) = addr {
            if let Some(r) = self.pinned(*o) {
                return Ok(Ea::Reg(r));
            }
        }
        self.ea_of(addr)
    }

    /// `tst.<size> ea`, unless `ea` is a register the previous instruction
    /// loaded: whether the original then relies on the flags of the load
    /// is not settled (RIP_$SEND_UPDATES 0xE6887E `move.b (8,A6),D0; bpl`
    /// and 0xE014DC `move.l (A3),D0; beq` do; 0xE0B640 `move.l (A1),D0;
    /// tst.l D0; bmi` and 0xE0BEAE do not).  After a store to memory the
    /// memory is tested again (0xE02806 `move.w D0,(0x412,A0); tst.w
    /// (0x412,A0)`).
    fn tst(&mut self, size: Size, ea: Ea) -> Res<()> {
        if let (
            Ea::Reg(r),
            Some(Insn::Move {
                dst: Ea::Reg(d), ..
            }),
        ) = (&ea, self.out.last())
        {
            if d == r {
                return Err(unsupported(
                    self.span,
                    "testing a register the previous instruction loaded (whether the load's \
                     flags are used is not settled, codegen-patterns 7) is",
                ));
            }
        }
        self.out.push(Insn::Tst {
            size,
            dst: ea.clone(),
        });
        Ok(())
    }

    /// Set the condition codes for `lhs op rhs` and give the condition
    /// that holds when the relation does.
    ///
    /// - Against 0 (or `nil`): `tst.<s> <lhs>`; there is no `cmpi #0` in
    ///   compiled code.  `=`/`<>` give `eq`/`ne`; for a signed value `<`
    ///   and `>=` give `mi`/`pl` (after `tst.w`, `bmi`/`bpl` 144 times and
    ///   `blt`/`bge` 4 times, unexplained), `<=` and `>` `le`/`gt` (`ble`/`bgt` 42 and 17
    ///   times after `tst.w`).  0xE1778C `tst.l (0x14,A5); bne.s`.
    ///   Unsigned `<`, `<=`, `>`, `>=` against 0 are not catalogued.
    /// - Against another constant: `cmpi.<s> #n,<lhs>`, the operand in a
    ///   register (0xE1779A `cmpi.w #0x400,D1`, a parameter loaded at
    ///   entry) or in memory (0xE4FE90 `cmpi.w #4,(0x12,A6)`; 0xE4BAE0
    ///   `cmpi.l #0x400,(0x10,A3)`).
    /// - Two variables: `cmp.<s> <rhs>,Dn` when `lhs` is held in data
    ///   register Dn.  Which operand the original put in a register when
    ///   neither is (0xE6A9B4 `move.w (A1),D5; cmp.w (0x14,A6),D5`: the
    ///   source order cannot be seen), or when both are, is not known.
    /// - A constant on the left: not supported yet.
    ///
    /// Signed values use `lt`/`le`/`gt`/`ge`, unsigned ones
    /// `cs`/`ls`/`hi`/`cc` (0xE1779E `bcc` for an unsigned `<`, 0xE177BE
    /// `bls` for an unsigned `>`; IR signedness, codegen-patterns 5).
    ///
    /// **Uncertain:** a register-held long constant is compared at
    /// 0xE177AE..0xE177B6 (`move.l #0x400,D3; add.l D1,D2; cmp.l D3,D2`),
    /// against 28 `add.l Dm,Dn` + `cmp` sites of which 15 compare with
    /// `cmpi.l` (0xE1216E `cmpi.l #0x500,D7` after the same zero-extended
    /// word sum).  Not explained; `cmpi` is used here.
    fn compare(&mut self, op: Rel, lhs: &Expr, rhs: &Expr) -> Res<Cond> {
        let ty = lhs.ty();
        let size = size_of(ty.width);
        let signed = ty.sign == Sign::Signed;
        if let Some(c) = a0_call(lhs) {
            // A pointer function's result against `nil`: `cmpa.w #0,A0`
            // after the call and its pop (codegen-patterns 3, "Callers of
            // pointer functions": 12 sites, e.g. the NET module-local 0xE5A128
            // at 0xE5A150 after ROUTE_$FIND_PORTP, AST_$LOAD_AOTE 0xE023BA).
            // Only `=`/`<>` with `nil` are seen.
            if !matches!(rhs, Expr::Const { value: 0, .. }) || !matches!(op, Rel::Eq | Rel::Ne) {
                return Err(unsupported(
                    self.span,
                    "comparing the value of a function whose result is in A0 other than `= nil` \
                     / `<> nil` is",
                ));
            }
            let a = self.call_a0(c)?;
            self.out.push(Insn::CmpaZero(a));
            return Ok(if op == Rel::Eq { Cond::Eq } else { Cond::Ne });
        }
        let Expr::Load { addr, .. } = lhs else {
            return self.compare_computed(op, lhs, rhs);
        };
        if let (Addr::Frame(o), Expr::Const { value: 0, .. }, Scalar::POINTER) = (addr, rhs, ty) {
            if *o < 0 && self.pinned(*o).is_none() && matches!(op, Rel::Eq | Rel::Ne) {
                let cc = if op == Rel::Eq { Cond::Eq } else { Cond::Ne };
                let stored = matches!(
                    self.out.last(),
                    Some(Insn::Move {
                        size: Size::L,
                        src: Ea::Reg(Reg::A(0)),
                        dst: Ea::Disp(d, FP),
                    }) if d == o
                );
                if stored && self.is_uplevel(*o) {
                    // A local a nested routine reaches, tested right after
                    // a pointer function's A0 was stored into it: A0 is
                    // tested (AST_$INVALIDATE 0xE0666A `move.l
                    // A0,(-20,A6); cmpa.w #0,A0`, again at 0xE0668C; the
                    // AST_ module-local 0xE05214 at 0xE0528C, 0xE052B0;
                    // bead domain-pascal-500).
                    self.out.push(Insn::CmpaZero(0));
                    return Ok(cc);
                }
                if stored {
                    // Any other local: the branch reads the flags of the
                    // store itself (AST_$RESERVE 0xE067BA `move.l
                    // A0,(-28,A6); bne.s`, again at 0xE067DA;
                    // AST_$TRUNCATE 0xE05CC8, 0xE05CEA; AST_$PURIFY
                    // 0xE056D8 `beq.w`).  Of the stores of a register
                    // into memory followed by a test of the same value,
                    // 50 branch on the store's flags (9 of them stores
                    // of A0), 9 test the register again (the 4 of the two
                    // routines above; 0xE554A8 `tst.w D0` at a join), 11
                    // test the memory, none of A0 (bead
                    // domain-pascal-500, `tools/resultsurvey.py
                    // storetest`).
                    return Ok(cc);
                }
                if self.a0_holds(*o) {
                    return Err(unsupported(
                        self.span,
                        "testing a pointer local against nil while A0 still holds it, not right \
                         after the store (A0 or the slot: not seen) is",
                    ));
                }
            }
        }
        match rhs {
            Expr::Const { value: 0, .. } => {
                let cc = match (op, signed) {
                    (Rel::Eq, _) => Cond::Eq,
                    (Rel::Ne, _) => Cond::Ne,
                    (Rel::Lt, true) => Cond::Mi,
                    (Rel::Ge, true) => Cond::Pl,
                    (Rel::Le, true) => Cond::Le,
                    (Rel::Gt, true) => Cond::Gt,
                    _ => {
                        return Err(unsupported(
                            self.span,
                            "an unsigned ordering comparison with 0 is",
                        ))
                    }
                };
                let ea = self.operand(addr)?;
                if let Ea::Reg(Reg::A(a)) = ea {
                    // A pointer in an address register against `nil`:
                    // `cmpa.w #0,An` (codegen-patterns 7.1: 151 sites,
                    // 0xE0243C; 7.4: 45 `while p <> nil` loops test their
                    // pointer so, e.g. 0xE3CCE4).  Only `=`/`<>` are seen.
                    if !matches!(op, Rel::Eq | Rel::Ne) {
                        return Err(unsupported(
                            self.span,
                            "an ordering comparison of a pointer with nil is",
                        ));
                    }
                    self.out.push(Insn::CmpaZero(a));
                    return Ok(cc);
                }
                self.tst(size, ea)?;
                Ok(cc)
            }
            Expr::Const { value, .. } => {
                let ea = self.operand(addr)?;
                if let Ea::Reg(Reg::A(_)) = ea {
                    return Err(unsupported(
                        self.span,
                        "comparing an address register with a constant is",
                    ));
                }
                self.out.push(Insn::Cmpi {
                    size,
                    imm: imm(*value, ty.width),
                    dst: ea.clone(),
                });
                Ok(rel_cond(op, signed))
            }
            Expr::Load { addr: raddr, .. } => {
                let held = match addr {
                    Addr::Frame(o) => self.pinned(*o),
                    _ => None,
                };
                let Some(Reg::D(d)) = held else {
                    return Err(unsupported(
                        self.span,
                        "comparing two variables when the left one is not held in a data \
                         register is",
                    ));
                };
                let src = self.operand(raddr)?;
                if matches!(src, Ea::Reg(_)) {
                    return Err(unsupported(
                        self.span,
                        "comparing two values held in registers (operand order not known) is",
                    ));
                }
                self.out.push(Insn::Cmp {
                    size,
                    src: src.clone(),
                    dst: d,
                });
                Ok(rel_cond(op, signed))
            }
            _ => Err(unsupported(
                self.span,
                "a comparison with this kind of value is",
            )),
        }
    }

    /// A new virtual register of `class` (`regs.rs`); [`allocate`] gives
    /// it a physical one once the body is selected.  Statements of a body
    /// of several statements may take registers of their own: the value
    /// gets the lowest register free over its live range (codegen-patterns
    /// 12.4 R1), not one pinned by the body.
    fn alloc(&mut self, class: RegClass) -> Res<Reg> {
        self.vregs
            .new_reg(class)
            .ok_or_else(|| unsupported(self.span, "a routine needing this many registers is"))
    }

    fn alloc_d(&mut self) -> Res<u8> {
        match self.alloc(RegClass::Data)? {
            Reg::D(n) => Ok(n),
            Reg::A(_) => unreachable!(),
        }
    }

    fn alloc_a(&mut self) -> Res<u8> {
        match self.alloc(RegClass::Addr)? {
            Reg::A(n) => Ok(n),
            Reg::D(_) => unreachable!(),
        }
    }

    /// The register a by-value parameter at frame offset `o` was loaded
    /// into at entry, if it was.
    fn pinned(&self, o: i32) -> Option<Reg> {
        self.pinned.iter().find(|&&(p, _)| p == o).map(|&(_, r)| r)
    }

    /// Load the by-value parameters the body reads into registers, in
    /// parameter order, before the first statement.
    ///
    /// TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A:
    ///
    /// ```text
    /// move.b  (8,A6),D0      ; is_vt, read once, by the third statement
    /// move.w  (0xa,A6),D1    ; queue_id, read once, by the fourth
    /// movea.l (0xc,A6),A0    ; queue, the base of all four stores
    /// ```
    ///
    /// The same in other bodies of several statements: module-local
    /// 0xE0A328 (DMA_; `dma/init_m68450_channel.c`) at 0xE0A32C, `move.w
    /// (0xc,A6),D0`, used only by the last of five statements; module-local
    /// 0xE758C4 (RING) at 0xE758C8..0xE758D2, two longwords into D0, D1;
    /// MMAP_$SET_WS_MAX 0xE0CA88..0xE0CA90, SIO6509_$INQ_LINE
    /// 0xE7266C..0xE72674 (codegen-patterns 12.2).  Registers: the lowest
    /// register of the class free over the value's live range (the
    /// allocator, codegen-patterns 12.4 R1-R3): data values in D registers,
    /// pointers the body dereferences in A registers; D2/A2 and up for a
    /// third value of a class or a value live across a call (module-local
    /// 0xE32B76; PROC1_$SET_TYPE 0xE152F2).  A known miss of that rule:
    /// MST_$VA_TO_SEGNO 0xE0DCD6, D2 for the first parameter (12.4 corpus,
    /// R5); ROUTE_$FIND_PORT 0xE15B06 (D3, 12.2) has not been checked
    /// against it.  A pointer also used as data is not supported yet.
    ///
    /// **By-address parameters** (the slot holds the argument's address)
    /// that the body reaches through more than once are loaded the same
    /// way, in parameter order with the by-value ones, into an address
    /// register that holds the address up to its last use (bead
    /// domain-pascal-b1v; codegen-patterns 12.2):
    ///
    /// - LOG_$READ's worker 0xE17780..0xE17788 `move.w (0xc,A6),D0; move.w
    ///   (0xe,A6),D1; movea.l (0x10,A6),A0`: `actual_len`, by address and
    ///   used nine times, after the two words; `buffer` (8,A6), used once,
    ///   is not loaded at entry (0xE177CC);
    /// - MMAP_$GET_WS_INDEX 0xE0CA48..0xE0CA4C `move.w (8,A6),D0; movea.l
    ///   (0xe,A6),A0` (used twice); (0xa,A6), used once, is loaded late
    ///   (0xE0CA62);
    /// - NAME_CONVERT_ACL_STATUS 0xE58620 (six uses), PEB_$TEST_PARITY_ERR
    ///   0xE12000.
    ///
    /// Not so DMA_$INIT_M68450_CHANNEL 0xE0A328, which reloads its slot
    /// before each of five stores; they are device-register stores
    /// (`dma/init_m68450_channel.c`), and `dpc-sema` rejects the `device`
    /// attribute (codegen-patterns 13).  A slot used once is loaded where
    /// it is needed, into a register no earlier statement left a value in
    /// (IIC_$RECEIVE 0xE70AC0..0xE70AD0 A0, A1, A2; PROC2_$ALIGN_CTL
    /// 0xE41D82; [`fresh_slot_loads`]), in bodies without branches and
    /// calls only.
    fn entry_loads(&mut self, f: &Function) -> Res<()> {
        let uses = param_uses(f);
        // (offset, IR type when by value), in parameter order.
        let mut params: Vec<(i32, Option<Scalar>)> = f
            .value_params
            .iter()
            .map(|p| (p.offset, Some(p.ty)))
            .chain(f.address_params.iter().map(|&o| (o, None)))
            .collect();
        params.sort_by_key(|&(o, _)| o);
        // A parameter a nested routine reaches is read from the frame at
        // each use (PMAP_$FLUSH 0xE1376C: its `(8,A6)` slot, read by
        // 0xE1359C as `(8,A2)`, reloaded at 0xE1378A, 0xE1379C, 0xE139AE,
        // 0xE139CA; `(0x16,A6)`, read by 0xE1360C, at 0xE1377E, 0xE1385A,
        // 0xE13916, ...; while `(0x12,A6)`, which no nested routine reads,
        // is loaded into D5 at entry, 0xE1377A.  RIP_$UPDATE_INT 0xE15922
        // likewise loads `(8)`, `(0x10)` and `(0x16)` but tests its byte
        // `(0x14,A6)`, read by its nested 0xE15888 as `(0x14,A1)`, in the
        // frame, 0xE15962 and 0xE159C4).
        params.retain(|&(o, _)| !self.is_uplevel(o));
        // A parameter whose address the body passes to a `var`, `out` or
        // `in out` parameter, or takes with `addr`, may change behind a
        // register: it is read from the frame at each use (bead
        // domain-pascal-pe9).  AREA_$TOUCH 0xE094FE loads its `(8)`,
        // `(0xe)` and `(0x16)` slots at entry (0xE0950C..0xE09514) but not
        // `(0xc,A6)`, which it passes by address to the module-local
        // 0xE0939C (`area_$find_entry_by_uid`, a `var` word it rewrites)
        // at 0xE095A6 and reads after that call, 0xE095BA `move.w
        // (0xc,A6),D4`.  That the original then held it in D4 (a load
        // after the last such call) is not reproduced: `dpc` reads it
        // from the frame at every use, or refuses where that code is not
        // known (codegen-patterns 12.2).
        let escaped = escaped_frame_objects(f);
        params.retain(|&(o, ty)| {
            let n = ty.map_or(4, |t| t.bytes() as i32);
            !escaped.iter().any(|&e| o <= e && e < o + n)
        });
        for (offset, ty) in params {
            let mine: Vec<ParamUse> = uses
                .iter()
                .filter(|(o, _)| *o == offset)
                .map(|&(_, u)| u)
                .collect();
            let Some(&first) = mine.first() else {
                continue;
            };
            if ty.is_none() && mine.len() < 2 {
                continue;
            }
            if mine.iter().any(|&u| u != first) {
                // ACL_$ADD_PROJ 0xE47EC0 / EC2_$INIT 0xE42C64: a pointer
                // that is also data is kept in a D register and copied to
                // an A register for each dereference.
                return Err(unsupported(
                    f.span,
                    "a parameter used both as a pointer and as a value is",
                ));
            }
            let class = match first {
                ParamUse::Data => RegClass::Data,
                ParamUse::Base => RegClass::Addr,
            };
            // Each load is a new value; the allocator gives it the lowest
            // register free over its live range (codegen-patterns 12.4 R1):
            // D0, D1, A0, A1 for the sites above, D2/A2 and up for a third
            // value of a class or for one live across a call (R3:
            // module-local 0xE32B76 (`sio/init_dtte.c`), 0xE32B7E `movea.l
            // (8,A6),A2` and 0xE32B82 `move.w (0xc,A6),D2` around three
            // `jsr EC_$INIT`).
            let r = self.alloc(class)?;
            let src = Ea::Disp(offset, FP);
            self.out.push(match r {
                Reg::D(_) => Insn::Move {
                    size: size_of(ty.expect("only by-value parameters are data").width),
                    src,
                    dst: Ea::Reg(r),
                },
                Reg::A(a) => Insn::Movea {
                    size: Size::L,
                    src,
                    dst: a,
                },
            });
            self.pinned.push((offset, r));
        }
        Ok(())
    }

    /// Make the unit's data area addressable: `lea <area>.l,A5` as the
    /// first instruction of the body, right after the register save
    /// (codegen-patterns 4: PROC1_$GET_LOADAV 0xE14BBE..0xE14BC0 `pea (A5);
    /// lea 0x00E254E8,A5`; MAC_OS_$OPEN 0xE0B24A `movem.l
    /// D2-D7/A2-A5,-(SP); lea 0x00E22990,A5`, before the parameter loads;
    /// XNS_IDP_$REGISTER_ADDR 0xE19006..0xE19010 likewise).  The address
    /// is a relocated reference to the start of the area
    /// ([`DATA_BASE_LABEL`]).  A5 joins the save set, so the frame saves
    /// it with `pea (A5)` or inside the `movem` (`frame.rs`).
    fn load_data_base(&mut self) {
        self.out.push(Insn::Lea {
            src: Ea::AbsL(DATA_BASE_LABEL.into(), 0),
            dst: DATA_BASE_A,
        });
        self.clobbered.insert(DATA_BASE);
    }

    /// `d := s` for a record or array of `bytes` bytes
    /// (codegen-patterns 5.2).
    ///
    /// Survey (compiled code, 0xE00800..0xE88834): block copies with
    /// `move.<s> (Am)+,(An)+`, after the source address is put in one
    /// address register and the destination address in another.  By size:
    ///
    /// - 10 to 18 bytes: unrolled, one `move.l` per longword and a
    ///   `move.w` for a final word: runs of 2, 3 and 4 `move.l` (15, 59
    ///   and 42 sites), every run of 2 followed by a `move.w`.
    ///   PROC1_$GET_LOADAV 0xE14BC6..0xE14BD0 (12 bytes): `lea (A5),A0;
    ///   movea.l (8,A6),A1; move.l (A0)+,(A1)+` ×3.  10 bytes: 0xE307FE;
    ///   14: 0xE419B8; 18: `move.l` ×4 then `move.w`, 3 sites with A0/A1.
    /// - 20 bytes and more: a `dbf` loop of `move.l` with a count of the
    ///   longwords less one (`moveq #4` the smallest of 237 loops), then a
    ///   `move.w` for a final word (37 loops, e.g. 0xE19DA2..0xE19DAA,
    ///   `moveq #4`: 22 bytes).
    ///   AST_ 0xE01494..0xE014A2: `lea (0xc,A2),A0; lea (-0x98,A6),A1;
    ///   moveq #35,D0; movea.l A0,A0; move.l (A0)+,(A1)+; dbf D0`.  The
    ///   count is loaded after both addresses (118 loops; 26 load it
    ///   earlier, mostly run-time counts), and a count above 127 is
    ///   `move.w #n` (0xE3B97A `move.w #130,D0`).  The loop head is
    ///   4-aligned with the 0x2048 filler (codegen-patterns 7), as at
    ///   0xE0149E.  **Uncertain:** 50 of 240 such loops start at 2 mod 4
    ///   with no filler, in compiled code too (0xE1155E, 0xE496C2); which
    ///   loops the original aligned is not known (codegen-patterns 5.2).
    /// - 6 and 8 bytes: no `(An)+` copy of exactly 8 bytes, nor of 6, was
    ///   found; such blocks are copied with displacement moves, `move.l
    ///   (0x30,A5),(-8,A6); move.l (0x34,A5),(-4,A6)` (0xE4AD5C) and many
    ///   `move.l`/`move.w` pairs.  Not supported yet: which operands the
    ///   original formed first has not been catalogued.
    /// - 12 bytes as displacement moves, XNS_IDP_$GET_STATS 0xE18FE6
    ///   `move.l (A5),(A0); move.l (4,A5),(4,A0); move.l (8,A5),(8,A0)`:
    ///   2 sites against 59 `(An)+` copies.  Taken to be three field
    ///   assignments in the source, not one record assignment
    ///   (**Uncertain**: the image cannot tell).
    ///
    /// Address registers: source first, then destination, each the lowest
    /// free (A0, A1 at 0xE14BC6..0xE14BC8; codegen-patterns 12.4 R1).
    fn copy(&mut self, dst: &Addr, src: &Addr, bytes: u32) -> Res<()> {
        if bytes % 2 != 0 || bytes < 10 {
            return Err(unsupported(
                self.span,
                format!("a {bytes}-byte record or array assignment is"),
            ));
        }
        let longs = bytes / 4;
        if longs - 1 > 0x7fff {
            return Err(unsupported(self.span, "a block copy this large is"));
        }
        let from = self.block_address(src)?;
        let to = self.block_address(dst)?;
        let mv = |size| Insn::Move {
            size,
            src: Ea::PostInc(from),
            dst: Ea::PostInc(to),
        };
        let start = self.out.len();
        if bytes <= 18 {
            for _ in 0..longs {
                self.out.push(mv(Size::L));
            }
        } else {
            let count = self.alloc_d()?;
            let n = longs - 1;
            self.out.push(if n <= 127 {
                Insn::Moveq {
                    value: n as i8,
                    dst: count,
                }
            } else {
                Insn::Move {
                    size: Size::W,
                    src: Ea::Imm(n as i64),
                    dst: Ea::Reg(Reg::D(count)),
                }
            });
            let head = self.new_label();
            self.out.push(Insn::Align4);
            self.place(head);
            self.out.push(mv(Size::L));
            self.out.push(Insn::Dbf {
                reg: count,
                label: head,
            });
        }
        if bytes % 4 == 2 {
            self.out.push(mv(Size::W));
        }
        if bytes <= 18 {
            // An unrolled copy is never interleaved with other code
            // (codegen-patterns 12.4 R10: 0xE04ADA..0xE04AE2).
            self.units.push((start, self.out.len()));
        }
        Ok(())
    }

    /// The address of a block in a new address register.
    ///
    /// - in the unit's data area: `lea (d,A5),An`, and `lea (A5),An` for
    ///   offset 0, not `movea.l A5,An` (PROC1_$GET_LOADAV 0xE14BC6;
    ///   0xE1AD52 `lea (0x14,A5),A0`; 0xE3E5CA `lea (A5),A0`);
    /// - in the frame: `lea (d,A6),An` (0xE01498 `lea (-0x98,A6),A1`);
    /// - the object a pointer in the frame points to (a by-address
    ///   parameter, a pointer variable): the pointer itself, `movea.l
    ///   (d,A6),An` (0xE14BC8 `movea.l (8,A6),A1`; 0xE2F148 `movea.l
    ///   (8,A6),A0` as a source).
    ///
    /// Other places (another unit's object, `movea.l #abs,A1` at 0xE336E2
    /// once; a field at a nonzero offset through a pointer) are not
    /// supported yet.
    fn block_address(&mut self, addr: &Addr) -> Res<u8> {
        let src = match addr {
            Addr::Static(_) | Addr::Frame(_) => self.ea_of(addr)?,
            Addr::Indirect { base, offset: 0 } => match &**base {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ty:
                        Scalar {
                            width: Width::Long, ..
                        },
                } if self.pinned(*o).is_none() => {
                    let a = self.alloc_a()?;
                    self.out.push(Insn::Movea {
                        size: Size::L,
                        src: Ea::Disp(*o, FP),
                        dst: a,
                    });
                    return Ok(a);
                }
                _ => {
                    return Err(unsupported(
                        self.span,
                        "copying a block through this pointer expression is",
                    ))
                }
            },
            _ => {
                return Err(unsupported(
                    self.span,
                    "copying a block at this kind of address is",
                ))
            }
        };
        let a = self.alloc_a()?;
        self.out.push(Insn::Lea { src, dst: a });
        Ok(a)
    }

    /// `d := e`: one move into the destination's effective address.
    ///
    /// The value, by kind:
    ///
    /// - zero: `clr.<s> <ea>` (TIME_$Q_INIT_QUEUE 0xE16C6E `clr.l (A0)`,
    ///   0xE16C70 `clr.l (4,A0)`; `clr.w` IIC_$RECEIVE 0xE70ACE; `clr.b`
    ///   XPD_$RESET_PTRACE_OPTS 0xE5B15E).  Device registers are cleared
    ///   with `move.b #0` instead (WIN_ module 0xE19112, the controller's
    ///   GO register per `win/wait_for_controller.c`; RING 0xE758E2), which
    ///   fits ref 3.15.3 (p. 3-57): the `device` attribute stops "gratuitous
    ///   read-modify-write references", and `clr` reads its operand on the
    ///   68000.  How those objects were declared is not known; `dpc-sema`
    ///   rejects `device`, `volatile` and `atomic` (ref 3.15), so no
    ///   store reaching here is to such an object.
    /// - boolean true: `st <ea>` (SMD_$ENABLE_TRACKING 0xE6E468 `st
    ///   (0xe0,A5)`, a boolean field per `smd/smd_internal.h`).
    /// - another constant: `move.<s> #n,<ea>` (module-local 0xE0A328 at
    ///   0xE0A334, `move.b #0x10,(7,A0)`; MMAP_$ALLOC_CONTIG 0xE0D906
    ///   `move.l #0x6000e,(A1)`; 0xE758D8 `move.w #0x200,(0xa,A0)`).
    /// - a by-value parameter loaded at entry: from its register
    ///   (0xE16C74 `move.b D0,(8,A0)`, 0xE16C78 `move.w D1,(0xa,A0)`).
    /// - another object in memory: memory to memory (PROC2_$GET_BOOT_FLAGS
    ///   0xE41B66 `move.w (0x1e4,A5),(A0)`; CAL_$SET_DRIFT 0xE68588
    ///   `move.l (A0),abs.l`; TIME_$Q_REMOVE_ELEM 0xE16E88 `move.l
    ///   (-4,A6),(A0)`).  A parameter that was not loaded at entry is not
    ///   a source yet: in a one-statement body its code is not known.
    /// - anything else: computed into a data register first.
    ///
    /// When both the destination and the value need a register of their
    /// own, which one the original loaded first is not known: rejected.
    fn store(&mut self, addr: &Addr, value: &Expr) -> Res<()> {
        let ty = value.ty();
        let size = size_of(ty.width);
        if matches!(addr, Addr::Frame(o) | Addr::Enclosing(o) if *o > 0) {
            return Err(unsupported(self.span, "assigning to a parameter is"));
        }
        if let Addr::Frame(o) = addr {
            if let Some(k) = self.ptr_locals.iter().position(|&(p, _)| p == *o) {
                // A pointer local held in an address register for the whole
                // body ([`loop_pointer_locals`]): every assignment writes
                // that register, a load straight into it (0xE3CCDA
                // `movea.l D0,A1`; 0xE3CCE2 `movea.l (A1),A1`, `r :=
                // r^.next`).
                let a = match self.ptr_locals[k].1 {
                    Some(a) => a,
                    None => {
                        let a = self.alloc_a()?;
                        self.ptr_locals[k].1 = Some(a);
                        a
                    }
                };
                let src = match value {
                    Expr::Load { addr: src, .. } => self.operand(src)?,
                    // A pointer function's result: `movea.l A0,An`
                    // (codegen-patterns 3, "Callers of pointer functions":
                    // KBD_$SET_KBD_TYPE 0xE7253E `movea.l A0,A2`).
                    v if a0_call(v).is_some() => {
                        Ea::Reg(Reg::A(self.call_a0(a0_call(v).unwrap())?))
                    }
                    v => match self.eval(v)? {
                        Val::Reg(r) => Ea::Reg(r),
                        Val::Tmp(d) => Ea::Reg(Reg::D(d)),
                        Val::Mem(ea) => ea,
                        Val::Imm(_) => {
                            return Err(unsupported(
                                self.span,
                                "a constant assigned to a pointer held in a register is",
                            ))
                        }
                    },
                };
                self.ptr_defs.push((*o, self.out.len(), self.span));
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src,
                    dst: a,
                });
                if self.pinned(*o).is_none() {
                    self.pinned.push((*o, Reg::A(a)));
                }
                return Ok(());
            }
            if self.reg_locals.contains(o) {
                // A local held in a register (codegen-patterns 12.5): its
                // value is computed into, or loaded into, a new data
                // register, which then stands for the variable
                // (TIME_$GET_ADJUST 0xE16AB4 `move.l (0xe2b0fc).l,D1`;
                // PROC1_$SET_VT 0xE149B6 `move.w SR,D3`).
                // A pointer function's result: `move.l A0,Dn` (TTY_$K_*
                // after TTY_$I_GET_DESC, e.g. 0xE6763C `move.l A0,D0`;
                // codegen-patterns 3, "Callers of pointer functions").
                let v = match a0_call(value) {
                    Some(c) => Val::Reg(Reg::A(self.call_a0(c)?)),
                    None => self.eval(value)?,
                };
                let d = match v {
                    Val::Tmp(d) => d,
                    v => self.in_tmp(v, ty.width)?,
                };
                self.pinned.push((*o, Reg::D(d)));
                return Ok(());
            }
        }
        if self.update_in_place(addr, value)? {
            return Ok(());
        }
        if self.addr_needs_reg(addr) && self.value_needs_reg(value) {
            return Err(unsupported(
                self.span,
                "a store whose destination and value both need a register is",
            ));
        }
        enum Src {
            Zero,
            True,
            Ea(Ea),
        }
        let src = match value {
            Expr::Const { value: 0, .. } => Src::Zero,
            Expr::Const { value: -1, ty } if *ty == Scalar::BOOLEAN => Src::True,
            Expr::Const { value, ty } => Src::Ea(Ea::Imm(imm(*value, ty.width))),
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if *o > 0 || self.pinned(*o).is_some() => match self.pinned(*o) {
                Some(r) => Src::Ea(Ea::Reg(r)),
                None => {
                    return Err(unsupported(
                        self.span,
                        "storing a parameter that was not loaded at entry is",
                    ))
                }
            },
            Expr::Load { addr: a, .. } => Src::Ea(self.ea_of(a)?),
            // A comparison stored as a boolean: whether the original set
            // the byte in memory with `s<cc>` directly is not catalogued.
            Expr::Test(_) => {
                return Err(unsupported(
                    self.span,
                    "storing a computed boolean (s<cc> to memory or through a register) is",
                ))
            }
            // A pointer function's result, from A0 (codegen-patterns 3,
            // "Callers of pointer functions": TIME_$GET_EC 0xE16734
            // `move.l A0,(0x1620,A5)`; AST_$INVALIDATE 0xE06664 `move.l
            // A0,(-20,A6)`; PEB_$LOAD_WCS 0xE32072 `move.l A0,abs.l`).
            e if a0_call(e).is_some() => {
                Src::Ea(Ea::Reg(Reg::A(self.call_a0(a0_call(e).unwrap())?)))
            }
            e => match self.eval(e)? {
                // A held value or a function's result is stored from its
                // register (0xE04704 `move.l D0,(A3)` after `bsr.w`;
                // 0xE16C74 `move.b D0,(8,A0)`).
                Val::Reg(r) => Src::Ea(Ea::Reg(r)),
                v => Src::Ea(Ea::Reg(Reg::D(self.in_tmp(v, ty.width)?))),
            },
        };
        let dst = self.ea_of(addr)?;
        self.out.push(match &src {
            Src::Zero => Insn::Clr {
                size,
                dst: dst.clone(),
            },
            Src::True => Insn::Scc {
                cond: Cond::T,
                dst: dst.clone(),
            },
            Src::Ea(ea) => Insn::Move {
                size,
                src: ea.clone(),
                dst: dst.clone(),
            },
        });
        if let (Addr::Frame(o), Src::Ea(Ea::Reg(r))) = (addr, &src) {
            // Not a pointer function's A0: whether a later push of the
            // local is made from A0 is not catalogued (12.5 counts data
            // registers only).
            if *o < 0 && self.loops.is_empty() && !self.escaped.contains(o) && *r != Reg::A(0) {
                self.stored.push((*o, size, *r, self.out.len()));
            }
        }
        Ok(())
    }

    /// The register that still holds the value the body last stored into
    /// the local at frame offset `o` from a register, for an argument push
    /// at `size` (codegen-patterns 12.5, "Stored locals pushed from the
    /// register"): the spin-lock token `ML_$SPIN_LOCK` returns, stored
    /// with `move.w D0,(-k,A6)`, is pushed for `ML_$SPIN_UNLOCK` from D0
    /// at all 33 sites where nothing wrote D0 in between, across `if`s
    /// (PKT_$NEXT_ID 0xE124A8 / 0xE124C4; NETBUF_$GETVA 0xE0ECAC), and
    /// from the slot at all 18 where a call came between (NETBUF_$RTNVA
    /// 0xE0ED40 / 0xE0ED52).
    ///
    /// Only when every path from the store reaches the push through the
    /// instructions in between: no label there that a branch from before
    /// the store targets, neither the store nor the push inside a loop;
    /// no call there, no write of the register, no other store to the
    /// slot ([`may_write_frame`]: none to the frame at a run-time index,
    /// none through a pointer when any frame object's address escapes),
    /// and the local's address never taken ([`escaped_frame_objects`]).
    fn forwarded(&self, o: i32, size: Size) -> Option<Reg> {
        if !self.loops.is_empty() {
            return None;
        }
        let &(_, sz, r, at) = self.stored.iter().rev().find(|s| s.0 == o)?;
        if sz != size {
            return None;
        }
        let region = &self.out[at..];
        let inside: Vec<u32> = region
            .iter()
            .filter_map(|i| match i {
                Insn::Label(l) => Some(*l),
                _ => None,
            })
            .collect();
        let from_before = self.out[..at].iter().any(|i| match i {
            Insn::Branch { label, .. } | Insn::Dbf { label, .. } => inside.contains(label),
            Insn::TableJump { targets, .. } => targets.iter().any(|l| inside.contains(l)),
            Insn::TableEntry { target, .. } => inside.contains(target),
            _ => false,
        });
        let clobbered = region.iter().any(|i| {
            matches!(
                i,
                Insn::Jsr(_)
                    | Insn::Bsr { .. }
                    | Insn::Pea(Ea::Disp(_, FP))
                    | Insn::Lea {
                        src: Ea::Disp(_, FP),
                        ..
                    }
            ) || crate::regs::access(i).defs.contains(&r)
                || may_write_frame(i, o, !self.escaped.is_empty())
        });
        (!from_before && !clobbered).then_some(r)
    }

    /// Whether A0 still holds the pointer local at frame offset `o` at the
    /// end of the instructions selected so far, on every path there: a
    /// pointer function's result stored into the slot (`move.l
    /// A0,(-k,A6)`), or the slot loaded into A0, with no call, write of A0,
    /// possible write of the slot ([`may_write_frame`]) or frame address
    /// taken after it (bead domain-pascal-500; codegen-patterns 3,
    /// "Callers of pointer functions").
    ///
    /// The image dereferences such a local through A0 right after the
    /// store (the DIR_ module-locals 0xE4DC78 `tst.w (A0)`, 0xE53B06 `and.b
    /// (A0),D0`, 0xE53E16 `andi.b #-64,(1,A0)`), on the path of the `nil`
    /// test's branch (AST_$RESERVE 0xE067E2 and AST_$INVALIDATE 0xE066A8
    /// `bset #6,(191,A0)`), and after the join of the two assignments of
    /// `p := f; if p = nil then p := g` (AST_$RESERVE 0xE067E8 `tst.b
    /// (185,A0)`; AST_$TRUNCATE 0xE05D1E), except for a local a nested
    /// routine reaches, which is reloaded at that join (AST_$INVALIDATE
    /// 0xE066AE `movea.l (-20,A6),A0`; the AST_ module-local 0xE05214 at
    /// 0xE052C4 into A1): for such a local a label with more than one
    /// predecessor ends it.  Not inside a loop (its back edge is not
    /// selected yet).
    fn a0_holds(&self, o: i32) -> bool {
        if !self.loops.is_empty() {
            return false;
        }
        let uplevel = self.is_uplevel(o);
        let escaped = !self.escaped.is_empty();
        // The instructions so far and a point after them.
        let mut code = self.out.clone();
        code.push(Insn::Label(u32::MAX));
        let n = code.len();
        let succ = crate::regs::successors(&code);
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, ss) in succ.iter().enumerate() {
            for j in ss.iter().flatten() {
                preds[*j].push(i);
            }
        }
        let transfer = |i: usize, inn: bool| -> bool {
            match &code[i] {
                Insn::Move {
                    size: Size::L,
                    src: Ea::Reg(Reg::A(0)),
                    dst: Ea::Disp(d, FP),
                } if *d == o => true,
                Insn::Movea {
                    size: Size::L,
                    src: Ea::Disp(d, FP),
                    dst: 0,
                } if *d == o => true,
                x if matches!(
                    x,
                    Insn::Jsr(_)
                        | Insn::Bsr { .. }
                        | Insn::Pea(Ea::Disp(_, FP))
                        | Insn::Lea {
                            src: Ea::Disp(_, FP),
                            ..
                        }
                ) || crate::regs::access(x).defs.contains(&Reg::A(0))
                    || may_write_frame(x, o, escaped) =>
                {
                    false
                }
                _ => inn,
            }
        };
        // Must-analysis: every path from the entry.
        let mut out = vec![true; n];
        loop {
            let mut changed = false;
            for i in 0..n {
                // The entry, and for a nested-reached local any join.
                let cut =
                    i == 0 || uplevel && matches!(code[i], Insn::Label(_)) && preds[i].len() > 1;
                let inn = !cut && preds[i].iter().all(|&p| out[p]);
                let v = transfer(i, inn);
                if v != out[i] {
                    out[i] = v;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        out[n - 1]
    }

    /// Record that the statement just selected (from `stmt_start`) wrote
    /// `bytes` bytes at `addr` ([`Self::cse_valid`]).
    fn wrote(&mut self, addr: &Addr, bytes: u32) {
        self.writes
            .push((self.stmt_start, self.out.len(), addr.clone(), bytes));
    }

    /// The plain variable `e` reads, and its size, when a computation of
    /// it may be reused (codegen-patterns 12.4 R7): a by-value parameter
    /// (loaded at entry or read from its slot), a local in the frame, a
    /// variable of the unit's data or another unit's.  Not a variable of
    /// the enclosing routine (re-read even within one statement, R8:
    /// 0xE03FEE/0xE03FF8, 0xE135DA/0xE135E2, MST_ 0xE30B5C), nor a local
    /// held in a register (12.5) or reached by a nested routine, nor an
    /// element or a field reached through a pointer (not surveyed).
    fn cse_leaf(&self, e: &Expr) -> Option<(Addr, u32)> {
        let Expr::Load { addr, ty } = e else {
            return None;
        };
        match addr {
            Addr::Frame(o) if *o < 0 && self.pinned(*o).is_some() => None,
            Addr::Frame(o)
                if self.reg_locals.contains(o)
                    || self.ptr_locals.iter().any(|&(p, _)| p == *o)
                    || self.is_uplevel(*o) =>
            {
                None
            }
            Addr::Frame(_) | Addr::Static(_) | Addr::Symbol { .. } => {
                Some((addr.clone(), ty.bytes()))
            }
            _ => None,
        }
    }

    /// A computation `key` of the plain variable `leaf`, reusable
    /// (codegen-patterns 12.4 R7): `compute` makes it into a new data
    /// register.
    ///
    /// The image reuses such values across statements, across an `if` and
    /// its join and past stores to other objects (`tools/csesurvey.py`):
    /// the zero-extended `offset` of 0xE177A6 at 0xE177D0, after the `if`
    /// and the stores through `actual_len`; FIM_$ACKNOWLEDGE's
    /// `PROC1_$AS_ID * 12` (0xE0A972..0xE0A97E) at 0xE0A9B0, past the
    /// stores to two other arrays at 0xE0A994 and 0xE0A9A6.  When a later
    /// computation of the same value finds an earlier one valid
    /// ([`Self::cse_valid`]), the earlier one is held in its register (the
    /// two-address operations of its users then work on the other operand
    /// or a copy, 0xE177B4 `add.l D1,D2`, 0xE177D0 `move.l D1,D3`) and the
    /// later one reads it.  Everything else is computed again, as before.
    fn cse_value(
        &mut self,
        key: CseKey,
        leaf: (Addr, u32),
        compute: impl FnOnce(&mut Self) -> Res<u8>,
    ) -> Res<Val> {
        let id = self.cse_next;
        self.cse_next += 1;
        let (reuse, hold) = match &self.cse_plan {
            CsePlan::Probe => (None, false),
            CsePlan::Hold(hits) => (
                hits.iter().find(|h| h.0 == id).map(|h| h.1),
                hits.iter().any(|h| h.1 == id),
            ),
        };
        if let Some(def) = reuse {
            let c = self
                .cse
                .iter()
                .find(|c| c.id == def)
                .cloned()
                .filter(|c| c.key == key && self.cse_valid(c));
            let Some(c) = c else {
                return Err(unsupported(
                    self.span,
                    "a reuse of a computed value that is no longer valid when held                      (codegen-patterns 12.4 R7) is",
                ));
            };
            self.cse_hits.push((id, def));
            return Ok(Val::Reg(Reg::D(c.reg)));
        }
        if let CsePlan::Probe = self.cse_plan {
            let found = self
                .cse
                .iter()
                .rev()
                .find(|c| c.key == key && self.cse_valid(c))
                .map(|c| c.id);
            if let Some(def) = found {
                // Computed again here, as before, but the hold selection
                // reuses it.  Not itself a computation to reuse: a later
                // one reuses the first, as the hold selection does.
                self.cse_hits.push((id, def));
                return Ok(Val::Tmp(compute(self)?));
            }
        }
        let d = compute(self)?;
        if self.loops.is_empty() {
            self.cse.push(Cse {
                key,
                leaf,
                id,
                reg: d,
                at: self.out.len(),
            });
        }
        if hold {
            self.cse_regs.push(d);
            return Ok(Val::Reg(Reg::D(d)));
        }
        Ok(Val::Tmp(d))
    }

    /// Whether the computation `c` still holds its value here, for every
    /// path that reaches here (codegen-patterns 12.4 R7):
    ///
    /// - neither it nor this point is inside a loop (a value computed
    ///   before a loop is reused after it, past its back edge, as in the
    ///   image: module-local 0xE1C9FC's `n * 4`, D1 from 0xE1CA0C, read
    ///   at 0xE1CA4A after the `dbf` loop 0xE1CA2C..0xE1CA46; whether the
    ///   image reuses one inside a loop is not checked), and no label
    ///   between them is the target of a
    ///   branch or `case` table from before it (so it dominates this
    ///   point: a value computed in an `if`'s condition is valid after
    ///   the `if`, one computed in a `then` part is not);
    /// - no call between (the image mostly computes again after a call:
    ///   by `tools/csesurvey.py agg`, sums 16 reused and 74 computed
    ///   again, differences 9 and 88, shifts 8 and 24; when it keeps one
    ///   in a callee-saved register is not known);
    /// - no store between that may write the variable it reads: one to
    ///   the variable itself (or to an element of the same array, frame
    ///   or unit data at a run-time index), or through a pointer when the
    ///   variable is in the unit's data or another unit's (the image
    ///   computes again after such stores: 0xE53462, 0xE6E924, 0xE71C0E,
    ///   and 0xE4B388 after a `bchg` of the variable's low byte) or in the
    ///   frame and some frame object's address escaped (as for
    ///   [`Self::forwarded`]).  Stores to other named objects do not
    ///   count (0xE0A9A6 before 0xE0A9B0), nor stores through a pointer
    ///   parameter when the variable is a parameter or a local (0xE177C0,
    ///   0xE177CA before 0xE177D0).  An instruction that writes memory
    ///   outside a recorded store statement counts as a possible write;
    /// - when it is held, its register not written since (an internal
    ///   check; in the probe its users modify it in place).
    fn cse_valid(&self, c: &Cse) -> bool {
        if !self.loops.is_empty() || c.at > self.out.len() {
            return false;
        }
        let region = &self.out[c.at..];
        let inside: Vec<u32> = region
            .iter()
            .filter_map(|i| match i {
                Insn::Label(l) => Some(*l),
                _ => None,
            })
            .collect();
        let from_before = self.out[..c.at].iter().any(|i| match i {
            Insn::Branch { label, .. } | Insn::Dbf { label, .. } => inside.contains(label),
            Insn::TableJump { targets, .. } => targets.iter().any(|l| inside.contains(l)),
            Insn::TableEntry { target, .. } => inside.contains(target),
            _ => false,
        });
        if from_before {
            return false;
        }
        let escaped = !self.escaped.is_empty();
        // A store statement since the computation that may write the
        // variable, whatever it emitted: one to a variable held in a
        // register writes no memory (no such store reaches here today:
        // register-held locals are not leaves, and [`Self::store`]
        // refuses assignments to parameters, which may be loaded at
        // entry).
        if self
            .writes
            .iter()
            .any(|w| w.1 > c.at && store_may_write(&w.2, w.3, &c.leaf.0, c.leaf.1, escaped))
        {
            return false;
        }
        region.iter().enumerate().all(|(k, i)| {
            let k = c.at + k;
            if matches!(
                i,
                Insn::Jsr(_) | Insn::Bsr { .. } | Insn::Words(_) | Insn::Encoded { .. }
            ) {
                return false;
            }
            // In the probe its users may still modify it in place.
            if self.cse_regs.contains(&c.reg)
                && crate::regs::access(i).defs.contains(&Reg::D(c.reg))
            {
                return false;
            }
            match mem_dst(i) {
                None | Some(Ea::PreDec(SP)) => true,
                Some(_) => self
                    .writes
                    .iter()
                    .find(|w| w.0 <= k && k < w.1)
                    .is_some_and(|w| !store_may_write(&w.2, w.3, &c.leaf.0, c.leaf.1, escaped)),
            }
        })
    }

    /// Whether forming `addr` takes a register that is not already
    /// holding the base.
    fn addr_needs_reg(&self, addr: &Addr) -> bool {
        match addr {
            // The static link is held in a register (codegen-patterns 6).
            Addr::Frame(_)
            | Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::Symbol { .. }
            | Addr::Const(_) => false,
            Addr::Index {
                base, index, scale, ..
            } => self.stepped_pointer(base, index, *scale).is_none(),
            Addr::Indirect { base, .. } => !matches!(
                &**base,
                Expr::Load { addr: Addr::Frame(o), .. } if self.pinned(*o).is_some()
            ),
        }
    }

    /// Whether `value` as a store's source takes a register of its own.
    fn value_needs_reg(&self, value: &Expr) -> bool {
        match value {
            Expr::Const { .. } => false,
            Expr::Load { addr, .. } => self.addr_needs_reg(addr),
            Expr::SetMember { .. }
            | Expr::AddrOf(_)
            | Expr::Binary { .. }
            | Expr::Unary { .. }
            | Expr::Convert { .. }
            | Expr::Call { .. }
            | Expr::SetSr(_)
            | Expr::Test(_) => true,
        }
    }

    /// A procedure call (codegen-patterns 1).
    ///
    /// 1. The pad, `subq.l #2,SP`, when the argument bytes are 2 mod 4
    ///    (codegen-patterns 1.4: AST_$PAGE_ZERO 0xE00EF0, before the
    ///    one-word argument of ML_$LOCK).
    /// 2. The arguments, right to left (codegen-patterns 1.1), each with
    ///    [`Self::push`]; two adjacent word constants are pushed as one
    ///    longword, the first parameter in the high half (0xE00EB4 `pea
    ///    (0x16).w` for MMU_$INSTALL's asid 0 and protection 0x16, whose
    ///    hand-written body reads them at (0x31,SP) and (0x33,SP),
    ///    `../source/domain_os/mmu/sau2/install.s`).  In the compiled code
    ///    0xE00800..0xE88834 no `move.w #n,-(SP)` / `clr.w -(SP)` is
    ///    directly followed by another one, except at a branch target
    ///    (0xE38B42/0xE38B44): every such pair was merged.  A run of
    ///    three word constants (which pair is merged?) is not supported
    ///    yet.
    /// 3. `jsr (sym).l` to another unit's routine (0xE00EF6), `bsr` to one
    ///    of this section (0xE00F02 `bsr.s`; the size is the emitter's,
    ///    `emit.rs`).
    /// 4. The caller pops the arguments and the pad: `addq.w #n,SP` up to
    ///    8 bytes, `lea (n,SP),SP` above (0xE00EFC `addq.w #4`, 0xE00EC8
    ///    `lea (0xc,SP),SP`; survey of compiled code: `addq.w` #4 ×1961, #6
    ///    ×1, #8 ×962; `lea` 12..68 bytes ×1850, never below 12).
    ///
    ///    **No pop** (`no_pop`) when every path from the call to the
    ///    epilogue, whose `unlk` discards the arguments, is free of
    ///    pushes, calls and conditional branches (`dbf` included); an
    ///    unconditional `bra` to a join may occur.  Survey of the
    ///    statement-level calls with arguments in compiled code
    ///    (codegen-patterns 1.1): 1135 such calls, none popped (0xE00F0C
    ///    `jsr ML_$UNLOCK` then `unlk` at 0xE00F12; PROC2_$UPGID_TO_UID
    ///    0xE41050, then two record copies and a status store; 0xE01166
    ///    `jsr ML_$UNLOCK; bra.s 0xE01176`, a join followed by stores);
    ///    every call with a push or a conditional branch on some path to the
    ///    epilogue is popped (2729 + 1742 + 240 calls, e.g. 0xE00BCA,
    ///    followed by an `if`).  The remaining 56 unpopped calls branch to
    ///    a pop shared at a join (see [`Self::if_`]).
    fn call(&mut self, c: &Call, no_pop: bool) -> Res<()> {
        // A value live across the call gets a callee-saved register from
        // the allocator (codegen-patterns 12.4 R3), and so may a value the
        // arguments read ([`held_through_calls`]).
        let start = self.out.len();
        match c.pad {
            0 => {}
            2 => self.out.push(Insn::Subq {
                size: Size::L,
                value: 2,
                dst: Ea::Reg(Reg::A(SP)),
            }),
            n => {
                return Err(unsupported(
                    self.span,
                    format!("an argument pad of {n} bytes is"),
                ))
            }
        }
        let word_const = |a: &Arg| match a {
            Arg::Value {
                value:
                    Expr::Const {
                        value,
                        ty:
                            Scalar {
                                width: Width::Word, ..
                            },
                    },
                slot: 2,
            } => Some(*value),
            _ => None,
        };
        let mut k = c.args.len();
        while k > 0 {
            let a = &c.args[k - 1];
            if k >= 2 {
                if let (Some(lo), Some(hi)) = (word_const(a), word_const(&c.args[k - 2])) {
                    if k >= 3 && word_const(&c.args[k - 3]).is_some() {
                        return Err(unsupported(
                            self.span,
                            "three or more adjacent word constant arguments are",
                        ));
                    }
                    let merged = ((hi & 0xffff) << 16) | (lo & 0xffff);
                    self.push_long_const(merged as i32 as i64);
                    k -= 2;
                    continue;
                }
            }
            // The same constant cell as the argument just pushed: its
            // address again with `move.l (SP),-(SP)` (TERM_$SEND_KBD_STRING
            // 0xE1AB10 after `pea (0x18,PC)`; all 33 `pea (d,PC)` followed
            // by an argument naming the same cell in compiled code,
            // codegen-patterns 1.5, `tools/pcsurvey.py dups`).
            if let (
                Arg::Address {
                    addr: Addr::Const(c),
                    ..
                },
                Some(Arg::Address {
                    addr: Addr::Const(prev),
                    ..
                }),
            ) = (a, c.args.get(k))
            {
                if c == prev {
                    self.out.push(Insn::Move {
                        size: Size::L,
                        src: Ea::Ind(SP),
                        dst: Ea::PreDec(SP),
                    });
                    k -= 1;
                    continue;
                }
            }
            self.push(a)?;
            k -= 1;
        }
        // The static link of a nested callee, last before the `bsr`
        // (codegen-patterns 6): the caller's frame, 0xE13928 `movea.l
        // A6,A1` (PMAP_$FLUSH calling 0xE1359C); or the caller's own link,
        // 0xE13718 `movea.l A4,A1` (the sibling 0xE1360C).
        match c.link {
            None => {}
            Some(LinkArg::OwnFrame) => self.out.push(Insn::Movea {
                size: Size::L,
                src: Ea::Reg(Reg::A(FP)),
                dst: STATIC_LINK_A,
            }),
            Some(LinkArg::OwnLink) => {
                let l = self
                    .link
                    .expect("the static link is loaded for a call passing it");
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src: Ea::Reg(Reg::A(l)),
                    dst: STATIC_LINK_A,
                });
            }
        }
        self.out.push(match &c.callee {
            Callee::External(sym) => Insn::Jsr(Ea::AbsL(sym.clone(), 0)),
            Callee::Local(sym) => Insn::Bsr {
                target: sym.clone(),
                short: true,
            },
        });
        self.call_stmts.push((start, self.out.len() - 1, self.span));
        let n = c.bytes();
        if !no_pop && n > 0 {
            self.out.push(if n <= 8 {
                Insn::Addq {
                    size: Size::W,
                    value: n as u8,
                    dst: Ea::Reg(Reg::A(SP)),
                }
            } else {
                Insn::Lea {
                    src: Ea::Disp(n as i32, SP),
                    dst: SP,
                }
            });
        }
        Ok(())
    }

    /// Push a longword constant (codegen-patterns 1.5): zero with `clr.l
    /// -(SP)` (374 sites in compiled code, e.g. 0xE135D0; 19 `move.l
    /// #0,-(SP)` remain unexplained), a value that sign-extends from 16
    /// bits with `pea (n).w` (128 sites, 0xE00EB4; no `move.l #n,-(SP)` of
    /// such a value other than 0), others with `move.l #n,-(SP)`.
    fn push_long_const(&mut self, v: i64) {
        let dst = Ea::PreDec(SP);
        self.out.push(if v == 0 {
            Insn::Clr { size: Size::L, dst }
        } else if (-0x8000..0x8000).contains(&v) {
            Insn::Pea(Ea::AbsW(v as i32))
        } else {
            Insn::Move {
                size: Size::L,
                src: Ea::Imm(v),
                dst,
            }
        });
    }

    /// Push one argument (codegen-patterns 1.3, 1.5).
    ///
    /// By value:
    /// - a longword constant: [`Self::push_long_const`];
    /// - a word constant: `clr.w -(SP)` for 0 (472 sites, e.g. 0xE135D2;
    ///   no `move.w #0,-(SP)`), else `move.w #n,-(SP)` (0xE00EF2);
    /// - boolean true: `st -(SP)` (0xE1365C; 196 sites); a byte 0
    ///   (`false`) clears its whole slot, `clr.w -(SP)` (RIP_$SEND_UPDATES
    ///   0xE688BC; the callee RIP_$BROADCAST reads the byte, 0xE872A6).
    ///   No `clr.b -(SP)` or `move.b #n,-(SP)` exists in compiled code, so
    ///   how other byte constants were pushed is not known: not supported
    ///   yet;
    /// - a value in memory: `move.<s> <ea>,-(SP)`, a byte with `move.b`
    ///   into the high half of its slot (0xE00EBE `move.l (8,A6),-(SP)`;
    ///   LOG_$READ 0xE17814 `move.w (A0),-(SP)`; KBD_$RCV 0xE1CE56 `move.b
    ///   D0,-(SP)`);
    /// - `addr(x)` of another unit's variable: `move.l #sym,-(SP)`
    ///   (0xE00EB8).
    ///
    /// By address:
    /// - a frame object: `pea (d,A6)` (PROC1_$BIND 0xE14DF4, 0xE14DF8);
    ///   an object in the unit's data area: `pea (d,A5)` (0xE02884);
    /// - another unit's variable: `move.l #sym,-(SP)` (UID_$NIL ×36,
    ///   TIME_$CLOCKH_EC ×24, ... : 362 of the 374 `move.l #abs,-(SP)` in
    ///   compiled code push a map symbol's own address).  An address
    ///   inside another unit's object is pushed both ways (12 `move.l #`,
    ///   e.g. 0xE32398 DIR_$MUTEX+0x14; 24 `pea abs.l`, e.g. 0xE0F5DC
    ///   ROUTE_$PORT+0x30): not supported yet;
    /// - the object a pointer in the frame points to (a by-address
    ///   parameter passed on): the pointer, `move.l (d,A6),-(SP)`
    ///   (LOG_$READ 0xE1780C); other objects reached through a pointer are
    ///   not catalogued yet.
    fn push(&mut self, a: &Arg) -> Res<()> {
        let dst = Ea::PreDec(SP);
        match a {
            Arg::Value { value, slot } => {
                if value.ty().bytes().max(2) != *slot {
                    return Err(unsupported(self.span, "an argument slot of this size is"));
                }
                match value {
                    Expr::Const { value: v, ty } => match ty.width {
                        Width::Long => self.push_long_const(*v),
                        Width::Word if *v == 0 => self.out.push(Insn::Clr { size: Size::W, dst }),
                        Width::Word => self.out.push(Insn::Move {
                            size: Size::W,
                            src: Ea::Imm(imm(*v, Width::Word)),
                            dst,
                        }),
                        Width::Byte if *v == -1 && *ty == Scalar::BOOLEAN => {
                            self.out.push(Insn::Scc { cond: Cond::T, dst })
                        }
                        // The whole slot cleared: RIP_$SEND_UPDATES
                        // 0xE688BC `clr.w -(SP)` for the byte parameter
                        // RIP_$BROADCAST reads with `move.b (8,A6),D2`
                        // (0xE872A6); the other path pushes `st -(SP)`
                        // (0xE6889E), so this is `false`.
                        Width::Byte if *v == 0 => self.out.push(Insn::Clr { size: Size::W, dst }),
                        Width::Byte => {
                            return Err(unsupported(
                                self.span,
                                "a byte constant argument other than `true` and 0 is",
                            ))
                        }
                    },
                    Expr::Load { addr, ty } => {
                        let src = match addr {
                            Addr::Frame(o) if self.pinned(*o).is_some() => {
                                Ea::Reg(self.pinned(*o).unwrap())
                            }
                            Addr::Frame(o) if self.forwarded(*o, size_of(ty.width)).is_some() => {
                                Ea::Reg(self.forwarded(*o, size_of(ty.width)).unwrap())
                            }
                            _ => self.ea_of(addr)?,
                        };
                        self.out.push(Insn::Move {
                            size: size_of(ty.width),
                            src: src.clone(),
                            dst,
                        });
                    }
                    Expr::AddrOf(Addr::Symbol { name, offset: 0 }) => self.out.push(Insn::Move {
                        size: Size::L,
                        src: Ea::ImmSym(name.clone()),
                        dst,
                    }),
                    Expr::AddrOf(_) | Expr::SetMember { .. } | Expr::Test(_) => {
                        return Err(unsupported(self.span, "this argument value is"))
                    }
                    // A computed value: evaluated right before its push, from
                    // its register (0xE0AA82 `move.w D2,-(SP)`; KBD_$RCV
                    // 0xE1CE56 `move.b D0,-(SP)`; arguments are evaluated in
                    // push order, codegen-patterns 12).
                    e => {
                        self.in_args = true;
                        let v = self.eval(e);
                        self.in_args = false;
                        let src = match v? {
                            Val::Reg(r) => Ea::Reg(r),
                            v => Ea::Reg(Reg::D(self.in_tmp(v, e.ty().width)?)),
                        };
                        self.out.push(Insn::Move {
                            size: size_of(e.ty().width),
                            src,
                            dst,
                        });
                    }
                }
            }
            Arg::Address { addr, slot: 4, .. } => match addr {
                // An enclosing routine's object: `pea (d,An)` off the
                // static link (0xE13648 `pea (-0x40,A4)` in 0xE1360C).
                Addr::Frame(_) | Addr::Enclosing(_) | Addr::Static(_) => {
                    let ea = self.ea_of(addr)?;
                    self.out.push(Insn::Pea(ea));
                }
                // A constant: its cell in the code section, `pea (d,PC)`
                // (codegen-patterns 1.5: TERM_$SEND_KBD_STRING 0xE1AB0C
                // `pea (0x18,PC)`, MST_$REMAP 0xE439F8 `pea (0x10,PC)`; 1025
                // `pea (d,PC)` in compiled code).  The emitter places the
                // cell and resolves the label (`emit.rs`, `Pool::place`).
                Addr::Const(c) => self.out.push(Insn::Pea(Ea::Pool(c.clone()))),
                Addr::Symbol { name, offset: 0 } => self.out.push(Insn::Move {
                    size: Size::L,
                    src: Ea::ImmSym(name.clone()),
                    dst,
                }),
                // The object a pointer in the frame points to, e.g. a
                // by-address parameter passed on by address: the pointer
                // itself, `move.l (d,A6),-(SP)` (LOG_$READ 0xE1780C and
                // 0xE17818 pass its `actual_len` and `buffer` slots on to
                // the worker 0xE17778).
                // The enclosing routine's by-address parameter passed on:
                // its slot through the static link, 0xE135B8 `move.l
                // (8,A2),-(SP)` in 0xE1359C.
                Addr::Indirect { base, offset: 0 }
                    if matches!(
                        &**base,
                        Expr::Load {
                            addr: Addr::Frame(o),
                            ty: Scalar {
                                width: Width::Long,
                                ..
                            },
                        } if self.pinned(*o).is_none()
                    ) || matches!(
                        &**base,
                        Expr::Load {
                            addr: Addr::Enclosing(_),
                            ty: Scalar {
                                width: Width::Long,
                                ..
                            },
                        }
                    ) =>
                {
                    let Expr::Load { addr, .. } = &**base else {
                        unreachable!()
                    };
                    let src = self.ea_of(addr)?;
                    self.out.push(Insn::Move {
                        size: Size::L,
                        src,
                        dst,
                    });
                }
                // An object reached through a pointer held in an address
                // register since the entry: `pea (d,An)`, `pea (An)`
                // (module-local 0xE32B76, `sio/init_dtte.c`: 0xE32B86 `pea
                // (0xc,A2)`, 0xE32B9E `pea (A2)`, the slot held in A2 across
                // the calls; also 0xE1BEB2, 0xE1D59E, 0xE3DB8C `pea (A2)`).
                Addr::Indirect { base, offset }
                    if matches!(
                        &**base,
                        Expr::Load { addr: Addr::Frame(o), .. }
                            if matches!(self.pinned(*o), Some(Reg::A(_)))
                    ) =>
                {
                    let ea = self.ea_of(&Addr::Indirect {
                        base: base.clone(),
                        offset: *offset,
                    })?;
                    self.out.push(Insn::Pea(ea));
                }
                // An array element: `pea (d,An,Dn.w)` (DISK_$REVALIDATE
                // 0xE6C07A `pea (-0x48,A0,D0.w)`), or `pea (d,Am)` after `lea
                // (0,An,Dn.l),Am` for a displacement beyond 8 bits
                // (MMAP_$UNAVAIL_REMOV 0xE0CC4C..0xE0CC52).
                Addr::Index { .. } => {
                    let ea = self.ea_of(addr)?;
                    self.out.push(Insn::Pea(ea));
                }
                _ => {
                    return Err(unsupported(
                        self.span,
                        "passing this kind of object by address is",
                    ))
                }
            },
            Arg::Address { .. } => {
                return Err(unsupported(self.span, "an address slot of this size is"))
            }
            Arg::Temp {
                value,
                offset,
                slot: 4,
            } => {
                // An expression passed by address: stored into its frame
                // temporary right before the push, then the temporary's
                // address pushed (codegen-patterns 1.5).  `addr(x)`:
                // TONE_$ENABLE 0xE1ACFC `lea (0x1268,A5),A0; move.l
                // A0,(-8,A6); pea (-8,A6)`, PCHIST_$CNTL 0xE5CF06..0xE5CF1A
                // (two in one call, A0 then A1).  Other values computed
                // into a register and stored, `move.<s> Dn,(d,A6); pea
                // (d,A6)` (GET_BUILD_TIME 0xE380F4, TERM_$CONTROL
                // 0xE66A08; `tools/pcsurvey.py temps`).
                let tmp = Ea::Disp(*offset, FP);
                match value {
                    Expr::AddrOf(a) if !matches!(a, Addr::Symbol { .. } | Addr::Const(_)) => {
                        let src = self.ea_of(a)?;
                        let r = self.alloc_a()?;
                        self.out.push(Insn::Lea { src, dst: r });
                        self.out.push(Insn::Move {
                            size: Size::L,
                            src: Ea::Reg(Reg::A(r)),
                            dst: tmp.clone(),
                        });
                    }
                    Expr::AddrOf(_) => {
                        return Err(unsupported(
                            self.span,
                            "`addr` of another unit's or a constant's object passed by address is",
                        ))
                    }
                    v => self.store(&Addr::Frame(*offset), v)?,
                }
                self.out.push(Insn::Pea(tmp));
            }
            Arg::Temp { .. } => {
                return Err(unsupported(self.span, "an address slot of this size is"))
            }
        }
        Ok(())
    }

    /// Store a constant into `count` consecutive elements from `start`
    /// ([`dpc_ir::StmtKind::Fill`], a `for` loop over an array).
    ///
    /// Module-local 0xE00EB0, 0xE00ECC..0xE00EDA:
    ///
    /// ```text
    /// movea.l #AST_$ZERO_BUFF,A0   ; the first element's address
    /// move.w  #0xFF,D0             ; then the count less one
    /// movea.l A0,A0                ; loop-head filler (codegen-patterns 7)
    /// clr.l   (A0)+
    /// dbf     D0,*-2
    /// ```
    ///
    /// The count is `moveq` up to 127, else `move.w` (as in the copy
    /// loops, 0xE01494, 0xE3B97A).  Other clearing loops in the image
    /// load the count *before* the address (0xE0277A `moveq #31,D0; lea
    /// (A3),A0`; 0xE09C98; ROUTE_ 0xE69D12 `move.w #128,D0; movea.l
    /// #abs,A0`), with no rule found; so only this shape is supported:
    /// another unit's array from its first byte (`movea.l #sym`, as
    /// DISK_$GET_ERROR_INFO 0xE6C090 loads another unit's object for a
    /// copy), cleared with `clr`.  Storing other constants, and arrays
    /// elsewhere, are not supported yet.
    fn fill(&mut self, start: &Addr, count: u32, value: &Expr) -> Res<()> {
        let Expr::Const { value: 0, ty } = value else {
            return Err(unsupported(
                self.span,
                "a `for` loop storing a constant other than 0 is",
            ));
        };
        let Addr::Symbol { name, offset: 0 } = start else {
            return Err(unsupported(
                self.span,
                "a `for` loop over an array other than another unit's, from its first byte, is",
            ));
        };
        let a = self.alloc_a()?;
        self.out.push(Insn::Movea {
            size: Size::L,
            src: Ea::ImmSym(name.clone()),
            dst: a,
        });
        let d = self.alloc_d()?;
        let n = count as i64 - 1;
        self.out.push(if n <= 127 {
            Insn::Moveq {
                value: n as i8,
                dst: d,
            }
        } else {
            Insn::Move {
                size: Size::W,
                src: Ea::Imm(imm(n, Width::Word)),
                dst: Ea::Reg(Reg::D(d)),
            }
        });
        let head = self.new_label();
        self.out.push(Insn::Align4);
        self.place(head);
        self.out.push(Insn::Clr {
            size: size_of(ty.width),
            dst: Ea::PostInc(a),
        });
        self.out.push(Insn::Dbf {
            reg: d,
            label: head,
        });
        Ok(())
    }

    /// `f := e` in a body of one statement: the value goes straight to
    /// the result register (PROC1_$TST_LOCK 0xE148E0 `move.b D1,D0`; the
    /// frame slot of the result variable is not used).  In a body of
    /// several statements: [`Self::set_result_web`].
    fn set_result(&mut self, f: &Function, e: &Expr) -> Res<()> {
        let res = f.result.expect("SetResult in a function");
        match res.class {
            ResultClass::Data => {}
            ResultClass::Address => return self.set_pointer_result(f, e),
            ResultClass::AddressAndData => {
                return Err(unsupported(self.span, "a result in A0 and D0 is"))
            }
        }
        if self.result_web {
            return self.set_result_web(res, e);
        }
        // A function result in D0 already (the tail wrapper `F :=
        // G(args)`: the call, then the epilogue, codegen-patterns 3,
        // ACL_$IS_SUSER), a held value or a computed one.
        let r = match self.eval(e)? {
            Val::Reg(r) => r,
            v => Reg::D(self.in_tmp(v, res.ty.width)?),
        };
        // The value is moved at the result's width; the upper bytes of D0
        // are left as they are (codegen-patterns 3: 0xE148E0).  When the
        // allocator puts the value in D0 itself the move is dropped
        // (`select_body`).
        self.result_moves.push(self.out.len());
        self.out.push(Insn::Move {
            size: size_of(res.ty.width),
            src: Ea::Reg(r),
            dst: Ea::Reg(Reg::D(0)),
        });
        Ok(())
    }

    /// `f := e` for a pointer result, which comes back in A0 (ref 5.5.10,
    /// p. 5-20; codegen-patterns 3, "Pointer results",
    /// `tools/resultsurvey.py pointer`), as the only statement of the
    /// body, from memory: stored into the result variable's frame slot and
    /// loaded from there into A0.  The module-local 0xE6AA38 (VFMT,
    /// `vfmt/main.c` `vfmt_get_next_arg`) `movea.l (8,A6),A0; move.l
    /// (A0),(-8,A6); movea.l (-8,A6),A0`; the same last statement at
    /// SIO_$I_GET_DESC 0xE66808, TTY_$I_GET_DESC 0xE667B0, KBD_$GET_DESC
    /// 0xE1AA92 and 0xE5A180 (5 of 5 pointer functions whose last
    /// statement stores memory into the frame).
    ///
    /// A call of another pointer function: its A0 is stored into the slot
    /// and left in A0, `move.l A0,(-4,A6)` then the epilogue (MST_$MAPS
    /// 0xE439C8, MST_$MAP 0xE438BE, MST_$MAP_TOP 0xE4391E, MST_$MAP_GLOBAL
    /// 0xE43974, MST_$REMAP 0xE439FE: the five map-named wrappers of
    /// `tools/resultsurvey.py callers` whose call is not popped).
    ///
    /// Refused: other values (a computed one is stored and moved,
    /// 0xE4D5A6 `move.l D0,(-12,A6); movea.l D0,A0`; `nil` as `suba.l
    /// A0,A0` at five sites, ROUTE_$FIND_PORTP 0xE15B84) and a
    /// non-pointer result in A0 (`a0_return`, ref 5.5.11; no site known).
    /// In a body of several statements: [`Self::set_pointer_result_web`].
    fn set_pointer_result(&mut self, f: &Function, e: &Expr) -> Res<()> {
        let res = f.result.expect("SetResult in a function");
        if res.ty != Scalar::POINTER {
            return Err(unsupported(
                self.span,
                "a non-pointer result in A0 (`a0_return`, ref 5.5.11) is",
            ));
        }
        if self.result_web {
            return self.set_pointer_result_web(e);
        }
        let Some(slot) = f.result_slot else {
            return Err(unsupported(
                self.span,
                "a pointer result without a frame slot is",
            ));
        };
        if let Some(c) = a0_call(e) {
            let a = self.call_a0(c)?;
            self.out.push(Insn::Move {
                size: Size::L,
                src: Ea::Reg(Reg::A(a)),
                dst: Ea::Disp(slot, FP),
            });
            return Ok(());
        }
        let src = match e {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if *o > 0 || self.pinned(*o).is_some() => None,
            Expr::Load { addr, .. } => Some(self.ea_of(addr)?),
            _ => None,
        };
        let Some(src) = src else {
            return Err(unsupported(
                self.span,
                "a pointer result other than one read from memory or a pointer function's \
                 result (a computed value, `nil`: codegen-patterns 3, \"Pointer results\") is",
            ));
        };
        self.out.push(Insn::Move {
            size: Size::L,
            src,
            dst: Ea::Disp(slot, FP),
        });
        self.out.push(Insn::Movea {
            size: Size::L,
            src: Ea::Disp(slot, FP),
            dst: 0,
        });
        Ok(())
    }

    /// `f := e` for a pointer result in a function body of several
    /// statements: the result variable is held in one address register,
    /// moved to A0 at the start of the epilogue ([`select_body`]), the
    /// counterpart of [`Self::set_result_web`] (codegen-patterns 3,
    /// "Pointer results": `movea.l Rn,A0` falls into 16 of the 40
    /// epilogues, from A2 in 12, `tools/resultsurvey.py pointer`).  Like
    /// any value it gets the lowest address register free over its live
    /// range, a callee-saved one when a call follows (DISK_$GET_BLOCK
    /// 0xE3BBBC `movea.l A0,A2` after DBUF_$GET_BLOCK, ML_$UNLOCK at
    /// 0xE3BBC4, `movea.l A2,A0` at 0xE3BBCA).  Assigned from a pointer
    /// function's result (copied from A0) or from memory (`movea.l`,
    /// EC2_$GET_EC1_ADDR 0xE42AC4 `movea.l (0xa8c,A1),A2`); other values
    /// (`nil`, `suba.l A2,A2` at 0xE42AAA; a computed address, `lea` at
    /// 0xE42B0A) are refused.
    fn set_pointer_result_web(&mut self, e: &Expr) -> Res<()> {
        let web = match self.web_a {
            Some(a) => a,
            None => {
                let a = self.alloc_a()?;
                self.web_a = Some(a);
                a
            }
        };
        let src = match e {
            e if a0_call(e).is_some() => Ea::Reg(Reg::A(self.call_a0(a0_call(e).unwrap())?)),
            Expr::Load { addr, .. } if !matches!(addr, Addr::Frame(o) if *o > 0) => {
                self.operand(addr)?
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "a pointer result in a body of several statements assigned other than a \
                     pointer function's result or a variable (`nil`, a computed address: \
                     codegen-patterns 3) is",
                ))
            }
        };
        if matches!(src, Ea::Reg(_)) {
            self.result_moves.push(self.out.len());
        }
        self.out.push(Insn::Movea {
            size: Size::L,
            src,
            dst: web,
        });
        Ok(())
    }

    /// `f := e` in a function body of several statements (codegen-patterns
    /// 3, "Result variable"; `tools/resultsurvey.py`): the result variable
    /// is held in one data register, which every assignment writes and
    /// which goes to the result register at the start of the epilogue
    /// ([`select_body`]).  Like any value, it gets the lowest register
    /// free over its live range, D0 when no call follows an assignment
    /// (PARITY_$CHK_IO 0xE0B19C/0xE0B1A6/0xE0B1AA `moveq #1,D0`, `moveq
    /// #2,D0`, `clr.w D0` on three paths; PROC1_$SUSPENDP 0xE14886 `st D0`
    /// then `sne D0` at 0xE148BC), a callee-saved one when a call follows
    /// (PKT_$NEXT_ID 0xE124AC `move.w (0x5c,A5),D2`, a call at 0xE124CA;
    /// PROC2_$GET_PID 0xE40CF6 `move.w (..),D2` inside an `if`; SMD_$N_DEVICES
    /// 0xE70032 `clr.w D2` and 0xE7004E `move.w D4,D2` in a loop); it is
    /// coloured before the other values (`regs.rs`, `allocate_apart`).
    ///
    /// - a constant: `clr`/`moveq`/`move #` ([`Self::const_into`]); `true`
    ///   as `st` (0xE41216 `st D2`, 0xE14886 `st D0`; no `moveq` writes
    ///   the register of any of the 31 byte results of
    ///   `tools/resultsurvey.py list reg`);
    /// - a value in a register (a function's result in D0, a held
    ///   parameter): copied (NETBUF_$RTNVA 0xE0ED4E `move.l D0,D2`;
    ///   GET_CHKSUM 0xE0F352);
    /// - memory: loaded (PKT_$NEXT_ID 0xE124AC);
    /// - a computed value: computed in the variable's register (AS_IO_SETUP
    ///   0xE6B74C, `move.w (A0),D2; ...; lsl.w #3,D2; ...; add.w D0,D2` at
    ///   0xE6B772..0xE6B780; Uncertain: the
    ///   sites cannot tell a local copied by a final `f := local` from the
    ///   result variable).
    fn set_result_web(&mut self, res: ResultSpec, e: &Expr) -> Res<()> {
        let w = res.ty.width;
        let web = match self.web {
            Some(d) => d,
            None => {
                let d = self.alloc_d()?;
                self.web = Some(d);
                d
            }
        };
        let start = self.out.len();
        if let Expr::Const { value, ty } = e {
            if *ty == Scalar::BOOLEAN && *value == -1 {
                self.out.push(Insn::Scc {
                    cond: Cond::T,
                    dst: Ea::Reg(Reg::D(web)),
                });
            } else {
                self.const_into(*value, w, web);
            }
            return Ok(());
        }
        match self.eval(e)? {
            // A new virtual register the value was computed in: computed
            // in the variable's register instead.  Not a physical one (an
            // `M$ARITH` result in D0, whose earlier uses in the argument
            // pushes are other values) nor one an earlier statement
            // defined: copied like a register value below.
            Val::Tmp(d)
                if crate::regs::is_virtual(Reg::D(d))
                    && !self.out[..start].iter().any(|i| {
                        let a = crate::regs::access(i);
                        a.defs.contains(&Reg::D(d)) || a.uses.contains(&Reg::D(d))
                    }) =>
            {
                let (from, to) = (Reg::D(d), Reg::D(web));
                for i in &mut self.out[start..] {
                    crate::regs::rename(i, &|r| if r == from { to } else { r });
                }
                // A register copied into it (a call's D0, `f := g(..) + n`)
                // may end up in the variable's own register (`regs.rs`):
                // the copy is then dropped, as no `move Dn,Dn` is in the
                // image.  Not when a branch or `s<cc>` reads its flags.
                for k in start..self.out.len() {
                    let copy = matches!(
                        &self.out[k],
                        Insn::Move { src: Ea::Reg(Reg::D(_)), dst: Ea::Reg(t), .. } if *t == to
                    );
                    let flags_read = matches!(
                        self.out.get(k + 1),
                        Some(Insn::Branch { .. } | Insn::Scc { .. })
                    );
                    if copy && !flags_read {
                        self.result_moves.push(k);
                    }
                }
            }
            Val::Imm(c) => self.const_into(c, w, web),
            v => {
                // A register value may end up in the variable's own
                // register (`regs.rs`): the move is then dropped.
                let src = Self::ea_val(&v);
                if matches!(src, Ea::Reg(_)) {
                    self.result_moves.push(self.out.len());
                }
                self.out.push(Insn::Move {
                    size: size_of(w),
                    src,
                    dst: Ea::Reg(Reg::D(web)),
                });
            }
        }
        Ok(())
    }

    /// Evaluate `e` into a new data register the caller may modify.
    fn eval_d(&mut self, e: &Expr) -> Res<u8> {
        let v = self.eval(e)?;
        self.in_tmp(v, e.ty().width)
    }

    /// Evaluate `e` as an operand (codegen-patterns 8, 12).
    ///
    /// - a constant is an immediate; a variable a memory operand (any
    ///   address register it needs is loaded now: 0xE148CE before
    ///   0xE148D4); a by-value parameter loaded at entry its register
    ///   (12.2), which is never modified: arithmetic works on a copy
    ///   (BAT_$GET_BAT_STEP 0xE3BB68 `move.w D0,D1; mulu.w #0x234,D1`;
    ///   FIM_$FREE_ASID 0xE0AA8A `move.w D2,D0; lsl.w #2,D0`);
    /// - a function call's value is D0 after the call
    ///   ([`Self::expr_call`]);
    /// - operators compute into a new data register ([`Self::binary`],
    ///   [`Self::unary`], [`Self::convert`]), a comparison into a boolean
    ///   ([`Self::test_value`]).
    fn eval(&mut self, e: &Expr) -> Res<Val> {
        match e {
            Expr::Const { value, ty } => Ok(Val::Imm(imm(*value, ty.width))),
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if self.pinned(*o).is_some() => Ok(Val::Reg(self.pinned(*o).unwrap())),
            Expr::Load { addr, .. } => Ok(Val::Mem(self.ea_of(addr)?)),
            Expr::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
                ty,
            } if self.const_product(lhs, rhs, *ty).is_some() => {
                let (x, c, leaf) = self.const_product(lhs, rhs, *ty).unwrap();
                let key = CseKey::Scaled(x.clone(), c);
                self.cse_value(key, leaf, |s| s.binary(BinOp::Mul, lhs, rhs, *ty))
            }
            Expr::Binary { op, lhs, rhs, ty } => Ok(Val::Tmp(self.binary(*op, lhs, rhs, *ty)?)),
            Expr::Unary { op, operand, ty } => {
                // `neg.<s>` / `not.<s>` on the operand in a register (negw
                // ×97, notw ×13, notl ×46 in compiled code).
                let d = self.eval_d(operand)?;
                self.out.push(Insn::Unary {
                    op: match op {
                        UnOp::Neg => UnaryOp::Neg,
                        UnOp::Not => UnaryOp::Not,
                    },
                    size: size_of(ty.width),
                    dst: Ea::Reg(Reg::D(d)),
                });
                Ok(Val::Tmp(d))
            }
            Expr::Convert { value, ty } => self.convert(value, *ty),
            // A pointer function's A0 is read only where the image reads
            // it (codegen-patterns 3, "Callers of pointer functions";
            // [`a0_call`]): a store, a copy into a register-held local, a
            // base, a test against `nil`, the caller's own pointer result.
            Expr::Call {
                reg: ResultClass::Address,
                ..
            } => Err(unsupported(
                self.span,
                "this use of the value of a function whose result is in A0 (passed as an \
                 argument, or in an expression: no compiled site, codegen-patterns 3) is",
            )),
            Expr::Call { call, .. } => self.expr_call(call),
            Expr::Test(c) => Ok(Val::Tmp(self.test_value(c)?)),
            Expr::SetMember {
                elem,
                set,
                set_width,
            } => {
                if matches!(**elem, Expr::Const { .. }) {
                    let r = self.set_bit_test(elem, set, *set_width, true)?;
                    self.out.push(Insn::Scc {
                        cond: Cond::Ne,
                        dst: Ea::Reg(Reg::D(r)),
                    });
                    return Ok(Val::Tmp(r));
                }
                if *set_width != Width::Long {
                    // `btst Dn,Dm` takes the bit number mod 32: on a word
                    // set held in a register an element above 15 would test
                    // a bit outside the set.  What the original did is not
                    // catalogued.
                    return Err(unsupported(
                        self.span,
                        "`in` on a one-word set with an element that is not constant is",
                    ));
                }
                Ok(Val::Tmp(self.set_member_long(elem, set)?))
            }
            Expr::AddrOf(_) => Err(unsupported(
                self.span,
                "an address (`addr`) as a value other than an argument is",
            )),
            Expr::SetSr(x) => Ok(Val::Tmp(self.set_sr(x)?)),
        }
    }

    /// `set_sr(x)` (ref Appendix E, p. E-5): "move.w SR,d0 / move.w
    /// newsr,SR / move.w d0,oldsr".  The image (compiled code, 5 `move.w
    /// SR,Dn`, each followed by the write): PROC1_$SET_VT 0xE149B6..0xE149B8
    /// `move.w SR,D3; move.w #0x2700,SR` and 0xE149F2..0xE149F4 `move.w
    /// SR,D1; move.w D3,SR`; PROC1_$TS_END_CALLBACK 0xE14A90..0xE14A92,
    /// 0xE14AF8..0xE14AFA; WIN_ 0xE19C36..0xE19C38 `move.w #0x2500,SR`.
    /// The old word goes to a new data register (the lowest free, D0 at
    /// 0xE14AF8; D1 at 0xE149F2 although D0 looks free there: Uncertain).
    /// The new word is a constant or a value already in a register at
    /// every site: a computed argument, or one in memory that needs an
    /// address register, is refused (whether it is formed before or after
    /// the `move.w SR` is not seen); one in memory that needs none is
    /// read in place (reference only, no image site).
    fn set_sr(&mut self, x: &Expr) -> Res<u8> {
        let src =
            match x {
                Expr::Const { value, .. } => Ea::Imm(imm(*value, Width::Word)),
                Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } if self.pinned(*o).is_some() => Ea::Reg(self.pinned(*o).unwrap()),
                Expr::Load { addr, .. } if !self.addr_needs_reg(addr) => {
                    if matches!(addr, Addr::Frame(o) if *o > 0) {
                        return Err(unsupported(
                            self.span,
                            "a `set_sr` argument that is a parameter not loaded at entry is",
                        ));
                    }
                    self.ea_of(addr)?
                }
                _ => return Err(unsupported(
                    self.span,
                    "a computed `set_sr` argument (ordered before or after `move.w SR`: not seen, \
                     codegen-patterns 10) is",
                )),
            };
        let d = self.alloc_d()?;
        self.out.push(Insn::MoveFromSr { dst: d });
        self.out.push(Insn::MoveToSr { src });
        Ok(d)
    }

    /// `v` in a new data register the caller may modify, at width `w`:
    ///
    /// - already one: itself;
    /// - a held register: a copy, `move.<w> Dm,Dn` (BAT_$GET_BAT_STEP
    ///   0xE3BB68; 0xE01BA4 `move.w D0,D1` for a function result);
    /// - memory: `move.<w> ea,Dn` (0xE148D4);
    /// - a constant: [`Self::load_const`].
    fn in_tmp(&mut self, v: Val, w: Width) -> Res<u8> {
        let src = match v {
            Val::Tmp(d) => return Ok(d),
            Val::Imm(c) => return self.load_const(c, w),
            Val::Reg(r) => Ea::Reg(r),
            Val::Mem(ea) => ea,
        };
        let d = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: size_of(w),
            src,
            dst: Ea::Reg(Reg::D(d)),
        });
        Ok(d)
    }

    /// The operand `v` as an effective address (a constant as an
    /// immediate, a new register as itself).
    fn ea_val(v: &Val) -> Ea {
        match v {
            Val::Tmp(d) => Ea::Reg(Reg::D(*d)),
            Val::Reg(r) => Ea::Reg(*r),
            Val::Mem(ea) => ea.clone(),
            Val::Imm(c) => Ea::Imm(*c),
        }
    }

    /// A constant in a new data register (codegen-patterns 3, 13):
    ///
    /// - 0: `clr.<w>` (PARITY_$CHK_IO 0xE0B1AA `clr.w D0`; IO_$ALLOC
    ///   0xE1A9CC `clr.l D0`);
    /// - -128..127: `moveq` (PARITY_$CHK_IO 0xE0B19C `moveq #1,D0`, a word
    ///   result; 0xE0B23A `moveq #-1,D0`; 0xE02D44 `moveq #15,D1` for a
    ///   byte operation; 0xE177A0 `moveq #-2,D2` for a word one);
    /// - others: `move.<w> #c` (0xE00A14 `move.l #0xffffff,D0`).
    fn load_const(&mut self, c: i64, w: Width) -> Res<u8> {
        let d = self.alloc_d()?;
        self.const_into(c, w, d);
        Ok(d)
    }

    /// Constant `c` into data register `d` ([`Self::load_const`]).
    fn const_into(&mut self, c: i64, w: Width, d: u8) {
        let signed = match w {
            Width::Byte => c as i8 as i64,
            Width::Word => c as i16 as i64,
            Width::Long => c as i32 as i64,
        };
        self.out.push(if signed == 0 {
            Insn::Clr {
                size: size_of(w),
                dst: Ea::Reg(Reg::D(d)),
            }
        } else if (-128..128).contains(&signed) {
            Insn::Moveq {
                value: signed as i8,
                dst: d,
            }
        } else {
            Insn::Move {
                size: size_of(w),
                src: Ea::Imm(c),
                dst: Ea::Reg(Reg::D(d)),
            }
        });
    }

    /// `x := x op c` for an object `x` that is not an array element:
    /// `addq`/`subq` for 1..8, `addi`/`subi`, `andi`, `ori` on the object
    /// in memory (codegen-patterns 8, 13).  Survey of compiled code: 1076
    /// `addq`/`subq` to memory (0xE00B3E `addq.w #1,abs.l`; ACL_$ENTER_SUPER
    /// 0xE46FA8 `addq.w #1,(0xb76,A0)`), 20 `addi`/`subi` (0xE010C4 `addi.l
    /// #0xc0,(0x3f4,A5)`), 560 `andi`/`ori` (0xE0086E `andi.l
    /// #-0x7f8001,(0x26,A2)`), against 6 loads, operations and stores back
    /// of the same plain operand (0xE025B0).  For an array element the
    /// image does both (14 `addq` to an indexed operand, e.g. 0xE13154;
    /// 10 load, operation and store, AUDIT_$SUSPEND 0xE70DC8..0xE70DCE):
    /// refused.  Returns whether the statement was this form.
    fn update_in_place(&mut self, addr: &Addr, value: &Expr) -> Res<bool> {
        let Expr::Binary { op, lhs, rhs, ty } = value else {
            return Ok(false);
        };
        let Expr::Load { addr: a, .. } = &**lhs else {
            return Ok(false);
        };
        let Expr::Const { value: c, .. } = &**rhs else {
            return Ok(false);
        };
        if a != addr || lhs.ty() != *ty {
            return Ok(false);
        }
        let alu = match op {
            BinOp::Add => AluOp::Add,
            BinOp::Sub => AluOp::Sub,
            BinOp::And => AluOp::And,
            BinOp::Or => AluOp::Or,
            _ => return Ok(false),
        };
        if matches!(addr, Addr::Index { .. }) {
            return Err(unsupported(
                self.span,
                "updating an array element in place (`addq` to the element or a load, \
                 operation and store; the image does both, codegen-patterns 8) is",
            ));
        }
        if let Addr::Frame(o) = addr {
            if self.pinned(*o).is_some() {
                return Err(unsupported(self.span, "assigning to a parameter is"));
            }
        }
        let size = size_of(ty.width);
        let dst = self.ea_of(addr)?;
        let c = imm(*c, ty.width);
        self.out.push(match alu {
            AluOp::Add if (1..=8).contains(&c) => Insn::Addq {
                size,
                value: c as u8,
                dst,
            },
            AluOp::Sub if (1..=8).contains(&c) => Insn::Subq {
                size,
                value: c as u8,
                dst,
            },
            op => Insn::AluImm {
                op,
                size,
                imm: c,
                dst,
            },
        });
        Ok(true)
    }

    /// `lhs op rhs` into a new data register (codegen-patterns 8).
    fn binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, ty: Scalar) -> Res<u8> {
        let w = ty.width;
        let size = size_of(w);
        let konst = |e: &Expr| match e {
            Expr::Const { value, .. } => Some(*value),
            _ => None,
        };
        match op {
            BinOp::Shl | BinOp::Shr | BinOp::Ashr => return self.shift(op, lhs, rhs, ty),
            BinOp::Mul => return self.mul(lhs, rhs, ty),
            BinOp::Div | BinOp::Mod => {
                return match self.div(op, lhs, rhs, ty, false)? {
                    Val::Tmp(d) => Ok(d),
                    v => self.in_tmp(v, w),
                }
            }
            _ => {}
        }
        let alu = match op {
            BinOp::Add => AluOp::Add,
            BinOp::Sub => AluOp::Sub,
            BinOp::And => AluOp::And,
            BinOp::Or => AluOp::Or,
            BinOp::Xor => AluOp::Eor,
            _ => unreachable!(),
        };
        // A constant operand: on the right for `-`; either side otherwise.
        let (x, c) = match (konst(lhs), konst(rhs)) {
            (_, Some(c)) => (Some(lhs), Some(c)),
            (Some(c), None) if op.commutative() => (Some(rhs), Some(c)),
            _ => (None, None),
        };
        if let (Some(x), Some(c)) = (x, c) {
            let c = imm(c, w);
            let xv = self.eval(x)?;
            match alu {
                // `x + c`, `x - c`: `addq`/`subq` for 1..8 (0xE00C10 `addq.l
                // #1,D0` after a load; 0xE010D0 `subq.l #1,D1`; 0xE68874
                // `addq.w #2,D0`).  `x + c` for c in 9..127 with `x` in memory:
                // the constant first, `moveq #c,Dn; add.<s> x,Dn` (23 sites,
                // e.g. 0xE008A8 `moveq #30,D0; add.l (-64,A6),D0`, 0xE30BB0
                // `moveq #64,D2; add.w abs.l,D2`; against 4 `addi` of such a
                // constant after a load).  Otherwise the value in a register
                // and `addi`/`subi` (0xE08840 `addi.l`, 0xE0B6E2 `subi.w
                // #0x400,D2`; `x - c` for c in 9..127: 13 `subi` after a load).
                AluOp::Add | AluOp::Sub if c < 0 => {
                    return Err(unsupported(
                        self.span,
                        "adding or subtracting a negative constant is",
                    ))
                }
                AluOp::Add | AluOp::Sub if c == 0 => return self.in_tmp(xv, w),
                AluOp::Add | AluOp::Sub if c <= 8 => {
                    let d = self.in_tmp(xv, w)?;
                    let dst = Ea::Reg(Reg::D(d));
                    self.out.push(if alu == AluOp::Add {
                        Insn::Addq {
                            size,
                            value: c as u8,
                            dst,
                        }
                    } else {
                        Insn::Subq {
                            size,
                            value: c as u8,
                            dst,
                        }
                    });
                    return Ok(d);
                }
                AluOp::Add if c <= 127 && !matches!(xv, Val::Tmp(_)) => {
                    if let Val::Reg(_) = xv {
                        return Err(unsupported(
                            self.span,
                            "adding a constant of 9..127 to a value held in a register (`moveq` \
                             and `add`, or a copy and `addi`: 11 sites each) is",
                        ));
                    }
                    let d = self.load_const(c, w)?;
                    self.out.push(Insn::Alu {
                        op: alu,
                        size,
                        src: Self::ea_val(&xv),
                        dst: d,
                    });
                    return Ok(d);
                }
                AluOp::Add | AluOp::Sub => {
                    let d = self.in_tmp(xv, w)?;
                    self.out.push(Insn::AluImm {
                        op: alu,
                        size,
                        imm: c,
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    return Ok(d);
                }
                // `x & c`: a computed value is masked in place, `andi`
                // (0xE11024 `lsr.w; andi.w`).  A variable, in memory or held
                // in a register, is ANDed into the constant loaded first
                // (0xE177A0 `moveq #-2,D2; and.w D1,D2`, D1 a parameter held
                // since the entry; 0xE60F4A `move.l #0xfffff,D0; and.l
                // (-28,A6),D0`; 0xE02D44 `moveq #15,D1; and.b (157,A3),D1`;
                // survey: 458 such constant-first `and`s against about 50
                // loads followed by `andi`, some of them conversions such as
                // 0xE00868 `andi.l #0xff`).
                AluOp::And => {
                    if let Val::Tmp(d) = xv {
                        self.out.push(Insn::AluImm {
                            op: alu,
                            size,
                            imm: c,
                            dst: Ea::Reg(Reg::D(d)),
                        });
                        return Ok(d);
                    }
                    let d = self.load_const(c, w)?;
                    self.out.push(Insn::Alu {
                        op: alu,
                        size,
                        src: Self::ea_val(&xv),
                        dst: d,
                    });
                    return Ok(d);
                }
                // `x ! c`, `xor(x, c)` on a computed value: `ori`/`eori` in
                // place (0xE03498 `andi.w #-0x4001,D1; ori.w #0x2000,D1`).
                // On a variable the image has both orders (4 constant-first
                // `or`s, 5 loads and `ori`): not settled.
                AluOp::Or | AluOp::Eor => {
                    let Val::Tmp(d) = xv else {
                        return Err(unsupported(
                            self.span,
                            "`!`/`xor` of a variable and a constant (operand order not settled) is",
                        ));
                    };
                    self.out.push(Insn::AluImm {
                        op: alu,
                        size,
                        imm: c,
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    return Ok(d);
                }
            }
        }
        if let (Some(c), AluOp::Sub) = (konst(lhs), alu) {
            // `c - x`: the constant first, then `sub.<s> x,Dn` (0xE08EE4
            // `moveq #31,D2; sub.w (-26,A6),D2`; 0xE10114 `move.l
            // #0x400,D2; sub.l (-372,A6),D2`; 0xE0E964).
            let d = self.load_const(imm(c, w), w)?;
            let xv = self.eval(rhs)?;
            if let Val::Tmp(_) = xv {
                return Err(unsupported(
                    self.span,
                    "a constant minus a computed value is",
                ));
            }
            self.out.push(Insn::Alu {
                op: alu,
                size,
                src: Self::ea_val(&xv),
                dst: d,
            });
            return Ok(d);
        }
        // Two variables or computed values: the left operand in a register
        // and the right one added, subtracted, ... into it (0xE177B4 `add.l
        // D1,D2`; 0xE0CDD6 `sub.l mem,Dn` after a load).  When only the
        // right operand is computed and the operator commutes, the left one
        // is applied to it.  **Uncertain:** the image cannot show which
        // source operand went to the register.
        let lv = self.eval(lhs)?;
        let rv = self.eval(rhs)?;
        let (d, src) = match (&lv, &rv) {
            (Val::Tmp(_), _) | (_, Val::Imm(_)) => (self.in_tmp(lv.clone(), w)?, rv),
            (_, Val::Tmp(r)) if op.commutative() => (*r, lv),
            // A variable and a held computed value (codegen-patterns 12.4
            // R7): the held value copied, the variable the source
            // (0xE177D0..0xE177D4 `move.l D1,D3; ...; add.l (0x14,A5),D3`,
            // `integer32(logfile_ptr) + offset` with `offset` zero-extended
            // at 0xE177A6).
            (Val::Mem(_), Val::Reg(Reg::D(r))) if op.commutative() && self.cse_regs.contains(r) => {
                (self.in_tmp(rv.clone(), w)?, lv)
            }
            (_, Val::Tmp(_)) => {
                return Err(unsupported(
                    self.span,
                    "a variable minus a computed value (evaluation order not catalogued) is",
                ))
            }
            _ => (self.in_tmp(lv.clone(), w)?, rv),
        };
        // `eor` takes only a data register as its source (MC68000 EOR).
        let src = match (alu, src) {
            (AluOp::Eor, Val::Mem(ea)) => Ea::Reg(Reg::D(self.in_tmp(Val::Mem(ea), w)?)),
            (_, v) => Self::ea_val(&v),
        };
        if matches!(src, Ea::Reg(Reg::A(_))) && alu != AluOp::Add && alu != AluOp::Sub {
            return Err(unsupported(
                self.span,
                "a logical operation on a pointer is",
            ));
        }
        self.out.push(Insn::Alu {
            op: alu,
            size,
            src,
            dst: d,
        });
        Ok(d)
    }

    /// The word that a 32-bit operand of sign `sign` stands for: the operand
    /// of a widening from a word of that sign, or a constant that fits a word
    /// of that sign.  These are the "W" operands of M$ARITH and the operands
    /// of `mul<s>.w` with a longword product (codegen-patterns 8).
    fn word_part(e: &Expr, sign: Sign) -> Option<Expr> {
        let wty = Scalar {
            width: Width::Word,
            sign,
        };
        match e {
            Expr::Const { value, .. } => {
                let fits = match sign {
                    Sign::Signed => (-0x8000..0x8000).contains(value),
                    Sign::Unsigned => (0..0x10000).contains(value),
                };
                fits.then_some(Expr::Const {
                    value: *value,
                    ty: wty,
                })
            }
            Expr::Convert { value, .. } if value.ty() == wty => Some((**value).clone()),
            _ => None,
        }
    }

    /// `lhs * rhs` into a new data register (codegen-patterns 8,
    /// "Multiplication"; beads domain-pascal-5aw.5, -1x1).
    ///
    /// - A constant factor done with shifts and adds ([`Self::mul_const`];
    ///   at a word's width also `mulu.w #c`), at either width; at a
    ///   longword width a word operand is extended first (0xE07442 `ext.l
    ///   D6; lsl.l #2,D6; ...` ×12).
    /// - A word times a word (or a constant that fits a word) used as a
    ///   longword: one `muls.w`/`mulu.w`, whose 32-bit product is the
    ///   value (PROC1_$LOADAV_CALLBACK 0xE14C14..0xE14C1C `move.w D1,D2;
    ///   muls.w #5240,D2; add.l D2,D0`, also 0xE14C4C; 0xE10A7E..0xE10A84
    ///   `move.w D2,D4; muls.w (-866,A2),D4; sub.l D0,D4`; TPAD_$DATA
    ///   0xE69230..0xE69236; SIO_$K_TIMED_BREAK 0xE67F28..0xE67F2E `move.w
    ///   (A0),D0; mulu.w #250,D0; move.l D0,(-10,A6)` for a `0..65535`
    ///   word; 0xE554AC, 0xE3C1BE).  `muls` for signed operands, `mulu`
    ///   for unsigned ones (the IR type's sign).
    /// - A word times a word for a word: `mul<s>.w` with the sign of the
    ///   type (0xE30A9A `muls.w D3,D0; move.w D0,(A0)+`; VTOC_$GET_UID
    ///   0xE3925A `mulu.w D4,D6`; **Uncertain:** the mnemonic of a word
    ///   product is the same either way).
    /// - Otherwise M$ARITH ([`Self::marith`]): `M$MI<s>$LLW` when one
    ///   operand is a word (or a constant that fits one), which is argument
    ///   2; `M$MI<s>$LLL` otherwise, a constant as argument 2 (0xE14BFA
    ///   `move.l #60296,-(SP); move.l D0,-(SP); jsr M$MIS$LLL`; 0xE1690C
    ///   `#250000`); 21 `M$MI*$LLW` and 11 `M$MI*$LLL` calls in compiled
    ///   code.  `S`/`U` from the type's sign (0xE1C2B0 `M$MIU$LLW` by
    ///   `#25000`).
    ///
    /// **Uncertain:** which of two variables goes to the register (as for
    /// `+`: the left one, or the computed one); which operand of `M$MI*$LLL`
    /// is argument 1 (the left one; 0xE3B220..0xE3B226 pushes a quotient
    /// as argument 1 for `(x div y) * y`).
    fn mul(&mut self, lhs: &Expr, rhs: &Expr, ty: Scalar) -> Res<u8> {
        let w = ty.width;
        let signed = ty.is_signed();
        if w == Width::Byte {
            return Err(unsupported(self.span, "a byte multiplication is"));
        }
        let konst = |e: &Expr| match e {
            Expr::Const { value, .. } => Some(*value),
            _ => None,
        };
        let factor = match (konst(lhs), konst(rhs)) {
            (_, Some(c)) => Some((lhs, c)),
            (Some(c), None) => Some((rhs, c)),
            (None, None) => None,
        };
        if let Some((x, c)) = factor {
            if w == Width::Word || by_shifts(c) {
                let d = self.eval_d(x)?;
                self.mul_const(d, c, w)?;
                return Ok(d);
            }
            if (1..=0xffff).contains(&c) && (c >> c.trailing_zeros()).count_ones() < 4 {
                return Err(unsupported(
                    self.span,
                    format!(
                        "a multiplication by {c} (shifts and adds or `mulu.w`, not settled for an \
                         odd part of three bits) is"
                    ),
                ));
            }
            let b = Expr::Const { value: c, ty };
            if let (Some(xw), Some(_)) = (Self::word_part(x, ty.sign), Self::word_part(&b, ty.sign))
            {
                let d = self.eval_d(&xw)?;
                self.out.push(Insn::Mul {
                    signed,
                    src: Ea::Imm(c),
                    dst: d,
                });
                return Ok(d);
            }
            self.marith_op("MI", ty, x, &b)?;
            return Ok(0);
        }
        if w == Width::Word {
            return self.mul_words(lhs, rhs, signed);
        }
        match (Self::word_part(lhs, ty.sign), Self::word_part(rhs, ty.sign)) {
            (Some(l), Some(r)) => self.mul_words(&l, &r, signed),
            (Some(_), None) => {
                // The word is argument 2 of `M$MI*$LLW`.
                self.marith_op("MI", ty, rhs, lhs)?;
                Ok(0)
            }
            _ => {
                self.marith_op("MI", ty, lhs, rhs)?;
                Ok(0)
            }
        }
    }

    /// A product of a plain variable ([`Self::cse_leaf`]) and a constant
    /// that [`Self::mul`] makes in a register with [`Self::mul_const`]
    /// (shifts and adds, or `mul<s>.w` for a word), a reusable computation
    /// like a scaled index (codegen-patterns 12.4 R7): the variable, the
    /// constant and the variable's size.
    fn const_product<'e>(
        &self,
        lhs: &'e Expr,
        rhs: &'e Expr,
        ty: Scalar,
    ) -> Option<(&'e Expr, i64, (Addr, u32))> {
        let (x, c) = match (lhs, rhs) {
            (x, Expr::Const { value, .. }) | (Expr::Const { value, .. }, x) => (x, *value),
            _ => return None,
        };
        if x.ty() != ty || !(ty.width == Width::Word || by_shifts(c)) {
            return None;
        }
        Some((x, c, self.cse_leaf(x)?))
    }

    /// Two word values multiplied with `muls.w`/`mulu.w`: the left one (or
    /// the computed one) in a register, the other the source operand
    /// (0xE10A7E `move.w D2,D4; muls.w (-866,A2),D4`, a held value copied).
    fn mul_words(&mut self, lhs: &Expr, rhs: &Expr, signed: bool) -> Res<u8> {
        let lv = self.eval(lhs)?;
        let rv = self.eval(rhs)?;
        let (d, src) = match (&lv, &rv) {
            (Val::Tmp(_), _) => (self.in_tmp(lv.clone(), Width::Word)?, rv),
            (_, Val::Tmp(r)) => (*r, lv),
            _ => (self.in_tmp(lv.clone(), Width::Word)?, rv),
        };
        let src = Self::ea_val(&src);
        if matches!(src, Ea::Reg(Reg::A(_))) {
            return Err(unsupported(self.span, "multiplying a pointer is"));
        }
        self.out.push(Insn::Mul {
            signed,
            src,
            dst: d,
        });
        Ok(d)
    }

    /// `lhs div rhs` or `lhs mod rhs` of type `ty` (ref "Div", p. 4-66:
    /// the quotient truncated toward zero; "Mod", pp. 4-115..4-116: without
    /// -iso the remainder has the dividend's sign; ref 4.3.3: unsigned
    /// forms; codegen-patterns 8).  `narrow`: a longword quotient or
    /// remainder used as a word.
    ///
    /// Words (both operands words of `ty`'s sign):
    /// - signed: the dividend extended, `ext.l`, then `divs.w`; `swap` for
    ///   the remainder (0xE31154..0xE3115C `move.w D2,D1; ext.l D1; divs.w
    ///   #31,D1; swap D1`; 0xE11BEA; CAL_$DECODE_TIME 0xE68800..0xE68808
    ///   `ext.l D0; divs.w #4,D0; swap D0` for `mod 4`; TPAD_$INIT
    ///   0xE335F4..0xE335FA `move.w (-34,A3),D0; ext.l D0; divs.w
    ///   (-38,A3),D0` by a variable);
    /// - unsigned: the dividend zero-extended, `clr.l Dn; move.w x,Dn`
    ///   (0xE382B4..0xE382C0 `clr.l D0; ...; move.w (2,A1),D0; ...; divu.w
    ///   #101,D0`; 0xE4CE1A..0xE4CE22 `clr.l D2; move.w D1,D2; divu.w
    ///   #111,D2; swap D2`; 0xE54B8C..0xE54B92 by a register), or `andi.l
    ///   #0xffff` on a computed one (0xE11818..0xE11822), then `divu.w`;
    /// - signed `div` by 2^k: [`Self::div_pow2`].
    ///
    /// Longwords:
    /// - `div` used as a word by a word (or a constant that fits one): the
    ///   longword dividend divided in place, `div<s>.w`, no extension; the
    ///   quotient is right only when it fits in 16 bits, which the image
    ///   accepts (0xE10CA6..0xE10CB2 ×8 `move.l (-786,A2),D0; sub.l
    ///   (-794,A2),D0; divs.w #250,D0; move.w D0,-(SP)`; PMAP_$PURIFIER_L
    ///   0xE13AF6..0xE13B00 `move.l abs,D2; divu.w #50,D2; move.w
    ///   D2,(1932,A5)`; 0xE3CB82..0xE3CB88 `divu.w (160,A1),D5`; RIP_$SERVER
    ///   0xE68AE8 and TPAD_$DATA 0xE6939A after a widened word sum and a
    ///   product);
    /// - signed `div` by 2^k: [`Self::div_pow2`]; unsigned: `lsr.l`
    ///   (0xE14086 `lsr.l #1,D0`; 0xE06E2E `lsr.l #8,D0; lsr.l #2,D0`);
    /// - other `div`: `M$DI<s>$LLW` (the divisor a word) or `M$DI<s>$LLL`
    ///   (0xE1405E `M$DIU$LLW` by `#12`; TIME_$GET_ADJUST 0xE16AC6
    ///   `M$DIS$LLL` by `#250000`);
    /// - `mod` by a word: `M$OI<s>$WLW`, a word result (AREA_$FREE_FROM
    ///   0xE07FEE `M$OIU$WLW` for `mod 11`; 0xE30D3E `M$OIS$WLW` for `mod
    ///   32`); used as a longword, an unsigned one is zero-extended into a
    ///   new register (0xE07FF6 `clr.l D1; move.w D0,D1`; 0xE07A3A,
    ///   0xE07DD2, 0xE5AA4C), a signed one is refused (all 16 `M$OIS$WLW`
    ///   results are used as words);
    /// - other `mod`: `M$OIS$LLL` / `M$OIU$LLL` (one entry, 0xE0AD7C;
    ///   TIME_$GET_ADJUST 0xE16AD8).
    fn div(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, ty: Scalar, narrow: bool) -> Res<Val> {
        let signed = ty.is_signed();
        let c = match rhs {
            Expr::Const { value, .. } => Some(*value),
            _ => None,
        };
        if c == Some(0) {
            return Err(unsupported(self.span, "a division by 0 is"));
        }
        let pow2 = c.filter(|&c| c > 1 && c & (c - 1) == 0);
        match ty.width {
            Width::Byte => Err(unsupported(self.span, "a byte `div`/`mod` is")),
            Width::Word => {
                if [lhs, rhs].iter().any(|e| {
                    matches!(e, Expr::Convert { value, .. }
                        if value.ty().width == ty.width && value.ty().sign != ty.sign)
                }) {
                    return Err(unsupported(
                        self.span,
                        "a word `div`/`mod` mixing signed and unsigned operands is",
                    ));
                }
                if c.is_some_and(|c| c < 1) || c == Some(1) {
                    return Err(unsupported(
                        self.span,
                        "a word `div`/`mod` by a constant below 2 is",
                    ));
                }
                if let Some(c) = pow2 {
                    if !signed && op == BinOp::Div {
                        // `lsr.w`, as `lsr.l` for longwords (0xE14086
                        // `lsr.l #1,D0`; 0xE06E2E): LOG_$READ's worker
                        // 0xE177D2..0xE177D8 `move.w (A0),D0; lsr.w #1,D0`
                        // halves a `0..65535` word (`log/read_internal.c`
                        // `*actual_len >> 1`; `div 2` in our
                        // reconstruction, Uncertain).
                        let d = self.eval_d(lhs)?;
                        self.shift_by(ShiftOp::Lsr, d, c.trailing_zeros(), Width::Word);
                        return Ok(Val::Tmp(d));
                    }
                    if !signed {
                        return Err(unsupported(
                            self.span,
                            "an unsigned `mod` by a power of two (mask or division?) is",
                        ));
                    }
                    if op == BinOp::Div {
                        return self.div_pow2(lhs, c, Width::Word);
                    }
                }
                let d = if signed {
                    let d = self.eval_d(lhs)?;
                    self.out.push(Insn::Ext {
                        size: Size::L,
                        dst: d,
                    });
                    d
                } else {
                    self.zero_extend(lhs)?
                };
                let src = match c {
                    Some(c) => Ea::Imm(c),
                    None => self.divisor(rhs)?,
                };
                self.out.push(Insn::Div {
                    signed,
                    src,
                    dst: d,
                });
                if op == BinOp::Mod {
                    self.out.push(Insn::Swap(d));
                }
                Ok(Val::Tmp(d))
            }
            Width::Long => {
                if let (Some(l), Some(r)) =
                    (Self::word_part(lhs, ty.sign), Self::word_part(rhs, ty.sign))
                {
                    // Two words (a `0..65535` subrange has an integer32
                    // host): the word division; used as a longword, an
                    // unsigned result is zero-extended into a new register
                    // (0xE382B4..0xE382CC `clr.l D0; move.w (2,A1),D0; clr.l
                    // D2; divu.w #101,D0; swap D0; move.w D0,D2; ... lsl.l
                    // #6,D2`).
                    let wty = Scalar {
                        width: Width::Word,
                        sign: ty.sign,
                    };
                    let v = self.div(op, &l, &r, wty, false)?;
                    if narrow {
                        return Ok(v);
                    }
                    if signed {
                        return Err(unsupported(
                            self.span,
                            "a signed word quotient or remainder used as a longword is",
                        ));
                    }
                    let d = self.alloc_d()?;
                    self.out.push(Insn::Clr {
                        size: Size::L,
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    self.out.push(Insn::Move {
                        size: Size::W,
                        src: Self::ea_val(&v),
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    return Ok(Val::Tmp(d));
                }
                let bw = Self::word_part(rhs, ty.sign);
                if pow2.is_some() && !signed && op == BinOp::Mod {
                    return Err(unsupported(
                        self.span,
                        "an unsigned `div`/`mod` by a power of two (shift or mask?) is",
                    ));
                }
                if op == BinOp::Div {
                    // A power of two is shifted even when the quotient is
                    // used as a word (0xE30D26..0xE30D32 `asr.l #5`, then
                    // `move.w D0,(-18,A6)`).
                    if let Some(c) = pow2 {
                        if signed {
                            return self.div_pow2(lhs, c, Width::Long);
                        }
                        let d = self.eval_d(lhs)?;
                        self.shift_by(ShiftOp::Lsr, d, c.trailing_zeros(), Width::Long);
                        return Ok(Val::Tmp(d));
                    }
                    if let (true, Some(b)) = (narrow, &bw) {
                        if c.is_some_and(|c| c < 0) {
                            return Err(unsupported(
                                self.span,
                                "a division by a negative constant is",
                            ));
                        }
                        let d = self.eval_d(lhs)?;
                        let src = match c {
                            Some(c) => Ea::Imm(c),
                            None => self.divisor(b)?,
                        };
                        self.out.push(Insn::Div {
                            signed,
                            src,
                            dst: d,
                        });
                        return Ok(Val::Tmp(d));
                    }
                    self.marith_op("DI", ty, lhs, rhs)?;
                    return Ok(Val::Tmp(0));
                }
                let s = if signed { 'S' } else { 'U' };
                if let Some(b) = bw {
                    self.marith(&format!("M$OI{s}$WLW"), lhs, &b)?;
                    if narrow {
                        return Ok(Val::Tmp(0));
                    }
                    if signed {
                        return Err(unsupported(
                            self.span,
                            "a signed `mod` by a word used as a longword (the 16 `M$OIS$WLW` \
                             results in the image are all used as words) is",
                        ));
                    }
                    let d = self.alloc_d()?;
                    self.out.push(Insn::Clr {
                        size: Size::L,
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    self.out.push(Insn::Move {
                        size: Size::W,
                        src: Ea::Reg(Reg::D(0)),
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    return Ok(Val::Tmp(d));
                }
                if signed && Self::word_part(lhs, ty.sign).is_some() {
                    return Err(unsupported(
                        self.span,
                        "a word `mod` a longword (`M$OIS$WWL`, which no compiled code calls, or \
                         `M$OIS$LLL`) is",
                    ));
                }
                self.marith(&format!("M$OI{s}$LLL"), lhs, rhs)?;
                Ok(Val::Tmp(0))
            }
        }
    }

    /// A word divisor as the source operand of `div<s>.w`: a variable in
    /// memory or held in a data register.  A computed divisor (whose
    /// evaluation order against the dividend is not catalogued) is refused.
    fn divisor(&mut self, e: &Expr) -> Res<Ea> {
        let start = self.out.len();
        let v = self.eval(e)?;
        if self.out.len() > start {
            return Err(unsupported(
                self.span,
                "a divisor of `div<s>.w` that takes instructions (computed, or reached through a \
                 register loaded for it: before or after the dividend?) is",
            ));
        }
        match v {
            Val::Imm(c) => Ok(Ea::Imm(c)),
            Val::Mem(ea) => Ok(ea),
            Val::Reg(r @ Reg::D(_)) => Ok(Ea::Reg(r)),
            _ => Err(unsupported(self.span, "this divisor is")),
        }
    }

    /// The unsigned word `e` zero-extended in a new data register:
    /// `clr.l Dn; move.w x,Dn` (0xE4CE1A..0xE4CE1C), `andi.l #0xffff` on a
    /// computed value (0xE11818).
    fn zero_extend(&mut self, e: &Expr) -> Res<u8> {
        let v = self.eval(e)?;
        if let Val::Tmp(d) = v {
            self.out.push(Insn::AluImm {
                op: AluOp::And,
                size: Size::L,
                imm: 0xffff,
                dst: Ea::Reg(Reg::D(d)),
            });
            return Ok(d);
        }
        let d = self.alloc_d()?;
        self.out.push(Insn::Clr {
            size: Size::L,
            dst: Ea::Reg(Reg::D(d)),
        });
        self.out.push(Insn::Move {
            size: Size::W,
            src: Self::ea_val(&v),
            dst: Ea::Reg(Reg::D(d)),
        });
        Ok(d)
    }

    /// Signed `x div 2^k` at width `w` (codegen-patterns 8): the value in a
    /// register, rounded toward zero by adding 2^k - 1 when it is negative,
    /// then shifted right arithmetically:
    ///
    /// ```text
    ///         bpl.s   L               ; on the flags of the instruction
    ///         addq/addi #2^k-1,Dn     ; that put x in Dn, or after tst
    /// L:      asr     #k,Dn           ; split above 8
    /// ```
    ///
    /// CAL_$WEEKDAY 0xE68698..0xE6869E `move.w D1,D3; bpl.s; addq.w #3,D3;
    /// asr.w #2,D3` (`y div 4`); PROC1_$LOADAV_CALLBACK 0xE14BEE..0xE14BF8
    /// `move.l (A5),D0; bpl.s; addi.l #255,D0; asr.l #8,D0`, and after a
    /// call 0xE14C0A `tst.l D0; bpl.s; ...`; `div 1024` as `addi.l #1023;
    /// asr.l #8; asr.l #2` (0xE06D5C, 0xE0896E); `tools/arithsurvey.py`
    /// finds no `div` by a power of two done with `divs.w` (63 such
    /// sequences, e.g. by 2 at 0xE17662, by 8 at 0xE3A3A8; the one `divs.w
    /// #4`, CAL_$DECODE_TIME 0xE68804, is a `mod`).  The join is not aligned (forward
    /// targets, codegen-patterns 7; the `movea.l A0,A0` before it at
    /// 0xE30D2E is one of MST_$INIT's exceptions).
    fn div_pow2(&mut self, x: &Expr, c: i64, w: Width) -> Res<Val> {
        let size = size_of(w);
        let d = self.eval_d(x)?;
        if !self.sets_flags(d, w) {
            self.out.push(Insn::Tst {
                size,
                dst: Ea::Reg(Reg::D(d)),
            });
        }
        if self.unpopped {
            return Err(unsupported(
                self.span,
                "a conditional branch after a call whose pop was skipped (codegen-patterns 1.1) is",
            ));
        }
        let l = self.new_label();
        self.jump(Cond::Pl, l);
        let bias = c - 1;
        let dst = Ea::Reg(Reg::D(d));
        self.out.push(if bias <= 8 {
            Insn::Addq {
                size,
                value: bias as u8,
                dst,
            }
        } else {
            Insn::AluImm {
                op: AluOp::Add,
                size,
                imm: bias,
                dst,
            }
        });
        self.place(l);
        self.shift_by(ShiftOp::Asr, d, c.trailing_zeros(), w);
        Ok(Val::Tmp(d))
    }

    /// Whether the instruction just selected wrote data register `d` at
    /// width `w` and set N from that value (a `move`, an extension, an
    /// arithmetic or logical operation, a shift into `d`).
    fn sets_flags(&self, d: u8, w: Width) -> bool {
        let size = size_of(w);
        let reg = Ea::Reg(Reg::D(d));
        match self.out.last() {
            Some(Insn::Move { size: s, dst, .. }) => *s == size && *dst == reg,
            Some(Insn::Ext { size: s, dst }) => *s == size && *dst == d,
            Some(Insn::Alu { size: s, dst, .. }) => *s == size && *dst == d,
            Some(Insn::Shift { size: s, dst, .. }) => *s == size && *dst == d,
            Some(
                Insn::AluImm { size: s, dst, .. }
                | Insn::Addq { size: s, dst, .. }
                | Insn::Subq { size: s, dst, .. }
                | Insn::Unary { size: s, dst, .. },
            ) => *s == size && *dst == reg,
            _ => false,
        }
    }

    /// `op` by `n` bits on Dd, at most 8 at a time (0xE02AA6).
    fn shift_by(&mut self, op: ShiftOp, d: u8, n: u32, w: Width) {
        let mut n = n;
        while n > 0 {
            let k = n.min(8);
            self.out.push(Insn::Shift {
                op,
                size: size_of(w),
                count: ShiftCount::Imm(k as u8),
                dst: d,
            });
            n -= k;
        }
    }

    /// An M$ARITH call `M$<kind><s>$LL<W|L>(a, b)` of type `ty`: `W` when
    /// `b` is a word (or a constant that fits one, [`Self::word_part`]).
    fn marith_op(&mut self, kind: &str, ty: Scalar, a: &Expr, b: &Expr) -> Res<()> {
        let s = if ty.is_signed() { 'S' } else { 'U' };
        match Self::word_part(b, ty.sign) {
            Some(bw) => self.marith(&format!("M${kind}{s}$LLW"), a, &bw),
            None => self.marith(&format!("M${kind}{s}$LLL"), a, b),
        }
    }

    /// A call of the M$ARITH entry `name` with argument 1 `a` (a longword)
    /// and argument 2 `b` (a word or a longword); the result is in D0, a
    /// longword or (`M$OI*$WLW`) a word (codegen-patterns 8; the frames in
    /// `../source/domain_os/math/sau2/arith.s`: `(4,SP)` a, `(8,SP)` b).
    /// Survey of the 92 calls in compiled code (`tools/arithsurvey.py`):
    ///
    /// 1. The operands are evaluated first, like any operator's
    ///    (0xE14BEE..0xE14C00: `x div 256` computed, then both pushes).
    /// 2. The pad `subq.l #2,SP` when `b` is a word (64 of 64 calls with a
    ///    word argument; none of the 28 others).
    /// 3. `b`, then `a`: a word by `move.w #c,-(SP)` (37), `move.w
    ///    ea,-(SP)` (27); a longword by `move.l` (0xE16ABE `move.l
    ///    #250000,-(SP)`), 0 by `clr.l -(SP)` (0xE10A74).
    /// 4. `jsr`, then `addq.w #8,SP` (89 of 92), except where the
    ///    procedure-call rule skips the pop (0xE16A92, 0xE16AD8, 0xE19BA6;
    ///    codegen-patterns 1.1).
    /// 5. The result is used in D0, modified in place (0xE39C3E `add.l
    ///    (14,A6),D0`; CAL_$WEEKDAY 0xE686C8 `muls.w #153,D0`; 0xE6929C
    ///    `divs.w #10,D0`).
    ///
    /// The call writes only D0 and A0 (`regs.rs`; 12.4 R3: D1/A1 live
    /// across 31 of 83 modelled calls).  It is not recorded for
    /// [`held_through_calls`] (R4(c)): a value whose last use is in its
    /// pushes stays in a scratch register at TIME_$GET_ADJUST 0xE16AD6
    /// (`move.l D1,-(SP)`, a local variable); **Uncertain** for a
    /// parameter loaded at entry.
    ///
    /// The same operand as `a` and `b` is pushed once and copied, `move.l
    /// (SP),-(SP)` (OS_$SHUTDOWN 0xE6D604, one site).
    ///
    /// Refused: both operands computed (the order differs, 0xE10A86..0xE10A98
    /// against 0xE69560..0xE69572); a computed operand with a pad (the
    /// pad is scheduled into the computation, 22 calls, e.g. CAL_$WEEKDAY
    /// 0xE686B0, or after it, 0xE0920C: codegen-patterns 12.4 R9), except
    /// another M$ARITH call's result as `a` (0xE16962..0xE16970: the inner
    /// call and its pop, then the outer pad); a second call while a
    /// result in D0 is still needed; a call among another call's
    /// arguments.
    fn marith(&mut self, name: &str, a: &Expr, b: &Expr) -> Res<()> {
        if self.in_args {
            return Err(unsupported(
                self.span,
                "an M$ARITH call among another call's arguments is",
            ));
        }
        let pre_called = self.called;
        let start = self.out.len();
        // (An error ends the routine's selection: the depth need not be
        // restored then.)
        self.marith_depth += 1;
        let av = self.eval(a)?;
        let mid = self.out.len();
        let bv = self.eval(b)?;
        self.marith_depth -= 1;
        let (a_code, b_code) = (mid > start, self.out.len() > mid);
        if a_code && b_code {
            return Err(unsupported(
                self.span,
                "an M$ARITH call with both operands computed (evaluation order not settled) is",
            ));
        }
        let is_d0 = |v: &Val| matches!(v, Val::Tmp(0) | Val::Reg(Reg::D(0)));
        let pad = b.ty().width == Width::Word;
        if pad && (a_code || b_code) {
            let ends_in_call = match &self.out[start..mid] {
                [.., Insn::Jsr(Ea::AbsL(s, _)), Insn::Addq { .. }]
                | [.., Insn::Jsr(Ea::AbsL(s, _))] => s.starts_with("M$"),
                _ => false,
            };
            if b_code || !ends_in_call || !is_d0(&av) {
                return Err(unsupported(
                    self.span,
                    "an M$ARITH call with a word argument and a computed operand (the pad is \
                     scheduled into the computation, codegen-patterns 8, 12.4 R9) is",
                ));
            }
        }
        if (pre_called || self.called) && !is_d0(&av) && !is_d0(&bv) {
            return Err(unsupported(
                self.span,
                "two function calls in one statement are",
            ));
        }
        if pad {
            self.out.push(Insn::Subq {
                size: Size::L,
                value: 2,
                dst: Ea::Reg(Reg::A(SP)),
            });
        }
        self.push_val(bv, b.ty().width)?;
        if a == b && !a_code && !b_code {
            // The same operand twice: the second copy from the stack
            // (OS_$SHUTDOWN 0xE6D600..0xE6D606 `move.l (-596,A6),-(SP);
            // move.l (SP),-(SP); jsr M$MIS$LLL`, `x * x`; codegen-patterns
            // 1.5 has the same for a constant passed twice by address).
            self.out.push(Insn::Move {
                size: Size::L,
                src: Ea::Ind(SP),
                dst: Ea::PreDec(SP),
            });
        } else {
            self.push_val(av, Width::Long)?;
        }
        self.out.push(Insn::Jsr(Ea::AbsL(name.to_string(), 0)));
        if self.call_no_pop && self.marith_depth == 0 {
            self.unpopped = true;
        } else {
            self.out.push(Insn::Addq {
                size: Size::W,
                value: 8,
                dst: Ea::Reg(Reg::A(SP)),
            });
        }
        self.called = true;
        Ok(())
    }

    /// Push an operand at width `w` (codegen-patterns 1.5): a longword
    /// constant by [`Self::push_long_const`], a word constant by `move.w
    /// #c,-(SP)` (`clr.w` for 0), anything else by `move.<w> ea,-(SP)`.
    fn push_val(&mut self, v: Val, w: Width) -> Res<()> {
        let dst = Ea::PreDec(SP);
        match (v, w) {
            (_, Width::Byte) => {
                return Err(unsupported(self.span, "a byte M$ARITH operand is"));
            }
            (Val::Imm(c), Width::Long) => self.push_long_const(c),
            (Val::Imm(0), Width::Word) => self.out.push(Insn::Clr { size: Size::W, dst }),
            (Val::Imm(c), Width::Word) => self.out.push(Insn::Move {
                size: Size::W,
                src: Ea::Imm(imm(c, Width::Word)),
                dst,
            }),
            (v, w) => self.out.push(Insn::Move {
                size: size_of(w),
                src: Self::ea_val(&v),
                dst,
            }),
        }
        Ok(())
    }

    /// `lshft`, `rshft`, `arshft` (ref pp. 4-109, 4-171, 4-29): `lsl`,
    /// `lsr`, `asr` on the value in a register.  A constant count above 8
    /// is split into shifts of 8 and the rest (0xE02AA6 `lsl.l #8,D0; lsl.l
    /// #2,D0`; 0xE00876 `lsl.l #8; lsl.l #7`); a count in a register shifts
    /// by it (0xE02D52 `lsl.w D1,D3`).
    fn shift(&mut self, op: BinOp, x: &Expr, count: &Expr, ty: Scalar) -> Res<u8> {
        let size = size_of(ty.width);
        let sop = match op {
            BinOp::Shl => ShiftOp::Lsl,
            BinOp::Shr => ShiftOp::Lsr,
            _ => ShiftOp::Asr,
        };
        let d = self.eval_d(x)?;
        match count {
            Expr::Const { value, .. } => {
                let bits = (ty.bytes() * 8) as i64;
                if !(0..bits).contains(value) {
                    return Err(unsupported(
                        self.span,
                        "a shift count outside 0..bits-1 (ref: results unpredictable) is",
                    ));
                }
                let mut n = *value as u8;
                while n > 0 {
                    let k = n.min(8);
                    self.out.push(Insn::Shift {
                        op: sop,
                        size,
                        count: ShiftCount::Imm(k),
                        dst: d,
                    });
                    n -= k;
                }
            }
            c => {
                let r = match self.eval(c)? {
                    Val::Reg(Reg::D(r)) | Val::Tmp(r) => r,
                    v => self.in_tmp(v, c.ty().width)?,
                };
                self.out.push(Insn::Shift {
                    op: sop,
                    size,
                    count: ShiftCount::Reg(r),
                    dst: d,
                });
            }
        }
        Ok(d)
    }

    /// Multiply the value in data register `d` by the constant `c`, in
    /// place, at width `w` (codegen-patterns 5.2, 8, 12.4 R8: ref p. 6-28,
    /// "Transforms integer multiplication by a constant into shift and add
    /// instructions").  With `c` = 2^s × m, m odd:
    ///
    /// - 2^s: `add.<w> Dn,Dn` for s = 1 (AUDIT_$IS_PROCESS_AUDITED
    ///   0xE70DA6), `lsl.<w> #s` otherwise, split above 8 (FIM_$GET_FIM_ADDR
    ///   0xE0AA16 `lsl.w #2`; 0xE02AA6 `lsl.l #8; lsl.l #2`);
    /// - m = 2^k + 1: the 2^s part, then a copy shifted by k and added
    ///   (RIP_$PACKET_LENGTH 0xE6886C..0xE68872, ×6: `add.w D0,D0; move.w
    ///   D0,D1; add.w D1,D1; add.w D1,D0`; ×9 0xE14404; ×72 DISK_$REVALIDATE
    ///   0xE6C072..0xE6C078; ×257 0xE074CC);
    /// - m = 2^k - 1 (k >= 3): the 2^s part, a copy, the value negated, the
    ///   copy shifted by k and added (×28 PEB_$GET_FP 0xE5AEF6..0xE5AF04:
    ///   `lsl.w #2,D0; move.w D0,D1; neg.w D0; lsl.w #3,D1; add.w D1,D0`;
    ///   ×56, ×60, ×120, ×248);
    /// - other odd parts with four or more bits set: `mulu.w #c` (BAT_$GET_BAT_STEP
    ///   0xE3BB6A `mulu.w #0x234`; survey of compiled code: every constant
    ///   multiplied with `mulu.w`/`muls.w` (92, 150, 153, 204, 228, 244, 250,
    ///   268, 300, 336, 363, 564, 580, 1083, 1244, 1624, 5240, 12345) has an
    ///   odd part that is not 2^k ± 1, and every one done with shifts (3, 5,
    ///   6, 9, 10, 12, 17, 18, 20, 24, 28, 36, 48, 56, 60, 72, 96, 120, 144,
    ///   192, 248, 257) has one that is, except ×26 (0xE71BBA, odd part 13
    ///   done bit by bit).  Odd parts of three bits are done both ways (13
    ///   with shifts; 21, 67, 145 with `mulu`): refused.  `mulu.w` at
    ///   a word width only (the low word of the product is the same for
    ///   both signs; 94 `mulu.w #c` feed a word index, 118 `muls.w #c` a
    ///   longword one, codegen-patterns 5.2).
    ///
    /// The partial product takes a new register (R8: D1 at 0xE68870, D3 at
    /// 0xE0CAAE).
    fn mul_const(&mut self, d: u8, c: i64, w: Width) -> Res<()> {
        let size = size_of(w);
        let reg = Ea::Reg(Reg::D(d));
        if !(0..=0xffff).contains(&c) {
            return Err(unsupported(
                self.span,
                "a multiplication by a negative or 32-bit constant is",
            ));
        }
        if c == 0 {
            return Err(unsupported(self.span, "a multiplication by 0 is"));
        }
        let c = c as u32;
        let s = c.trailing_zeros();
        let m = c >> s;
        let plus = m > 1 && (m - 1).is_power_of_two();
        let minus = m > 3 && (m + 1).is_power_of_two();
        if m != 1 && !plus && !minus {
            if m.count_ones() < 4 {
                return Err(unsupported(
                    self.span,
                    format!(
                        "a multiplication by {c} (shifts and adds or `mulu.w`, not settled for an \
                         odd part of three bits) is"
                    ),
                ));
            }
            if w != Width::Word {
                return Err(unsupported(
                    self.span,
                    "a 32-bit multiplication by a constant with `mul<s>.w` is",
                ));
            }
            self.out.push(Insn::Mul {
                signed: false,
                src: Ea::Imm(c as i64),
                dst: d,
            });
            return Ok(());
        }
        self.shift_left(d, s, w);
        if m == 1 {
            return Ok(());
        }
        let u = self.alloc_d()?;
        self.out.push(Insn::Move {
            size,
            src: reg.clone(),
            dst: Ea::Reg(Reg::D(u)),
        });
        let k = if plus {
            (m - 1).trailing_zeros()
        } else {
            self.out.push(Insn::Unary {
                op: UnaryOp::Neg,
                size,
                dst: reg.clone(),
            });
            (m + 1).trailing_zeros()
        };
        self.shift_left(u, k, w);
        self.out.push(Insn::Alu {
            op: AluOp::Add,
            size,
            src: Ea::Reg(Reg::D(u)),
            dst: d,
        });
        Ok(())
    }

    /// `d <<= n`: `add.<w> Dd,Dd` for 1, `lsl.<w>` by at most 8 at a time
    /// otherwise (see [`Self::mul_const`]).
    fn shift_left(&mut self, d: u8, n: u32, w: Width) {
        let size = size_of(w);
        if n == 1 {
            self.out.push(Insn::Alu {
                op: AluOp::Add,
                size,
                src: Ea::Reg(Reg::D(d)),
                dst: d,
            });
            return;
        }
        let mut n = n;
        while n > 0 {
            let k = n.min(8);
            self.out.push(Insn::Shift {
                op: ShiftOp::Lsl,
                size,
                count: ShiftCount::Imm(k as u8),
                dst: d,
            });
            n -= k;
        }
    }

    /// `value` converted to `ty` (ref 4.3.1, p. 4-5):
    ///
    /// - a signed word widened: `move.w x,Dn; ext.l Dn` (0xE02C28..0xE02C2A
    ///   `move.w D2,D0; ext.l D0`, then stored with `move.l`; 610 `ext.l`
    ///   in compiled code);
    /// - an unsigned word (`0..65535`): `clr.l Dn; move.w x,Dn`
    ///   (0xE177A6..0xE177AA, codegen-patterns 5); one already in a
    ///   register is masked, `andi.l #0xffff` (0xE11818);
    /// - an unsigned byte (`char`) to a word: `clr.w Dn; move.b x,Dn`
    ///   (0xE0E316..0xE0E318); a signed byte: `move.b; ext.w`;
    /// - narrowing (`chr`, an integer32 into a word): the low bytes of the
    ///   value in a register, no instruction.
    ///
    /// A byte widened to a longword is done both as `clr.l; move.b` and
    /// as `move.b; andi.l #0xff` (0xE00864..0xE00868): refused.
    fn convert(&mut self, value: &Expr, ty: Scalar) -> Res<Val> {
        let from = value.ty();
        if from.width == ty.width {
            return self.eval(value);
        }
        if from.bytes() > ty.bytes() {
            // A longword quotient or remainder used as a word: `div<s>.w`
            // in place, or `M$OI*$WLW` ([`Self::div`]).  A word one
            // narrowed to a byte (`chr(w mod 10)`) is the word operation,
            // below.
            if let Expr::Binary {
                op: op @ (BinOp::Div | BinOp::Mod),
                lhs,
                rhs,
                ty: t,
            } = value
            {
                if from.width != Width::Long {
                    let v = self.eval(value)?;
                    return Ok(Val::Tmp(self.in_tmp(v, from.width)?));
                }
                if ty.width != Width::Word {
                    return Err(unsupported(
                        self.span,
                        "a longword `div`/`mod` used as a byte is",
                    ));
                }
                let v = self.div(*op, lhs, rhs, *t, true)?;
                return Ok(Val::Tmp(self.in_tmp(v, from.width)?));
            }
            let v = self.eval(value)?;
            return Ok(Val::Tmp(self.in_tmp(v, from.width)?));
        }
        if matches!(value, Expr::Convert { value: inner, .. }
            if matches!(**inner, Expr::Binary { op: BinOp::Div | BinOp::Mod, .. }))
        {
            // `dpc-ir` divides a word sum in 32 bits and narrows the result
            // (codegen-patterns 8); widened again, would the original have
            // used the longword quotient (M$ARITH) or the word one?
            return Err(unsupported(
                self.span,
                "a word `div`/`mod` of a sum, difference or product used as a longword is",
            ));
        }
        if let Expr::Binary { op: BinOp::Shl, .. } = value {
            // A word `lshft` used as a longword: shifted at the word's
            // width and extended, or extended and shifted in 32 bits (ref
            // 4.3.1, p. 4-5, and the expansion pp. 4-29 and 4-171 describe
            // for `arshft` and `rshft`; p. 4-109 says nothing for `lshft`)?
            // Not settled; the two differ when bits reach the sign.
            return Err(unsupported(
                self.span,
                "an `lshft` of a word operand used as a longword is",
            ));
        }
        let widening = matches!(
            (from.width, ty.width),
            (Width::Word, Width::Long) | (Width::Byte, Width::Word)
        );
        if let (Some(leaf), true) = (self.cse_leaf(value), widening) {
            let key = CseKey::Widen(value.clone(), ty.width);
            return self.cse_value(key, leaf, |s| match s.widen(value, ty)? {
                Val::Tmp(d) => Ok(d),
                _ => unreachable!("a widening computes into a new register"),
            });
        }
        self.widen(value, ty)
    }

    /// The widening of [`Self::convert`].
    fn widen(&mut self, value: &Expr, ty: Scalar) -> Res<Val> {
        let from = value.ty();
        let unsigned = from.sign == Sign::Unsigned;
        match (from.width, ty.width) {
            (Width::Word, Width::Long) | (Width::Byte, Width::Word) if unsigned => {
                let v = self.eval(value)?;
                if let Val::Tmp(d) = v {
                    let mask = if from.width == Width::Word {
                        0xffff
                    } else {
                        0xff
                    };
                    self.out.push(Insn::AluImm {
                        op: AluOp::And,
                        size: size_of(ty.width),
                        imm: mask,
                        dst: Ea::Reg(Reg::D(d)),
                    });
                    return Ok(Val::Tmp(d));
                }
                let d = self.alloc_d()?;
                self.out.push(Insn::Clr {
                    size: size_of(ty.width),
                    dst: Ea::Reg(Reg::D(d)),
                });
                self.out.push(Insn::Move {
                    size: size_of(from.width),
                    src: Self::ea_val(&v),
                    dst: Ea::Reg(Reg::D(d)),
                });
                Ok(Val::Tmp(d))
            }
            (Width::Word, Width::Long) | (Width::Byte, Width::Word) => {
                let d = self.eval_d(value)?;
                self.out.push(Insn::Ext {
                    size: size_of(ty.width),
                    dst: d,
                });
                Ok(Val::Tmp(d))
            }
            _ => Err(unsupported(
                self.span,
                "widening a byte to a longword (`clr.l; move.b` or `move.b; andi.l #0xff`) is",
            )),
        }
    }

    /// A function call in an expression (ref 5.2; codegen-patterns 1, 3):
    /// the call sequence of a procedure call ([`Self::call`]), then the
    /// result in D0, read from there (0xE04704 `move.l D0,(A3)`; 0xE0082E
    /// `tst.b D0`; 0xE01BA4 `move.w D0,D1` before arithmetic).  The pop
    /// follows the procedure-call rule ([`Self::call_no_pop`]).  Not
    /// supported yet: a call among another call's arguments (the two pops
    /// are merged, 0xE69FF6, 0xE748E6), two calls in one statement (the
    /// first result would have to be saved), results in A0.
    fn expr_call(&mut self, c: &Call) -> Res<Val> {
        self.call_value(c)?;
        Ok(Val::Reg(Reg::D(0)))
    }

    /// A call of a function whose result is in A0 (a pointer, ref 5.5.10,
    /// p. 5-20), read there: returns A0 (physical).  The caller reads it
    /// in the instruction after the call and its pop (`tools/resultsurvey.py
    /// callers`, 234 calls; codegen-patterns 3, "Callers of pointer
    /// functions"): `move.l A0,<mem>` (a frame slot 34, other memory 21),
    /// `move.l A0,Dn` or `movea.l A0,An` into a register-held local (48
    /// and 108), A0 as a base (8), `cmpa.w #0,A0` (12); never pushed as an
    /// argument.
    fn call_a0(&mut self, c: &Call) -> Res<u8> {
        self.call_value(c)?;
        Ok(0)
    }

    /// The call of [`Self::expr_call`] / [`Self::call_a0`].
    fn call_value(&mut self, c: &Call) -> Res<()> {
        if self.in_args {
            return Err(unsupported(
                self.span,
                "a function call among another call's arguments (pops merged, \
                 codegen-patterns 1.1) is",
            ));
        }
        if self.called {
            return Err(unsupported(
                self.span,
                "two function calls in one statement are",
            ));
        }
        // Inside an M$ARITH call's operands the outer call's pushes follow:
        // popped.
        let no_pop = self.call_no_pop && self.marith_depth == 0;
        self.call(c, no_pop)?;
        self.unpopped |= no_pop;
        self.called = true;
        Ok(())
    }

    /// A comparison as a boolean value: the compare, then `s<cc>` into the
    /// data register the compare read last, or a new one (AUDIT_$IS_PROCESS_AUDITED
    /// 0xE70DA8..0xE70DAC `tst.w (-2,A5,D0.w); seq D0`, the index register;
    /// ACL_$IN_SUBSYS 0xE470B6..0xE470BA `tst.w (-0x3d5a,A1); sgt D0`, the
    /// index register of the `lea` before; module-local 0xE464AA..0xE464B0
    /// `cmpi.w #2,(-2,A0,D1.w); sne D1`; PROC1_$TST_LOCK 0xE148DC..0xE148DE
    /// `btst D0,D1; sne D1`, the set's register).  **Uncertain:** the rule
    /// is read off these four sites.  Only comparisons are supported.
    fn test_value(&mut self, c: &IrCond) -> Res<u8> {
        let IrCond::Compare { op, lhs, rhs } = c else {
            return Err(unsupported(
                self.span,
                "a condition other than a comparison as a value (`not`, `and then`, a boolean \
                 variable) is",
            ));
        };
        let start = self.out.len();
        let cc = self.compare(*op, lhs, rhs)?;
        // The data registers the compare's operands used, last allocated
        // first.
        let mut used: Vec<u8> = self.out[start..]
            .iter()
            .flat_map(|i| crate::regs::access(i).uses)
            .filter_map(|r| match r {
                Reg::D(n) if crate::regs::is_virtual(r) => Some(n),
                _ => None,
            })
            .collect();
        used.sort_unstable();
        // A register held for the whole body is not overwritten.
        let held: Vec<u8> = self
            .pinned
            .iter()
            .filter_map(|&(_, r)| match r {
                Reg::D(n) => Some(n),
                _ => None,
            })
            .collect();
        let d = match used.iter().rev().find(|n| !held.contains(n)) {
            Some(&n) => n,
            None => self.alloc_d()?,
        };
        self.out.push(Insn::Scc {
            cond: cc,
            dst: Ea::Reg(Reg::D(d)),
        });
        Ok(d)
    }

    /// `lhs op rhs` with a left operand that is not a variable: the value
    /// computed into a data register, then `cmpi #c,Dn` (0xE1216E `add.l
    /// D6,D7; cmpi.l #0x500,D7`) or `cmp.<s> rhs,Dn`.  Against 0, whether
    /// the original relied on the flags of the operation or tested again is
    /// not catalogued: refused.
    fn compare_computed(&mut self, op: Rel, lhs: &Expr, rhs: &Expr) -> Res<Cond> {
        let ty = lhs.ty();
        let size = size_of(ty.width);
        let signed = ty.sign == Sign::Signed;
        if let Expr::Const { .. } = lhs {
            // A constant on the left (`3 < w`): whether the original
            // swapped the operands is not catalogued.
            return Err(unsupported(
                self.span,
                "a comparison whose left operand is not a variable is",
            ));
        }
        if matches!(rhs, Expr::Const { value: 0, .. }) {
            return Err(unsupported(
                self.span,
                "comparing a computed value with 0 (the flags of the operation or a `tst`) is",
            ));
        }
        let lv = self.eval(lhs)?;
        let d = match lv {
            Val::Reg(Reg::D(r)) => r,
            v => self.in_tmp(v, ty.width)?,
        };
        match rhs {
            Expr::Const { value, .. } => self.out.push(Insn::Cmpi {
                size,
                imm: imm(*value, ty.width),
                dst: Ea::Reg(Reg::D(d)),
            }),
            Expr::Load { .. } => {
                let src = match self.eval(rhs)? {
                    Val::Mem(ea) => ea,
                    Val::Reg(r) => Ea::Reg(r),
                    _ => unreachable!(),
                };
                self.out.push(Insn::Cmp { size, src, dst: d });
            }
            _ => return Err(unsupported(self.span, "comparing two computed values is")),
        }
        Ok(rel_cond(op, signed))
    }

    /// `k in s` for a constant `k` and a set of one word or one longword:
    /// the set's value in a data register, then `btst #k` on it
    /// (REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A416 `move.w (22,A0),D0; btst
    /// #13,D0`; 0xE013BA `move.w (190,A2),D0; btst #4,D0`; 0xE3EF4A
    /// `move.l (d,An),Dn` then `btst #k` with k >= 16 for a longword).  In
    /// compiled code a `btst #k` on a register that a `move.w` from memory
    /// loaded has 0 <= k <= 15 (250 sites, 58 of them with k < 8); `btst
    /// #k,<memory>` (422 sites) needs k < 8 and is taken to be the test of
    /// a bit field or a byte set (not catalogued).  Returns the register,
    /// a new one when `value` (the boolean is then set in it with `sne`,
    /// 0xE4A41A `sne D0`), else possibly a held one.
    fn set_bit_test(&mut self, elem: &Expr, set: &Expr, w: Width, value: bool) -> Res<u8> {
        let Expr::Const { value: k, .. } = elem else {
            unreachable!()
        };
        if !(0..(w.bytes() * 8) as i64).contains(k) {
            // `btst #k,Dn` takes k mod 32: an element beyond the set's bits
            // would test a bit outside the set (garbage above a word set,
            // another element of a longword one).  The value is false (ref
            // 3.9.3, p. 3-16: one bit per ordinal from 0), but what the
            // original emitted for it is not catalogued.
            return Err(unsupported(
                self.span,
                "`in` with a constant element outside the set's bits is",
            ));
        }
        let v = self.eval(set)?;
        let r = match v {
            Val::Reg(Reg::D(r)) if !value => r,
            Val::Tmp(r) => r,
            Val::Mem(ea) => {
                let d = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(w),
                    src: ea,
                    dst: Ea::Reg(Reg::D(d)),
                });
                d
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "`in` on a set held in a register, as a value, is",
                ))
            }
        };
        self.out.push(Insn::BtstImm {
            bit: *k as u8,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok(r)
    }

    /// `elem in set` for a one-longword set in memory, as a boolean in a
    /// data register.  PROC1_$TST_LOCK 0xE148CE..0xE148DE:
    ///
    /// ```text
    /// movea.l PROC1_$CURRENT_PCB.l,A0   ; the set's address first
    /// move.w  (8,A6),D0                 ; then the element
    /// move.l  (0x40,A0),D1              ; then the set's value
    /// btst    D0,D1                     ; register btst: bit mod 32
    /// sne     D1                        ; boolean in the set's register
    /// ```
    ///
    /// The order (the set operand's address before the element) is seen
    /// at this one site only (codegen-patterns 5.1).
    fn set_member_long(&mut self, elem: &Expr, set: &Expr) -> Res<u8> {
        let Expr::Load {
            addr,
            ty: Scalar {
                width: Width::Long, ..
            },
        } = set
        else {
            return Err(unsupported(
                self.span,
                "`in` on a set that is not a variable is",
            ));
        };
        let ea = self.ea_of(addr)?;
        let bit = match elem {
            // Not copied: `btst` only reads it.
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if matches!(self.pinned(*o), Some(Reg::D(_))) => match self.pinned(*o) {
                Some(Reg::D(n)) => n,
                _ => unreachable!(),
            },
            e => self.eval_d(e)?,
        };
        let r = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: Size::L,
            src: ea,
            dst: Ea::Reg(Reg::D(r)),
        });
        self.out.push(Insn::Btst {
            bit,
            dst: Ea::Reg(Reg::D(r)),
        });
        self.out.push(Insn::Scc {
            cond: Cond::Ne,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok(r)
    }

    /// The operand of an array element (codegen-patterns 5.2): the element
    /// is at `base + index × scale + offset`.
    ///
    /// - The base: the unit's data area (A5), the frame (A6), or the object
    ///   a pointer held in an address register since the entry points to.
    ///   Another unit's array is refused: the image splits its address
    ///   between a `movea.l #` immediate and the displacement in a way not
    ///   catalogued (ACL_$IN_SUBSYS 0xE470AA `movea.l #0xe97294,A0` and
    ///   `(-0x3d5a,A1)`; module-local 0xE464A0 `movea.l #0xe2622c,A0` and
    ///   `(-2,A0,D1.w)`, PROC1_$TYPE being 0xE2612C), and the base load is
    ///   scheduled into the index computation (12.4 R11).
    /// - The index in a data register ([`Self::index_reg`]), `.w` for a word
    ///   index, `.l` for a longword one.  **Uncertain:** words are also used
    ///   as `.l` indices after `ext.l` (0xE01D10..0xE01D16, an index held in
    ///   a register); every word index `dpc` forms is read from memory or a
    ///   parameter, as at the `.w` sites below.
    /// - The displacement (lower bound and field offsets folded in): within
    ///   -128..127, the indexed mode `(d,An,Dn.w)` (AUDIT_$IS_PROCESS_AUDITED
    ///   0xE70DA8 `tst.w (-2,A5,D0.w)`, a 1-based array; FIM_$GET_FIM_ADDR
    ///   0xE0AA18 `move.l (0x3c,A5,D0.w),D0`); beyond, `lea (0,An,Dn.w),Am`
    ///   and `(d,Am)` (BAT_$GET_BAT_STEP 0xE3BB6E..0xE3BB72 `lea
    ///   (0,A5,D1.w),A0; move.w (-0x220,A0),D0`; ACL_$IN_SUBSYS 0xE470B2).
    fn index_ea(&mut self, base: &Addr, index: &Expr, scale: u32, offset: i32) -> Res<Ea> {
        if let Some(p) = self.stepped_pointer(base, index, scale) {
            // Through the `for` loop's stepped pointer ([`Self::for_`]):
            // `(d,Ap)`, the lower bound and field offsets in `d` (0xE177E8
            // `move.w (-2,A0),(-2,A1)`; PKT_$RECENTLY_MISSING 0xE128DC
            // `cmp.l (-8,A0),D1`).  An access at displacement 0 is made
            // with `(An)+` where it is the pointer's last use (0xE00ED8
            // `clr.l (A0)+`; AST_ 0xE0287A `bclr #7,(A2)+; addq.l #3,A2`),
            // which `dpc` does not do.
            if offset == 0 {
                return Err(unsupported(
                    self.span,
                    "an element of a stepped array at displacement 0 (`(An)+`, \
                     codegen-patterns 7.4) is",
                ));
            }
            if !(-0x8000..0x8000).contains(&offset) {
                return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
            }
            return Ok(Ea::Disp(offset, p));
        }
        let (areg, disp0) = match base {
            // AUDIT_$SUSPEND 0xE70DBA..0xE70DC6 loads the base between
            // the index load and its scaling: not modelled.
            Addr::Static(_) if self.data_absolute => {
                return Err(unsupported(
                    self.span,
                    "an array element in the unit's data addressed absolutely (--a5-absolute; \
                     the `lea` is scheduled into the index computation, AUDIT_$SUSPEND \
                     0xE70DC0) is",
                ))
            }
            Addr::Static(o) => (DATA_BASE_A, *o),
            Addr::Frame(o) => (FP, *o),
            Addr::Indirect { base: p, offset: o }
                if matches!(
                    &**p,
                    Expr::Load { addr: Addr::Frame(f), .. }
                        if matches!(self.pinned(*f), Some(Reg::A(_)))
                ) =>
            {
                let a = self.eval_a(p)?;
                (a, *o)
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "an element of an array other than one in the unit's data, the frame, or \
                     through a pointer held since the entry (address split and scheduling not \
                     catalogued, codegen-patterns 5.2) is",
                ))
            }
        };
        let long = index.ty().width == Width::Long;
        let x = self.index_reg(index, scale)?;
        let disp = disp0 + offset;
        if (-128..128).contains(&disp) {
            return Ok(Ea::Idx(disp, areg, x, long));
        }
        if !(-0x8000..0x8000).contains(&disp) {
            return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
        }
        let a = self.alloc_a()?;
        self.out.push(Insn::Lea {
            src: Ea::Idx(0, areg, x, long),
            dst: a,
        });
        Ok(if disp == 0 {
            Ea::Ind(a)
        } else {
            Ea::Disp(disp, a)
        })
    }

    /// The pointer a `for` loop steps through the array at `base` for
    /// elements of `scale` bytes, when `index` is that loop's index.
    fn stepped_pointer(&self, base: &Addr, index: &Expr, scale: u32) -> Option<u8> {
        self.stepped
            .iter()
            .rev()
            .find(|(o, a, _)| is_index(index, *o) && a.scale == scale && a.base == *base)
            .map(|&(_, _, r)| r)
    }

    /// The scaled index of an array element in a data register.
    ///
    /// - Element size 1: the index itself, in any data register.
    /// - Otherwise a register the scaling may modify: an index read from
    ///   memory is scaled in place (FIM_$GET_FIM_ADDR 0xE0AA10..0xE0AA16
    ///   `move.w abs.l,D0; lsl.w #2,D0`; AUDIT_$IS_PROCESS_AUDITED
    ///   0xE70DA0..0xE70DA6); a by-value parameter is loaded, then copied,
    ///   then scaled, in a one-statement body too (BAT_$GET_BAT_STEP
    ///   0xE3BB64..0xE3BB6A `move.w (8,A6),D0; move.w D0,D1; mulu.w
    ///   #0x234,D1`; module-local 0xE4649C..0xE464A8 `move.w (8,A6),D0; ...;
    ///   move.w D0,D1; add.w D1,D1`), as one loaded at entry is (FIM_$FREE_ASID
    ///   0xE0AA8A `move.w D2,D0; lsl.w #2,D0`).  The scaling is
    ///   [`Self::mul_const`].
    fn index_reg(&mut self, index: &Expr, scale: u32) -> Res<u8> {
        let w = index.ty().width;
        if scale == 1 {
            let v = self.eval(index)?;
            return match v {
                Val::Tmp(d) | Val::Reg(Reg::D(d)) => Ok(d),
                v => self.in_tmp(v, w),
            };
        }
        // A scaled index is a reusable computation (codegen-patterns 12.4
        // R7: FIM_$ACKNOWLEDGE's `PROC1_$AS_ID * 12`, 0xE0A972..0xE0A97E,
        // indexes FIM_$QUIT_EC at 0xE0A994 and FIM_$DELIV_EC at 0xE0A9B0).
        if let Some(leaf) = self.cse_leaf(index) {
            let key = CseKey::Scaled(index.clone(), scale as i64);
            return match self.cse_value(key, leaf, |s| s.scaled_index(index, scale))? {
                Val::Tmp(d) | Val::Reg(Reg::D(d)) => Ok(d),
                _ => unreachable!("a scaled index is in a data register"),
            };
        }
        self.scaled_index(index, scale)
    }

    /// The scaling of [`Self::index_reg`].
    fn scaled_index(&mut self, index: &Expr, scale: u32) -> Res<u8> {
        let w = index.ty().width;
        let mut v = self.eval(index)?;
        if let (
            Val::Mem(_),
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            },
        ) = (&v, index)
        {
            if *o > 0 {
                let r = self.in_tmp(v, w)?;
                v = Val::Reg(Reg::D(r));
            }
        }
        let d = self.in_tmp(v, w)?;
        self.mul_const(d, scale as i64, w)?;
        Ok(d)
    }

    /// Evaluate a pointer into a new address register, or give the
    /// register a by-value parameter was loaded into at entry.
    fn eval_a(&mut self, e: &Expr) -> Res<u8> {
        if let Expr::Load {
            addr: Addr::Frame(o),
            ..
        } = e
        {
            if let Some(r) = self.pinned(*o) {
                return match r {
                    Reg::A(a) => Ok(a),
                    // `entry_loads` puts dereferenced pointers in A
                    // registers.
                    Reg::D(_) => unreachable!("pointer parameter pinned in a data register"),
                };
            }
            if *o < 0 && self.a0_holds(*o) {
                // A pointer local stored from a pointer function's A0 and
                // dereferenced while A0 still holds it: A0 is the base
                // (0xE4DC78 `tst.w (A0)` after `move.l A0,(-248,A6)`;
                // [`Self::a0_holds`]).
                self.a0_bases.push((self.out.len(), self.span));
                return Ok(0);
            }
        }
        match e {
            // `movea.l <ea>,An`: 0xE148CE `movea.l abs.l,A0` (an extern
            // pointer variable); LOG_$READ 0xE17810 `movea.l (0xc,A6),A0`
            // (a by-address parameter).
            // A slot used once in a body of several statements: a register
            // no earlier statement left a value in ([`fresh_slot_loads`];
            // IIC_$RECEIVE 0xE70AC0..0xE70AD0).  By-value pointers are
            // loaded at entry there ([`Self::entry_loads`]).
            // A slot a nested routine reaches is reloaded like any memory
            // operand, into the lowest free register (PMAP_$FLUSH: `movea.l
            // (8,A6),A0` at 0xE1378A and again at 0xE1379C, in the next
            // statement).
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if self.multi && *o > 0 && !self.is_uplevel(*o) => {
                let a = self.alloc_a()?;
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src: Ea::Disp(*o, FP),
                    dst: a,
                });
                self.fresh.push((Reg::A(a), self.stmt_start, self.span));
                Ok(a)
            }
            Expr::Load {
                addr,
                ty: Scalar {
                    width: Width::Long, ..
                },
            } => {
                let ea = self.ea_of(addr)?;
                // TODO(codegen): the allocator lets a pointer chain reuse a
                // dying base (`movea.l (d,A0),A0`); when the original did is
                // not known (codegen-patterns 12.4 R2: 35 such loads into
                // the same register, 144 into another).
                let a = self.alloc_a()?;
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src: ea,
                    dst: a,
                });
                Ok(a)
            }
            // A pointer function's result as the base, A0 itself
            // (codegen-patterns 3, "Callers of pointer functions": 0xE4E990
            // `tst.w (A0)`, 0xE4EA0E `clr.w (A0)` after a `bsr.w` to the
            // DIR module-local 0xE4B340; VFMT_$MAIN 0xE6AD10 `move.w
            // (2,A0),D7` after the module-local 0xE6AA38).
            e if a0_call(e).is_some() => self.call_a0(a0_call(e).unwrap()),
            _ => Err(unsupported(self.span, "this pointer expression is")),
        }
    }

    /// The effective address of `addr`; any address register it needs is
    /// loaded now.
    fn ea_of(&mut self, addr: &Addr) -> Res<Ea> {
        match addr {
            // A local held in a register (codegen-patterns 12.5).
            Addr::Frame(o) if *o < 0 && self.pinned(*o).is_some() => {
                Ok(Ea::Reg(self.pinned(*o).unwrap()))
            }
            // Frame objects and arguments: `(d,A6)` (0xE148D4).
            Addr::Frame(o) => Ok(Ea::Disp(*o, FP)),
            // A constant's cell in the code section, `(d,PC)`
            // (codegen-patterns 1.5).
            Addr::Const(c) => Ok(Ea::Pool(c.clone())),
            // The enclosing routine's frame objects, through the static
            // link held in a register since the entry (codegen-patterns 6:
            // 0xE135AA `btst #0,(0x15,A2)`, 0xE1362E `(-0x50,A4)`).
            Addr::Enclosing(o) => {
                let Some(l) = self.link else {
                    unreachable!(
                        "the static link is loaded when the body reaches the enclosing frame"
                    )
                };
                if !(-0x8000..0x8000).contains(o) {
                    return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
                }
                Ok(Ea::Disp(*o, l))
            }
            // Other units' objects: absolute long (0xE148CE).
            Addr::Symbol { name, offset } => Ok(Ea::AbsL(name.clone(), *offset)),
            // The unit's own data: `(d,A5)` (PROC2_$GET_BOOT_FLAGS 0xE41B66
            // `move.w (0x1e4,A5),(A0)`; XNS_IDP_$GET_STATS 0xE18FE8
            // `move.l (4,A5),(4,A0)`), `(A5)` for offset 0 (0xE18FE6
            // `move.l (A5),(A0)`).
            Addr::Static(o) => {
                if !(0..0x8000).contains(o) {
                    return Err(unsupported(
                        self.span,
                        "a static variable beyond 32K into the data area is",
                    ));
                }
                let base = self.static_base()?;
                Ok(if *o == 0 {
                    Ea::Ind(base)
                } else {
                    Ea::Disp(*o, base)
                })
            }
            Addr::Index {
                base,
                index,
                scale,
                offset,
            } => self.index_ea(base, index, *scale, *offset),
            // Through a pointer: `(d,An)` (0xE148D8), `(An)` when d = 0
            // (LOG_$READ 0xE17814 `move.w (A0),-(SP)`).
            Addr::Indirect { base, offset } => {
                if !(-0x8000..0x8000).contains(offset) {
                    return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
                }
                let a = self.eval_a(base)?;
                Ok(if *offset == 0 {
                    Ea::Ind(a)
                } else {
                    Ea::Disp(*offset, a)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpc_ir::{ResultSpec, Stmt};
    use dpc_syntax::SourceMap;

    fn span() -> Span {
        let f = SourceMap::new().add("t.pas", String::new());
        Span::new(f, 0, 0)
    }

    fn func(frame_size: u32, result: Option<ResultSpec>, body: Vec<StmtKind>) -> Function {
        Function {
            symbol: "F".into(),
            global: true,
            code_section: ".text".into(),
            frame_size,
            result,
            loads_data_base: false,
            data_absolute: false,
            value_params: Vec::new(),
            address_params: Vec::new(),
            scalar_locals: Vec::new(),
            nesting: None,
            uplevel_refs: Vec::new(),
            result_slot: None,
            body: body
                .into_iter()
                .map(|kind| Stmt { kind, span: span() })
                .collect(),
            span: span(),
        }
    }

    fn text(f: &Function) -> Vec<String> {
        select(f).unwrap().iter().map(|i| i.to_string()).collect()
    }

    fn load(addr: Addr, ty: Scalar) -> Expr {
        Expr::Load { addr, ty }
    }

    /// codegen-patterns 2: an empty procedure is a bare `rts` (TIME_$Q_INIT
    /// 0xE16C5C), even if it would load the data base; one with locals
    /// is refused.  An empty function keeps its frame.
    #[test]
    fn empty_procedure() {
        assert_eq!(text(&func(0, None, vec![])), ["rts"]);
        assert_eq!(text(&with_data(func(0, None, vec![]))), ["rts"]);
        let d = select(&func(4, None, vec![])).unwrap_err();
        assert!(
            d.message.starts_with("an empty procedure with locals"),
            "{}",
            d.message
        );
        let word = ResultSpec {
            class: ResultClass::Data,
            ty: Scalar::signed(Width::Word),
        };
        assert_eq!(
            text(&func(0, Some(word), vec![])),
            ["link.w\t%fp,#0", "unlk\t%fp", "rts"]
        );
    }

    #[test]
    fn link_rounds_frame_up_to_4() {
        let word = ResultSpec {
            class: ResultClass::Data,
            ty: Scalar::signed(Width::Word),
        };
        for (raw, disp) in [(1, "#-4"), (4, "#-4"), (5, "#-8"), (12, "#-0xc")] {
            assert_eq!(
                text(&func(raw, Some(word), vec![]))[0],
                format!("link.w\t%fp,{disp}")
            );
        }
    }

    #[test]
    fn tst_lock_shape() {
        let pcb = load(
            Addr::Symbol {
                name: "PROC1_$CURRENT_PCB".into(),
                offset: 0,
            },
            Scalar::POINTER,
        );
        let e = Expr::SetMember {
            elem: Box::new(load(Addr::Frame(8), Scalar::signed(Width::Word))),
            set: Box::new(load(
                Addr::Indirect {
                    base: Box::new(pcb),
                    offset: 0x40,
                },
                Scalar::unsigned(Width::Long),
            )),
            set_width: Width::Long,
        };
        let f = func(
            1,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::BOOLEAN,
            }),
            vec![StmtKind::SetResult(e)],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "movea.l\t(PROC1_$CURRENT_PCB).l,%a0",
                "move.w\t(8,%fp),%d0",
                "move.l\t(0x40,%a0),%d1",
                "btst\t%d0,%d1",
                "sne\t%d1",
                "move.b\t%d1,%d0",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    #[test]
    fn result_already_in_d0_is_not_moved() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::signed(Width::Long),
            }),
            vec![StmtKind::SetResult(load(
                Addr::Indirect {
                    base: Box::new(load(Addr::Frame(8), Scalar::POINTER)),
                    offset: 0,
                },
                Scalar::signed(Width::Long),
            ))],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "movea.l\t(8,%fp),%a0",
                "move.l\t(%a0),%d0",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// A constant result: `clr.w D0` (PARITY_$CHK_IO 0xE0B1AA), `moveq`
    /// (0xE0B19C `moveq #1,D0`, 0xE0B23A `moveq #-1,D0`, word results),
    /// `move.<s> #n` otherwise.
    #[test]
    fn constant_results() {
        for (ty, v, want) in [
            (Scalar::signed(Width::Word), 0, "clr.w\t%d0"),
            (Scalar::signed(Width::Long), 0, "clr.l\t%d0"),
            (Scalar::signed(Width::Word), 1, "moveq\t#1,%d0"),
            (Scalar::signed(Width::Word), -1, "moveq\t#-1,%d0"),
            (Scalar::signed(Width::Word), 300, "move.w\t#0x12c,%d0"),
            (Scalar::signed(Width::Long), 0x10000, "move.l\t#0x10000,%d0"),
        ] {
            let f = func(
                0,
                Some(ResultSpec {
                    class: ResultClass::Data,
                    ty,
                }),
                vec![StmtKind::SetResult(Expr::Const { value: v, ty })],
            );
            assert_eq!(text(&f)[1], want);
        }
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let word = ResultSpec {
            class: ResultClass::Data,
            ty: Scalar::signed(Width::Word),
        };
        let d = select(&func(0x8001, Some(word), vec![])).unwrap_err();
        assert!(d.message.ends_with("not supported yet"), "{}", d.message);
    }

    /// Phase 2 puts the save the body needs between `link` and the body,
    /// and the restore from the frame before `unlk`.
    #[test]
    fn two_phase_frame_around_a_body_with_held_registers() {
        let f = func(16, None, vec![]);
        // A synthetic body: three values held across a call.
        let mut v = crate::regs::VRegs::new();
        let d = v.new_reg(RegClass::Data).unwrap();
        let a = v.new_reg(RegClass::Addr).unwrap();
        let d2 = v.new_reg(RegClass::Data).unwrap();
        let Reg::A(an) = a else { unreachable!() };
        let mut insns = vec![
            Insn::Move {
                size: Size::L,
                src: Ea::Disp(0xc, FP),
                dst: Ea::Reg(d),
            },
            Insn::Movea {
                size: Size::L,
                src: Ea::Disp(8, FP),
                dst: an,
            },
            Insn::Move {
                size: Size::L,
                src: Ea::Disp(0x10, FP),
                dst: Ea::Reg(d2),
            },
            Insn::Jsr(Ea::AbsL("X".into(), 0)),
            Insn::Move {
                size: Size::L,
                src: Ea::Reg(d),
                dst: Ea::Ind(an),
            },
            Insn::Move {
                size: Size::L,
                src: Ea::Reg(d2),
                dst: Ea::Disp(4, an),
            },
        ];
        let saves = crate::regs::allocate(&mut insns, &[], &[]).unwrap();
        insns.truncate(1);
        let text: Vec<String> = wrap(&f, Body { insns, saves })
            .unwrap()
            .iter()
            .map(|i| i.to_string())
            .collect();
        // 0xE17778..0xE17780, 0xE177F6..0xE177FE.
        assert_eq!(
            text,
            [
                "link.w\t%fp,#-0x10",
                "movem.l\t%d2-%d3/%a2,-(%sp)",
                "move.l\t(0xc,%fp),%d2",
                "movem.l\t(-0x1c,%fp),%d2-%d3/%a2",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    #[test]
    fn scratch_only_bodies_save_nothing() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::signed(Width::Word),
            }),
            vec![StmtKind::SetResult(load(
                Addr::Frame(8),
                Scalar::signed(Width::Word),
            ))],
        );
        let body = select_body(&f).unwrap();
        assert!(body.saves.is_empty());
        assert_eq!(body.insns.len(), 1);
    }

    // ---- statement sequences and stores (codegen-patterns 12.2, 13) ----

    fn word() -> Scalar {
        Scalar::signed(Width::Word)
    }

    fn param(offset: i32, ty: Scalar) -> Expr {
        load(Addr::Frame(offset), ty)
    }

    /// `(*p).offset` for the pointer parameter at `p`.
    fn through(p: i32, offset: i32) -> Addr {
        Addr::Indirect {
            base: Box::new(param(p, Scalar::POINTER)),
            offset,
        }
    }

    fn store(addr: Addr, value: Expr) -> StmtKind {
        StmtKind::Store { addr, value }
    }

    fn konst(value: i64, ty: Scalar) -> Expr {
        Expr::Const { value, ty }
    }

    fn vparams(f: &mut Function, ps: &[(i32, Scalar)]) {
        f.value_params = ps
            .iter()
            .map(|&(offset, ty)| dpc_ir::ValueParam { offset, ty })
            .collect();
    }

    fn err(f: &Function) -> String {
        select(f).unwrap_err().message
    }

    /// TIME_$Q_INIT_QUEUE 0xE16C5E..0xE16C7E.
    #[test]
    fn q_init_queue_shape() {
        let long = Scalar::signed(Width::Long);
        let mut f = func(
            0,
            None,
            vec![
                store(through(12, 0), konst(0, long)),
                store(through(12, 4), konst(0, long)),
                store(through(12, 8), param(8, Scalar::BOOLEAN)),
                store(through(12, 10), param(10, word())),
            ],
        );
        vparams(
            &mut f,
            &[(8, Scalar::BOOLEAN), (10, word()), (12, Scalar::POINTER)],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "move.b\t(8,%fp),%d0",
                "move.w\t(0xa,%fp),%d1",
                "movea.l\t(0xc,%fp),%a0",
                "clr.l\t(%a0)",
                "clr.l\t(4,%a0)",
                "move.b\t%d0,(8,%a0)",
                "move.w\t%d1,(0xa,%a0)",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// Entry loads follow parameter order, not the order of first use,
    /// and a parameter the body does not read is not loaded.
    #[test]
    fn entry_loads_in_parameter_order() {
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), param(14, word())),
                store(through(8, 4), param(12, word())),
            ],
        );
        vparams(
            &mut f,
            &[
                (8, Scalar::POINTER),
                (12, word()),
                (14, word()),
                (16, word()),
            ],
        );
        assert_eq!(
            &text(&f)[1..4],
            [
                "movea.l\t(8,%fp),%a0",
                "move.w\t(0xc,%fp),%d0",
                "move.w\t(0xe,%fp),%d1",
            ]
        );
    }

    /// A one-statement body is not preloaded: the pointer is read where
    /// the store needs it (TIME_$Q_FLUSH_QUEUE 0xE16C84..0xE16C88).
    #[test]
    fn one_statement_store_through_pointer() {
        let mut f = func(
            0,
            None,
            vec![store(through(8, 0), konst(0, Scalar::POINTER))],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "movea.l\t(8,%fp),%a0",
                "clr.l\t(%a0)",
                "unlk\t%fp",
                "rts"
            ]
        );
    }

    #[test]
    fn constant_store_forms() {
        let cases: Vec<(Expr, &str)> = vec![
            (konst(0, Scalar::BOOLEAN), "clr.b\t(%a0)"),
            (konst(-1, Scalar::BOOLEAN), "st\t(%a0)"),
            // A char 0xFF is not a boolean: move.b with 0..255.
            (
                konst(255, Scalar::unsigned(Width::Byte)),
                "move.b\t#0xff,(%a0)",
            ),
            (
                konst(0x10, Scalar::unsigned(Width::Byte)),
                "move.b\t#0x10,(%a0)",
            ),
            (konst(-1, word()), "move.w\t#-1,(%a0)"),
            (
                konst(40000, Scalar::unsigned(Width::Word)),
                "move.w\t#-0x63c0,(%a0)",
            ),
            (
                konst(0x6000e, Scalar::signed(Width::Long)),
                "move.l\t#0x6000e,(%a0)",
            ),
            (konst(0, word()), "clr.w\t(%a0)"),
        ];
        for (value, want) in cases {
            let f = func(0, None, vec![store(through(8, 0), value)]);
            assert_eq!(text(&f)[2], want);
        }
    }

    /// codegen-patterns 3, "Result variable", and 12.4 R5: PARITY_$CHK_IO
    /// 0xE0B180..0xE0B1AA.  The result variable is one register, D0 here
    /// (no call), written on three paths; of the two longwords loaded at
    /// entry, the one used later shares D0 with it (`(8)`->D1,
    /// `(0xc)`->D0).
    #[test]
    fn result_variable_on_several_paths() {
        let long = Scalar::signed(Width::Long);
        let res = Some(ResultSpec {
            class: ResultClass::Data,
            ty: word(),
        });
        let eq = |o: i32| IrCond::Compare {
            op: Rel::Eq,
            lhs: param(o, long),
            rhs: load(Addr::Static(4), long),
        };
        let set = |v| {
            vec![Stmt {
                kind: StmtKind::SetResult(konst(v, word())),
                span: span(),
            }]
        };
        let mut f = func(
            4,
            res,
            vec![
                StmtKind::If {
                    cond: eq(8),
                    then_: set(1),
                    else_: vec![Stmt {
                        kind: StmtKind::If {
                            cond: eq(12),
                            then_: set(2),
                            else_: set(0),
                        },
                        span: span(),
                    }],
                },
                store(Addr::Static(0), konst(0, word())),
            ],
        );
        vparams(&mut f, &[(8, long), (12, long)]);
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "move.l\t(8,%fp),%d1",
                "move.l\t(0xc,%fp),%d0",
                "cmp.l\t(4,%a5),%d1",
                "bne.s\t1f",
                "moveq\t#1,%d0",
                "bra.s\t2f",
                "1:",
                "cmp.l\t(4,%a5),%d0",
                "bne.s\t3f",
                "moveq\t#2,%d0",
                "bra.s\t4f",
                "3:",
                "clr.w\t%d0",
                "4:",
                "2:",
                "clr.w\t(%a5)",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// codegen-patterns 3: assigned before a call, the result variable is
    /// held in a callee-saved register and moved to D0 at the end
    /// (PKT_$NEXT_ID 0xE124AC, 0xE124D0); `true` as `st` (0xE41216).
    #[test]
    fn result_variable_across_a_call() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::BOOLEAN,
            }),
            vec![
                StmtKind::SetResult(konst(-1, Scalar::BOOLEAN)),
                StmtKind::Call(Call {
                    callee: Callee::External("P".into()),
                    args: Vec::new(),
                    pad: 0,
                    link: None,
                }),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "move.l\t%d2,-(%sp)",
                "st\t%d2",
                "jsr\t(P).l",
                "move.b\t%d2,%d0",
                "move.l\t(-4,%fp),%d2",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// codegen-patterns 3, "Pointer results": the module-local 0xE6AA38
    /// `movea.l (8,A6),A0; move.l (A0),(-8,A6); movea.l (-8,A6),A0`.
    #[test]
    fn pointer_result_through_its_slot() {
        let res = Some(ResultSpec {
            class: ResultClass::Address,
            ty: Scalar::POINTER,
        });
        let mut f = func(
            8,
            res,
            vec![StmtKind::SetResult(load(through(8, 0), Scalar::POINTER))],
        );
        f.address_params = vec![8];
        f.result_slot = Some(-8);
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-8",
                "movea.l\t(8,%fp),%a0",
                "move.l\t(%a0),(-8,%fp)",
                "movea.l\t(-8,%fp),%a0",
                "unlk\t%fp",
                "rts",
            ]
        );
        // Not from memory, or in a body of several statements: refused.
        let mut g = func(8, res, vec![StmtKind::SetResult(konst(0, Scalar::POINTER))]);
        g.result_slot = Some(-8);
        assert!(err(&g).starts_with("a pointer result other than one read from memory"));
        let mut h = func(
            8,
            res,
            vec![
                store(Addr::Static(0), konst(0, word())),
                StmtKind::SetResult(load(Addr::Static(4), Scalar::POINTER)),
            ],
        );
        h.result_slot = Some(-8);
        // In a body of several statements the result variable is a
        // register, A0 itself when no call follows (`set_pointer_result_web`).
        assert_eq!(
            text(&h),
            [
                "link.w\t%fp,#-8",
                "clr.w\t(%a5)",
                "movea.l\t(4,%a5),%a0",
                "unlk\t%fp",
                "rts",
            ]
        );
        let mut k = func(
            8,
            res,
            vec![
                store(Addr::Static(0), konst(0, word())),
                StmtKind::SetResult(konst(0, Scalar::POINTER)),
            ],
        );
        k.result_slot = Some(-8);
        assert!(err(&k).starts_with("a pointer result in a body of several statements"));
    }

    /// IO_$ALLOC 0xE1A9C2..0xE1A9D0: a status store, then a constant
    /// result; IIC_$RECEIVE_CHECK 0xE70ADE likewise with a boolean.
    #[test]
    fn status_store_then_constant_result() {
        let long = Scalar::signed(Width::Long);
        let mut f = func(
            4,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: long,
            }),
            vec![
                store(through(0x14, 0), konst(0, long)),
                StmtKind::SetResult(konst(0, long)),
            ],
        );
        f.address_params = vec![8, 0xc, 0x10, 0x14];
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "movea.l\t(0x14,%fp),%a0",
                "clr.l\t(%a0)",
                "clr.l\t%d0",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// Ref 3.15.3 p. 3-57: the original merged some adjacent stores.
    #[test]
    fn adjacent_stores_are_rejected() {
        let f = func(
            0,
            None,
            vec![
                store(through(8, 0), konst(0, word())),
                store(through(8, 2), konst(0, word())),
            ],
        );
        assert!(err(&f).starts_with("a store adjacent to the previous"));
        // Two bytes make a word: rejected too.
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 4), konst(0, Scalar::BOOLEAN)),
                store(through(8, 5), konst(0, Scalar::BOOLEAN)),
            ],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert!(err(&f).starts_with("a store adjacent to the previous"));
        // Not adjacent (a gap), too big together, or 3 bytes (a byte above
        // a word: module-local 0xE32B76, 0xE32BA6/0xE32BAA): accepted.
        for (o, ty) in [
            (4, word()),
            (2, Scalar::signed(Width::Long)),
            (2, Scalar::BOOLEAN),
        ] {
            let mut f = func(
                0,
                None,
                vec![
                    store(through(8, 0), konst(0, word())),
                    store(through(8, o), konst(0, ty)),
                ],
            );
            vparams(&mut f, &[(8, Scalar::POINTER)]);
            assert!(select(&f).is_ok(), "{o}: {:?}", select(&f));
        }
    }

    /// A third value of a class takes D2, callee-saved, without any call
    /// (codegen-patterns 12.4 R1: 0xE177A0 `moveq #-2,D2` while D0 and D1
    /// hold parameters).
    #[test]
    fn third_entry_load_of_a_class_takes_d2() {
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), param(12, word())),
                store(through(8, 4), param(14, word())),
                store(through(8, 8), param(16, word())),
            ],
        );
        vparams(
            &mut f,
            &[
                (8, Scalar::POINTER),
                (12, word()),
                (14, word()),
                (16, word()),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "move.l\t%d2,-(%sp)",
                "movea.l\t(8,%fp),%a0",
                "move.w\t(0xc,%fp),%d0",
                "move.w\t(0xe,%fp),%d1",
                "move.w\t(0x10,%fp),%d2",
                "move.w\t%d0,(%a0)",
                "move.w\t%d1,(4,%a0)",
                "move.w\t%d2,(8,%a0)",
                "move.l\t(-4,%fp),%d2",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    #[test]
    fn pointer_used_as_data_is_rejected() {
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), konst(0, word())),
                store(
                    Addr::Symbol {
                        name: "X".into(),
                        offset: 0,
                    },
                    param(8, Scalar::POINTER),
                ),
            ],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert_eq!(
            err(&f),
            "a parameter used both as a pointer and as a value is not supported yet"
        );
    }

    /// In a body of several statements, a statement takes registers of its
    /// own, the lowest free over their live ranges (codegen-patterns 12.4
    /// R1); a plain variable is read again by each statement (R8:
    /// PROC1_$AS_ID at FIM_$ACKNOWLEDGE 0xE0A972, 0xE0A980, 0xE0A99A).
    #[test]
    fn statement_registers_in_multi_statement_bodies() {
        let ext = |name: &str| {
            Box::new(load(
                Addr::Symbol {
                    name: name.into(),
                    offset: 0,
                },
                Scalar::POINTER,
            ))
        };
        let f = func(
            0,
            None,
            vec![
                store(
                    Addr::Indirect {
                        base: ext("P"),
                        offset: 0,
                    },
                    konst(0, word()),
                ),
                store(
                    Addr::Indirect {
                        base: ext("Q"),
                        offset: 4,
                    },
                    konst(0, word()),
                ),
            ],
        );
        assert_eq!(
            text(&f)[1..5],
            [
                "movea.l\t(P).l,%a0",
                "clr.w\t(%a0)",
                "movea.l\t(Q).l,%a0",
                "clr.w\t(4,%a0)",
            ]
        );
    }

    #[test]
    fn unsettled_one_statement_stores_are_rejected() {
        // The value is a parameter that was not loaded at entry.
        let mut f = func(0, None, vec![store(through(8, 0), param(12, word()))]);
        vparams(&mut f, &[(8, Scalar::POINTER), (12, word())]);
        assert_eq!(
            err(&f),
            "storing a parameter that was not loaded at entry is not supported yet"
        );
        // Both sides need an address register.
        let f = func(
            0,
            None,
            vec![store(through(8, 0), load(through(12, 0), word()))],
        );
        assert_eq!(
            err(&f),
            "a store whose destination and value both need a register is not supported yet"
        );
        // Assigning to a parameter.
        let f = func(0, None, vec![store(Addr::Frame(8), konst(1, word()))]);
        assert_eq!(err(&f), "assigning to a parameter is not supported yet");
    }

    /// Memory to memory: a local into the object a pointer parameter
    /// points to (TIME_$Q_REMOVE_ELEM 0xE16E80..0xE16E88).
    #[test]
    fn memory_to_memory_store() {
        let f = func(
            4,
            None,
            vec![store(
                through(0x10, 0),
                load(Addr::Frame(-4), Scalar::signed(Width::Long)),
            )],
        );
        assert_eq!(
            &text(&f)[1..3],
            ["movea.l\t(0x10,%fp),%a0", "move.l\t(-4,%fp),(%a0)"]
        );
    }

    // ---- module data and block copies (codegen-patterns 4, 5.2) ----

    fn copy(dst: Addr, src: Addr, bytes: u32) -> StmtKind {
        StmtKind::Copy { dst, src, bytes }
    }

    fn with_data(mut f: Function) -> Function {
        f.loads_data_base = true;
        f
    }

    /// PROC1_$GET_LOADAV 0xE14BBA..0xE14BD8.
    #[test]
    fn get_loadav_shape() {
        let f = with_data(func(
            0,
            None,
            vec![copy(through(8, 0), Addr::Static(0), 12)],
        ));
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "pea\t(%a5)",
                "lea\t(.Ldata).l,%a5",
                "lea\t(%a5),%a0",
                "movea.l\t(8,%fp),%a1",
                "move.l\t(%a0)+,(%a1)+",
                "move.l\t(%a0)+,(%a1)+",
                "move.l\t(%a0)+,(%a1)+",
                "movea.l\t(-4,%fp),%a5",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// The data base load joins a `movem` when other registers are
    /// saved (MAC_OS_$OPEN 0xE0B24A), and a routine that does not load it
    /// (an inheriting worker, 0xE17778) saves nothing for it.
    #[test]
    fn data_base_save_and_inheritance() {
        let st = vec![store(Addr::Static(0x1e4), konst(0, word()))];
        let f = func(0, None, st.clone());
        assert_eq!(
            text(&f),
            ["link.w\t%fp,#0", "clr.w\t(0x1e4,%a5)", "unlk\t%fp", "rts"]
        );
        let body = select_body(&with_data(func(0, None, st))).unwrap();
        assert_eq!(body.saves, RegSet::of(&[DATA_BASE]));
    }

    /// Offset 0 of the data area is `(A5)` (XNS_IDP_$GET_STATS 0xE18FE6
    /// `move.l (A5),(A0)`), others `(d,A5)`; memory to memory from the
    /// data area (PROC2_$GET_BOOT_FLAGS 0xE41B62..0xE41B66).
    #[test]
    fn data_area_operands() {
        let f = with_data(func(
            0,
            None,
            vec![store(through(8, 0), load(Addr::Static(0x1e4), word()))],
        ));
        assert_eq!(
            &text(&f)[2..5],
            [
                "lea\t(.Ldata).l,%a5",
                "movea.l\t(8,%fp),%a0",
                "move.w\t(0x1e4,%a5),(%a0)"
            ]
        );
        let f = with_data(func(
            0,
            None,
            vec![store(
                through(8, 0),
                load(Addr::Static(0), Scalar::signed(Width::Long)),
            )],
        ));
        assert_eq!(text(&f)[4], "move.l\t(%a5),(%a0)");
        let f = with_data(func(
            0,
            None,
            vec![store(Addr::Static(0x8000), konst(0, word()))],
        ));
        assert!(err(&f).starts_with("a static variable beyond 32K"));
    }

    /// 10 to 18 bytes unrolled, a final word with `move.w`; frame blocks
    /// with `lea (d,A6)` (0xE307F8..0xE30802: `lea (A2),A0; lea
    /// (-0x18,A6),A1`, two `move.l` and a `move.w`).
    #[test]
    fn unrolled_block_copies() {
        for (bytes, longs, word) in [
            (10, 2, true),
            (12, 3, false),
            (14, 3, true),
            (16, 4, false),
            (18, 4, true),
        ] {
            let f = with_data(func(
                0x20,
                None,
                vec![copy(Addr::Frame(-0x18), Addr::Static(0x14), bytes)],
            ));
            let t = text(&f);
            assert_eq!(
                &t[3..5],
                ["lea\t(0x14,%a5),%a0", "lea\t(-0x18,%fp),%a1"],
                "{bytes}"
            );
            let l = t.iter().filter(|i| *i == "move.l\t(%a0)+,(%a1)+").count();
            let w = t.iter().any(|i| i == "move.w\t(%a0)+,(%a1)+");
            assert_eq!((l, w), (longs, word), "{bytes}");
        }
    }

    /// 20 bytes and more: a `dbf` loop (AST_ 0xE01494..0xE014A2), the
    /// count after both addresses, the loop head aligned, a final word
    /// after the loop (0xE19DA2..0xE19DAA), `move.w #n` above 127
    /// (0xE3B97A `move.w #130,D0`).
    #[test]
    fn block_copy_loops() {
        let f = func(
            0x98,
            None,
            vec![copy(Addr::Frame(-0x98), through(8, 0), 144)],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-0x98",
                "movea.l\t(8,%fp),%a0",
                "lea\t(-0x98,%fp),%a1",
                "moveq\t#0x23,%d0",
                ".balignw\t4,0x2048",
                "1:",
                "move.l\t(%a0)+,(%a1)+",
                "dbf\t%d0,1b",
                "unlk\t%fp",
                "rts",
            ]
        );
        let f = func(
            0x18,
            None,
            vec![copy(Addr::Frame(-0x18), through(8, 0), 22)],
        );
        assert_eq!(
            &text(&f)[3..9],
            [
                "moveq\t#4,%d0",
                ".balignw\t4,0x2048",
                "1:",
                "move.l\t(%a0)+,(%a1)+",
                "dbf\t%d0,1b",
                "move.w\t(%a0)+,(%a1)+",
            ]
        );
        let f = func(
            0x20c,
            None,
            vec![copy(Addr::Frame(-0x20c), through(8, 0), 0x20c)],
        );
        assert_eq!(text(&f)[3], "move.w\t#0x82,%d0");
    }

    #[test]
    fn unsettled_block_copies_are_rejected() {
        for bytes in [6, 8, 11] {
            let f = func(
                0x10,
                None,
                vec![copy(Addr::Frame(-0x10), through(8, 0), bytes)],
            );
            assert_eq!(
                err(&f),
                format!("a {bytes}-byte record or array assignment is not supported yet")
            );
        }
        // Another unit's object; a field through a pointer.
        let sym = Addr::Symbol {
            name: "X".into(),
            offset: 0,
        };
        let f = func(0x10, None, vec![copy(Addr::Frame(-0x10), sym, 12)]);
        assert!(err(&f).starts_with("copying a block at this kind of address"));
        let f = func(
            0x10,
            None,
            vec![copy(Addr::Frame(-0x10), through(8, 4), 12)],
        );
        assert!(err(&f).starts_with("copying a block at this kind of address"));
    }

    // ---------------------------------------------------------------
    // Calls and fill loops (codegen-patterns 1, 7)
    // ---------------------------------------------------------------

    fn long() -> Scalar {
        Scalar::signed(Width::Long)
    }

    fn val(value: Expr) -> Arg {
        let slot = value.ty().bytes().max(2);
        Arg::Value { value, slot }
    }

    fn ext(name: &str, args: Vec<Arg>, pad: u32) -> StmtKind {
        StmtKind::Call(Call {
            callee: Callee::External(name.into()),
            args,
            pad,
            link: None,
        })
    }

    fn sym(name: &str) -> Addr {
        Addr::Symbol {
            name: name.into(),
            offset: 0,
        }
    }

    fn proc_(frame: u32, body: Vec<StmtKind>) -> Function {
        func(frame, None, body)
    }

    /// Module-local 0xE00EB0 and AST_$PAGE_ZERO 0xE00EEC
    /// (`tools/imgdis.sh E00EB0 66`).
    #[test]
    fn zero_page_shape() {
        let ppn = || val(param(8, long()));
        let f = proc_(
            12,
            vec![
                ext(
                    "MMU_$INSTALL",
                    vec![
                        ppn(),
                        val(Expr::AddrOf(sym("AST_$ZERO_BUFF"))),
                        val(konst(0, word())),
                        val(konst(0x16, word())),
                    ],
                    0,
                ),
                StmtKind::Fill {
                    start: sym("AST_$ZERO_BUFF"),
                    count: 256,
                    value: konst(0, long()),
                },
                ext("MMU_$REMOVE", vec![ppn()], 0),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-0xc",
                "pea\t(0x16).w",
                "move.l\t#AST_$ZERO_BUFF,-(%sp)",
                "move.l\t(8,%fp),-(%sp)",
                "jsr\t(MMU_$INSTALL).l",
                "lea\t(0xc,%sp),%sp",
                "movea.l\t#AST_$ZERO_BUFF,%a0",
                "move.w\t#0xff,%d0",
                ".balignw\t4,0x2048",
                "1:",
                "clr.l\t(%a0)+",
                "dbf\t%d0,1b",
                "move.l\t(8,%fp),-(%sp)",
                "jsr\t(MMU_$REMOVE).l",
                "unlk\t%fp",
                "rts",
            ]
        );
        let lock = |name: &str| ext(name, vec![val(konst(0x14, word()))], 2);
        let f = proc_(
            0,
            vec![
                lock("ML_$LOCK"),
                StmtKind::Call(Call {
                    callee: Callee::Local("ZERO_PAGE".into()),
                    args: vec![ppn()],
                    pad: 0,
                    link: None,
                }),
                lock("ML_$UNLOCK"),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "subq.l\t#2,%sp",
                "move.w\t#0x14,-(%sp)",
                "jsr\t(ML_$LOCK).l",
                "addq.w\t#4,%sp",
                "move.l\t(8,%fp),-(%sp)",
                "bsr.s\tZERO_PAGE",
                "addq.w\t#4,%sp",
                "subq.l\t#2,%sp",
                "move.w\t#0x14,-(%sp)",
                "jsr\t(ML_$UNLOCK).l",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// The body of `f` without `link` and `unlk; rts`.
    fn body_text(f: &Function) -> Vec<String> {
        let t = text(f);
        t[1..t.len() - 2].to_vec()
    }

    #[test]
    fn constant_pushes() {
        let byte_true = konst(-1, Scalar::BOOLEAN);
        let cases: Vec<(Expr, &str)> = vec![
            (konst(0, long()), "clr.l\t-(%sp)"),
            (konst(1, long()), "pea\t(1).w"),
            (konst(-1, long()), "pea\t(-1).w"),
            (konst(0x7fff, long()), "pea\t(0x7fff).w"),
            (konst(-0x8000, long()), "pea\t(-0x8000).w"),
            (konst(0x8000, long()), "move.l\t#0x8000,-(%sp)"),
            (konst(0x3a0001, long()), "move.l\t#0x3a0001,-(%sp)"),
            (konst(0, word()), "clr.w\t-(%sp)"),
            (konst(-2, word()), "move.w\t#-2,-(%sp)"),
            (byte_true, "st\t-(%sp)"),
            // false: 0xE688BC
            (konst(0, Scalar::BOOLEAN), "clr.w\t-(%sp)"),
        ];
        for (e, want) in cases {
            // One argument and something after the call, so it is popped.
            let f = proc_(0, vec![ext("P", vec![val(e)], 0), ext("Q", vec![], 0)]);
            assert_eq!(body_text(&f)[0], want);
        }
        // Other bytes: not catalogued.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(0x41, Scalar::unsigned(Width::Byte)))],
                2,
            )],
        );
        assert!(err(&f).starts_with("a byte constant argument other than `true` and 0"));
    }

    #[test]
    fn adjacent_word_constants_merge() {
        // (a: word; b: word; c: long): c pushed first, then a:b as one
        // longword with `a' in the high half.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![
                    val(konst(1, word())),
                    val(konst(2, word())),
                    val(param(8, long())),
                ],
                0,
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "move.l\t(8,%fp),-(%sp)",
                "move.l\t#0x10002,-(%sp)",
                "jsr\t(P).l"
            ]
        );
        // Both zero: one clr.l; a negative low half does not sign into the
        // high half.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(0, word())), val(konst(0, word()))],
                0,
            )],
        );
        assert_eq!(body_text(&f)[0], "clr.l\t-(%sp)");
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(0, word())), val(konst(-1, word()))],
                0,
            )],
        );
        assert_eq!(body_text(&f)[0], "move.l\t#0xffff,-(%sp)");
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(-1, word())), val(konst(-1, word()))],
                0,
            )],
        );
        assert_eq!(body_text(&f)[0], "pea\t(-1).w");
        // A word constant next to a word that is not constant: separate.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(3, word())), val(param(8, word()))],
                0,
            )],
        );
        assert_eq!(
            body_text(&f)[..2],
            ["move.w\t(8,%fp),-(%sp)", "move.w\t#3,-(%sp)"]
        );
        // Three in a row: which pair merges is not known.
        let w = || val(konst(1, word()));
        let f = proc_(0, vec![ext("P", vec![w(), w(), w()], 2)]);
        assert!(err(&f).starts_with("three or more adjacent word constant"));
    }

    #[test]
    fn pops_and_pads() {
        let l = || val(param(8, long()));
        let w = || val(param(8, word()));
        let after = || ext("Q", vec![], 0);
        // 4 and 8 bytes: addq.w; 12: lea; a pad counts.
        for (args, pad, pop) in [
            (vec![l()], 0, "addq.w\t#4,%sp"),
            (vec![w()], 2, "addq.w\t#4,%sp"),
            (vec![l(), l()], 0, "addq.w\t#8,%sp"),
            (vec![l(), w()], 2, "addq.w\t#8,%sp"),
            (vec![w(), l(), l()], 2, "lea\t(0xc,%sp),%sp"),
            (vec![l(), l(), l(), l()], 0, "lea\t(0x10,%sp),%sp"),
        ] {
            let n = args.len();
            let f = proc_(0, vec![ext("P", args, pad), after()]);
            let t = body_text(&f);
            let first = if pad == 2 { 1 } else { 0 };
            if pad == 2 {
                assert_eq!(t[0], "subq.l\t#2,%sp");
            }
            assert_eq!(t[first + n], "jsr\t(P).l");
            assert_eq!(t[first + n + 1], pop);
        }
        // No arguments: no pop.
        let f = proc_(0, vec![ext("P", vec![], 0), after()]);
        assert_eq!(body_text(&f), ["jsr\t(P).l", "jsr\t(Q).l"]);
        // The last call is not popped.
        let f = proc_(0, vec![ext("P", vec![l()], 0)]);
        assert_eq!(body_text(&f), ["move.l\t(8,%fp),-(%sp)", "jsr\t(P).l"]);
        // Nor one followed only by statements that push nothing
        // (PROC2_$UPGID_TO_UID 0xE41050: `jsr ML_$UNLOCK`, record copies,
        // no pop).
        let f = proc_(
            0,
            vec![ext("P", vec![l()], 0), store(sym("G"), konst(3, word()))],
        );
        assert_eq!(
            body_text(&f),
            ["move.l\t(8,%fp),-(%sp)", "jsr\t(P).l", "move.w\t#3,(G).l"]
        );
    }

    #[test]
    fn value_and_address_pushes() {
        let after = || ext("Q", vec![], 0);
        let addr = |a: Addr| Arg::Address {
            addr: a,
            slot: 4,
            writable: true,
        };
        // After a call, so that the statement may take A0.
        let f = proc_(
            4,
            vec![
                after(),
                ext(
                    "P",
                    vec![
                        addr(Addr::Frame(-4)),
                        addr(sym("UID_$NIL")),
                        val(load(through(8, 2), word())),
                        val(load(Addr::Frame(0xc), Scalar::BOOLEAN)),
                    ],
                    0,
                ),
                after(),
            ],
        );
        assert_eq!(
            body_text(&f)[1..],
            [
                "move.b\t(0xc,%fp),-(%sp)",
                "movea.l\t(8,%fp),%a0",
                "move.w\t(2,%a0),-(%sp)",
                "move.l\t#UID_$NIL,-(%sp)",
                "pea\t(-4,%fp)",
                "jsr\t(P).l",
                "lea\t(0xc,%sp),%sp",
                "jsr\t(Q).l",
            ]
        );
        // Not catalogued: an address inside another unit's object, the
        // address of a frame object as a value.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![addr(Addr::Symbol {
                    name: "X".into(),
                    offset: 4,
                })],
                0,
            )],
        );
        assert!(err(&f).starts_with("passing this kind of object by address"));
        let f = proc_(
            4,
            vec![ext("P", vec![val(Expr::AddrOf(Addr::Frame(-4)))], 0)],
        );
        assert!(err(&f).starts_with("this argument value"));
    }

    #[test]
    fn fill_forms() {
        let fill = |count, value| StmtKind::Fill {
            start: sym("BUF"),
            count,
            value,
        };
        let f = proc_(0, vec![fill(32, konst(0, word()))]);
        assert_eq!(
            body_text(&f),
            [
                "movea.l\t#BUF,%a0",
                "moveq\t#0x1f,%d0",
                ".balignw\t4,0x2048",
                "1:",
                "clr.w\t(%a0)+",
                "dbf\t%d0,1b",
            ]
        );
        let f = proc_(0, vec![fill(0x10000, konst(0, Scalar::BOOLEAN))]);
        assert_eq!(body_text(&f)[1], "move.w\t#-1,%d0");
        assert_eq!(body_text(&f)[4], "clr.b\t(%a0)+");
        let f = proc_(0, vec![fill(4, konst(1, long()))]);
        assert!(err(&f).starts_with("a `for` loop storing a constant other than 0"));
        let f = proc_(
            0,
            vec![StmtKind::Fill {
                start: Addr::Static(4),
                count: 4,
                value: konst(0, long()),
            }],
        );
        assert!(err(&f).starts_with("a `for` loop over an array other than"));
    }

    /// Registers around calls (codegen-patterns 12.4 R3): a value that dies
    /// before a call takes a scratch register, one live across a call a
    /// callee-saved one.
    #[test]
    fn registers_after_calls() {
        let fill = || StmtKind::Fill {
            start: sym("BUF"),
            count: 4,
            value: konst(0, long()),
        };
        let f = proc_(0, vec![fill(), ext("P", vec![], 0)]);
        assert_eq!(body_text(&f)[..2], ["movea.l\t#BUF,%a0", "moveq\t#3,%d0"]);
        let f = proc_(0, vec![ext("P", vec![], 0), fill()]);
        assert!(select(&f).is_ok());
        let f = proc_(0, vec![ext("P", vec![], 0), fill(), fill()]);
        assert!(select(&f).is_ok());
        // Dead before the call statement: A0 (12.4 R1; R4(c),
        // [`held_through_calls`], holds only what the arguments read).
        let mut f = proc_(
            0,
            vec![store(through(8, 0), konst(0, long())), ext("P", vec![], 0)],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert_eq!(
            body_text(&f),
            ["movea.l\t(8,%fp),%a0", "clr.l\t(%a0)", "jsr\t(P).l"]
        );
        // Live across the call: A2, saved with `pea (A2)` (module-local 0xE32B76,
        // 0xE32B7E, A2 around `jsr EC_$INIT`).
        let mut f = proc_(
            0,
            vec![
                store(through(8, 0), konst(0, long())),
                ext("P", vec![], 0),
                store(through(8, 4), konst(0, long())),
            ],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#0",
                "pea\t(%a2)",
                "movea.l\t(8,%fp),%a2",
                "clr.l\t(%a2)",
                "jsr\t(P).l",
                "clr.l\t(4,%a2)",
                "movea.l\t(-4,%fp),%a2",
                "unlk\t%fp",
                "rts",
            ]
        );
        // A parameter that is only pushed is not loaded at entry.
        let mut f = proc_(
            0,
            vec![
                ext("P", vec![val(param(8, long()))], 0),
                ext("Q", vec![val(param(8, long()))], 0),
            ],
        );
        vparams(&mut f, &[(8, long())]);
        assert_eq!(
            body_text(&f),
            [
                "move.l\t(8,%fp),-(%sp)",
                "jsr\t(P).l",
                "addq.w\t#4,%sp",
                "move.l\t(8,%fp),-(%sp)",
                "jsr\t(Q).l",
            ]
        );
    }

    fn locals(f: &mut Function, ls: &[(i32, Scalar)]) {
        f.scalar_locals = ls
            .iter()
            .map(|&(offset, ty)| dpc_ir::ValueParam { offset, ty })
            .collect();
    }

    /// codegen-patterns 10: `disable` / `enable` (TIME_$GET_ADJUST
    /// 0xE16AB0, 0xE16ABA), `set_sr` (PROC1_$SET_VT 0xE149B6..0xE149B8),
    /// its result stored through a register (ref p. E-5) or discarded.
    #[test]
    fn interrupt_builtins() {
        let w = word();
        let sr = |x: Expr| Expr::SetSr(Box::new(x));
        let f = proc_(
            0,
            vec![
                StmtKind::Disable,
                store(sym("W"), sr(konst(0x2700, w))),
                StmtKind::Enable,
                StmtKind::Discard(sr(load(sym("W"), w))),
            ],
        );
        assert_eq!(
            body_text(&f),
            [
                "ori.w\t#0x700,%sr",
                "move.w\t%sr,%d0",
                "move.w\t#0x2700,%sr",
                "move.w\t%d0,(W).l",
                "andi.w\t#0xf8ff,%sr",
                "move.w\t%sr,%d0",
                "move.w\t(W).l,%sr",
            ]
        );
        // A computed argument, or one that needs an address register.
        let f = proc_(
            0,
            vec![StmtKind::Discard(sr(Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(load(sym("W"), w)),
                rhs: Box::new(konst(1, w)),
                ty: w,
            }))],
        );
        assert!(
            err(&f).starts_with("a computed `set_sr` argument"),
            "{}",
            err(&f)
        );
        let mut f = proc_(0, vec![StmtKind::Discard(sr(param(8, w)))]);
        vparams(&mut f, &[(8, w)]);
        assert!(err(&f).starts_with("a `set_sr` argument that is a parameter"));
    }

    /// codegen-patterns 12.5: TIME_$GET_ADJUST 0xE16AA8..0xE16AE6, the
    /// local `current_delta` (-4) in D1 across both M$ARITH calls, the
    /// by-address slot (8) in A1, `link #-4` with the slot untouched.
    #[test]
    fn get_adjust_shape() {
        let l = long();
        let cur = || load(Addr::Frame(-4), l);
        let div = |op| Expr::Binary {
            op,
            lhs: Box::new(cur()),
            rhs: Box::new(konst(250000, l)),
            ty: l,
        };
        let mut f = proc_(
            4,
            vec![
                StmtKind::Disable,
                store(Addr::Frame(-4), load(sym("TIME_$CURRENT_DELTA"), l)),
                StmtKind::Enable,
                store(through(8, 0), div(BinOp::Div)),
                store(
                    through(8, 4),
                    Expr::Binary {
                        op: BinOp::Mul,
                        lhs: Box::new(div(BinOp::Mod)),
                        rhs: Box::new(konst(4, l)),
                        ty: l,
                    },
                ),
            ],
        );
        f.address_params = vec![8];
        locals(&mut f, &[(-4, l)]);
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "movea.l\t(8,%fp),%a1",
                "ori.w\t#0x700,%sr",
                "move.l\t(TIME_$CURRENT_DELTA).l,%d1",
                "andi.w\t#0xf8ff,%sr",
                "move.l\t#0x3d090,-(%sp)",
                "move.l\t%d1,-(%sp)",
                "jsr\t(M$DIS$LLL).l",
                "addq.w\t#8,%sp",
                "move.l\t%d0,(%a1)",
                "move.l\t#0x3d090,-(%sp)",
                "move.l\t%d1,-(%sp)",
                "jsr\t(M$OIS$LLL).l",
                "lsl.l\t#2,%d0",
                "move.l\t%d0,(4,%a1)",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// Which locals [`register_locals`] holds in a register: one
    /// assignment of a non-constant at the top level, reads after it, no
    /// other reference.
    #[test]
    fn register_local_rule() {
        let w = word();
        let x = || load(Addr::Frame(-2), w);
        let set = |v: Expr| store(Addr::Frame(-2), v);
        let use_ = || store(sym("W"), x());
        let cases: Vec<(&str, Vec<StmtKind>, bool)> = vec![
            (
                "one store, then reads",
                vec![set(load(sym("V"), w)), use_(), use_()],
                true,
            ),
            ("a constant", vec![set(konst(5, w)), use_()], false),
            (
                "two stores",
                vec![set(load(sym("V"), w)), set(load(sym("U"), w)), use_()],
                false,
            ),
            ("a read first", vec![use_(), set(load(sym("V"), w))], false),
            (
                "a store in an `if`",
                vec![
                    StmtKind::If {
                        cond: IrCond::Test(load(sym("B"), Scalar::BOOLEAN)),
                        then_: vec![Stmt {
                            kind: set(load(sym("V"), w)),
                            span: span(),
                        }],
                        else_: vec![],
                    },
                    use_(),
                ],
                false,
            ),
            (
                "its address passed",
                vec![
                    set(load(sym("V"), w)),
                    StmtKind::Call(Call {
                        callee: Callee::External("Q".into()),
                        args: vec![Arg::Address {
                            addr: Addr::Frame(-2),
                            slot: 4,
                            writable: true,
                        }],
                        pad: 0,
                        link: None,
                    }),
                ],
                false,
            ),
            (
                "passed by value",
                vec![
                    set(load(sym("V"), w)),
                    StmtKind::Call(Call {
                        callee: Callee::External("Q".into()),
                        args: vec![Arg::Value {
                            value: x(),
                            slot: 2,
                        }],
                        pad: 2,
                        link: None,
                    }),
                ],
                false,
            ),
            (
                "read inside a function call's argument",
                vec![
                    set(load(sym("V"), w)),
                    store(
                        sym("W"),
                        Expr::Call {
                            call: Call {
                                callee: Callee::External("F".into()),
                                args: vec![Arg::Value {
                                    value: Expr::Binary {
                                        op: BinOp::Add,
                                        lhs: Box::new(x()),
                                        rhs: Box::new(konst(1, w)),
                                        ty: w,
                                    },
                                    slot: 2,
                                }],
                                pad: 2,
                                link: None,
                            },
                            ty: w,
                            reg: ResultClass::Data,
                        },
                    ),
                ],
                false,
            ),
            (
                "a byte of it read",
                vec![
                    set(load(sym("V"), w)),
                    store(
                        sym("C"),
                        load(Addr::Frame(-1), Scalar::unsigned(Width::Byte)),
                    ),
                ],
                false,
            ),
        ];
        for (what, body, held) in cases {
            let mut f = proc_(2, body);
            locals(&mut f, &[(-2, w)]);
            assert_eq!(register_locals(&f) == [-2], held, "{what}");
        }
    }

    fn for_loop(from: Expr, to: Expr, down: bool, body: Vec<StmtKind>) -> StmtKind {
        StmtKind::For(ForLoop {
            index: dpc_ir::ValueParam {
                offset: -2,
                ty: word(),
            },
            from,
            to,
            down,
            body: body
                .into_iter()
                .map(|kind| Stmt { kind, span: span() })
                .collect(),
        })
    }

    fn index() -> Expr {
        load(Addr::Frame(-2), word())
    }

    /// codegen-patterns 7.4: DISK_$UNASSIGN_ALL 0xE6BE24..0xE6BE3C, the
    /// count, then the index, both callee-saved across the call; the index
    /// stepped before `dbf`.
    #[test]
    fn for_with_index_and_call() {
        let body = vec![
            store(Addr::Frame(-6), index()),
            ext(
                "DISK_$UNASSIGN",
                vec![
                    Arg::Address {
                        addr: Addr::Frame(-4),
                        slot: 4,
                        writable: true,
                    },
                    Arg::Address {
                        addr: Addr::Frame(-6),
                        slot: 4,
                        writable: true,
                    },
                ],
                0,
            ),
        ];
        let f = proc_(
            8,
            vec![for_loop(konst(1, word()), konst(10, word()), false, body)],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-8",
                "movem.l\t%d2-%d3,-(%sp)",
                "moveq\t#9,%d2",
                "moveq\t#1,%d3",
                ".balignw\t4,0x2048",
                "2:",
                "move.w\t%d3,(-6,%fp)",
                "pea\t(-6,%fp)",
                "pea\t(-4,%fp)",
                "jsr\t(DISK_$UNASSIGN).l",
                "addq.w\t#8,%sp",
                "addq.w\t#1,%d3",
                "dbf\t%d2,2b",
                "movem.l\t(-0x10,%fp),%d2-%d3",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// No index register when the body does not read the index; `clr.w`
    /// for an index from 0, `move.w #n` for a count above 127, `subq.w`
    /// for `downto`.
    #[test]
    fn for_forms() {
        let f = proc_(
            4,
            vec![for_loop(
                konst(0, word()),
                konst(299, word()),
                false,
                vec![store(sym("W"), konst(1, word()))],
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "move.w\t#0x12b,%d0",
                ".balignw\t4,0x2048",
                "2:",
                "move.w\t#1,(W).l",
                "dbf\t%d0,2b",
            ]
        );
        let f = proc_(
            4,
            vec![for_loop(
                konst(9, word()),
                konst(1, word()),
                true,
                vec![store(sym("W"), index())],
            )],
        );
        let t = body_text(&f);
        assert_eq!(t[..2], ["moveq\t#8,%d0", "moveq\t#9,%d1"]);
        assert_eq!(t[t.len() - 2..], ["subq.w\t#1,%d1", "dbf\t%d0,2b"]);
        let f = proc_(
            4,
            vec![for_loop(
                konst(0, word()),
                konst(3, word()),
                false,
                vec![store(sym("W"), index())],
            )],
        );
        assert_eq!(body_text(&f)[1], "clr.w\t%d1");
    }

    /// Stepped pointers (codegen-patterns 7.4): LOG_$READ's worker
    /// 0xE177E0..0xE177F2 `lea (2,A1),A0; lea (2,A2),A1; move.w
    /// (-2,A0),(-2,A1); addq.l #2,A1; addq.l #2,A0; dbf`; the source formed
    /// first, stepped last; `subq.l` for `downto`; `lea (s,An),An` above 8.
    #[test]
    fn stepped_pointer_loops() {
        let through = |name: &str| Addr::Indirect {
            base: Box::new(load(sym(name), Scalar::POINTER)),
            offset: 0,
        };
        let elem = |name: &str, scale: u32, offset: i32| Addr::Index {
            base: Box::new(through(name)),
            index: Box::new(index()),
            scale,
            offset,
        };
        let f = proc_(
            4,
            vec![for_loop(
                konst(1, word()),
                konst(16, word()),
                false,
                vec![store(elem("DST", 2, -2), load(elem("SRC", 2, -2), word()))],
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "pea\t(%a2)",
                "moveq\t#0xf,%d0",
                "movea.l\t(SRC).l,%a0",
                "movea.l\t(DST).l,%a1",
                "lea\t(2,%a0),%a2",
                "lea\t(2,%a1),%a0",
                ".balignw\t4,0x2048",
                "2:",
                "move.w\t(-2,%a2),(-2,%a0)",
                "addq.l\t#2,%a0",
                "addq.l\t#2,%a2",
                "dbf\t%d0,2b",
                "movea.l\t(-8,%fp),%a2",
            ]
        );
        // Downward, 12-byte elements of a 0-based array, a field at 4, and
        // the index read besides.
        let f = proc_(
            4,
            vec![for_loop(
                konst(4, word()),
                konst(1, word()),
                true,
                vec![store(elem("TAB", 12, 4), index())],
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "moveq\t#3,%d0",
                "moveq\t#4,%d1",
                "movea.l\t(TAB).l,%a0",
                "lea\t(0x30,%a0),%a1",
                ".balignw\t4,0x2048",
                "2:",
                "move.w\t%d1,(4,%a1)",
                "subq.w\t#1,%d1",
                "lea\t(-0xc,%a1),%a1",
                "dbf\t%d0,2b",
            ]
        );
        // From 0: the pointer is the array's address, `lea (An),Ap`
        // (0xE3E4C6 `lea (A3),A2`); no index register.
        let f = proc_(
            4,
            vec![for_loop(
                konst(0, word()),
                konst(3, word()),
                false,
                vec![store(elem("TAB", 12, 4), konst(0, word()))],
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "moveq\t#3,%d0",
                "movea.l\t(TAB).l,%a0",
                "lea\t(%a0),%a1",
                ".balignw\t4,0x2048",
                "2:",
                "clr.w\t(4,%a1)",
                "lea\t(0xc,%a1),%a1",
                "dbf\t%d0,2b",
            ]
        );
    }

    /// Refused `for` loops (codegen-patterns 7.4).
    #[test]
    fn unsettled_for_loops() {
        let one = |from: Expr, to: Expr, down: bool, body: Vec<StmtKind>| {
            err(&proc_(4, vec![for_loop(from, to, down, body)]))
        };
        let w = || store(sym("W"), index());
        assert!(one(konst(3, word()), konst(2, word()), false, vec![w()])
            .starts_with("a `for` loop that never runs"));
        // The initial value equals the count: 0xE6A8BC copies the count.
        assert!(one(konst(19, word()), konst(0, word()), true, vec![w()])
            .contains("initial value equals the `dbf` count"));
        assert!(
            one(konst(200, word()), konst(300, word()), false, vec![w()])
                .contains("outside -128..127")
        );
        // A run-time bound without a call in the body.
        assert!(
            one(konst(1, word()), load(sym("N"), word()), false, vec![w()])
                .contains("no call in its body")
        );
        assert!(
            one(konst(0, word()), load(sym("N"), word()), false, vec![w()])
                .contains("with these bounds")
        );
        // An array of words indexed by the index: stepped pointers, only
        // through a pointer variable.
        let elem = |base: Addr, index: Expr| Addr::Index {
            base: Box::new(base),
            index: Box::new(index),
            scale: 2,
            offset: -2,
        };
        assert!(one(
            konst(1, word()),
            konst(4, word()),
            false,
            vec![store(elem(sym("TAB"), index()), konst(0, word()))]
        )
        .contains("not reached through a pointer"));
        let through = |offset| Addr::Indirect {
            base: Box::new(load(sym("P"), Scalar::POINTER)),
            offset,
        };
        assert!(one(
            konst(1, word()),
            konst(4, word()),
            false,
            vec![store(elem(through(4), index()), konst(0, word()))]
        )
        .contains("at an offset from its pointer"));
        // `a[i + 1]`: the displacement folding of an index expression is
        // not implemented.
        let plus = Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(index()),
            rhs: Box::new(konst(1, word())),
            ty: word(),
        };
        assert!(one(
            konst(1, word()),
            konst(4, word()),
            false,
            vec![store(elem(through(0), plus), konst(0, word()))]
        )
        .contains("index computed from a `for` index"));
        // Displacement 0 (`(An)+`): a 0-based array from 0... from 1.
        let elem0 = Addr::Index {
            base: Box::new(through(0)),
            index: Box::new(index()),
            scale: 2,
            offset: 0,
        };
        assert!(one(
            konst(1, word()),
            konst(4, word()),
            false,
            vec![store(elem0, konst(0, word()))]
        )
        .contains("at displacement 0"));
        // The pointer assigned in the body.
        assert!(one(
            konst(1, word()),
            konst(4, word()),
            false,
            vec![
                store(elem(through(0), index()), konst(0, word())),
                store(sym("P"), konst(0, Scalar::POINTER)),
            ]
        )
        .contains("leaves unchanged"));
        // `next` as the last statement: a branch to the next instruction.
        assert!(one(
            konst(1, word()),
            konst(4, word()),
            false,
            vec![w(), StmtKind::Next]
        )
        .contains("branch to the next instruction"));
    }

    /// `while` (codegen-patterns 7.4): `bra` to the test, the head aligned
    /// like any loop head, the test branching back when it holds.
    #[test]
    fn while_shape() {
        let f = proc_(
            0,
            vec![StmtKind::While {
                cond: IrCond::Compare {
                    op: Rel::Lt,
                    lhs: load(sym("W"), word()),
                    rhs: konst(10, word()),
                },
                body: vec![Stmt {
                    kind: store(sym("N"), konst(0, word())),
                    span: span(),
                }],
            }],
        );
        assert_eq!(
            body_text(&f),
            [
                "bra.s\t2f",
                ".balignw\t4,0x2048",
                "1:",
                "clr.w\t(N).l",
                "2:",
                "cmpi.w\t#0xa,(W).l",
                "blt.s\t1b",
            ]
        );
    }

    /// An `if` part ending in `exit` is followed by no `bra` to the join
    /// (it would never be reached), as for `return`.
    #[test]
    fn exit_ends_an_if_part() {
        let st = |kind| Stmt { kind, span: span() };
        let f = proc_(
            0,
            vec![StmtKind::While {
                cond: IrCond::Compare {
                    op: Rel::Lt,
                    lhs: load(sym("W"), word()),
                    rhs: konst(10, word()),
                },
                body: vec![
                    st(StmtKind::If {
                        cond: IrCond::Compare {
                            op: Rel::Eq,
                            lhs: load(sym("W"), word()),
                            rhs: konst(3, word()),
                        },
                        then_: vec![st(store(sym("N"), konst(1, word()))), st(StmtKind::Exit)],
                        else_: vec![st(store(sym("N"), konst(2, word())))],
                    }),
                    st(store(sym("W"), konst(0, word()))),
                ],
            }],
        );
        assert_eq!(
            body_text(&f),
            [
                "bra.s\t2f",
                ".balignw\t4,0x2048",
                "1:",
                "cmpi.w\t#3,(W).l",
                "bne.s\t4f",
                "move.w\t#1,(N).l",
                "bra.s\t3f",
                "4:",
                "move.w\t#2,(N).l",
                "clr.w\t(W).l",
                "2:",
                "cmpi.w\t#0xa,(W).l",
                "blt.s\t1b",
                "3:",
            ]
        );
    }

    /// A pointer local assigned in a loop is held in an address register
    /// (the DISK_ module-local 0xE3CCCE); not in a body that calls.
    #[test]
    fn loop_pointer_local_rule() {
        let p = || load(Addr::Frame(-4), Scalar::POINTER);
        let walk = |extra: Option<StmtKind>| {
            let mut body = vec![
                store(Addr::Frame(-4), load(sym("HEAD"), Scalar::POINTER)),
                StmtKind::While {
                    cond: IrCond::Compare {
                        op: Rel::Ne,
                        lhs: p(),
                        rhs: konst(0, Scalar::POINTER),
                    },
                    body: vec![Stmt {
                        kind: store(
                            Addr::Frame(-4),
                            load(
                                Addr::Indirect {
                                    base: Box::new(p()),
                                    offset: 0,
                                },
                                Scalar::POINTER,
                            ),
                        ),
                        span: span(),
                    }],
                },
            ];
            body.extend(extra);
            let mut f = proc_(4, body);
            locals(&mut f, &[(-4, Scalar::POINTER)]);
            f
        };
        let f = walk(None);
        assert_eq!(loop_pointer_locals(&f, &[]), [-4]);
        assert_eq!(
            body_text(&f),
            [
                "movea.l\t(HEAD).l,%a0",
                "bra.s\t2f",
                ".balignw\t4,0x2048",
                "1:",
                "movea.l\t(%a0),%a0",
                "2:",
                "cmpa.w\t#0,%a0",
                "bne.s\t1b",
            ]
        );
        let f = walk(Some(ext("X", vec![], 0)));
        assert!(loop_pointer_locals(&f, &[]).is_empty());
    }

    // ---- nested routines (codegen-patterns 6) ----------------------------

    fn nested(link: StaticLink, frame: u32, body: Vec<StmtKind>) -> Function {
        let mut f = proc_(frame, body);
        f.symbol = "N".into();
        f.global = false;
        f.nesting = Some(dpc_ir::Nesting {
            parent: "P".into(),
            link,
        });
        f
    }

    fn local_call(name: &str, args: Vec<Arg>, pad: u32, link: Option<LinkArg>) -> StmtKind {
        StmtKind::Call(Call {
            callee: Callee::Local(name.into()),
            args,
            pad,
            link,
        })
    }

    /// 0xE1359C: the link copied from A1 at entry into a callee-saved
    /// register (it lives across the call), the enclosing routine's slot
    /// passed on through it (0xE135B8 `move.l (8,A2),-(SP)`) and its object
    /// reached through the slot (0xE135DA `movea.l (8,A2),A0; move.w
    /// (0xc,A0),-(SP)`); `link #-4` for the link slot.
    #[test]
    fn static_link_passed() {
        let slot = || load(Addr::Enclosing(8), Scalar::POINTER);
        let f = nested(
            StaticLink::Passed,
            4,
            vec![
                ext(
                    "AST_$INVALIDATE_PAGE",
                    vec![Arg::Address {
                        addr: Addr::Indirect {
                            base: Box::new(slot()),
                            offset: 0,
                        },
                        slot: 4,
                        writable: true,
                    }],
                    0,
                ),
                ext(
                    "NETLOG",
                    vec![val(load(
                        Addr::Indirect {
                            base: Box::new(slot()),
                            offset: 0xc,
                        },
                        word(),
                    ))],
                    2,
                ),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "pea\t(%a2)",
                "movea.l\t%a1,%a2",
                "move.l\t(8,%a2),-(%sp)",
                "jsr\t(AST_$INVALIDATE_PAGE).l",
                "addq.w\t#4,%sp",
                "subq.l\t#2,%sp",
                "movea.l\t(8,%a2),%a0",
                "move.w\t(0xc,%a0),-(%sp)",
                "jsr\t(NETLOG).l",
                "movea.l\t(-8,%fp),%a2",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// 0xE1360C: the dynamic link as the static link, `movea.l (A6),An`;
    /// in a body without calls the lowest free register (0xE15888
    /// `movea.l (A6),A1` beside a parameter in A0).  The enclosing
    /// routine's local stored through it (0xE1375E `clr.w (-0x50,A4)`) and
    /// its parameter read (0xE158A2 `tst.b (0x14,A1)`).
    #[test]
    fn static_link_caller_frame() {
        let f = nested(
            StaticLink::CallerFrame,
            4,
            vec![
                store(Addr::Enclosing(-2), konst(0, word())),
                store(sym("W"), load(Addr::Enclosing(0x14), word())),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "movea.l\t(%fp),%a0",
                "clr.w\t(-2,%a0)",
                "move.w\t(0x14,%a0),(W).l",
                "unlk\t%fp",
                "rts",
            ]
        );
        // No reference to the enclosing frame: no link load.
        let f = nested(
            StaticLink::Passed,
            4,
            vec![store(sym("W"), konst(0, word()))],
        );
        assert!(!text(&f).iter().any(|l| l.contains("%a1")));
        // A nested routine does not assign the enclosing routine's
        // parameters either.
        let f = nested(
            StaticLink::CallerFrame,
            4,
            vec![store(Addr::Enclosing(8), konst(0, word()))],
        );
        assert!(
            err(&f).starts_with("assigning to a parameter"),
            "{}",
            err(&f)
        );
    }

    /// The caller side: the parent passes its own frame (0xE13928
    /// `movea.l A6,A1; bsr.w`), a sibling its own link (0xE13718 `movea.l
    /// A4,A1; bsr.w`), last before the `bsr`, after the pushes.
    #[test]
    fn calls_pass_the_static_link() {
        let mut f = proc_(
            0,
            vec![local_call(
                "N",
                vec![val(konst(5, long()))],
                0,
                Some(LinkArg::OwnFrame),
            )],
        );
        f.symbol = "P".into();
        assert_eq!(
            body_text(&f),
            ["pea\t(5).w", "movea.l\t%fp,%a1", "bsr.s\tN"]
        );
        let f = nested(
            StaticLink::CallerFrame,
            4,
            vec![
                local_call("M", vec![], 0, Some(LinkArg::OwnLink)),
                store(Addr::Enclosing(-2), konst(0, word())),
            ],
        );
        assert_eq!(
            text(&f),
            [
                "link.w\t%fp,#-4",
                "pea\t(%a2)",
                "movea.l\t(%fp),%a2",
                "movea.l\t%a2,%a1",
                "bsr.s\tM",
                "clr.w\t(-2,%a2)",
                "movea.l\t(-8,%fp),%a2",
                "unlk\t%fp",
                "rts",
            ]
        );
    }

    /// The link is loaded after the parameters loaded at entry (0xE082A8
    /// `move.l (8,A6),D7; move.w (0xc,A6),D3; movea.l (A6),A4`).
    #[test]
    fn static_link_after_entry_loads() {
        let mut f = nested(
            StaticLink::CallerFrame,
            4,
            vec![
                store(Addr::Enclosing(-2), load(Addr::Frame(8), word())),
                store(sym("W"), load(Addr::Frame(8), word())),
            ],
        );
        vparams(&mut f, &[(8, word())]);
        assert_eq!(
            body_text(&f),
            [
                "move.w\t(8,%fp),%d0",
                "movea.l\t(%fp),%a0",
                "move.w\t%d0,(-2,%a0)",
                "move.w\t%d0,(W).l",
            ]
        );
    }

    /// The enclosing routine's side (bead domain-pascal-thc): a parameter
    /// its nested routines reach is not loaded at entry but read in the
    /// frame at each use (PMAP_$FLUSH 0xE13788 and 0xE1379C reload
    /// `(8,A6)`, read by 0xE1359C; `(0x12,A6)` into D5 at entry).
    #[test]
    fn uplevel_parameters_stay_in_the_frame() {
        let body = || {
            vec![
                store(sym("W"), load(Addr::Frame(10), word())),
                StmtKind::If {
                    cond: IrCond::Compare {
                        op: Rel::Eq,
                        lhs: load(Addr::Frame(8), word()),
                        rhs: konst(3, word()),
                    },
                    then_: vec![Stmt {
                        kind: store(sym("X"), konst(0, word())),
                        span: span(),
                    }],
                    else_: vec![],
                },
                store(sym("W"), load(Addr::Frame(10), word())),
            ]
        };
        let mut f = proc_(0, body());
        vparams(&mut f, &[(8, word()), (10, word())]);
        let t = body_text(&f);
        assert_eq!(t[..2], ["move.w\t(8,%fp),%d0", "move.w\t(0xa,%fp),%d1"]);
        assert!(t.contains(&"cmpi.w\t#3,%d0".to_string()), "{t:?}");
        let mut f = proc_(0, body());
        vparams(&mut f, &[(8, word()), (10, word())]);
        f.uplevel_refs = vec![(8, 2)];
        let t = body_text(&f);
        assert_eq!(t[..2], ["move.w\t(0xa,%fp),%d0", "move.w\t%d0,(W).l"]);
        assert!(t.contains(&"cmpi.w\t#3,(8,%fp)".to_string()), "{t:?}");
    }

    /// Bead domain-pascal-pe9: a by-value parameter whose address is
    /// passed to a `var`, `out` or `in out` parameter is read from its
    /// slot at each use, not held from the entry (AREA_$TOUCH 0xE094FE,
    /// 0xE095A6 / 0xE095BA); passed to a value or `in` parameter, it is
    /// held (MST_$TOUCH 0xE0DD4E, 0xE0DEFC, 0xE0E07A).
    #[test]
    fn parameters_passed_by_address_for_writing_stay_in_the_frame() {
        let body = |writable| {
            vec![
                store(sym("W"), load(Addr::Frame(10), word())),
                StmtKind::If {
                    cond: IrCond::Compare {
                        op: Rel::Eq,
                        lhs: load(Addr::Frame(8), word()),
                        rhs: konst(3, word()),
                    },
                    then_: vec![Stmt {
                        kind: store(sym("X"), konst(0, word())),
                        span: span(),
                    }],
                    else_: vec![],
                },
                ext(
                    "P",
                    vec![Arg::Address {
                        addr: Addr::Frame(8),
                        slot: 4,
                        writable,
                    }],
                    0,
                ),
                StmtKind::If {
                    cond: IrCond::Compare {
                        op: Rel::Eq,
                        lhs: load(Addr::Frame(8), word()),
                        rhs: konst(4, word()),
                    },
                    then_: vec![Stmt {
                        kind: store(sym("X"), konst(0, word())),
                        span: span(),
                    }],
                    else_: vec![],
                },
                store(sym("W"), load(Addr::Frame(10), word())),
            ]
        };
        let mut f = proc_(0, body(false));
        vparams(&mut f, &[(8, word()), (10, word())]);
        let t = body_text(&f);
        assert_eq!(
            t[..3],
            [
                "movem.l\t%d2-%d3,-(%sp)",
                "move.w\t(8,%fp),%d2",
                "move.w\t(0xa,%fp),%d3"
            ]
        );
        assert!(t.contains(&"cmpi.w\t#4,%d2".to_string()), "{t:?}");
        let mut f = proc_(0, body(true));
        vparams(&mut f, &[(8, word()), (10, word())]);
        let t = body_text(&f);
        assert_eq!(t[1..3], ["move.w\t(0xa,%fp),%d2", "move.w\t%d2,(W).l"]);
        assert!(t.contains(&"cmpi.w\t#3,(8,%fp)".to_string()), "{t:?}");
        assert!(t.contains(&"cmpi.w\t#4,(8,%fp)".to_string()), "{t:?}");
        assert!(!t.iter().any(|l| l.contains("%d3")), "{t:?}");
    }

    /// What lets a frame object's address escape for writing (bead
    /// domain-pascal-pe9): `var`-like arguments and `addr`, through
    /// fields and elements; not value or `in` arguments, other frames or
    /// the objects pointers point to.
    #[test]
    fn escaped_frame_objects_cases() {
        let arg = |addr, writable| Arg::Address {
            addr,
            slot: 4,
            writable,
        };
        let f = proc_(
            0,
            vec![
                ext("P", vec![arg(Addr::Frame(8), true)], 0),
                ext("P", vec![arg(Addr::Frame(10), false)], 0),
                ext("P", vec![val(Expr::AddrOf(Addr::Frame(-4)))], 0),
                ext(
                    "P",
                    vec![arg(
                        Addr::Index {
                            base: Box::new(Addr::Frame(-20)),
                            index: Box::new(load(Addr::Frame(12), word())),
                            scale: 2,
                            offset: 0,
                        },
                        true,
                    )],
                    0,
                ),
                ext("P", vec![arg(Addr::Enclosing(8), true)], 0),
                ext("P", vec![arg(through(14, 0), true)], 0),
            ],
        );
        assert_eq!(escaped_frame_objects(&f), [8, -4, -20]);
    }

    #[test]
    fn store_may_write_cases() {
        // codegen-patterns 12.4 R7: which stores end the reuse of a
        // computation of a plain variable.
        let sym = |name: &str, offset| Addr::Symbol {
            name: name.into(),
            offset,
        };
        let elem = |base: Addr| Addr::Index {
            base: Box::new(base),
            index: Box::new(load(Addr::Frame(8), word())),
            scale: 4,
            offset: 0,
        };
        let w = |a: &Addr, l: &Addr, escaped| store_may_write(a, 2, l, 2, escaped);
        // The variable itself, or an overlapping part of it.
        assert!(w(&Addr::Frame(-4), &Addr::Frame(-4), false));
        assert!(store_may_write(
            &Addr::Static(5),
            1,
            &Addr::Static(4),
            2,
            false
        ));
        assert!(w(&sym("X", 0), &sym("X", 0), false));
        // Another variable.
        assert!(!w(&Addr::Frame(-6), &Addr::Frame(-4), false));
        assert!(!w(&Addr::Static(0), &Addr::Static(2), false));
        assert!(!w(&sym("Y", 0), &sym("X", 0), false));
        assert!(!w(&Addr::Static(0), &sym("X", 0), false));
        assert!(!w(&elem(sym("Y", 0)), &sym("X", 0), false));
        assert!(!w(&Addr::Enclosing(-4), &Addr::Frame(-4), false));
        // An element at a run-time index of the same area or array.
        assert!(w(&elem(Addr::Frame(-20)), &Addr::Frame(-4), false));
        assert!(w(&elem(Addr::Static(100)), &Addr::Static(2), false));
        assert!(w(&elem(sym("X", 0)), &sym("X", 8), false));
        // Through a pointer: the unit's data and other units' objects
        // always, the frame once some frame object's address escaped.
        let p = through(14, 0);
        assert!(w(&p, &Addr::Static(2), false));
        assert!(w(&p, &sym("X", 0), false));
        assert!(!w(&p, &Addr::Frame(-4), false));
        assert!(!w(&p, &Addr::Frame(12), false));
        assert!(w(&p, &Addr::Frame(-4), true));
    }
}
