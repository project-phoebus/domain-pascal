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
    Access, Addr, Arg, BinOp, BitField, Call, Callee, CaseArm, Cond as IrCond, Expr, ForLoop,
    Function, LinkArg, Rel, ResultClass, ResultSpec, Scalar, SetOp, Sign, StaticLink, Stmt,
    StmtKind, UnOp, Width,
};
use dpc_syntax::{Diagnostic, Span};
use std::collections::BTreeSet;

use crate::emit::DATA_BASE_LABEL;
use crate::frame::{frame_for, FP, SP};
use crate::insn::{AluOp, BitOp, Cond, Ea, Insn, Reg, RegSet, ShiftCount, ShiftOp, Size, UnaryOp};
use crate::regs::{allocate_apart, call_results_first, RegClass, VRegs, DATA_BASE};
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

/// The relation `r` with its operands exchanged: `a r b` = `b (swap_rel
/// r) a`.
fn swap_rel(r: Rel) -> Rel {
    match r {
        Rel::Eq | Rel::Ne => r,
        Rel::Lt => Rel::Gt,
        Rel::Gt => Rel::Lt,
        Rel::Le => Rel::Ge,
        Rel::Ge => Rel::Le,
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

/// Whether a multiplication by the constant `c` that is not done with
/// shifts and adds ([`by_shifts`]) is one `mul<s>.w` (codegen-patterns 8,
/// "Multiplication by a constant"; `tools/arithsurvey.py scale`): `c` in
/// 1..=0xffff with an odd part of four bits or more (23: ×46, ×92; 75:
/// ×150; 51, 57, 61, 125, 141, 153, 203, 311, 655, 1083, 12345), or one of the
/// odd parts of three bits seen with a multiply: 11 (×44, TPAD_$SET_UNIT_MODE
/// 0xE69826), 25 (×100, VTOC_$MOUNT 0xE38614), 21 (×336, 0xE395AA; ×84,
/// HINT_$GET_HINTS 0xE4999A), 67 (×268, 0xE1D7A2), 145 (×580, 0xE2FB08).  Odd part 13 is done bit by bit
/// (×26, NETLOG_$LOG_IT 0xE71BBA, the one such site) and no other odd part
/// of three bits is seen: refused (bead domain-pascal-irb4).
fn by_mul_insn(c: i64) -> bool {
    if !(1..=0xffff).contains(&c) || by_shifts(c) {
        return false;
    }
    let m = (c >> c.trailing_zeros()) as u32;
    m.count_ones() >= 4 || [11, 21, 25, 67, 145].contains(&m)
}

/// Whether a product of `x` by a constant of 1..=127 done with a multiply
/// puts the constant in the register (`moveq #c,Dn; mul<s>.w x,Dn`,
/// [`Selector::mul_const_first`]): `x` a variable.  For a computed value
/// see [`in_place`].
fn const_first(x: &Expr) -> bool {
    matches!(x, Expr::Load { .. })
}

/// Whether a computed `x` times a constant of 1..=127 done with a
/// multiply is multiplied in place, `mul<s>.w #c`: a zero-extended byte
/// (RIP_$BROADCAST 0xE872F8 `clr.w D5; move.b (14,A2),D5; mulu.w #92,D5`,
/// the one in-place site).  Other computed values are refused
/// ([`computed_factor`]): the two others, masks, have the constant first
/// (HINT_$GET_HINTS 0xE4998E..0xE4999A `moveq #63,D3; moveq #84,D5; and.w
/// D0,D3; movea.l #sym,A1; muls.w D3,D5`; 0xE49A5C..0xE49A68).
fn in_place(x: &Expr) -> bool {
    matches!(x, Expr::Convert { value, .. }
        if value.ty().width == Width::Byte && !value.ty().is_signed()
            && matches!(**value, Expr::Load { .. }))
}

/// The refusal of a computed value other than a zero-extended byte times
/// a constant `c` of 1..=127 done with a multiply ([`in_place`]).
fn computed_factor(span: Span, c: i64) -> Diagnostic {
    unsupported(
        span,
        format!(
            "a multiplication by {c} of a computed value (`mul<s>.w #{c}` in place for a \
             zero-extended byte, RIP_$BROADCAST 0xE872F8; the constant first for a mask, \
             HINT_$GET_HINTS 0xE4998E; not settled, codegen-patterns 8; bead \
             domain-pascal-q322) is"
        ),
    )
}

/// The refusal of a multiplication by `c` whose odd part has three bits
/// and is not one seen with a multiply ([`by_mul_insn`]).
fn three_bit_odd_part(span: Span, c: i64) -> Diagnostic {
    unsupported(
        span,
        format!(
            "a multiplication by {c} (shifts and adds or `mul<s>.w`: odd parts of three bits \
             other than 11, 21, 25, 67 and 145 not settled, ×26 bit by bit at 0xE71BBA; bead \
             domain-pascal-irb4) is"
        ),
    )
}

fn size_of(w: Width) -> Size {
    match w {
        Width::Byte => Size::B,
        Width::Word => Size::W,
        Width::Long => Size::L,
    }
}

/// The memory operand `ea` moved `k` bytes up, when its mode allows it
/// (a 16-bit displacement, an 8-bit one with an index, an absolute
/// address); `None` otherwise.  Used to read the low bytes of a
/// big-endian object ([`Selector::convert`]).
fn displace(ea: &Ea, k: i32) -> Option<Ea> {
    Some(match ea {
        Ea::Ind(a) => Ea::Disp(k, *a),
        Ea::Disp(d, a) if i16::try_from(d + k).is_ok() => Ea::Disp(d + k, *a),
        Ea::Idx(d, a, x, l) if i8::try_from(d + k).is_ok() => Ea::Idx(d + k, *a, *x, *l),
        Ea::AbsL(s, o) => Ea::AbsL(s.clone(), o + k),
        Ea::AbsW(v) if i16::try_from(v + k).is_ok() => Ea::AbsW(v + k),
        _ => return None,
    })
}

/// Whether `value` narrowed to `ty` is read in place, its low bytes at
/// the higher address ([`displace`], [`Selector::convert`]): a load of a
/// non-volatile object one step wider (a longword as a word, a word as a
/// byte), not an element with a computed index.  A volatile object is
/// read at its own size (ref 3.15.1-3.15.3); a longword's low byte is
/// not catalogued, nor is an element's low part (AST_$TOUCH 0xE0350E
/// reads an element's longword whole, `move.l (A1),D5; move.w
/// D5,-(SP)`; codegen-patterns 8, "Narrowing and widening").  These
/// reach the selector only through `chr` (`dpc-ir` refuses them in an
/// assignment), which reads them whole as before.
fn narrows_in_place(value: &Expr, ty: Scalar) -> bool {
    value.ty().bytes() == 2 * ty.bytes()
        && matches!(
            value,
            Expr::Load {
                addr: Addr::Frame(_)
                    | Addr::Enclosing(_)
                    | Addr::Static(_)
                    | Addr::Symbol { .. }
                    | Addr::OtherStatic { .. }
                    | Addr::Indirect { .. },
                ..
            }
        )
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
    // For the driver's internal-error backstop (`diag::set_context`).
    let outer = dpc_syntax::diag::set_context(Some(f.span));
    let insns = select_in_context(f, sched);
    dpc_syntax::diag::set_context(outer);
    insns
}

fn select_in_context(f: &Function, sched: Schedule) -> Result<Vec<Insn>, Diagnostic> {
    if f.is_empty_procedure() {
        if let Some(pc) = f.param_copies.first() {
            // A bare `rts` has no frame for the copy; whether the original
            // copied an unused parameter is not visible (codegen-patterns
            // 1.2, 2).
            return Err(unsupported(
                pc.span,
                "an empty procedure with a value parameter passed by address (a bare `rts` \
                 in the image, so not copied; codegen-patterns 1.2, 2) is",
            ));
        }
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
///
/// A body whose end nothing reaches (its last instruction a `bra` back
/// into it, no branch to the labels after that: a loop with no test and
/// no way out) has no epilogue: the image's 6 compiled routines with no
/// reachable `unlk`/`rts` all end in such a `bra`, followed by the next
/// routine or the routine's cells (DXM_$HELPER_COMMON 0xE1721C `bra.s
/// 0xE171FC`, then DXM_$HELPER_WIRED at 0xE1721E; PKT_$PING_SERVER
/// 0xE12D2E, PMAP_$PURIFIER_L 0xE14166, PMAP_$PURIFIER_R 0xE143AE,
/// PROC2_$DELETE 0xE743CC, the module-local 0xE76048 at 0xE76286;
/// codegen-patterns 7.2).  The prologue still saves the registers the body
/// writes (0xE171E8 `movem.l D2/A2-A4,-(SP)`).
pub fn wrap(f: &Function, body: Body) -> Result<Vec<Insn>, Diagnostic> {
    let frame = frame_for(f.frame_size, body.saves, f.span)?;
    let mut out = frame.prologue();
    let reached = end_reached(&body.insns);
    out.extend(body.insns);
    if reached {
        out.extend(frame.epilogue());
    }
    Ok(out)
}

/// Whether the end of `body` (where the epilogue goes) is reached: by
/// falling off its last instruction, or by a branch to a label after it.
fn end_reached(body: &[Insn]) -> bool {
    let Some(last) = body
        .iter()
        .rposition(|i| !matches!(i, Insn::Label(_) | Insn::Align4))
    else {
        return true;
    };
    if !matches!(body[last], Insn::Branch { cond: Cond::T, .. }) {
        return true;
    }
    let tail: Vec<u32> = body[last + 1..]
        .iter()
        .filter_map(|i| match i {
            Insn::Label(l) => Some(*l),
            _ => None,
        })
        .collect();
    body.iter().any(|i| match i {
        Insn::Branch { label, .. } | Insn::Dbf { label, .. } => tail.contains(label),
        Insn::TableEntry { target, .. } => tail.contains(target),
        _ => false,
    })
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
/// earlier statements ([`fresh_slot_loads`]; IIC_$RECEIVE 0xE70ABA)
/// before a body's first branch or call, and after it only where the
/// lowest free register is clean on every path
/// ([`check_late_slot_loads`]); not to the end of a statement
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
    // A pointer local that no call follows is found by allocation; the
    // body is selected again with it held in A0 ([`Selector::a0_locals`]).
    let (body, hits, in_a0) = match select_body_with(f, CsePlan::Probe, sched, &[])? {
        Selected::Body(b, h) => (b, h, Vec::new()),
        Selected::InA0(in_a0) => match select_body_with(f, CsePlan::Probe, sched, &in_a0)? {
            Selected::Body(b, h) => (b, h, in_a0),
            Selected::InA0(..) => unreachable!("an A0-held local is not found again"),
        },
    };
    if hits.is_empty() {
        return Ok(body);
    }
    let Selected::Body(body, again) =
        select_body_with(f, CsePlan::Hold(hits.clone()), sched, &in_a0)?
    else {
        unreachable!("an A0-held local is not found again")
    };
    if again != hits {
        // The second selection must find the reuses the first one did:
        // the validity of a reuse depends only on labels, calls and
        // stores, which holding a value does not change.
        return Err(unsupported(
            f.span,
            "a body whose reusable computed values change when they are held (codegen-patterns \
             12.4 R7) is",
        ));
    }
    Ok(body)
}

/// What [`select_body_with`] gives: the body and the reuses found, or the
/// pointer locals to select again held in A0 ([`Selector::a0_locals`]).
enum Selected {
    Body(Body, Vec<(u32, u32)>),
    InA0(Vec<i32>),
}

fn select_body_with(
    f: &Function,
    plan: CsePlan,
    sched: Schedule,
    in_a0: &[i32],
) -> Result<Selected, Diagnostic> {
    let mut s = Selector::new(f.span);
    s.cse_plan = plan;
    s.data_label = crate::emit::data_base_label(&f.data_section);
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
    s.address_slots = f.address_params.clone();
    // Labels emit nothing: a body is its other statements.
    let stmts: Vec<&Stmt> = f
        .body
        .iter()
        .filter(|st| !matches!(st.kind, StmtKind::Label(_)))
        .collect();
    let control_only = stmts.len() == 1 && !is_simple(stmts[0]);
    s.multi_body = stmts.len() >= 2 || control_only;
    // The routine's labels and the `goto`s to each ([`Selector::place_user_label`]).
    let mut nlabels = 0;
    dpc_ir::visit_stmts(&f.body, &mut |st| {
        if let StmtKind::Label(l) = st.kind {
            nlabels = nlabels.max(l as usize + 1);
        }
    });
    s.user_labels = vec![None; nlabels];
    s.gotos_total = vec![0; nlabels];
    s.gotos_seen = vec![0; nlabels];
    dpc_ir::visit_stmts(&f.body, &mut |st| {
        if let StmtKind::Goto(l) = st.kind {
            s.gotos_total[l as usize] += 1;
        }
    });
    s.label_clean = label_tails(&f.body, nlabels);
    // The entry loads (their range in `out`) after a block of another
    // data section loaded before them, checked after allocation.
    let mut hoisted: Option<std::ops::Range<usize>> = None;
    if s.multi_body {
        // A body of one `if`, `case` or loop statement loads the by-value
        // parameters it reads at entry as a body of several statements
        // does (codegen-patterns 12.2, "One-statement bodies"; bead
        // domain-pascal-s4a5): KBD_$RCV's nested 0xE1CC64 `move.b
        // (8,A6),D1` before its `case` chain, tty_$i_set_raw_mode 0xE1BF82
        // `move.b (0xc,A6),D0; bpl` (read once, in the condition),
        // PROC2_$SET_CLEANUP 0xE41580 (read in the body only, past a test
        // of a global), DBUF_$UPDATE_VOL 0xE3AAB0 (a `for'), and every
        // other such body surveyed (`tools/localsurvey.py onestmt`); none
        // reads one in its slot.
        // A function of one such statement that assigns a by-value
        // parameter to its result: KBD_$RCV's nested 0xE1CC64 keeps `ch`
        // in D1 apart from the result in D0 (`move.b D1b,D0b` at
        // 0xE1CCBA, the selector widened into D0), where the allocator's
        // result variable shares a register with a parameter it is
        // assigned from (0xE15222; codegen-patterns 12.4 R5).  Which
        // applies is not settled (bead domain-pascal-0zo8).
        if control_only && f.result.is_some() {
            let mut copies_param = false;
            dpc_ir::visit_stmts(&f.body, &mut |st| {
                if let StmtKind::SetResult(e) = &st.kind {
                    let mut e = e;
                    while let Expr::Convert { value, .. } = e {
                        e = value;
                    }
                    if let Expr::Load {
                        addr: Addr::Frame(o),
                        ..
                    } = e
                    {
                        copies_param |= f.value_params.iter().any(|p| p.offset == *o);
                    }
                }
            });
            if copies_param {
                return Err(unsupported(
                    f.body[0].span,
                    "a function body of one `if`, `case` or loop statement that assigns a \
                     by-value parameter to the result (kept apart at 0xE1CCBA, shared at \
                     0xE15222; not settled, codegen-patterns 12.2) is",
                ));
            }
        }
        let held: Vec<i32> = register_locals(f).iter().map(|&(o, _)| o).collect();
        reject_mergeable_stores(&f.body, &held)?;
        // A parameter loaded at entry and live across a call is held in a
        // callee-saved register by the allocator (codegen-patterns 12.4
        // R3: PROC1_$SET_TYPE 0xE152F2 D2, D3 around a `jsr`;
        // module-local 0xE32B76, 0xE32B7E A2, D2 around three).
        // The block of another data section that the first statement
        // reaches is loaded before the parameters loaded at entry and the
        // static link, as A5 is (KBD_$INIT 0xE3336A `movea.l #0xE2DDE4,A0`
        // before 0xE33370 `movea.l (8,A6),A2`; NETWORK_$READ_SERVICE
        // 0xE71D80 before `movea.l (8,A6),A0`; the RINGLOG_ module-local
        // 0xE721D4 before `movea.l (A6),A2`; no routine loads its
        // parameters first when its first statement starts with the block,
        // `tools/sectsurvey.py otherdata`).
        // Where the block of a first statement that reaches it otherwise
        // is loaded relative to those loads is not seen: refused below.
        let first = stmts.first().copied();
        if let Some(section) = first.and_then(first_other_static) {
            s.other_base(&section)?;
        }
        let pinned = s.pinned.len();
        let entry_start = s.out.len();
        s.entry_loads(f)?;
        if !s.other_base_loads.is_empty() {
            hoisted = Some(entry_start..s.out.len());
        }
        if s.pinned.len() > pinned
            && first.is_some_and(|st| first_other_static(st).is_none() && mentions_other_static(st))
        {
            return Err(unsupported(
                first.unwrap().span,
                "a first statement that reaches another data section's block other than by a \
                 constant store, in a routine that loads parameters at entry (which the \
                 original loaded first is not seen, codegen-patterns 11) is",
            ));
        }
        s.entry_regs = s.pinned.iter().map(|&(_, r)| r).collect();
        if s.pinned.len() > pinned {
            s.entry_end = Some(entry_start..s.out.len());
        }
        // A function body of several statements holds its result variable
        // in a register up to the epilogue (codegen-patterns 3, "Result
        // variable"; [`Selector::set_result_web`]).
        s.result_web = f.result.is_some();
    }
    s.load_link(f)?;
    s.param_copies(f)?;
    let reg_locals = register_locals(f);
    s.reg_locals = reg_locals.iter().map(|&(o, _)| o).collect();
    for &(o, web) in &reg_locals {
        if web {
            let d = s.alloc_d()?;
            s.webs.push((o, d));
        }
    }
    s.ptr_locals = loop_pointer_locals(f, &s.reg_locals)
        .into_iter()
        .map(|o| (o, None))
        .collect();
    // A `with` record reached at run time: its pointer, computed once,
    // held in an address register for the body (codegen-patterns 5.2,
    // "`with`"; NETLOG_$LOG_IT 0xE71BCA).
    s.ptr_locals.extend(f.with_temps.iter().map(|&o| (o, None)));
    let mut taken = s.reg_locals.clone();
    taken.extend(s.ptr_locals.iter().map(|&(o, _)| o));
    let call_ptrs = call_pointer_locals(f, &taken);
    for &o in &call_ptrs {
        // An integer local of [`integer_call_local`] is parked, where a
        // call follows too; a pointer local that no call follows is held
        // in A0 itself when allocation finds it there.
        let integer = f
            .scalar_locals
            .iter()
            .any(|l| l.offset == o && l.ty != Scalar::POINTER);
        if in_a0.contains(&o) {
            s.a0_locals.push(o);
        } else if integer {
            s.parked.push(Park {
                off: o,
                d: None,
                home: None,
            });
        } else {
            s.ptr_locals.push((o, None));
        }
    }
    s.seq(f, &f.body, Tail::END)?;
    // An element address of another unit's array or through a loaded
    // pointer formed as a value ([`Selector::lea_into`]): read twice or
    // more, or live across a call, the allocator's choice of its register
    // is the image's (codegen-patterns 5.2, "The element's address as a
    // value"; `tools/regalloc.py elemaddr`): read twice or more with no
    // call, it takes the lowest register free over its range, the dying
    // base's counted free, where that and R2 differ 129 times against 6
    // (XPD_$FIND_INDEX 0xE5AF60 `lea (0,A0,D1.l),A0`), and with R4(c) for
    // a last read that is a call's argument 197 of 208 sites fit
    // (MMAP_$FREE_LIST 0xE0CB50 `lea (0,A0,D0.l),A2`, `pea (-8192,A2);
    // bsr.w`); live across a call, the lowest callee-saved register free
    // (R3), the base in another 122 times and in it 21 (MMAP_$AVAIL
    // 0xE0CC80 `lea (0,A0,D0.l),A2`).  Read once in no call's range it fits
    // the first 98 times and R2 99 times (0xE32CD6 `lea (78,A0,D3.l),A0`,
    // stored; 0xE1C242 `lea (0,A0,D0.w),A1`, stored), neither 114
    // (SIO6509_$INIT 0xE33548 `lea (-32,A3,D0.w),A3`): not settled,
    // refused.
    for &(k, span) in &s.elem_leas {
        let Insn::Lea { dst, .. } = s.out[k] else {
            unreachable!("an element address's `lea`")
        };
        let r = Reg::A(dst);
        let mut reads = 0;
        let mut call = false;
        let mut across = false;
        for i in &s.out[k + 1..] {
            let a = crate::regs::access(i);
            if a.uses.contains(&r) {
                reads += 1;
                across |= call;
            }
            if a.defs.contains(&r) && !a.uses.contains(&r) {
                break;
            }
            call |= matches!(i, Insn::Jsr(_) | Insn::Bsr { .. });
        }
        if reads < 2 && !across {
            return Err(unsupported(
                span,
                "the address of an element of another unit's array or through a pointer loaded \
                 from memory, read once and not across a call (the `lea` into the base's \
                 register or another, 98 against 99 sites: not settled, codegen-patterns 5.2; \
                 bead domain-pascal-1ibr) is",
            ));
        }
    }
    if !s.other_base_loads.is_empty()
        && s.out
            .iter()
            .any(|i| matches!(i, Insn::Branch { .. } | Insn::Dbf { .. } | Insn::Label(_)))
    {
        // Across a branch target the image both keeps the block's register
        // (NETWORK_$READ_SERVICE 0xE71D9A after an `if`; loops, TPAD_$INIT
        // 0xE335A4) and loads it again (AST_$ACTIVATE_ASTE_CANNED 0xE2F266,
        // NAME_$INIT 0xE317C0, NET_IO_$BOOT_DEVICE 0xE31C66): not settled
        // (codegen-patterns 11).
        return Err(unsupported(
            f.span,
            "a routine that reaches another data section's block and branches (where the \
             block's address is loaded again is not settled, codegen-patterns 11) is",
        ));
    }
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
    // Branches to a `bra` go to its target, and a `bra` no longer
    // reached is replaced by a label nothing names (dropped below with
    // the other unused labels; codegen-patterns 7.1, "Branch threading").
    thread_branches(&mut s.out, &mut s.next_label, &s.before_bra, f.span)?;
    // Labels nothing branches to (the join after a `then` part that ends
    // in `return`, ...) are dropped; they emit no bytes either way.
    // A `case` table's entries name their targets like branches (a
    // target before a `bra` is refused, [`thread_branches`]); its own
    // label is named by the dispatch.
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
    // An `exit` or `next` whose target is the very next instruction (a
    // `next` ending a loop body, an `exit` before an inner loop's end):
    // whether the original emitted the branch is not known.
    for &(at, span) in &s.loop_jumps {
        let label = match s.out[at] {
            Insn::Branch { label, .. } => label,
            // Dropped by [`thread_branches`]: the branches to it now go
            // where it went, and nothing reaches it.
            Insn::Label(_) => continue,
            _ => unreachable!("loop_jumps records branches"),
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
    // Likewise a `goto` to the next instruction (`goto L; L: ...`, or
    // `if c then goto L` before it): not seen.
    debug_assert_eq!(s.open_back, 0, "every backward goto selected");
    for &(at, span) in &s.goto_jumps {
        let label = match s.out[at] {
            Insn::Branch { label, .. } => label,
            // Dropped by [`thread_branches`]: the branches to it now go
            // where it went, and nothing reaches it.
            Insn::Label(_) => continue,
            _ => unreachable!("goto_jumps records branches"),
        };
        let next = s.out[at + 1..]
            .iter()
            .take_while(|i| matches!(i, Insn::Label(_) | Insn::Align4))
            .any(|i| matches!(i, Insn::Label(l) if *l == label));
        if next {
            return Err(unsupported(
                span,
                "a `goto` that would branch to the next instruction is",
            ));
        }
    }
    // References to volatile objects (ref 3.15.1-3.15.3): the
    // instructions that reference one, kept in order by the scheduler;
    // none of them a read-modify-write of a device object.
    let volatile_insns = volatile_references(&s.out, &s.volatile_eas, f.span)?;
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
        .chain(
            s.parked
                .iter()
                .filter_map(|p| p.home.map(|(a, _)| Reg::A(a))),
        )
        .collect();
    // The registers holding another data section's block: not held
    // through a call whose arguments use them last (XNS_IDP_$INIT
    // 0xE3026E..0xE30280: `movea.l #0xE2B314,A0` in one statement,
    // `pea (0x520,A0)` for the call of the next; MAC_OS_$INIT 0xE2F504),
    // loaded again after it ([`Selector::other_base`]).
    let blocks: Vec<Reg> = s
        .other_base_loads
        .iter()
        .filter_map(|&i| match s.out[i] {
            Insn::Movea { dst, .. } => Some(Reg::A(dst)),
            _ => None,
        })
        .collect();
    // A block's register loaded before a call and read after it (`l := b
    // + g(x)`: the operand's base taken before the call is evaluated):
    // the survey sees no block register live across a call, every use
    // after one follows a load of its own (codegen-patterns 11); where the
    // original loads the block for such an expression is not seen.
    if !blocks.is_empty() {
        let live_out = crate::regs::liveness(&s.out, exit, &[]);
        if let Some(&(_, _, span)) = s.call_stmts.iter().find(|&&(_, c, _)| {
            blocks
                .iter()
                .any(|&r| live_out[c].contains(&crate::regs::RegKey(r)))
        }) {
            return Err(unsupported(
                span,
                "another data section's block reached on both sides of a call in one statement \
                 (where the original loads the block is not seen, codegen-patterns 11) is",
            ));
        }
    }
    let mut holds = held_through_calls(
        &s.out,
        &s.call_stmts,
        exit,
        HeldKinds {
            link: s.link.map(Reg::A),
            ptrs: &held_ptrs,
            blocks: &blocks,
            params: &s.entry_regs,
            data_ptrs: &s.data_ptr_regs,
        },
    )?;
    // A parked local's data register stays the local's until its last
    // dereference, though only the address register is read there: the
    // image leaves D0 alone over that range (TTY_$K_SET_INPUT_FLAG
    // 0xE6768E..0xE676A2 builds its set in D1 and D2, saving D2, with D0
    // free; TTY_$K_SET_DELAY 0xE67A42 `move.w (A2),D1`; the DIR_
    // module-local 0xE52394 at 0xE5243E `move.w (22,A0),D1`).
    for p in &s.parked {
        if let (Some((d, _)), Some((a, at))) = (p.d, p.home) {
            for (i, insn) in s.out.iter().enumerate().skip(at + 1) {
                if matches!(insn, Insn::Jsr(_) | Insn::Bsr { .. }) {
                    // Parked where a call follows: the call clobbers D0, which is free
                    // from there (TTY_$K_FLUSH_INPUT 0xE1C0BE `move.w
                    // D0,(-6,A6)`, the spin lock's token).
                    break;
                }
                if crate::regs::access(insn).uses.contains(&Reg::A(a)) {
                    holds.push((i, Reg::D(d)));
                }
            }
        }
    }
    let (mut apart, late) = fresh_slot_loads(&s.out, &s.fresh);
    apart.extend(argument_list_loads(&s.out, &s.call_stmts)?);
    for &d in &s.callee_only {
        apart.extend([(Reg::D(d), Reg::D(0)), (Reg::D(d), Reg::D(1))]);
    }
    apart.extend(s.colour_apart.iter().copied());
    // A register-held local read before the first instruction that
    // assigns it (a read the loop's back edge reaches, or of a local not
    // yet assigned): its register is live into the body.  Where the
    // original colours such a value is not known; refused rather than
    // reported as the allocator's internal check.
    for &(_, d) in &s.webs {
        let r = Reg::D(d);
        let first = s.out.iter().find_map(|i| {
            let a = crate::regs::access(i);
            if a.uses.contains(&r) {
                Some(true)
            } else if a.defs.contains(&r) {
                Some(false)
            } else {
                None
            }
        });
        if first == Some(true) {
            return Err(unsupported(
                f.span,
                "a register-held local read before the code first assigns it (in a loop, \
                 assigned later in its body; or not yet assigned; codegen-patterns 12.5) is",
            ));
        }
    }
    let first = s.web.map(|w| (Reg::D(w), s.entry_regs.clone()));
    // A pointer local assigned a call's result before a parameter that
    // dies in the same block (codegen-patterns 12.4 R1b, `regs.rs`).
    let results: Vec<usize> = s
        .ptr_defs
        .iter()
        .filter(|(o, _, _)| call_ptrs.contains(o))
        .map(|&(_, k, _)| k)
        .collect();
    let mut after = call_results_first(&s.out, exit, &holds, &results, &s.entry_regs);
    after.extend(s.colour_after.iter().copied());
    // The pointer locals of `call_pointer_locals` that could stay in A0
    // itself: nothing writes the physical A0
    // from the copy `movea.l A0,An` to the copy's last use.
    let a0_free: Vec<i32> = s
        .ptr_defs
        .iter()
        .filter(|(o, _, _)| call_ptrs.contains(o))
        .filter(|&&(_, k, _)| {
            let Some(Insn::Movea { dst: v, .. }) = s.out.get(k) else {
                return false;
            };
            let v = Reg::A(*v);
            let Some(last) = s
                .out
                .iter()
                .rposition(|i| crate::regs::access(i).uses.contains(&v))
            else {
                return false;
            };
            !s.out[k + 1..=last].iter().any(|i| {
                matches!(i, Insn::Jsr(_) | Insn::Bsr { .. })
                    || crate::regs::access(i).defs.contains(&Reg::A(0))
            })
        })
        .map(|&(o, _, _)| o)
        .collect();
    let pre = s.out.clone();
    let mut saves = allocate_apart(&mut s.out, exit, &holds, &apart, first.clone(), &after)
        .map_err(|what| unsupported(f.span, what))?;
    // A slot loaded late into a register that holds a dead slot load on
    // every path passes over it ([`pass_dead_slot_loads`], 12.4 R4(b));
    // the body is coloured again with the slot kept apart from those loads.
    let mut passed: Vec<usize> = Vec::new();
    loop {
        let more = pass_dead_slot_loads(&pre, &s.out, &late, &apart);
        if more.is_empty() {
            break;
        }
        for (k, pair) in more {
            if !passed.contains(&k) {
                passed.push(k);
            }
            apart.push(pair);
        }
        s.out = pre.clone();
        saves = allocate_apart(&mut s.out, exit, &holds, &apart, first.clone(), &after).map_err(
            |_| {
                late_slot_refusal(
                    late.iter()
                        .find(|l| passed.contains(&l.0))
                        .map_or(f.span, |l| l.2),
                )
            },
        )?;
    }
    check_assigned_flags(&s.out, &s.flag_moves)?;
    if let Some(range) = hoisted {
        // A parameter loaded at entry into a scratch address register
        // after the block's load: the one such site has the parameter in
        // the lower register, `movea.l #0xE248FC,A1; movea.l (8,A6),A0`
        // (NETWORK_$READ_SERVICE 0xE71D80), against the lowest-free
        // allocation in definition order that gives the block A0 here;
        // the sites that agree hold the parameter or static link in A2
        // (KBD_$INIT 0xE3336A, the RINGLOG_ module-local 0xE721D4).  Not
        // settled (codegen-patterns 11, "Placement at entry").
        if s.out[range].iter().any(|i| {
            crate::regs::access(i)
                .defs
                .iter()
                .any(|r| matches!(r, Reg::A(0) | Reg::A(1)))
        }) {
            return Err(unsupported(
                f.span,
                "a parameter loaded at entry into A0 or A1 after another data section's block \
                 (which of the two takes the lower register is not settled, codegen-patterns 11) \
                 is",
            ));
        }
    }
    let dropped = reuse_pointer_copies(&mut s.out, &s.ptr_copies);
    if !dropped.is_empty() {
        // A register only a dropped copy took is not saved.
        let mut used = RegSet::of(&[Reg::A(5)]);
        for i in &s.out {
            let a = crate::regs::access(i);
            for r in a.uses.iter().chain(a.defs.iter()) {
                used.insert(*r);
            }
        }
        saves = saves.intersection(used);
    }
    for &(k, span) in &s.copy_blocks {
        // A block copy of a value parameter is seen from A0 into A1 only
        // (`tools/framesurvey.py`-style search of `movea.l (d,A6),An; lea
        // (k,A6),Am; move (An)+,(Am)+`: 31 slot copies A0 to A1,
        // XPD_$SET_PTRACE_OPTS 0xE5AFB0, OS_$INIT 0xE33808; the three
        // others in nested routines or after a branch).  Where an earlier
        // copy or an entry load holds A0 or A1, which registers the
        // original takes is not known (codegen-patterns 1.2).
        let from_a0 = matches!(s.out.get(k), Some(Insn::Movea { dst: 0, .. }));
        let into_a1 = matches!(s.out.get(k + 1), Some(Insn::Lea { dst: 1, .. }));
        if !(from_a0 && into_a1) {
            return Err(unsupported(
                span,
                "a block copy of a value parameter passed by address while an earlier copy or \
                 an entry load holds A0 or A1 (only `A0` to `A1` is seen, codegen-patterns 1.2) \
                 is",
            ));
        }
    }
    let mut to_a0 = Vec::new();
    for &(o, k, span) in &s.ptr_defs {
        // A pointer local assigned from calls that no call follows, in A0
        // or A1 by allocation: kept in A0 itself, the copy dropped
        // (TTY_$I_INQ_RAW 0xE67414 `move.b (1241,A0),(A1)`, the out
        // parameter's slot in A1; TTY_$K_INQ_FLAGS 0xE674BA; beads
        // domain-pascal-yzxr, -78n2).  The image's D0 form of such locals
        // (`move.l A0,D0` after the call, `movea.l D0,A0` where it is
        // dereferenced: TTY_$K_INQ_FUNC_ENABLED 0xE6763C / 0xE67642) is
        // that of a 4-byte integer local ([`Park`]); a pointer local is
        // no longer held so (codegen-patterns 3, "Pointer locals held in
        // D0").  The copies to A1 where D0 is busy (0xE4D7FC, 0xE528CE)
        // are not modelled.  One in a callee-saved register (live across
        // a call, or held through one) is kept.
        if let Insn::Movea { dst, .. } = s.out[k] {
            if call_ptrs.contains(&o) && dst < 2 {
                if !parkable_pointer_local(f, o) || !a0_free.contains(&o) {
                    return Err(unsupported(
                        span,
                        "a pointer local assigned from a call that no call follows, assigned \
                         more than once, inside a statement, read other than dereferenced, or \
                         with A0 written before its last use (A0 or A1 in the image; not \
                         settled) is",
                    ));
                }
                if !to_a0.contains(&o) {
                    to_a0.push(o);
                }
            }
        }
    }
    if !to_a0.is_empty() {
        return Ok(Selected::InA0(to_a0));
    }
    for p in &s.parked {
        // The image's D0 sites all have D0 and A0; where D0 is busy it
        // has A1 instead (0xE4D7FC: D0 and D1 hold other values over the
        // local's range), which is not modelled.
        let (Some((_, d_at)), Some((_, a_at))) = (p.d, p.home) else {
            return Err(unsupported(
                f.span,
                "a local assigned an `a0_return` result, held in D0 and never dereferenced, is",
            ));
        };
        let in_d0 = matches!(
            &s.out[d_at],
            Insn::Move {
                dst: Ea::Reg(Reg::D(0)),
                ..
            }
        );
        let in_a0 = matches!(
            &s.out[a_at],
            Insn::Movea {
                src: Ea::Reg(Reg::D(0)),
                dst: 0,
                ..
            }
        );
        let callee_saved = matches!(
            &s.out[a_at],
            Insn::Movea {
                src: Ea::Reg(Reg::D(0)),
                dst: 2..,
                ..
            }
        );
        if !(in_d0 && (in_a0 || callee_saved)) {
            return Err(unsupported(
                f.span,
                "a local assigned an `a0_return` result held in D0, where D0 or A0 is busy \
                 (A1 in the image, 0xE4D7FC; not settled) is",
            ));
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
    // After the checks above, whose refusals are the more basic ones (a
    // pointer local in A1 writes A1 before a slot load, TTY_$K_*).
    check_late_slot_loads(&s.out, &late, &passed, exit, &holds)?;
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
                && !dropped.contains(&k)
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
    let mut volatile_insns: Vec<usize> = volatile_insns.iter().map(|&k| new_index[k]).collect();
    let mut out: Vec<Insn> = s
        .out
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(i, _)| i)
        .collect();
    // A parked local's `movea.l D0,An` selected before an `if` whose two
    // parts dereference it moves down to the condition's branch (bead
    // domain-pascal-dgb; TTY_$K_SET_INPUT_FLAG 0xE67684..0xE6768C
    // `movea.l (0x10,A6),A1; movea.l D0,A0; tst.b (A1); bpl.b`, and
    // TTY_$K_SET_OUTPUT_FLAG, TTY_$K_SET_ECHO_FLAG; TTY_$K_ENABLE_FUNC
    // 0xE675DC with A2).  It is selected first so that the allocator
    // keeps the condition's registers off An (A1 above), and then sits
    // just before the branch, where the scheduler lists it after the
    // condition's first load (12.4 R11).  Only the condition's own
    // instructions lie between, and none of them may touch D0 or An.
    for &(m, b, span) in &s.hoists {
        let (m, b) = (new_index[m], new_index[b]);
        let Insn::Movea {
            src: Ea::Reg(src),
            dst,
            ..
        } = out[m].clone()
        else {
            unreachable!("a hoist is a movea.l")
        };
        let touched = out[m + 1..b].iter().any(|i| {
            let a = crate::regs::access(i);
            matches!(
                i,
                Insn::Label(_) | Insn::Branch { .. } | Insn::Jsr(_) | Insn::Bsr { .. }
            ) || a.defs.contains(&src)
                || a.defs.contains(&Reg::A(dst))
                || a.uses.contains(&Reg::A(dst))
        });
        let split = units.iter().any(|&(u0, u1)| u0 < b && m < u1);
        if touched || split || !matches!(out[b], Insn::Branch { .. }) {
            return Err(unsupported(
                span,
                "an `if` whose condition uses the register of a pointer local held in D0 is",
            ));
        }
        let mv = out.remove(m);
        out.insert(b - 1, mv);
        for k in volatile_insns.iter_mut() {
            if *k == m {
                *k = b - 1;
            } else if m < *k && *k < b {
                *k -= 1;
            }
        }
    }
    // Phase 1c: the order inside basic blocks (`sched.rs`).
    crate::sched::schedule(&mut out, &units, &volatile_insns, sched);
    Ok(Selected::Body(
        Body {
            insns: out,
            saves: saves.union(s.clobbered),
        },
        s.cse_hits,
    ))
}

/// Branch threading (codegen-patterns 7.1, "Branch threading"; bead
/// domain-pascal-o6p).  Compiled code has no branch to a `bra`
/// (`tools/threadsurvey.py tobra`: none of the `b<cc>`, `bra`, `dbf` or
/// `case` table entries of 0xE00800..0xE88834), and 845 forward branches
/// go past a `bra` to that `bra`'s own target, the outer join or the
/// epilogue (`thread`: TTY_$K_SET_FLAG 0xE6746A `beq.b 0xE67482` past the
/// `bra.b 0xE67482` at 0xE6747A).  So a branch to a label that a `bra`
/// follows takes the `bra`'s target, transitively.  A `bra` that nothing
/// reaches after that (the instruction before it an unconditional `bra`)
/// is dropped: no such `bra` is in the image (`dead`: none), and a
/// `while` whose opening `bra` follows one is entered by the threaded
/// branch to its test alone (TTY_$K_GET 0xE1C44A `bpl.w 0xE1C4FC`, the
/// head 0xE1C4D6 right after the `then` part's `bra.s` at 0xE1C4D4;
/// TTY_$K_PUT 0xE67C0E), its head still aligned in aligning code (the module-local
/// 0xE4AFA8: `bne.b 0xE4B0FC` to the test, `bra.s` at 0xE4B0F4, the
/// filler at 0xE4B0F6, the head 0xE4B0F8).  The sizes follow from the
/// threaded layout (`size_branches`): 263 of the threaded branches are
/// `.w` though a `.s` would have reached the `bra`.
///
/// At the bottom of a loop with no test (codegen-patterns 7.2, "Loops
/// with no test"; bead domain-pascal-pkle), a conditional branch over the
/// `bra` back to the head becomes the negated branch to the head, and a
/// `bra` that the dropping of that `bra` leaves going to the next
/// instruction is dropped too (compiled code has neither).
///
/// The dropped `bra` becomes a fresh label that nothing names, so the
/// indices into `out` stay valid; the unused labels go later.  Refused:
/// a `dbf` or a `case` table entry whose target a `bra` follows (an arm
/// of only `return`; no table entry in the image goes to the epilogue
/// unless its default does; bead domain-pascal-r4yn), a cycle of `bra`s,
/// any other branch that threading leaves pointing at the next
/// instruction (not seen), and a loop head that falls into a `bra` (a `repeat` before a
/// `while`; bead domain-pascal-tfae).  An `exit`, `next` or `goto` whose
/// `bra` is dropped leaves its entry in `loop_jumps` / `goto_jumps`
/// pointing at a label; [`select_body_with`] skips those.
fn thread_branches(
    out: &mut [Insn],
    next_label: &mut u32,
    before_bra: &[(u32, Span)],
    span: Span,
) -> Res<()> {
    use std::collections::HashMap;
    let span_of = |l: u32| {
        before_bra
            .iter()
            .find(|(m, _)| *m == l)
            .map_or(span, |&(_, s)| s)
    };
    let refused = |l: u32, what: &str| {
        unsupported(
            span_of(l),
            format!(
                "{what} whose target a `bra` follows (compiled code has no branch to a `bra`; \
                 only branches are seen threaded, codegen-patterns 7.1) is"
            ),
        )
    };
    // The `bra` right after each label (other labels between).
    let mut succ: HashMap<u32, u32> = HashMap::new();
    let mut pos: HashMap<u32, usize> = HashMap::new();
    for (k, i) in out.iter().enumerate() {
        if let Insn::Label(l) = i {
            pos.insert(*l, k);
            let next = out[k + 1..].iter().find(|i| !matches!(i, Insn::Label(_)));
            if let Some(Insn::Branch {
                cond: Cond::T,
                label,
                ..
            }) = next
            {
                succ.insert(*l, *label);
            }
        }
    }
    let resolve = |l: u32| -> Option<u32> {
        let mut t = l;
        for _ in 0..=succ.len() {
            match succ.get(&t) {
                Some(&m) => t = m,
                None => return Some(t),
            }
        }
        None
    };
    for (k, i) in out.iter_mut().enumerate() {
        match i {
            Insn::Branch { label, back, .. } if succ.contains_key(label) => {
                let Some(t) = resolve(*label) else {
                    return Err(refused(*label, "a cycle of `bra`s (a branch)"));
                };
                *label = t;
                *back = pos[&t] < k;
            }
            Insn::Dbf { label, .. } if succ.contains_key(label) => {
                return Err(refused(*label, "a `dbf`"));
            }
            Insn::TableEntry { target, .. } if succ.contains_key(target) => {
                return Err(refused(*target, "a `case` table entry"));
            }
            _ => {}
        }
    }
    // The labels still named after threading.
    let used: std::collections::HashSet<u32> = out
        .iter()
        .flat_map(|i| match i {
            Insn::Branch { label, .. } | Insn::Dbf { label, .. } => vec![*label],
            Insn::TableIndex { table, .. } => vec![*table],
            Insn::TableEntry { target, .. } => vec![*target],
            _ => vec![],
        })
        .collect();
    // A `bra` after an unconditional `bra`, with no named label between
    // (a loop head's alignment may be: the head of a `while` whose body
    // starts with a `while`, its back edge threaded to the inner test).
    let mut dead = Vec::new();
    for k in 0..out.len() {
        if !matches!(out[k], Insn::Branch { cond: Cond::T, .. }) {
            continue;
        }
        let prev = out[..k].iter().rev().find(|i| {
            !matches!(i, Insn::Label(l) if !used.contains(l)) && !matches!(i, Insn::Align4)
        });
        if matches!(prev, Some(Insn::Branch { cond: Cond::T, .. })) {
            dead.push(k);
        }
    }
    // The dropped `bra`s that went back to a loop head: the bottom of a
    // loop with no test (`repeat ... until false`) after a body that ends
    // in a jump.
    let dead_back: Vec<usize> = dead
        .iter()
        .copied()
        .filter(|&k| matches!(out[k], Insn::Branch { back: true, .. }))
        .collect();
    for &k in &dead {
        out[k] = Insn::Label(*next_label);
        *next_label += 1;
    }
    // A conditional branch over a `bra` back to a loop head (the bottom of
    // a loop with no test, after an `if c then exit` that ends its body)
    // is the branch on the negated condition to the head, with no `bra`:
    // compiled code has no conditional branch over a `bra` (codegen-
    // patterns 7.1; `tools/threadsurvey.py over`: none), and PKT_$SAR_
    // INTERNET's wait 0xE72152 `cmp.w D4w,D2w; bne.w 0xE71FE8` falls into
    // the code after the loop (codegen-patterns 7.2).
    for k in 0..out.len() {
        let Insn::Branch {
            cond,
            label: over,
            short,
            ..
        } = out[k]
        else {
            continue;
        };
        if cond == Cond::T {
            continue;
        }
        let Some(j) =
            (k + 1..out.len()).find(|&j| !matches!(out[j], Insn::Label(l) if !used.contains(&l)))
        else {
            continue;
        };
        let Insn::Branch {
            cond: Cond::T,
            label: head,
            back: true,
            ..
        } = out[j]
        else {
            continue;
        };
        let lands = out[j + 1..]
            .iter()
            .take_while(|i| matches!(i, Insn::Label(_) | Insn::Align4))
            .any(|i| matches!(i, Insn::Label(l) if *l == over));
        if !lands {
            continue;
        }
        out[k] = Insn::Branch {
            cond: cond.negate(),
            label: head,
            short,
            back: true,
        };
        out[j] = Insn::Label(*next_label);
        *next_label += 1;
    }
    // The dropped `bra` of a `while` that starts a `while`'s body leaves
    // the two heads' alignments together: one is enough (the outer head
    // is no longer a branch target).
    for k in 0..out.len() {
        if matches!(out[k], Insn::Align4)
            && matches!(
                out[k + 1..]
                    .iter()
                    .find(|i| !matches!(i, Insn::Label(l) if !used.contains(l))),
                Some(Insn::Align4)
            )
        {
            out[k] = Insn::Label(*next_label);
            *next_label += 1;
        }
    }
    // A branch now to the next instruction (the labels and a loop head's
    // alignment between): not seen.
    for k in 0..out.len() {
        let Insn::Branch { cond, label, .. } = out[k] else {
            continue;
        };
        let to = out[k + 1..]
            .iter()
            .take_while(|i| matches!(i, Insn::Label(_) | Insn::Align4))
            .position(|i| matches!(i, Insn::Label(l) if *l == label))
            .map(|n| k + 1 + n);
        // A `bra` to the next instruction once the `bra` back to the head
        // of a loop with no test is dropped: an `exit` or `return` ending
        // the body (MSG_$SARI's wait, whose last `case` arm ends in `exit`:
        // 0xE59F76..0xE59F98, then the code after the loop at 0xE59F9E,
        // with no `bra` between).  Compiled code has no branch to the next
        // instruction (codegen-patterns 7.2; `tools/threadsurvey.py next`:
        // none).
        if let Some(to) = to {
            if cond == Cond::T && dead_back.iter().any(|&d| k < d && d < to) {
                out[k] = Insn::Label(*next_label);
                *next_label += 1;
                continue;
            }
        }
        if to.is_some() && !dead.is_empty() {
            return Err(unsupported(
                span,
                "a branch that would go to the next instruction once a `bra` no longer reached \
                 is dropped (not seen, codegen-patterns 7.1) is",
            ));
        }
    }
    // A loop head that a `bra` still follows (a `repeat`, or a backward
    // `goto`'s label, before a `while`: the back edge goes to the inner
    // test, and the head, reached by falling into it, is a branch target
    // no longer).  Compiled code has no 0x2048 filler before a `bra` that
    // falls through to it, and a head entered so is not seen in aligning
    // code: whether the original still aligns it is not known (bead
    // domain-pascal-tfae).
    for k in 0..out.len() {
        if !matches!(out[k], Insn::Align4) {
            continue;
        }
        let rest = || {
            out[k + 1..]
                .iter()
                .skip_while(|i| matches!(i, Insn::Label(_) | Insn::Align4))
        };
        let head = out[k + 1..].iter().find_map(|i| match i {
            Insn::Label(l) => Some(*l),
            _ => None,
        });
        let next = rest().next();
        if matches!(next, Some(Insn::Branch { cond: Cond::T, .. })) {
            return Err(unsupported(
                head.map_or(span, span_of),
                "a `repeat` or a backward `goto`'s label right before a `while` (the back edge \
                 goes to the `while`'s test; whether the head, falling into the opening `bra`, \
                 stays aligned is not seen, codegen-patterns 7.1) is",
            ));
        }
    }
    Ok(())
}

/// The memory operands instruction `i` reads or writes (not those whose
/// address only it takes: `pea`, `lea`, `jsr`).
fn memory_operands(i: &Insn) -> Vec<&Ea> {
    let v: Vec<&Ea> = match i {
        Insn::Move { src, dst, .. } => vec![src, dst],
        Insn::Movea { src, .. }
        | Insn::Cmp { src, .. }
        | Insn::Alu { src, .. }
        | Insn::Mul { src, .. }
        | Insn::Div { src, .. } => vec![src],
        Insn::Addq { dst, .. }
        | Insn::Subq { dst, .. }
        | Insn::Clr { dst, .. }
        | Insn::Btst { dst, .. }
        | Insn::Scc { dst, .. }
        | Insn::Tst { dst, .. }
        | Insn::Cmpi { dst, .. }
        | Insn::AluImm { dst, .. }
        | Insn::Unary { dst, .. }
        | Insn::BtstImm { dst, .. }
        | Insn::BitImm { dst, .. }
        | Insn::Bit { dst, .. }
        | Insn::AluToMem { dst, .. } => vec![dst],
        _ => vec![],
    };
    v.into_iter()
        .filter(|e| !matches!(e, Ea::Reg(_) | Ea::Imm(_) | Ea::ImmSym(_)))
        .collect()
}

/// The memory operand instruction `i` reads before it writes it: the
/// read-modify-write instructions, and `clr` and `s<cc>`, which on the
/// MC68000/68010 read their operand before writing it (MC68000 PRM).
fn read_modify_write_operand(i: &Insn) -> Option<&Ea> {
    let dst = match i {
        Insn::Addq { dst, .. }
        | Insn::Subq { dst, .. }
        | Insn::Clr { dst, .. }
        | Insn::Scc { dst, .. }
        | Insn::AluImm { dst, .. }
        | Insn::Unary { dst, .. }
        | Insn::BitImm { dst, .. }
        | Insn::Bit { dst, .. }
        | Insn::AluToMem { dst, .. } => dst,
        _ => return None,
    };
    (!matches!(dst, Ea::Reg(_))).then_some(dst)
}

/// The indices in `out` of the instructions that reference a volatile
/// object (`eas`, the operands [`Selector::ea_of`] gave for
/// [`Addr::Volatile`] addresses; virtual registers name one value each,
/// so an operand names one object).  A read-modify-write instruction on
/// a device object is refused (ref 3.15.3 p. 3-57, "the compiler [avoids]
/// using instructions that do unnecessary reads"; codegen-patterns 13):
/// the selector does not choose one for a device ([`Selector::store`]), so
/// one here comes from a form it has no device variant of.
fn volatile_references(out: &[Insn], eas: &[(Ea, Access)], span: Span) -> Res<Vec<usize>> {
    let mut v = Vec::new();
    for (k, i) in out.iter().enumerate() {
        if let Some(d) = read_modify_write_operand(i) {
            if eas.iter().any(|(e, a)| a.device && e == d) {
                return Err(unsupported(
                    span,
                    "a read-modify-write instruction on a device object (ref 3.15.3) is",
                ));
            }
        }
        if memory_operands(i)
            .iter()
            .any(|m| eas.iter().any(|(e, _)| e == *m))
        {
            v.push(k);
        }
    }
    Ok(v)
}

/// The values to hold live through a call (codegen-patterns 12.4 R4(c)),
/// as `(index of the jsr/bsr, register)` for [`allocate`].
///
/// A value defined before a call statement whose last use is in that
/// statement's arguments:
///
/// - in a body without branches it is held through the call, so it takes
///   a callee-saved register: PROC1_$SET_ASID 0xE148FE D2 (`move.w
///   D2,-(SP)` at 0xE1490E); TTY_$I_FLUSH_OUTPUT 0xE1B80C A2,
///   TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39A A2, module-local 0xE1D592 at
///   0xE1D598 A2, SIO_$I_INIT 0xE67E64 A2.  These are
///   all five such parameter loads in the image's routines without
///   branches where liveness alone leaves a scratch register free
///   (`tools/regalloc.py callargs`, `callparams`);
/// - in a body with branches, a parameter loaded at entry (`params`) is
///   held too, wherever the call lies (bead domain-pascal-9j1z;
///   `tools/regalloc.py callparams`, the parameters loaded once before
///   the routine's first branch, branch target or call, whose last use on
///   some path pushes an argument of a call, with a scratch register free
///   by liveness): before the first branch, 2 of 2 callee-saved
///   (VOLX_$GET_INFO 0xE6B5D4 A3, 0xE6B5D8 A4); in a conditional part, 26
///   of 27 (AREA_$ASSOC 0xE096B0 D2, `move.w D2,-(SP)` at 0xE096D2;
///   XPD_$GET_FP_INT 0xE5C560 A2, a call in each part); after a join
///   inside one, 5 of 5 (SOCK_$PUT_INT 0xE161A2 A2, `pea (A2)` at
///   0xE161E4; the nested 0xE1CC10 of KBD_$RCV keeps `kbd` in A2,
///   `move.l (0x48,A2),-(SP)` at 0xE1CC52); whether the
///   value is also used on another path (10 of 10) or passed to several
///   calls (7 of 7) does not matter, nor does the class (A 29 of 29, D 4
///   of 5).  Refused:
///   - the one other site's kind, a pointer parameter held in a data
///     register (`data_ptrs`): EC2_$ADVANCE 0xE42CB8 pushes D0, a scratch
///     register, `move.l D0,-(SP)` at 0xE42CDA, the only such site;
///   - a call on every path after a join: its 4 sites are all
///     callee-saved, but at each no scratch register is free by
///     liveness, so they do not show whether the value is held;
/// - other values (register-held locals, computed values) are refused in
///   a body with branches: not surveyed, and DTTY_$TSTART 0xE1D6E0 keeps
///   the local `current` in D0, a scratch register, for `pea
///   (5,A3,D0.w)` at 0xE1D6F8 inside an `if`.
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
///
/// `blocks`: the registers holding another data section's block
/// ([`Selector::other_base`]): never held, loaded again after the call.
fn held_through_calls(
    out: &[Insn],
    calls: &[(usize, usize, Span)],
    exit: &[Reg],
    kinds: HeldKinds,
) -> Res<Vec<(usize, Reg)>> {
    let HeldKinds {
        link,
        ptrs,
        blocks,
        params,
        data_ptrs,
    } = kinds;
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
                    || blocks.contains(&r)
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
                    if !params.contains(&r) {
                        return Err(unsupported(
                            span,
                            "in a body with branches, a call whose arguments are the last use of \
                             a register-held local or computed value (held through the call or \
                             not: DTTY_$TSTART 0xE1D6F8 does not hold one; not surveyed, \
                             codegen-patterns 12.4 R4(c), bead domain-pascal-ig8d) is",
                        ));
                    }
                    if data_ptrs.contains(&r) {
                        return Err(unsupported(
                            span,
                            "in a body with branches, a call whose arguments are the last use of \
                             a pointer parameter held in a data register (not held at \
                             EC2_$ADVANCE 0xE42CDA, the only site; codegen-patterns 12.4 R4(c), \
                             bead domain-pascal-b3wz) is",
                        ));
                    }
                    if after_a_branch(out, call) && on_every_path(out, call) {
                        return Err(unsupported(
                            span,
                            "a call on every path after a join whose arguments are the last use \
                             of a parameter loaded at entry (held through the call or not: no \
                             site with a scratch register free, codegen-patterns 12.4 R4(c), \
                             bead domain-pascal-cs2n) is",
                        ));
                    }
                }
                holds.push((call, r));
            }
        }
    }
    Ok(holds)
}

/// The kinds of values [`held_through_calls`] treats apart.
struct HeldKinds<'a> {
    /// The static link's register.
    link: Option<Reg>,
    /// Pointer locals assigned from calls.
    ptrs: &'a [Reg],
    /// Registers holding another data section's block.
    blocks: &'a [Reg],
    /// The parameters loaded at entry ([`Selector::entry_loads`]).
    params: &'a [Reg],
    /// Those of `params` held in a data register although pointers.
    data_ptrs: &'a [Reg],
}

/// A branch, `dbf`, `case` dispatch or branch target comes before
/// instruction `i` of `out`.
fn after_a_branch(out: &[Insn], i: usize) -> bool {
    out[..i].iter().any(|x| {
        matches!(
            x,
            Insn::Branch { .. } | Insn::Dbf { .. } | Insn::TableJump { .. } | Insn::Label(_)
        )
    })
}

/// Every path from the start of the body to its end goes through
/// instruction `i` of `out`.
fn on_every_path(out: &[Insn], i: usize) -> bool {
    if out.is_empty() {
        return true;
    }
    let succ = crate::regs::successors(out);
    let mut seen = vec![false; out.len()];
    let mut work = vec![0usize];
    while let Some(j) = work.pop() {
        if j == i || seen[j] {
            continue;
        }
        seen[j] = true;
        if matches!(out[j], Insn::Rts) {
            return false;
        }
        for s in &succ[j] {
            match s {
                Some(k) => work.push(*k),
                None => return false,
            }
        }
    }
    true
}

/// The register pairs to keep apart for by-address slots loaded for one
/// use in a body of several statements (codegen-patterns 12.4 R4(b)), as
/// `(slot value, earlier value)` for [`allocate_apart`], and the loads that
/// come after the body's first branch, call or label, which
/// [`check_late_slot_loads`] checks after allocation (as `(index of the
/// load in out, statement)`).
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
/// statement), neither once (0xE3353A).  Before the first branch or call
/// of a routine that has some, the rule holds as without them (bead
/// domain-pascal-5aw.12; ad-hoc script over `tools/nestsurvey.py`'s
/// routines, not kept: of the slot loads in that straight-line prefix
/// whose register is dead after it, 496 of 553 fit this rule, the others
/// mostly argument lists, R4(a), or values held through a call, R4(c);
/// the value-parameter copies there, 133 of 135,
/// [`Selector::param_copies`]); DIR_$GET_ENTRYU 0xE4D51E `movea.l
/// (0x10,A6),A0` in its `if` condition, after the copy's `(A0)+`.  After
/// it, see [`check_late_slot_loads`].
fn fresh_slot_loads(
    out: &[Insn],
    fresh: &[(Reg, usize, Span)],
) -> (Vec<(Reg, Reg)>, LateSlotLoads) {
    if fresh.is_empty() {
        return (Vec::new(), Vec::new());
    }
    // The first instruction after which the straight-line rule is not
    // known: a branch, a call, or a label (a loop head, a join).
    let first_flow = out
        .iter()
        .position(|i| {
            matches!(
                i,
                Insn::Branch { .. }
                    | Insn::Dbf { .. }
                    | Insn::Jsr(_)
                    | Insn::Bsr { .. }
                    | Insn::Label(_)
                    | Insn::TableIndex { .. }
            )
        })
        .unwrap_or(out.len());
    // Registers used up by `(An)+` / `-(An)`.
    let consumed: Vec<u8> = out.iter().flat_map(autoinc_regs).collect();
    let mut apart = Vec::new();
    let mut late = Vec::new();
    for &(v, start, span) in fresh {
        let load = out
            .iter()
            .position(|i| crate::regs::access(i).defs.contains(&v))
            .unwrap_or(out.len());
        if load > first_flow {
            // After the first branch or call: kept apart only from the
            // address values of earlier statements since a call that
            // reaches the load in straight-line code (the rule above,
            // restarted at the call; PROC2_$PGUID_TO_UPGID 0xE410B8
            // `movea.l (0x10,A6),A1` after `(0xc,A6)` in A0 and ML_$UNLOCK).
            let call = out[..load]
                .iter()
                .rposition(|i| {
                    matches!(
                        i,
                        Insn::Jsr(_)
                            | Insn::Bsr { .. }
                            | Insn::Label(_)
                            | Insn::Branch { .. }
                            | Insn::Dbf { .. }
                            | Insn::TableIndex { .. }
                            | Insn::TableJump { .. }
                    )
                })
                .filter(|&c| c < start && matches!(out[c], Insn::Jsr(_) | Insn::Bsr { .. }));
            if let Some(c) = call {
                for i in &out[c + 1..start] {
                    for d in crate::regs::access(i).defs {
                        if let Reg::A(n) = d {
                            if crate::regs::is_virtual(d) && !consumed.contains(&n) {
                                apart.push((v, d));
                            }
                        }
                    }
                }
            }
            late.push((load, call, span));
            continue;
        }
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
    (apart, late)
}

/// The address registers an instruction steps with `(An)+` / `-(An)`
/// (the stack pointer excepted).
fn autoinc_regs(insn: &Insn) -> Vec<u8> {
    let eas: Vec<&Ea> = match insn {
        Insn::Move { src, dst, .. } => vec![src, dst],
        Insn::Clr { dst, .. } | Insn::Tst { dst, .. } => vec![dst],
        Insn::Cmp { src, .. } | Insn::Alu { src, .. } => vec![src],
        _ => Vec::new(),
    };
    eas.into_iter()
        .filter_map(|ea| match ea {
            Ea::PostInc(a) | Ea::PreDec(a) if *a != SP => Some(*a),
            _ => None,
        })
        .collect()
}

/// The slot loads [`check_late_slot_loads`] checks: the index of each
/// load in the body, the call after which it follows in straight-line
/// code, if any, and its statement.
type LateSlotLoads = Vec<(usize, Option<usize>, Span)>;

/// The refusal of a slot load after the first branch or call whose
/// register the original's choice is not known for (12.4 R4(b)).
fn late_slot_refusal(span: Span) -> Diagnostic {
    unsupported(
        span,
        "in a body of several statements, a pointer parameter or by-address parameter used \
         once after a branch or a call, whose lowest free register an earlier statement left a \
         value in on some path (the original's choice there is not known, codegen-patterns \
         12.4 R4(b)), is",
    )
}

/// What reaches an address register at an instruction, on some path
/// ([`reaching_aregs`], [`reach_kinds`]): nothing wrote it on that path.
const REACH_ENTRY: u8 = 1;
/// A call wrote it.
const REACH_CALL: u8 = 2;
/// A block copy used it up (`(An)+` / `-(An)`).
const REACH_USED_UP: u8 = 4;
/// A parameter slot load `movea.l (d,A6),An`, d > 0.
const REACH_SLOT: u8 = 8;
/// Anything else.
const REACH_OTHER: u8 = 16;

/// Per instruction of an allocated body, before it: for A0..A4 the
/// instructions whose definition of the register reaches it on some path
/// (`usize::MAX` for the body's entry); `None` for unreachable code.
fn reaching_aregs(out: &[Insn]) -> Vec<Option<[BTreeSet<usize>; 5]>> {
    let succ = crate::regs::successors(out);
    let mut inn: Vec<Option<[BTreeSet<usize>; 5]>> = vec![None; out.len()];
    if !out.is_empty() {
        inn[0] = Some(std::array::from_fn(|_| BTreeSet::from([usize::MAX])));
    }
    loop {
        let mut changed = false;
        for i in 0..out.len() {
            let Some(mut st) = inn[i].clone() else {
                continue;
            };
            for d in crate::regs::access(&out[i]).defs {
                if let Reg::A(n) = d {
                    if n < 5 {
                        st[n as usize] = BTreeSet::from([i]);
                    }
                }
            }
            for s in succ[i].iter().flatten() {
                let merged = match &inn[*s] {
                    None => st.clone(),
                    Some(t) => std::array::from_fn(|n| t[n].union(&st[n]).copied().collect()),
                };
                if inn[*s].as_ref() != Some(&merged) {
                    inn[*s] = Some(merged);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    inn
}

/// Whether the late load at `k` may pass over dead slot loads
/// ([`pass_dead_slot_loads`]): a slot load whose slot no other
/// instruction of the body names, the shape the rule is verified on
/// (`tools/regalloc.py slotpass`, 41 of 41), or a copy `movea.l Dn,An`
/// ([`Selector::copy_to_a`]; review of bead domain-pascal-i24w, ad-hoc
/// script over the whole image: 9 copies after the first branch, call
/// or label in the rule's shape, 9 hits, 0xE09A94 A1 past A0, 0xE769AA
/// A3 past A0-A2).  A slot named elsewhere too (a parameter read from
/// the frame at each use, an escaped one in `dpc`) goes the other way
/// (same script, 35 loads of the rule's shape): past a dead load of the
/// same slot the image reuses its register 8 times of 8 (the AREA_
/// module-local 0xE08816 at 0xE0887C, 0xE08894, ... A0; VTOCE_$TRUNCATE
/// 0xE39E78, 0xE39E86 A0; IO_$ACQUIRE 0xE1A6AC A1), past dead loads of
/// other slots it takes the rule's register 22 times and reuses the
/// lowest 5 (VTOCE_$TRUNCATE 0xE39E90, XPD_$PUT_REGISTERS 0xE5C3B6,
/// FILE_$READ_LOCK_ENTRYI 0xE60BC0 A0); such a load is not passed over
/// and stays refused.
fn passes_dead_slot_loads(out: &[Insn], k: usize) -> bool {
    match out.get(k) {
        Some(Insn::Movea {
            src: Ea::Reg(Reg::D(_)),
            ..
        }) => true,
        Some(Insn::Movea {
            src: src @ Ea::Disp(d, FP),
            ..
        }) if *d > 0 => {
            let slot = src.to_string();
            !out.iter()
                .enumerate()
                .any(|(i, x)| i != k && x.to_string().contains(&slot))
        }
        _ => false,
    }
}

/// The `REACH_*` kinds of the definitions `defs` of An ([`reaching_aregs`]).
fn reach_kinds(out: &[Insn], n: u8, defs: &BTreeSet<usize>) -> u8 {
    defs.iter().fold(0, |m, &j| {
        m | match out.get(j) {
            None => REACH_ENTRY,
            Some(Insn::Jsr(_) | Insn::Bsr { .. }) => REACH_CALL,
            Some(i) if autoinc_regs(i).contains(&n) => REACH_USED_UP,
            Some(Insn::Movea {
                src: Ea::Disp(d, FP),
                ..
            }) if *d > 0 => REACH_SLOT,
            Some(_) => REACH_OTHER,
        }
    })
}

/// After allocation, for each slot load of [`check_late_slot_loads`]
/// whose register holds a dead parameter slot load on every path to it
/// (only `REACH_SLOT` reaches it): `(index of the load, (its value, the
/// value of each such reaching load))`, from `pre`, the body before
/// allocation, to keep apart in a new colouring (bead
/// domain-pascal-i24w).  Such a register is passed over in the image when
/// a free register above it is untouched or used up
/// ([`check_late_slot_loads`]); the pairs already in `apart` are left out.
fn pass_dead_slot_loads(
    pre: &[Insn],
    out: &[Insn],
    late: &[(usize, Option<usize>, Span)],
    apart: &[(Reg, Reg)],
) -> Vec<(usize, (Reg, Reg))> {
    if late.is_empty() {
        return Vec::new();
    }
    let reach = reaching_aregs(out);
    let mut more = Vec::new();
    for &(k, _, _) in late {
        let (Some(Insn::Movea { dst, .. }), Some(Insn::Movea { dst: v, .. })) =
            (out.get(k), pre.get(k))
        else {
            continue;
        };
        if !passes_dead_slot_loads(out, k) {
            continue;
        }
        let Some(Some(st)) = reach.get(k) else {
            continue;
        };
        let defs = &st[*dst as usize];
        if reach_kinds(out, *dst, defs) != REACH_SLOT {
            continue;
        }
        for &j in defs {
            if let Some(Insn::Movea { dst: w, .. }) = pre.get(j) {
                let pair = (Reg::A(*v), Reg::A(*w));
                if crate::regs::is_virtual(pair.1)
                    && !apart.contains(&pair)
                    && !more.iter().any(|(_, p)| *p == pair)
                {
                    more.push((k, pair));
                }
            }
        }
    }
    more
}

/// After allocation: a slot loaded for one use after the body's first
/// branch, call or label (codegen-patterns 12.4 R4(b), beads
/// domain-pascal-178, -i24w).  Its register is the lowest one free over
/// its range (what [`allocate_apart`] gives it, R1; [`fresh_slot_loads`]
/// keeps it apart from the address values of earlier statements since a
/// call it follows in straight-line code; [`pass_dead_slot_loads`] from
/// dead slot loads; `passed` lists the loads so moved).
///
/// - When every free register below it holds a dead parameter slot load
///   on every path (at least one), and no instruction wrote it on any
///   path or a block copy used it up on every path, the original takes
///   it (bead domain-pascal-i24w; `tools/regalloc.py slotpass`, 41 loads
///   of that shape, 41 hits: TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E8 and
///   TTY_$K_INQ_PGROUP 0xE6796E `movea.l (0xc,A6),A3` past A2, the
///   status loaded at entry; PBU_$ADVANCE_EC_INT 0xE88432 A3 past A1;
///   SMD_$READ_CRSR_BITMAP 0xE6FD54 A3, 0xE6FD5C A4; used up:
///   XNS_IDP_$DEMUX 0xE18C26 A1 past A0 after an `(A1)+` copy).  A load
///   moved by [`pass_dead_slot_loads`] that does not end in this shape is
///   refused: past a dead slot load, a register a call wrote is taken 24
///   times and passed over 5 (VTOCE_$LOOKUP_FM 0xE39A9C, SMD_$INVERT_DISP
///   0xE703C6 A0 against TTY_$K_SET_FLAG 0xE67454 A1; bead
///   domain-pascal-quio), and when every free register holds one the
///   choice looks round-robin (MSG_$SEND_HW 0xE59B6E..0xE59B8C,
///   TPAD_$SET_UNIT_MODE 0xE69834 A4, 0xE6983C A0; bead
///   domain-pascal-aerg).
/// - Otherwise the original takes the register when no earlier
///   instruction left a value in it on any path to the load (a value used
///   up by `(An)+` / `-(An)`, or one a call clobbered after, does not
///   count), and passes over the registers written since such a call
///   when every dirty register was written there.  Otherwise its choice
///   is not known and the load is refused; so is A0 after a call when A1
///   was written before the load (other than by a call).
///
/// Survey (`tools/regalloc.py slots`, routines of 0x30-0x150 bytes, the
/// slot loads after the first branch, call or label whose slot no other
/// instruction names and whose value lives in one block and across no
/// call: 350): the second rule takes 174 and fits 172 (TTY_$K_SET_INPUT_FLAG
/// 0xE67684 `movea.l (0x10,A6),A1` after the call, A0 holding the
/// descriptor; FILE_$TRUNCATE_D 0xE74022 A0; PROC2_$PGUID_TO_UPGID
/// 0xE410B8 A1 after 0xE410B2 A0, both after ML_$UNLOCK); the two misses
/// are MAC_$DEMUX 0xE0BCD2 (A1 after a copy used up A0 and A1) and
/// 0xE6AA1C (A3 while A1 is free and clean).  In 114 the lowest free
/// register is dirty on some path otherwise; the lowest one that is not
/// dirty on every path fits 86 of them, and the exceptions include
/// callee-saved registers taken while a scratch one is free and clean
/// (0xE0D6DE A3, 0xE69834 A4) and a dirty A0 reused (0xE19158, after a
/// `lea (0,A5,D1.l),A0` that only formed an address).  In 62, after a
/// call with A1 written before, the original takes A0 40 times, A1 17
/// times (PROC2_$DEBUG 0xE41710, PROC2_$GET_EC 0xE40148,
/// XPD_$SET_PTRACE_OPTS 0xE5B066, against PROC2_$SET_NAME 0xE3EABE and
/// PROC2_$SIGNAL 0xE3F080) and A2-A4 5 times.
fn check_late_slot_loads(
    out: &[Insn],
    late: &[(usize, Option<usize>, Span)],
    passed: &[usize],
    exit: &[Reg],
    holds: &[(usize, Reg)],
) -> Res<()> {
    if late.is_empty() {
        return Ok(());
    }
    // Per instruction, before it, on some path: the address registers an
    // earlier instruction left a value in (bit n for An); those written
    // by an instruction that is not a call; whether a call (not M$ARITH)
    // came before.
    #[derive(Clone, Copy, Default, PartialEq)]
    struct State {
        dirty: u8,
        written: u8,
        call: bool,
    }
    let amask = |regs: &[Reg]| {
        regs.iter().fold(0u8, |m, r| match r {
            Reg::A(n) if *n < 5 => m | 1 << n,
            _ => m,
        })
    };
    let succ = crate::regs::successors(out);
    let mut inn: Vec<Option<State>> = vec![None; out.len()];
    if !out.is_empty() {
        inn[0] = Some(State::default());
    }
    loop {
        let mut changed = false;
        for i in 0..out.len() {
            let Some(st) = inn[i] else { continue };
            let acc = crate::regs::access(&out[i]);
            let defs = amask(&acc.defs);
            let o = if matches!(out[i], Insn::Jsr(_) | Insn::Bsr { .. }) {
                State {
                    dirty: st.dirty & !defs,
                    written: st.written,
                    // M$ARITH writes only D0 and A0 (12.4 R3).
                    call: st.call || defs & 2 != 0,
                }
            } else {
                let used_up =
                    autoinc_regs(&out[i])
                        .iter()
                        .fold(0u8, |m, &n| if n < 5 { m | 1 << n } else { m });
                State {
                    dirty: (st.dirty | defs) & !used_up,
                    written: st.written | defs,
                    call: st.call,
                }
            };
            for s in succ[i].iter().flatten() {
                let merged = match inn[*s] {
                    None => o,
                    Some(t) => State {
                        dirty: t.dirty | o.dirty,
                        written: t.written | o.written,
                        call: t.call || o.call,
                    },
                };
                if inn[*s] != Some(merged) {
                    inn[*s] = Some(merged);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let reach = reaching_aregs(out);
    let live_out = crate::regs::liveness(out, exit, holds);
    for &(k, call, span) in late {
        let Some(Insn::Movea { dst, .. }) = out.get(k) else {
            continue;
        };
        // The registers below the load's that are free over its range
        // (not live out of an instruction from the load to the one before
        // its last use).
        let mine = crate::regs::RegKey(Reg::A(*dst));
        let mut end = k;
        while live_out.get(end).is_some_and(|l| l.contains(&mine)) {
            end += 1;
        }
        let free_below: Vec<u8> = (0..*dst)
            .filter(|&n| {
                !live_out[k..end]
                    .iter()
                    .any(|l| l.contains(&crate::regs::RegKey(Reg::A(n))))
            })
            .collect();
        if let Some(Some(r)) = reach.get(k).filter(|_| passes_dead_slot_loads(out, k)) {
            let own = reach_kinds(out, *dst, &r[*dst as usize]);
            if matches!(own, REACH_ENTRY | REACH_USED_UP)
                && !free_below.is_empty()
                && free_below
                    .iter()
                    .all(|&n| reach_kinds(out, n, &r[n as usize]) == REACH_SLOT)
            {
                continue;
            }
        }
        if passed.contains(&k) {
            return Err(late_slot_refusal(span));
        }
        let st = inn[k].unwrap_or_default();
        // The registers written since the call the load follows in
        // straight-line code: kept apart ([`fresh_slot_loads`]).  When one
        // of them below the load's register is dirty, the load took the
        // straight-line rule's register, which the image shows only when
        // every dirty register was written there.
        let local = call.map_or(0, |c| {
            out[c + 1..k]
                .iter()
                .fold(0u8, |m, i| m | amask(&crate::regs::access(i).defs))
        });
        let below = (1u8 << dst) - 1;
        if st.dirty & (1 << dst) != 0 || (st.dirty & below & local != 0 && st.dirty & !local != 0) {
            return Err(late_slot_refusal(span));
        }
        if *dst == 0 && st.call && st.written & 2 != 0 {
            return Err(unsupported(
                span,
                "in a body of several statements, a pointer parameter or by-address parameter \
                 used once after a call, in A0 while A1 was written before (A0 or A1 in the \
                 original, codegen-patterns 12.4 R4(b)), is",
            ));
        }
    }
    Ok(())
}

/// After allocation: the copies `movea.l Dn,An` of pointers held in data
/// registers ([`Selector::copy_to_a`]) whose value an address register
/// still holds on every path are dropped, and that register is read
/// instead (codegen-patterns 12.2, "Pointers in data registers").  The
/// image copies again only where no address register holds the copy any
/// more: of the 981 `movea.l Dn,An` in the routines `tools/regalloc.py`
/// analyses whose data register is defined on every path, 969 are made
/// where no address register holds a copy of the same value, 12 where one
/// does (`tools/regalloc.py ptrcopies`).  OSINFO_$GET_MMAP reads A1, the
/// copy of D3 made at 0xE5C870, at 0xE5C8C8 and 0xE5C952 (past two `if`s
/// that do not write A1), but copies again at 0xE5C798 and 0xE5C9A2
/// (calls and a loop wrote A1) and at 0xE5C870 (after the `jsr` at
/// 0xE5C864); EC2_$INIT 0xE42C70 makes one copy for its two stores.
///
/// A copy whose register is not the holder's is dropped only when its
/// value lives in straight-line code that writes neither register (the
/// holder is renamed in); otherwise it stays.  Returns the dropped
/// copies' indices; each is left in `out` as an empty `Insn::Words`
/// (no registers, no bytes), which the caller leaves out of the body.
fn reuse_pointer_copies(out: &mut [Insn], copies: &[usize]) -> Vec<usize> {
    if copies.is_empty() {
        return Vec::new();
    }
    // Per address register A0..A4, the data register it holds a copy of.
    type Holds = [Option<u8>; 5];
    let mut dropped: Vec<usize> = Vec::new();
    loop {
        let succ = crate::regs::successors(out);
        let mut inn: Vec<Option<Holds>> = vec![None; out.len()];
        if !out.is_empty() {
            inn[0] = Some([None; 5]);
        }
        loop {
            let mut changed = false;
            for i in 0..out.len() {
                let Some(mut st) = inn[i] else { continue };
                {
                    let acc = crate::regs::access(&out[i]);
                    for d in &acc.defs {
                        match *d {
                            Reg::A(n) if n < 5 => st[n as usize] = None,
                            Reg::D(n) => {
                                for h in st.iter_mut() {
                                    if *h == Some(n) {
                                        *h = None;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if copies.contains(&i) {
                        if let Insn::Movea {
                            src: Ea::Reg(Reg::D(d)),
                            dst,
                            ..
                        } = out[i]
                        {
                            st[dst as usize] = Some(d);
                        }
                    }
                }
                for s in succ[i].iter().flatten() {
                    let merged = match inn[*s] {
                        None => st,
                        Some(t) => {
                            let mut m = t;
                            for (k, h) in m.iter_mut().enumerate() {
                                if *h != st[k] {
                                    *h = None;
                                }
                            }
                            m
                        }
                    };
                    if inn[*s] != Some(merged) {
                        inn[*s] = Some(merged);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut progress = false;
        for &k in copies {
            if dropped.contains(&k) {
                continue;
            }
            let Insn::Movea {
                src: Ea::Reg(Reg::D(d)),
                dst: a,
                ..
            } = out[k]
            else {
                continue;
            };
            let Some(st) = inn[k] else { continue };
            // The copy's own register first: it needs no renaming.
            let Some(j) = (a < 5 && st[a as usize] == Some(d))
                .then_some(a)
                .or_else(|| (0..5u8).find(|&j| st[j as usize] == Some(d)))
            else {
                continue;
            };
            if j == a {
                out[k] = Insn::Words(Vec::new());
                dropped.push(k);
                progress = true;
                break;
            }
            // The copy's uses, up to its last, in straight-line code that
            // writes neither An nor the holder.
            let live = crate::regs::liveness(out, &[], &[]);
            let ra = crate::regs::RegKey(Reg::A(a));
            let mut uses = Vec::new();
            let mut i = k;
            let mut ok = true;
            while live[i].contains(&ra) {
                i += 1;
                let Some(x) = out.get(i) else {
                    ok = false;
                    break;
                };
                let acc = crate::regs::access(x);
                let flow = matches!(
                    x,
                    Insn::Label(_)
                        | Insn::Branch { .. }
                        | Insn::Dbf { .. }
                        | Insn::Jsr(_)
                        | Insn::Bsr { .. }
                        | Insn::TableIndex { .. }
                        | Insn::TableJump { .. }
                        | Insn::Rts
                );
                if flow || acc.defs.contains(&Reg::A(a)) || acc.defs.contains(&Reg::A(j)) {
                    ok = false;
                    break;
                }
                if acc.uses.contains(&Reg::A(a)) {
                    uses.push(i);
                }
            }
            if !ok || uses.is_empty() {
                continue;
            }
            for &u in &uses {
                crate::regs::rename(&mut out[u], &|r| if r == Reg::A(a) { Reg::A(j) } else { r });
            }
            out[k] = Insn::Words(Vec::new());
            dropped.push(k);
            progress = true;
            break;
        }
        if !progress {
            dropped.sort_unstable();
            return dropped;
        }
    }
}

/// The pointer field `p.f` of `p.f^` with the record reached through a
/// pointer in a frame slot (`base` is the load of the field;
/// [`Selector::push`], 0xE1CC52 `move.l (0x48,A2),-(SP)` in KBD_$RCV's
/// nested 0xE1CC10): the frame offset of the record's pointer and the
/// field's address, `(d,An)` once evaluated.  Whether that pointer is
/// held is the caller's to check.
fn field_pointer(base: &Expr) -> Option<(i32, &Addr)> {
    let Expr::Load {
        addr: field @ Addr::Indirect { base: rec, .. },
        ty: Scalar {
            width: Width::Long, ..
        },
    } = base
    else {
        return None;
    };
    match &**rec {
        Expr::Load {
            addr: Addr::Frame(o),
            ..
        } => Some((*o, field)),
        _ => None,
    }
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
/// placed in machine registers"), each with whether it is a *web*: one
/// register for the whole body that every assignment writes
/// ([`Selector::webs`]).  The slot stays allocated: TIME_$GET_ADJUST
/// 0xE16AA8 `link.w A6,#-4` never touches its frame, its local is in D1
/// (0xE16AB4..0xE16AD6).
///
/// The rule (bead domain-pascal-iyd; `tools/localsurvey.py`): a local
/// whose only references are whole reads and whole assignments (not
/// dereferenced, its address never passed or taken, not a `for` index;
/// `crate::locals`), assigned at least once, is held in a register,
/// unless every read of it is a call's argument pushed by value as it is
/// (TTY_$I_BUF_PUT 0xE1AF60 `move.w D0,(-6,A6)`, 0xE1AF88 `move.w
/// (-6,A6),-(SP)`: the spin-lock token).  Of the 43 such locals of
/// ../source's TTY Pascal it fits 41 (30 in registers, 11 in the frame;
/// TTY_$I_BUF_INSERT's `new_tail`, assigned in both arms of an `if`, D1 at
/// 0xE1AF20 / 0xE1AF24); the two others, TTY_$I_PUT_CHARS' `col_mod`
/// (0xE1B1D0) and TTY_$K_GET's `done` (0xE1C426), are in the frame for a
/// reason not known.  In the C corpus (data locals with a register or
/// frame note in map-named routines; weaker evidence) locals read only
/// as pushed arguments are in the frame 125 times against 5 (UID_$GEN's
/// `token` in D3, 0xE1A040, read in a loop), and the others in a
/// register 275 times against 26 where the routine saves at most three
/// of D2-D7, less often the more it saves (110/55 with five, 193/107 with
/// six): under pressure the original spills by a priority not modelled
/// (the allocator refuses a body that runs out).
///
/// A local of the narrow rule of bead domain-pascal-0wc (assigned once,
/// by a routine-level statement, with a non-constant value, read only by
/// later routine-level statements) takes the register its value is
/// computed in at the assignment, as before; every other local is a web.
fn register_locals(f: &Function) -> Vec<(i32, bool)> {
    let uses = crate::locals::body_uses(f);
    uses.locals
        .iter()
        .filter(|l| {
            // Every read pushes it as an argument (and there is one).
            let only_pushed = l.loads.iter().all(|r| r.whole) && !l.loads.is_empty();
            !l.other && !l.deref && !l.for_index && !l.stores.is_empty() && !only_pushed
        })
        .map(|l| {
            let narrow = matches!(l.stores[..], [s] if !s.at.nested && !s.konst
                && l.loads.iter().all(|r| r.at.top > s.at.top));
            (l.offset, !narrow)
        })
        .collect()
}

/// A node [`walk`] visits.
#[derive(Clone, Copy)]
enum Node<'a> {
    Expr(&'a Expr),
    Addr(&'a Addr),
    Call(&'a Call),
}

/// Check, once the body is allocated, the tests [`Selector::tst`] left to
/// the flags of an assignment's `move` (`flag_moves`: the move's index,
/// the test's span, whether it copies a function's result out of D0;
/// bead domain-pascal-1gyq, codegen-patterns 7.1 "Flags of a load"):
///
/// - a register compared again right after the branch (`move.w (A3),D0;
///   beq.s; cmpi.w #255,D0`, a compound condition or a test in the next
///   statement): after a load the image goes both ways, 104 on the load's
///   flags (DIR_$READ_LINKU 0xE4D6E6) and 12 with a `tst` first
///   (DIR_$GET_ENTRYU 0xE4D522 `move.w (A0),D0; tst.w D0; beq.s; cmpi.w
///   #255,D0`), the same shapes; no rule found (bead domain-pascal-rmse):
///   refused.  After a call's
///   result (`move.w D0,D2; beq.s; cmpi.w #1,D2`, MSG_$WAITI 0xE59C84):
///   on the flags 4 times, never a `tst`;
/// - the branch: on a load's flags only `beq`/`bne`/`bmi`/`bpl`/`ble`
///   are seen, on a call's result's only `beq`/`bne` ([`Selector::
///   assigned_flags`]): others refused;
/// - a function's result copied into D1 (a local not held across a
///   call): the original tests D0 and copies after the branch
///   (XPD_$CONTINUE_PROC 0xE5BEF8 `tst.w D0; beq.s; move.w D0,D1`;
///   XPD_$GET_TARGET_INFO 0xE5C158); not reproduced (bead
///   domain-pascal-59gz): refused.
fn check_assigned_flags(out: &[Insn], moves: &[(usize, Span, bool)]) -> Res<()> {
    for &(k, span, call) in moves {
        let Some(Insn::Move {
            dst: Ea::Reg(r @ Reg::D(n)),
            ..
        }) = out.get(k)
        else {
            // Allocation renames registers in place and keeps the
            // indices; a test already left to this move must not go
            // unchecked.
            unreachable!("a flag move is a `move` into a data register after allocation")
        };
        // The instruction that reads the flags: the branch the survey
        // counts (an `s<cc>`: one site, `slt`, 0xE3358C).
        let branch = match out.get(k + 1) {
            Some(Insn::Branch { cond, .. }) => Some(*cond),
            _ => None,
        };
        let seen = if call {
            matches!(branch, Some(Cond::Eq | Cond::Ne))
        } else {
            matches!(
                branch,
                Some(Cond::Eq | Cond::Ne | Cond::Mi | Cond::Pl | Cond::Le)
            )
        };
        if !seen {
            return Err(Selector::assigned_ordered(span));
        }
        if call && *n < 2 {
            return Err(unsupported(
                span,
                "a function's result assigned to a local in a scratch register and tested next \
                 (the original tests D0 and copies it after the branch, XPD_$CONTINUE_PROC \
                 0xE5BEF8; not reproduced, bead domain-pascal-59gz) is",
            ));
        }
        let again = match out.get(k + 2) {
            Some(Insn::Tst { dst, .. } | Insn::Cmpi { dst, .. }) => *dst == Ea::Reg(*r),
            Some(Insn::Cmp { src, dst, .. }) => *dst == *n || *src == Ea::Reg(*r),
            _ => false,
        };
        if again && !call {
            return Err(unsupported(
                span,
                "a local assigned a load and compared twice right after (on the load's flags \
                 or after a `tst`: both seen, no rule found, codegen-patterns 7.1, bead \
                 domain-pascal-rmse) is",
            ));
        }
    }
    Ok(())
}

/// The data section of `st`'s target when `st` stores a constant into
/// another data section's block ([`Addr::OtherStatic`]): the statement
/// starts with that block's address (KBD_$INIT 0xE33374 `clr.l
/// (0x58,A0)`).
fn first_other_static(st: &Stmt) -> Option<String> {
    match &st.kind {
        StmtKind::Store {
            addr,
            value: Expr::Const { .. },
        } => match addr.unwrap_volatile().0 {
            Addr::OtherStatic { section, .. } => Some(section.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `st` reaches another data section's block anywhere.
fn mentions_other_static(st: &Stmt) -> bool {
    let mut found = false;
    walk(std::slice::from_ref(st), &mut |n| {
        if let Node::Addr(Addr::OtherStatic { .. }) = n {
            found = true;
        }
    });
    found
}

/// Visit `e` and every expression, address and call inside it.
fn walk_expr<'a>(e: &'a Expr, v: &mut dyn FnMut(Node<'a>)) {
    v(Node::Expr(e));
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
            walk_addr(addr, v)
        }
        Expr::SetMember { elem, set, .. } => {
            walk_expr(elem, v);
            walk_expr(set, v);
        }
        Expr::SetMemberAt { elem, set, .. }
        | Expr::BitElem {
            index: elem,
            array: set,
            ..
        } => {
            walk_expr(elem, v);
            walk_addr(set, v);
        }
        Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
            walk_expr(lhs, v);
            walk_expr(rhs, v);
        }
        Expr::Unary { operand: x, .. }
        | Expr::Convert { value: x, .. }
        | Expr::SetSr(x)
        | Expr::SetOf { elem: x, .. } => walk_expr(x, v),
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
        | Addr::OtherStatic { .. }
        | Addr::Symbol { .. }
        | Addr::Const(_) => {}
        Addr::Volatile { addr, .. } => walk_addr(addr, v),
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
    if let Some(x) = c.pointer() {
        walk_expr(x, v);
    }
}

fn walk_cond<'a>(c: &'a IrCond, v: &mut dyn FnMut(Node<'a>)) {
    match c {
        IrCond::Compare { lhs, rhs, .. } => {
            walk_expr(lhs, v);
            walk_expr(rhs, v);
        }
        IrCond::Test(e) | IrCond::Odd(e) => walk_expr(e, v),
        IrCond::Not(x) => walk_cond(x, v),
        IrCond::AndThen(a, b) | IrCond::OrElse(a, b) | IrCond::And(a, b) | IrCond::Or(a, b) => {
            walk_cond(a, v);
            walk_cond(b, v);
        }
        IrCond::Block { lhs, rhs, .. } => {
            walk_addr(lhs, v);
            walk_addr(rhs, v);
        }
    }
}

/// Visit every expression, address and call of `body`, nested statements
/// included.
/// How an 8-byte copy's source register is loaded ([`Selector::copy8`]).
enum Src8 {
    /// `lea <ea>,An`.
    Lea(Ea),
    /// `movea.l (o,A6),An`: the pointer in a frame slot.
    Slot(i32),
    /// `movea.l #sym,An`: another unit's object.
    Sym(String),
}

/// The operand `n` bytes after `ea` (a memory operand by displacement or
/// absolute address), if it has one in the same form.
fn ea_plus(ea: &Ea, n: i32) -> Option<Ea> {
    let fits = |d: i32| (-0x8000..0x8000).contains(&d);
    match ea {
        Ea::Ind(r) => Some(Ea::Disp(n, *r)),
        Ea::Disp(d, r) if fits(d + n) => Some(Ea::Disp(d + n, *r)),
        Ea::Idx(d, r, x, l) if (-0x80..0x80).contains(&(d + n)) => Some(Ea::Idx(d + n, *r, *x, *l)),
        Ea::AbsL(s, d) => Some(Ea::AbsL(s.clone(), d + n)),
        _ => None,
    }
}

fn walk<'a>(body: &'a [Stmt], v: &mut dyn FnMut(Node<'a>)) {
    for st in body {
        match &st.kind {
            StmtKind::SetResult(e) | StmtKind::Discard(e) => walk_expr(e, v),
            StmtKind::Store { addr, value }
            | StmtKind::StoreBits { addr, value, .. }
            | StmtKind::StoreBitElem {
                array: addr,
                index: value,
                ..
            } => {
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
                if let Some(u) = until {
                    walk_cond(u, v);
                }
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
            | StmtKind::Next
            | StmtKind::Label(_)
            | StmtKind::Goto(_) => {}
        }
    }
}

/// The object `x` when `e` is `addr(x)`, or a type transfer of it to
/// another 4-byte type (`integer32(addr(x))`: ref "Type Transfer
/// Functions", p. 4-197, "does not change any bits"; `dpc-ir` wraps it in
/// a same-width [`Expr::Convert`]).  The image stores, pushes and
/// computes with such a transfer as with the address itself
/// (codegen-patterns 5.2, "`addr(x)` as a value": NETBUF_$INIT 0xE2F63C
/// `move.l #NETWORK_$BUFPOOL,(0x330,A0)`, SOCK_$INIT 0xE2FE4C `lea
/// (0x4,A2),A1; move.l A1,(0xc,A0)`, FLP_$DINIT 0xE3E15E `pea (0x80,A5)`
/// by value, NET_$OPEN 0xE5A1B8 in a difference).
/// The frame offset of the slot `a` is the object of, when `a` is a whole
/// object reached through a parameter slot (`Addr::Indirect` on the slot's
/// load, offset 0): `addr(p)` of a parameter passed by address is the
/// slot's value ([`Selector::is_address_slot`] tells such slots from
/// by-value pointer parameters).
fn by_address_slot(a: &Addr) -> Option<i32> {
    match a {
        Addr::Indirect { base, offset: 0 } => match &**base {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if *o > 0 => Some(*o),
            _ => None,
        },
        _ => None,
    }
}

/// The frame offset of the slot `a` is reached through, when `a` is an
/// object at a constant offset in the object a parameter slot points to
/// (`Addr::Indirect` on the slot's load): the whole object or a field.
fn slot_root(a: &Addr) -> Option<i32> {
    match a {
        Addr::Indirect { base, .. } => by_address_slot(&Addr::Indirect {
            base: base.clone(),
            offset: 0,
        }),
        _ => None,
    }
}

fn addr_value(e: &Expr) -> Option<&Addr> {
    match e {
        Expr::AddrOf(a) => Some(a),
        Expr::Convert { value, ty } if value.ty().width == ty.width => addr_value(value),
        _ => None,
    }
}

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

/// Whether `e` is the value of a function whose result is in D0.
fn d0_call(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Call {
            reg: ResultClass::Data,
            ..
        }
    )
}

/// The memory operand an instruction writes, if any.
fn mem_dst(i: &Insn) -> Option<&Ea> {
    let dst = match i {
        Insn::Move { dst, .. }
        | Insn::Clr { dst, .. }
        | Insn::Scc { dst, .. }
        | Insn::Unary { dst, .. }
        | Insn::Addq { dst, .. }
        | Insn::Subq { dst, .. }
        | Insn::AluImm { dst, .. }
        | Insn::BitImm { dst, .. }
        | Insn::Bit { dst, .. }
        | Insn::AluToMem { dst, .. } => dst,
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
        OtherStatic(&'a str, i32),
        Symbol(&'a str, i32),
        Enclosing,
        Pointer,
        Const,
    }
    fn root(a: &Addr) -> (Root<'_>, bool) {
        match a {
            Addr::Frame(o) => (Root::Frame(*o), true),
            Addr::Static(o) => (Root::Static(*o), true),
            Addr::OtherStatic { section, offset } => (Root::OtherStatic(section, *offset), true),
            Addr::Symbol { name, offset } => (Root::Symbol(name, *offset), true),
            Addr::Enclosing(_) => (Root::Enclosing, true),
            Addr::Indirect { .. } => (Root::Pointer, true),
            Addr::Const(_) => (Root::Const, true),
            Addr::Index { base, .. } => (root(base).0, false),
            Addr::Volatile { addr, .. } => root(addr),
        }
    }
    let overlap = |a: i32, b: i32| a < b + lbytes as i32 && b < a + wbytes as i32;
    match (root(w), l) {
        ((Root::Frame(p), exact), Addr::Frame(o)) => !exact || overlap(p, *o),
        ((Root::Static(p), exact), Addr::Static(o)) => !exact || overlap(p, *o),
        ((Root::OtherStatic(s, p), exact), Addr::OtherStatic { section, offset }) => {
            s == section && (!exact || overlap(p, *offset))
        }
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
            Addr::Index { base, .. } | Addr::Volatile { addr: base, .. } => root(base),
            // Another frame (the enclosing routine's objects are never
            // held in a register, [`Function::uplevel_refs`]), the unit's
            // data, or the object a pointer points to.
            Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::OtherStatic { .. }
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

/// Whether `e` reads the object at `a` (the same address, wrapper
/// included).
fn expr_reads_addr(e: &Expr, a: &Addr) -> bool {
    let mut hit = false;
    walk_expr(e, &mut |n| {
        if let Node::Expr(Expr::Load { addr, .. } | Expr::Bits { addr, .. }) = n {
            hit |= addr == a;
        }
    });
    hit
}

/// Whether `e` reads the frame object at `off` as a value.
fn expr_reads_frame(e: &Expr, off: i32) -> bool {
    let mut hit = false;
    walk_expr(e, &mut |n| match n {
        Node::Expr(Expr::Load {
            addr: Addr::Frame(o),
            ..
        }) => hit |= *o == off,
        // A bit field's unit may start inside the object at `off`.
        Node::Expr(Expr::Bits {
            addr: Addr::Frame(o),
            field,
            ..
        }) => hit |= off < *o + field.unit.bytes() as i32 && *o < off + 4,
        _ => {}
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
/// array's address (`base`: an [`Addr::Indirect`] at offset 0, the unit's
/// data [`Addr::Static`], the frame [`Addr::Frame`], or another unit's
/// object [`Addr::Symbol`]), the element size, and how its pointer is
/// formed (decided by [`Selector::step_plan`]; [`StepForm::New`] until
/// then).
#[derive(Clone, Debug, PartialEq)]
struct SteppedArray {
    base: Addr,
    scale: u32,
    form: StepForm,
}

/// How a stepped pointer is formed (codegen-patterns 7.4, "Forming"; bead
/// domain-pascal-xm5w, `tools/loopalign.py ptrs`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum StepForm {
    /// `lea (from × size,Bn),Ap` into a new register, Bn the pointer
    /// variable's register, A5 or A6 (LOG_$READ's worker 0xE177E0;
    /// DBUF_$INVALIDATE 0xE3AA0E `lea (36,A5),A2`; MSG_$$SEND 0xE0DB7A
    /// `lea (4,A6),A0`).
    New,
    /// `movea.l A5|A6|#sym,Ap`, then `addq.l #k,Ap` (`lea (k,Ap),Ap`
    /// above 8) with k = from × size, in place (EC2_$REGISTER_EC1
    /// 0xE42968 `movea.l A5,A0; moveq #1,D1; addq.l #4,A0`; FILE_$PRIV_CREATE
    /// 0xE5D63E `movea.l A6,A0; moveq #35,D0; addq.l #4,A0`; TIME_$INIT
    /// 0xE2FE94 `moveq #63,D3; movea.l #abs,A2; moveq #1,D4; lea
    /// (12,A2),A2`).
    InPlace,
}

impl SteppedArray {
    /// The array's own offset from the register its pointer is formed
    /// from: the unit's data area's, the frame's, the symbol's; 0 through
    /// a pointer (the array at offset 0, [`stepped_arrays`]).  Each
    /// element is reached at this offset plus the lower bound and field
    /// offsets of its [`Addr::Index`] (DBUF_$INVALIDATE 0xE3AA16 `(-8,A3)`:
    /// the array at 0x10, 36-byte elements from 1, the field at 0xC).
    fn base_offset(&self) -> i32 {
        match &self.base {
            Addr::Static(o) | Addr::Frame(o) => *o,
            Addr::Symbol { offset, .. } => *offset,
            _ => 0,
        }
    }
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
            // The index is read first ([`Selector::store_bit_elem`]).
            StmtKind::StoreBitElem { array, index, .. } => {
                walk_expr(index, v);
                walk_addr(array, v);
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
                if let Some(u) = until {
                    walk_cond(u, v);
                }
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

/// Whether `e` reads a pointer that `p` reads or is (an equal load of a
/// longword through which `p` is reached).
fn shares_pointer(e: &Expr, p: &Expr) -> bool {
    let mut loads = Vec::new();
    walk_expr(p, &mut |n| {
        if let Node::Expr(x @ Expr::Load { ty, .. }) = n {
            if ty.width == Width::Long {
                loads.push(x);
            }
        }
    });
    let mut found = false;
    walk_expr(e, &mut |n| {
        if let Node::Expr(x) = n {
            found |= loads.contains(&x);
        }
    });
    found
}

/// Whether [`Selector::mul_const`] multiplies by `c` with a `neg`: an odd
/// part 2^k - 1 of k >= 3 (×28, ×56, ×60, ×120, ×248; codegen-patterns 8).
fn neg_form(c: u32) -> bool {
    if c == 0 {
        return false;
    }
    let m = c >> c.trailing_zeros();
    let plus = m > 1 && (m - 1).is_power_of_two();
    let minus = m > 3 && (m + 1).is_power_of_two();
    minus && !plus
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
/// - an array at an address other than through a pointer, in the unit's
///   data area ([`Addr::Static`]), in the frame ([`Addr::Frame`]) or in
///   another unit's object ([`Addr::Symbol`]): another data section's
///   block (bead domain-pascal-hxwq), the enclosing routine's frame;
///   (those three are accepted here; how their pointers are formed, and
///   what is refused of them, is [`Selector::step_plan`]'s);
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
            form: StepForm::New,
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
            if matches!(
                a.base,
                Addr::Static(_) | Addr::Frame(_) | Addr::Symbol { .. }
            ) {
                // A constant address: the unit's data area, the frame,
                // another unit's object ([`Selector::step_plan`]).
                continue;
            }
            return Err(
                "a `for` loop stepping a pointer through an array of another data section's \
                 block or of the enclosing routine's frame (not surveyed, codegen-patterns \
                 7.4; bead domain-pascal-hxwq) is",
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
        StmtKind::StoreBits { addr: a, field, .. } => {
            ok &= !hits(a, Some(field.unit.bytes() as i32))
        }
        StmtKind::StoreBitElem {
            array: a, bytes, ..
        } => ok &= !hits(a, Some(*bytes as i32)),
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

/// Apply `f` to every address of `body`, nested ones included, each
/// before the ones inside it (the mutable counterpart of [`walk`]).
fn addrs_mut(body: &mut [Stmt], f: &mut dyn FnMut(&mut Addr)) {
    for st in body {
        match &mut st.kind {
            StmtKind::SetResult(e) | StmtKind::Discard(e) => expr_addrs_mut(e, f),
            StmtKind::Store { addr, value }
            | StmtKind::StoreBits { addr, value, .. }
            | StmtKind::StoreBitElem {
                array: addr,
                index: value,
                ..
            } => {
                addr_mut(addr, f);
                expr_addrs_mut(value, f);
            }
            StmtKind::Copy { dst, src, .. } => {
                addr_mut(dst, f);
                addr_mut(src, f);
            }
            StmtKind::Call(c) => call_addrs_mut(c, f),
            StmtKind::Fill { start, value, .. } => {
                addr_mut(start, f);
                expr_addrs_mut(value, f);
            }
            StmtKind::If { cond, then_, else_ } => {
                cond_addrs_mut(cond, f);
                addrs_mut(then_, f);
                addrs_mut(else_, f);
            }
            StmtKind::Repeat { body, until } => {
                addrs_mut(body, f);
                if let Some(u) = until {
                    cond_addrs_mut(u, f);
                }
            }
            StmtKind::While { cond, body } => {
                cond_addrs_mut(cond, f);
                addrs_mut(body, f);
            }
            StmtKind::For(l) => {
                expr_addrs_mut(&mut l.from, f);
                expr_addrs_mut(&mut l.to, f);
                addrs_mut(&mut l.body, f);
            }
            StmtKind::Case {
                selector,
                arms,
                otherwise,
            } => {
                expr_addrs_mut(selector, f);
                for a in arms {
                    addrs_mut(&mut a.body, f);
                }
                if let Some(o) = otherwise {
                    addrs_mut(o, f);
                }
            }
            StmtKind::Return
            | StmtKind::Disable
            | StmtKind::Enable
            | StmtKind::Exit
            | StmtKind::Next
            | StmtKind::Label(_)
            | StmtKind::Goto(_) => {}
        }
    }
}

fn addr_mut(a: &mut Addr, f: &mut dyn FnMut(&mut Addr)) {
    f(a);
    match a {
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::Static(_)
        | Addr::OtherStatic { .. }
        | Addr::Symbol { .. }
        | Addr::Const(_) => {}
        Addr::Volatile { addr, .. } => addr_mut(addr, f),
        Addr::Indirect { base, .. } => expr_addrs_mut(base, f),
        Addr::Index { base, index, .. } => {
            addr_mut(base, f);
            expr_addrs_mut(index, f);
        }
    }
}

fn expr_addrs_mut(e: &mut Expr, f: &mut dyn FnMut(&mut Addr)) {
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => addr_mut(addr, f),
        Expr::SetMember { elem, set, .. } => {
            expr_addrs_mut(elem, f);
            expr_addrs_mut(set, f);
        }
        Expr::SetMemberAt { elem, set, .. }
        | Expr::BitElem {
            index: elem,
            array: set,
            ..
        } => {
            expr_addrs_mut(elem, f);
            addr_mut(set, f);
        }
        Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
            expr_addrs_mut(lhs, f);
            expr_addrs_mut(rhs, f);
        }
        Expr::Unary { operand: x, .. }
        | Expr::Convert { value: x, .. }
        | Expr::SetSr(x)
        | Expr::SetOf { elem: x, .. } => expr_addrs_mut(x, f),
        Expr::Call { call, .. } => call_addrs_mut(call, f),
        Expr::Test(c) => cond_addrs_mut(c, f),
    }
}

fn call_addrs_mut(c: &mut Call, f: &mut dyn FnMut(&mut Addr)) {
    for a in &mut c.args {
        match a {
            Arg::Value { value, .. } | Arg::Temp { value, .. } => expr_addrs_mut(value, f),
            Arg::Address { addr, .. } => addr_mut(addr, f),
        }
    }
    if let Some(x) = c.pointer_mut() {
        expr_addrs_mut(x, f);
    }
}

fn cond_addrs_mut(c: &mut IrCond, f: &mut dyn FnMut(&mut Addr)) {
    match c {
        IrCond::Compare { lhs, rhs, .. } => {
            expr_addrs_mut(lhs, f);
            expr_addrs_mut(rhs, f);
        }
        IrCond::Test(e) | IrCond::Odd(e) => expr_addrs_mut(e, f),
        IrCond::Not(x) => cond_addrs_mut(x, f),
        IrCond::AndThen(a, b) | IrCond::OrElse(a, b) | IrCond::And(a, b) | IrCond::Or(a, b) => {
            cond_addrs_mut(a, f);
            cond_addrs_mut(b, f);
        }
        IrCond::Block { lhs, rhs, .. } => {
            addr_mut(lhs, f);
            addr_mut(rhs, f);
        }
    }
}

/// A `with a[i] do` temporary ([`Function::with_temps`]) of a `for`
/// loop's stepped array: the temporary's frame offset and the array's
/// index in the loop's list.
type StepTemp = (i32, usize);

/// The `with` temporaries of `body` that hold the address of an element
/// of a stepped array of `arrays` (`with a[i] do`, `i` the `for` index at
/// `off`).
fn stepped_with_temps(
    body: &[Stmt],
    arrays: &[SteppedArray],
    off: i32,
    temps: &[i32],
) -> Vec<StepTemp> {
    let mut out = Vec::new();
    dpc_ir::visit_stmts(body, &mut |st| {
        if let StmtKind::Store {
            addr: Addr::Frame(t),
            value: Expr::AddrOf(Addr::Index {
                base, index, scale, ..
            }),
        } = &st.kind
        {
            if !temps.contains(t) || !is_index(index, off) {
                return;
            }
            if let Some(k) = arrays
                .iter()
                .position(|a| a.scale == *scale && a.base == **base)
            {
                out.push((*t, k));
            }
        }
    });
    out
}

/// `body` with each `with a[i] do` temporary of a stepped array at an
/// offset from its register rebased to hold the stepped pointer itself,
/// the array's offset ([`SteppedArray::base_offset`]) moved into the
/// references through it (codegen-patterns 7.4, "`with`":
/// DBUF_$INVALIDATE 0xE3AA12 `movea.l A2,A3`, the array at 0x10 of the
/// unit's data and its field at 0xC reached as `(-8,A3)`, A2 `lea
/// (36,A5),A2`; XNS_IDP_$OS_DEMUX 0xE185BC `movea.l A1,A0`, A1 `movea.l
/// A5,A1`, the channels at 0xA0 reached at `(216,A0)`).  The temporary is
/// assigned once, at its `with` statement, and referenced only inside it,
/// so its value less the offset and each reference plus it are the same
/// addresses.
fn with_rebase(body: &[Stmt], arrays: &[SteppedArray], temps: &[StepTemp]) -> Vec<Stmt> {
    let mut out = body.to_vec();
    let shift = |t: i32| {
        temps
            .iter()
            .find(|&&(x, _)| x == t)
            .map(|&(_, k)| arrays[k].base_offset())
    };
    addrs_mut(&mut out, &mut |a| {
        if let Addr::Indirect { base, offset } = a {
            if let Expr::Load {
                addr: Addr::Frame(t),
                ..
            } = &**base
            {
                *offset += shift(*t).unwrap_or(0);
            }
        }
    });
    for &(t, k) in temps {
        rebase_store(&mut out, t, arrays[k].base_offset());
    }
    out
}

/// Give the element address stored to `with` temporary `t` the offset
/// `-o` ([`with_rebase`]).
fn rebase_store(body: &mut [Stmt], t: i32, o: i32) {
    for st in body {
        match &mut st.kind {
            StmtKind::Store {
                addr: Addr::Frame(x),
                value: Expr::AddrOf(Addr::Index { offset, .. }),
            } if *x == t => *offset -= o,
            StmtKind::If { then_, else_, .. } => {
                rebase_store(then_, t, o);
                rebase_store(else_, t, o);
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                rebase_store(body, t, o)
            }
            StmtKind::For(l) => rebase_store(&mut l.body, t, o),
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms {
                    rebase_store(&mut a.body, t, o);
                }
                if let Some(x) = otherwise {
                    rebase_store(x, t, o);
                }
            }
            _ => {}
        }
    }
}

/// The displacements at which `body` (rebased, [`with_rebase`]) reaches
/// the elements of stepped array `arrays[k]` (the `for` index at `off`)
/// through its pointer, directly or through one of its `with`
/// temporaries; the copy into a temporary (displacement 0,
/// [`Selector::lea_into`]) is not one.
fn step_displacements(
    body: &[Stmt],
    arrays: &[SteppedArray],
    k: usize,
    off: i32,
    temps: &[StepTemp],
) -> Vec<i32> {
    let a = &arrays[k];
    let mut out = Vec::new();
    walk(body, &mut |n| match n {
        Node::Addr(Addr::Index {
            base,
            index,
            scale,
            offset,
        }) if *scale == a.scale && **base == a.base && is_index(index, off) => {
            let d = a.base_offset() + offset;
            if d != 0 {
                out.push(d);
            }
        }
        Node::Addr(Addr::Indirect { base, offset }) => {
            if let Expr::Load {
                addr: Addr::Frame(t),
                ..
            } = &**base
            {
                if temps.iter().any(|&(x, j)| x == *t && j == k) {
                    out.push(*offset);
                }
            }
        }
        _ => {}
    });
    out
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
            Node::Addr(Addr::Frame(o)) if off <= *o && *o < off + 4 => addrs += 1,
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
            IrCond::Test(e) | IrCond::Odd(e) => exprs(e, off, st),
            IrCond::Not(x) => cond(x, off, st),
            IrCond::AndThen(a, b) | IrCond::OrElse(a, b) | IrCond::And(a, b) | IrCond::Or(a, b) => {
                cond(a, off, st);
                cond(b, off, st);
            }
            IrCond::Block { lhs, rhs, .. } => {
                addr(lhs, off, st);
                addr(rhs, off, st);
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
                StmtKind::Store { addr: a, value }
                | StmtKind::StoreBits { addr: a, value, .. }
                | StmtKind::StoreBitElem {
                    array: a,
                    index: value,
                    ..
                } => {
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
                    if let Some(u) = until {
                        cond(u, off, st);
                    }
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
                | StmtKind::Next
                | StmtKind::Label(_)
                | StmtKind::Goto(_) => {}
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
/// ([`held_through_calls`]).  One that no call follows is found by
/// allocation (A0 or A1) and selected again held in A0 itself
/// ([`Selector::a0_locals`], [`select_body_with`]) when
/// [`parkable_pointer_local`], else refused.
/// Pointer locals assigned otherwise (`nil`, a memory load) stay in the
/// frame as before (codegen-patterns 12.5: often in a register in the
/// image, AST_$ADD_AOTES 0xE010C0; not settled).
fn call_pointer_locals(f: &Function, exclude: &[i32]) -> Vec<i32> {
    struct St {
        /// The local's type: a pointer, or a 4-byte integer
        /// ([`integer_call_local`]).
        ty: Scalar,
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
            Node::Addr(Addr::Frame(o)) if off <= *o && *o < off + 4 => addrs += 1,
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
                    if value.ty() != st.ty || a0_call(value).is_none() {
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
                    if let Some(u) = until {
                        cond(u, off, st);
                    }
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
        .filter(|l| {
            (l.ty == Scalar::POINTER || is_long_integer(l.ty)) && !exclude.contains(&l.offset)
        })
        .filter(|l| {
            !f.uplevel_refs
                .iter()
                .any(|&(p, n)| p < l.offset + 4 && l.offset < p + n as i32)
        })
        .filter(|l| {
            let mut st = St {
                ty: l.ty,
                first: None,
                bad: false,
            };
            stmts(&f.body, l.offset, &mut st);
            !st.bad
                && st.first == Some(true)
                && (l.ty == Scalar::POINTER || integer_call_local(f, l.offset))
        })
        .map(|l| l.offset)
        .collect()
}

/// A 4-byte integer type (not a pointer).
fn is_long_integer(ty: Scalar) -> bool {
    !ty.pointer && ty.bytes() == 4
}

/// Whether the 4-byte integer local at `off`, which [`call_pointer_locals`]
/// otherwise takes (every assignment a call of a function whose result
/// comes back in A0, `a0_return`, ref 5.5.11 p. 5-20: "Puts into A0 any
/// values returned from that function"; every read a type transfer to a
/// pointer type, ref 4 "Type Transfer Functions" p. 4-197, as the base of
/// a dereference), is held in a data register from its assignment and
/// moved to an address register where it is first dereferenced ([`Park`]),
/// whether a call follows or not (bead domain-pascal-yzxr): `t :=
/// tty_$i_get_desc(line, status)` with `t: integer32` and the function
/// declared `integer32; options(a0_return)`, then `tty_$desc_ptr_t(t)^`.
/// The integer is a data value, so it takes a data register (`move.l
/// A0,D0`, TTY_$K_FLUSH_INPUT 0xE1C0A8); the transfer gives the pointer
/// in an address register (`movea.l D0,A2`, 0xE1C0AE), which later
/// dereferences share.  Only the shape of [`parkable_pointer_local`]
/// (assigned once, by a statement of the routine's own statement part,
/// read only so) with at least one dereference; any other integer local
/// assigned an A0 result keeps its frame slot, as before.
fn integer_call_local(f: &Function, off: i32) -> bool {
    let mut derefs = 0;
    walk(&f.body, &mut |n| {
        if let Node::Addr(Addr::Indirect { base, .. }) = n {
            if matches!(&**base, Expr::Load { addr: Addr::Frame(o), .. } if *o == off) {
                derefs += 1;
            }
        }
    });
    derefs > 0 && parkable_pointer_local(f, off)
}

/// Whether the local at `off`, one of [`call_pointer_locals`], has the
/// shape of the image's held locals: a pointer local kept in A0 when no
/// call follows its assignment ([`Selector::a0_locals`]), an integer local
/// held in D0 ([`integer_call_local`], [`Park`]; beads domain-pascal-dgb,
/// -78n2): assigned exactly once, by a statement of the
/// routine's own statement part, and otherwise only dereferenced (the
/// sites: TTY_$K_* and SIO_$K_*, `tty := tty_$i_get_desc(line, status)`
/// and then `tty^...`; no site reads such a local whole, but RIP_$UPDATE_INT
/// 0xE159B6, whose `move.l A0,D0; bne.s` is a `nil` test, not supported).
fn parkable_pointer_local(f: &Function, off: i32) -> bool {
    let top = f
        .body
        .iter()
        .filter(|s| matches!(&s.kind, StmtKind::Store { addr: Addr::Frame(o), .. } if *o == off))
        .count();
    let (mut addrs, mut loads, mut derefs) = (0, 0, 0);
    walk(&f.body, &mut |n| match n {
        Node::Addr(Addr::Frame(o)) if off <= *o && *o < off + 4 => addrs += 1,
        Node::Expr(Expr::Load {
            addr: Addr::Frame(o),
            ..
        }) if *o == off => loads += 1,
        Node::Addr(Addr::Indirect { base, .. }) if matches!(&**base, Expr::Load { addr: Addr::Frame(o), .. } if *o == off) => {
            derefs += 1
        }
        _ => {}
    });
    top == 1 && addrs == loads + 1 && loads == derefs
}

/// Whether `st` calls a routine.
fn has_call(st: &Stmt) -> bool {
    let mut found = false;
    walk(std::slice::from_ref(st), &mut |n| {
        found |= matches!(n, Node::Call(_))
    });
    found
}

/// Whether `st` refers to the frame object at `off` (a 4-byte local).
fn refers_to(st: &Stmt, off: i32) -> bool {
    let mut found = false;
    walk(std::slice::from_ref(st), &mut |n| {
        if let Node::Addr(Addr::Frame(o)) = n {
            found |= off <= *o && *o < off + 4;
        }
    });
    found
}

/// Whether `stmts` ends in an unconditional jump (`return`, `exit`,
/// `next`, `goto`), so that no `bra` to a join follows it: the `bra`
/// would never be reached.
fn ends_in_jump(stmts: &[Stmt]) -> bool {
    matches!(
        stmts.last().map(|s| &s.kind),
        Some(StmtKind::Return | StmtKind::Exit | StmtKind::Next | StmtKind::Goto(_))
    )
}

/// Whether `rest`, the statements after an unconditional jump, are
/// reached: only through a label that starts them.
fn reached_after_jump(rest: &[Stmt]) -> bool {
    rest.is_empty() || matches!(rest[0].kind, StmtKind::Label(_))
}

/// Whether an `exit` of `body`'s own loop (not one of a loop nested in
/// it) is in `body`: what leaves a loop with no test to the statement
/// after it.
fn exits_loop(body: &[Stmt]) -> bool {
    body.iter().any(|s| match &s.kind {
        StmtKind::Exit => true,
        StmtKind::If { then_, else_, .. } => exits_loop(then_) || exits_loop(else_),
        StmtKind::Case {
            arms, otherwise, ..
        } => {
            arms.iter().any(|a| exits_loop(&a.body)) || otherwise.as_deref().is_some_and(exits_loop)
        }
        _ => false,
    })
}

/// Whether `rest` emits nothing (labels only): what follows it follows
/// the statement before it.
fn only_labels(rest: &[Stmt]) -> bool {
    rest.iter().all(|s| matches!(s.kind, StmtKind::Label(_)))
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
            | StmtKind::Label(_)
            | StmtKind::Goto(_)
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
/// it, the continuation included, is not reached.  A `goto` continues at
/// its label, whose own continuation is `labels[l]` ([`label_tails`]); a
/// label emits nothing.
fn seq_clean(stmts: &[Stmt], tail_clean: bool, labels: &[bool]) -> bool {
    for st in stmts {
        match &st.kind {
            StmtKind::Return => return true,
            StmtKind::Goto(l) => return labels[*l as usize],
            StmtKind::Label(_) => {}
            // A function call inside the value pushes and calls.
            StmtKind::SetResult(e)
            | StmtKind::Store { value: e, .. }
            | StmtKind::StoreBits { value: e, .. }
            | StmtKind::StoreBitElem { index: e, .. }
                if expr_has_call(e) =>
            {
                return false
            }
            StmtKind::SetResult(_)
            | StmtKind::Store { .. }
            | StmtKind::StoreBits { .. }
            | StmtKind::StoreBitElem { .. } => {}
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

/// For each label of `body` ([`StmtKind::Label`]), whether the code from
/// it to the epilogue is clean ([`Tail::clean`], [`seq_clean`]), with the
/// continuations [`Selector::seq`] gives nested sequences: an `if`'s parts
/// and a `case`'s arms that of the statement, a loop's body none.  A
/// `goto` reaches its label's continuation, so the labels are solved
/// together: each starts clean and loses it until nothing changes (a
/// cycle of `goto`s with nothing unclean never reaches the epilogue).
fn label_tails(body: &[Stmt], count: usize) -> Vec<bool> {
    fn walk(body: &[Stmt], tail_clean: bool, labels: &mut [bool], changed: &mut bool) {
        for (i, st) in body.iter().enumerate() {
            let next = seq_clean(&body[i + 1..], tail_clean, labels);
            match &st.kind {
                StmtKind::Label(l) => {
                    if labels[*l as usize] && !next {
                        labels[*l as usize] = false;
                        *changed = true;
                    }
                }
                StmtKind::If { then_, else_, .. } => {
                    walk(then_, next, labels, changed);
                    walk(else_, next, labels, changed);
                }
                StmtKind::Case {
                    arms, otherwise, ..
                } => {
                    for a in arms {
                        walk(&a.body, next, labels, changed);
                    }
                    if let Some(o) = otherwise {
                        walk(o, next, labels, changed);
                    }
                }
                StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                    walk(body, false, labels, changed)
                }
                StmtKind::For(l) => walk(&l.body, false, labels, changed),
                _ => {}
            }
        }
    }
    let mut labels = vec![true; count];
    loop {
        let mut changed = false;
        walk(body, true, &mut labels, &mut changed);
        if !changed {
            return labels;
        }
    }
}

/// Whether evaluating `e` calls a function, an M$ARITH entry included, or
/// branches conditionally (a signed `div` by a power of two,
/// [`Selector::div_pow2`]).
fn expr_has_call(e: &Expr) -> bool {
    match e {
        Expr::Const { .. } | Expr::AddrOf(_) => false,
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } => addr_has_call(addr),
        Expr::SetMember { elem, set, .. } => expr_has_call(elem) || expr_has_call(set),
        Expr::SetMemberAt { elem, set, .. }
        | Expr::BitElem {
            index: elem,
            array: set,
            ..
        } => expr_has_call(elem) || addr_has_call(set),
        Expr::SetOp { lhs, rhs, .. } => expr_has_call(lhs) || expr_has_call(rhs),
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
        Expr::Unary { operand: x, .. }
        | Expr::Convert { value: x, .. }
        | Expr::SetSr(x)
        | Expr::SetOf { elem: x, .. } => expr_has_call(x),
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
        // The compare and the branch over the move ([`Selector::min_max`]).
        BinOp::Min | BinOp::Max => true,
        _ => false,
    }
}

fn addr_has_call(a: &Addr) -> bool {
    match a {
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::Static(_)
        | Addr::OtherStatic { .. }
        | Addr::Symbol { .. }
        | Addr::Const(_) => false,
        Addr::Volatile { addr, .. } => addr_has_call(addr),
        Addr::Indirect { base, .. } => expr_has_call(base),
        Addr::Index { base, index, .. } => addr_has_call(base) || expr_has_call(index),
    }
}

fn cond_has_call(c: &IrCond) -> bool {
    match c {
        IrCond::Compare { lhs, rhs, .. } => expr_has_call(lhs) || expr_has_call(rhs),
        IrCond::Test(e) | IrCond::Odd(e) => expr_has_call(e),
        IrCond::Not(x) => cond_has_call(x),
        IrCond::AndThen(a, b) | IrCond::OrElse(a, b) | IrCond::And(a, b) | IrCond::Or(a, b) => {
            cond_has_call(a) || cond_has_call(b)
        }
        IrCond::Block { lhs, rhs, .. } => addr_has_call(lhs) || addr_has_call(rhs),
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
/// (`a := 0; b := 0` on two `integer16`s becomes one 32-bit store).  When
/// the original merged has not been settled (codegen-patterns 13, "Store
/// merging"; `tools/storesurvey.py merge`, bead domain-pascal-2m0): it
/// merged constant pairs through an address register (TTY_$K_RESET
/// 0xE67322, KBD_$INIT 0xE333B6, PROC1_$BIND 0xE14DA6) and in the frame
/// (ASKNODE_$PROPAGATE_WHO 0xE65EBC), and left 118 constant pairs of every
/// kind of base apart (TTY_$I_INIT 0xE3332E, TPAD_$INIT 0xE3363E,
/// ASKNODE_$SERVER 0xE65A3E).  So two consecutive stores of constants to adjacent objects
/// of the same base that fit together in a word or a longword are
/// rejected rather than guessed at, and so are two copies of adjacent
/// memory to adjacent memory (a merged one, one `move.l`, cannot be told
/// from a longword copy in the image; ../source reads SOCK_$GET 0xE16108
/// as one).  A word and a longword of constants (6 bytes), the word the
/// lower, are rejected too: FIM_$BUILD_DF 0xE0A896 zeroes a word at 0x54
/// and a longword at 0x56 as `clr.l (0x54,A2); clr.w (0x58,A2)`, merged
/// and split at the other boundary, while 0xE14CBC keeps such a pair
/// apart.  With the longword the lower, a merge split at the longword
/// gives the same two stores (0xE70B5E `clr.w (0x1a0,A5); clr.l
/// (0x19c,A5)`, in source order or merged keeping it).
///
/// Not candidates, and so accepted:
/// - a byte and a word (3 bytes) cannot be one 68000 reference, as at
///   module-local 0xE32B76, 0xE32BA6/0xE32BAA (`clr.b (0x36,A2); move.w
///   D2,(0x34,A2)`, a byte and the word below it).  TIME_$Q_INIT_QUEUE's
///   stores (0..4, 4..8, 8, 10..12) never fit: 0xE16C6E..0xE16C78;
/// - a pair whose values are not both constants and not both loads of
///   adjacent memory in the same arrangement (a computed value, a
///   register, a load from elsewhere): no one reference stores both (a
///   merge would need the two put together in a register, which no
///   store in the compiled code shows: codegen-patterns 13), and the
///   image keeps 270 such adjacent pairs apart (TPAD_$SET_UNIT_MODE
///   0xE69830..0xE69840, three words from three by-address parameters;
///   0xE02D40 `move.w D2,(-0x4e,A6); move.w D3,(-0x50,A6)`; 0xE07526);
/// - two bytes whose lower one is at an odd address, in the frame or the
///   module's static data (an even base: A6, and A5's block, every `lea
///   abs.l,A5` target and `D` segment being 4-aligned): a word there is
///   an odd-address reference, which the 68010 traps.  The image's 13
///   such pairs at an odd displacement are all apart: MST_$DISKLESS_INIT
///   0xE30DD2 (absolute, 0xE24353), 0xE62A42 (the frame), 11 through an
///   address register (0xE04CAC, 0xE19F1A).
///
/// The `clr.b` of the adjacent bytes 12 and 13 in XPD_$RESET_PTRACE_OPTS
/// (0xE5B15E, 0xE5B164) are not consecutive in the image (offsets 12, 0,
/// 13, 4, 8).  Each statement sequence (a body, an `if` arm, a loop body)
/// is checked.  An assignment to a register-held local (`held`,
/// [`register_locals`]) writes no memory.
fn reject_mergeable_stores(body: &[Stmt], held: &[i32]) -> Res<()> {
    let mut prev: Option<(&Addr, u32, &Expr)> = None;
    for st in body {
        match &st.kind {
            StmtKind::If { then_, else_, .. } => {
                reject_mergeable_stores(then_, held)?;
                reject_mergeable_stores(else_, held)?;
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                reject_mergeable_stores(body, held)?
            }
            StmtKind::For(l) => reject_mergeable_stores(&l.body, held)?,
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms {
                    reject_mergeable_stores(&a.body, held)?;
                }
                if let Some(o) = otherwise {
                    reject_mergeable_stores(o, held)?;
                }
            }
            _ => {}
        }
        let StmtKind::Store { addr, value } = &st.kind else {
            prev = None;
            continue;
        };
        if matches!(addr, Addr::Frame(o) if held.contains(o)) {
            // A register-held local (12.5): no memory is written.
            prev = None;
            continue;
        }
        let bytes = value.ty().bytes();
        if let Some((pa, pb, pv)) = prev {
            if let (Some((k1, o1)), Some((k2, o2))) = (split_addr(pa), split_addr(addr)) {
                let adjacent = o1 + pb as i32 == o2 || o2 + bytes as i32 == o1;
                let one = matches!(pb + bytes, 2 | 4)
                    && one_reference(&k1, o1.min(o2), pb + bytes)
                    && mergeable_values(pv, value, o2 - o1, held);
                // A word and, above it, a longword of constants: merged
                // and split again at the other boundary at FIM_$BUILD_DF
                // 0xE0A896 (`clr.l (0x54,A2); clr.w (0x58,A2)` for a word
                // at 0x54 and a longword at 0x56), apart at 0xE14CBC.
                // With the longword below, merged or not they are the same
                // two stores (apart in source order at 0xE70B5E).
                let word_lower = if pb == 2 { o1 < o2 } else { o2 < o1 };
                let six = pb + bytes == 6
                    && word_lower
                    && matches!((pv, value), (Expr::Const { .. }, Expr::Const { .. }));
                if k1 == k2 && adjacent && (one || six) {
                    return Err(unsupported(
                        st.span,
                        "a store adjacent to the previous statement's store (the original \
                         compiler may merge them, ref 3.15.3 p. 3-57) is",
                    ));
                }
            }
        }
        prev = Some((addr, bytes, value));
    }
    Ok(())
}

/// Whether `bytes` bytes at offset `lo` from the base `base` (a base from
/// [`split_addr`]) can be one 68000 reference: not a word at an odd
/// address.  The frame and the module's static data are even (A6 and A5
/// hold even addresses); the parity of a pointer's or a symbol's target
/// is not known here, so a pair through one stays a candidate.
fn one_reference(base: &Addr, lo: i32, bytes: u32) -> bool {
    let even_base = matches!(base, Addr::Frame(_) | Addr::Enclosing(_) | Addr::Static(_));
    !(bytes > 1 && even_base && lo % 2 != 0)
}

/// Whether the values of two stores to adjacent objects could be stored
/// by one reference (ref 3.15.3 p. 3-57): both constants, or both loads,
/// at the stores' widths, of memory of one base whose offsets differ by
/// `delta`, the difference of the destinations' offsets (a copy of
/// adjacent memory).  A computed value, a register-held local or a load
/// from elsewhere is not (codegen-patterns 13, "Store merging": 270 such
/// pairs in the image, none merged).
fn mergeable_values(first: &Expr, second: &Expr, delta: i32, held: &[i32]) -> bool {
    match (first, second) {
        (Expr::Const { .. }, Expr::Const { .. }) => true,
        (Expr::Load { addr: a1, .. }, Expr::Load { addr: a2, .. }) => {
            let held_local = |a: &Addr| matches!(a, Addr::Frame(o) if held.contains(o));
            if held_local(a1) || held_local(a2) {
                return false;
            }
            match (split_addr(a1), split_addr(a2)) {
                (Some((b1, s1)), Some((b2, s2))) => b1 == b2 && s2 - s1 == delta,
                // A load whose address is not a base and an offset (an
                // element at a run-time index): adjacency is not known.
                _ => true,
            }
        }
        _ => false,
    }
}

/// The frame offset of the parameter (a by-value pointer or a by-address
/// slot, `o > 0`) that `addr` dereferences, when `addr` is a reference to
/// a device object through it ([`Addr::Volatile`], [`Access::device`]).
///
/// Such a pointer is read from the frame for each reference, into the
/// lowest free register, rather than loaded at entry and held: the
/// module-local 0xE0A328 (DMA_; `dma/init_m68450_channel.c`) reloads
/// `movea.l (8,A6),A0` before each of its five stores to the M68450
/// channel's registers (0xE0A330..0xE0A35A), while it loads its other
/// parameter, a word, at entry (`move.w (0xc,A6),D0`), and the image's
/// other by-address slots used more than once are loaded at entry
/// (codegen-patterns 12.2).  How its parameter was declared is not known;
/// this is the device reading (ref 3.15.3: references to a device are
/// made as written).  A pointer local is not affected: WIN_'s
/// module-local 0xE190BC keeps the register block's address in A0 for all
/// its references (0xE190EC..0xE19112).
fn device_param_base(addr: &Addr) -> Option<i32> {
    let Addr::Volatile { addr, access } = addr else {
        return None;
    };
    if !access.device {
        return None;
    }
    match &**addr {
        Addr::Indirect { base, .. } => match &**base {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if *o > 0 => Some(*o),
            _ => None,
        },
        _ => None,
    }
}

/// The base of `addr` (with offset 0) and the offset.
fn split_addr(addr: &Addr) -> Option<(Addr, i32)> {
    match addr {
        Addr::Frame(o) => Some((Addr::Frame(0), *o)),
        Addr::Enclosing(o) => Some((Addr::Enclosing(0), *o)),
        Addr::Static(o) => Some((Addr::Static(0), *o)),
        Addr::OtherStatic { section, offset } => Some((
            Addr::OtherStatic {
                section: section.clone(),
                offset: 0,
            },
            *offset,
        )),
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
        // ref 3.15.3 p. 3-57: the device attribute "prevents this
        // optimization" (merging adjacent references), so two device
        // stores are not candidates.  A volatile one still is (3.15.1
        // names no such rule).
        Addr::Volatile { access, .. } if access.device => None,
        Addr::Volatile { addr, .. } => split_addr(addr),
    }
}

/// How the body uses a by-value parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamUse {
    /// Its value is read as data.
    Data,
    /// It is a pointer the body dereferences (`p^`, `p^.f`).
    Base,
    /// Its whole value is stored to memory (`g := p`, `g := addr(ec)`):
    /// data unless the body also dereferences it.  A pointer parameter
    /// that is dereferenced and stored is held in an address register
    /// and stored from it (codegen-patterns 12.2, "Pointers in data
    /// registers": EC_$INIT 0xE15202..0xE1520C `movea.l (8,A6),A0; clr.l
    /// (A0); move.l A0,(4,A0)`; the AST module-local 0xE00F7C, 0xE00FA0
    /// `move.l A1,(0x3ec,A5)`; DISK_$ADD_QUE 0xE3C804 `move.l
    /// A1,(-4,A6)`).
    Stored,
    /// Its whole value is compared with 0 (`p = nil`, `addr(ec) <> nil`):
    /// data, but a pointer also dereferenced is not settled (below,
    /// [`Selector::entry_loads`]).
    ZeroCompare,
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
    /// The uses walked since `start`, a loop's, as if the loop were
    /// walked twice (below, `Repeat`).  Walking it twice would take time
    /// exponential in the loop nesting (bead domain-pascal-1dn: 30 nested
    /// `repeat`s did not finish); what the walk is for (the first use of
    /// each parameter, whether all its uses agree, whether there are two)
    /// needs no more than two of each use, so that is all that is kept.
    fn twice(&mut self, start: usize) {
        let mut seen: Vec<((i32, ParamUse), u8)> = Vec::new();
        let tail = self.out.split_off(start);
        let mut kept = Vec::new();
        for u in tail {
            match seen.iter_mut().find(|(k, _)| *k == u) {
                Some((_, n)) if *n >= 2 => continue,
                Some((_, n)) => *n += 1,
                None => seen.push((u, 1)),
            }
            kept.push(u);
        }
        self.out.extend_from_slice(&kept);
        self.out.extend_from_slice(&kept);
    }

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
            // The address a by-address slot holds, as a value in an
            // expression (`addr(p)`, ref "Addr", p. 4-13): the slot is
            // read as data (EC2_$INIT 0xE42C64..0xE42C68 `move.l
            // (8,A6),D0; cmpi.l #0x3e8,D0`, its `out` parameter, ec2.ins.pas).
            // Passed as an argument it stays a base ([`Self::call_args`]).
            Expr::AddrOf(Addr::Indirect { base, offset: 0 })
                if matches!(&**base, Expr::Load { addr: Addr::Frame(o), .. }
                    if self.is_slot(*o)) =>
            {
                let Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } = &**base
                else {
                    unreachable!()
                };
                self.out.push((*o, ParamUse::Data))
            }
            // A bit field is read in memory (a parameter's in its slot).
            Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
                self.address(addr)
            }
            Expr::SetMember { elem, set, .. } => {
                self.expr(elem);
                self.expr(set);
            }
            Expr::SetMemberAt { elem, set, .. }
            | Expr::BitElem {
                index: elem,
                array: set,
                ..
            } => {
                self.expr(elem);
                self.address(set);
            }
            Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            Expr::Unary { operand: x, .. }
            | Expr::Convert { value: x, .. }
            | Expr::SetSr(x)
            | Expr::SetOf { elem: x, .. } => self.expr(x),
            Expr::Call { call, .. } => self.call_args(call),
            Expr::Test(c) => self.cond(c),
            Expr::Const { .. } => {}
        }
    }

    /// The parameter whose whole longword value `value` is, as a store's
    /// value or a comparison's operand ([`ParamUse::Stored`],
    /// [`ParamUse::ZeroCompare`]): a by-value parameter's load or
    /// `addr` of a by-address parameter's object, through type transfers
    /// that keep the width.
    fn whole_pointer(&self, value: &Expr) -> Option<i32> {
        match value {
            Expr::Convert { value, ty } if ty.width == Width::Long => self.whole_pointer(value),
            Expr::Load {
                addr: Addr::Frame(o),
                ty,
            } if self.is_param(*o) && ty.width == Width::Long => Some(*o),
            Expr::AddrOf(Addr::Indirect { base, offset: 0 }) => match &**base {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } if self.is_slot(*o) => Some(*o),
                _ => None,
            },
            _ => None,
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
            // A parameter that is the pointer to a device object is read
            // from the frame at each reference, not loaded at entry
            // ([`device_param_base`]; module-local 0xE0A328).
            a @ Addr::Volatile { addr, .. } => {
                if device_param_base(a).is_none() {
                    self.address(addr)
                }
            }
            Addr::Frame(_)
            | Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::OtherStatic { .. }
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
                // `addr(p)` pushed: the address, as for a by-address
                // argument (`pea`, codegen-patterns 5.2 "`addr(x)` as a
                // value").
                Arg::Value {
                    value: Expr::AddrOf(addr),
                    ..
                } => self.address(addr),
                Arg::Value { value, .. } | Arg::Temp { value, .. } => self.expr(value),
                Arg::Address { addr, .. } => self.address(addr),
            }
        }
        // A routine pointer that is a by-value parameter itself is read
        // from its slot right before the call, like a parameter that is
        // only pushed (no compiled call goes through a pointer loaded at
        // entry: codegen-patterns 1.6); one reached through a parameter
        // (`p^.f`) makes that parameter a base like any dereference
        // (SIO_$I_CTS_CHANGE 0xE1C6E0 `movea.l (8,A6),A2`, then 0xE1C724
        // `movea.l (0x38,A2),A0; jsr (A0)`).
        match c.pointer() {
            None
            | Some(Expr::Load {
                addr: Addr::Frame(_),
                ..
            }) => {}
            Some(x) => self.expr(x),
        }
    }

    fn cond(&mut self, c: &IrCond) {
        match c {
            IrCond::Compare { lhs, rhs, .. }
                if matches!(
                    (lhs, rhs),
                    (Expr::Const { value: 0, .. }, _) | (_, Expr::Const { value: 0, .. })
                ) && (self.whole_pointer(lhs).is_some()
                    || self.whole_pointer(rhs).is_some()) =>
            {
                let o = self
                    .whole_pointer(lhs)
                    .or_else(|| self.whole_pointer(rhs))
                    .unwrap();
                self.out.push((o, ParamUse::ZeroCompare));
            }
            IrCond::Compare { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            IrCond::Test(e) | IrCond::Odd(e) => self.expr(e),
            IrCond::Not(x) => self.cond(x),
            IrCond::AndThen(a, b) | IrCond::OrElse(a, b) | IrCond::And(a, b) | IrCond::Or(a, b) => {
                self.cond(a);
                self.cond(b);
            }
            IrCond::Block { lhs, rhs, .. } => {
                self.address(lhs);
                self.address(rhs);
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
                    let start = self.out.len();
                    self.stmts(body);
                    if let Some(u) = until {
                        self.cond(u);
                    }
                    self.twice(start);
                }
                StmtKind::While { cond, body } => {
                    let start = self.out.len();
                    self.cond(cond);
                    self.stmts(body);
                    self.twice(start);
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
                    let start = self.out.len();
                    self.stmts(&l.body);
                    self.twice(start);
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
                | StmtKind::Next
                | StmtKind::Label(_)
                | StmtKind::Goto(_) => {}
                StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e),
                StmtKind::Store { addr, value } => {
                    self.address(addr);
                    match self.whole_pointer(value) {
                        Some(o) => self.out.push((o, ParamUse::Stored)),
                        None => self.expr(value),
                    }
                }
                // The value is evaluated before the destination's address
                // when it is not a plain load ([`Selector::store_bits`]).
                StmtKind::StoreBits { addr, value, .. } => {
                    self.expr(value);
                    self.address(addr);
                }
                // The index is read first ([`Selector::bit_elem_test`],
                // [`Selector::store_bit_elem`]).
                StmtKind::StoreBitElem { array, index, .. } => {
                    self.expr(index);
                    self.address(array);
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
    /// Pairs `(l, p)` of virtual registers where `p`, defined first, is
    /// coloured right after `l` (`regs.rs`, `allocate_apart`'s `late`):
    /// an 8-byte copy whose destination base is loaded before the
    /// source's register ([`Selector::copy8`]).
    colour_after: Vec<(Reg, Reg)>,
    /// Data values coloured as if live across a call: kept apart from D0
    /// and D1 ([`Selector::call_compare_test`]).
    callee_only: Vec<u8>,
    /// Further pairs of values kept in different registers
    /// ([`allocate_apart`]'s `apart`).
    colour_apart: Vec<(Reg, Reg)>,
    /// Call statements: the index in `out` of their first instruction and
    /// of the `jsr`/`bsr`, and the statement ([`held_through_calls`]).
    call_stmts: Vec<(usize, usize, Span)>,
    /// The statement being selected, for diagnostics.
    span: Span,
    /// The index in `out` of the first instruction of the statement being
    /// selected.
    stmt_start: usize,
    /// The statement (its start in `out` and span) that last addressed an
    /// element of another unit's array ([`Self::other_unit_index`]).
    other_elem: Option<(usize, Span)>,
    /// The other units' arrays whose elements were addressed since the
    /// routine's entry or the last call ([`Self::other_unit_index`]).
    other_syms: Vec<String>,
    /// How many elements of another unit's array or through a pointer
    /// loaded from memory were addressed ([`Self::split_base`]): a call
    /// checks whether its first pushed argument formed one.
    split_elems: usize,
    /// By-address slots loaded for one use in a body of several
    /// statements: the register, the start of its statement in `out`, and
    /// the statement ([`fresh_slot_loads`]).
    fresh: Vec<(Reg, usize, Span)>,
    /// The `movea.l Dn,An` copies of pointers held in data registers
    /// ([`Self::copy_to_a`]), by index in `out`.
    ptr_copies: Vec<usize>,
    /// Parameters loaded at entry, and locals held in a register once
    /// assigned ([`register_locals`]): frame offset and the (virtual)
    /// register the value is in.
    pinned: Vec<(i32, Reg)>,
    /// The frame offsets of the locals held in a register
    /// ([`register_locals`]).
    reg_locals: Vec<i32>,
    /// The register-held locals that are webs ([`register_locals`]): frame
    /// offset and the one (virtual) data register that stands for the
    /// variable in the whole body; every assignment writes it
    /// ([`Selector::store_web`]).
    webs: Vec<(i32, u8)>,
    /// The statement being selected is in a body of several statements
    /// and does not directly follow a call there: a slot it uses once is
    /// loaded into a fresh register ([`Self::eval_a`],
    /// [`fresh_slot_loads`]; R4(b)).
    multi: bool,
    /// The body has several statements, or is one `if`/`repeat`
    /// ([`select_body`]).
    multi_body: bool,
    /// The routine copied value parameters on entry
    /// ([`Self::param_copies`]).
    copied: bool,
    /// The index of each block copy of a value parameter (its slot's
    /// `movea.l`, then the `lea` of the copy) and its span, checked after
    /// allocation ([`Self::param_copies`]).
    copy_blocks: Vec<(usize, Span)>,
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
    /// A comparison is being formed as a boolean value (`s<cc>`), not
    /// for a branch ([`Self::test_value`]): the one-bit tests of a
    /// variable in memory are refused there ([`Self::one_bit_test`]).
    test_as_value: bool,
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
    /// Blocks of the unit's other data sections whose address a register
    /// holds: the Pascal data section and the (virtual) address register
    /// loaded with `movea.l #block,An` ([`Self::other_base`]).  A call
    /// ends them.
    other_bases: Vec<(String, u8)>,
    /// The indices in `out` of those loads.
    other_base_loads: Vec<usize>,
    /// The label of the routine's data area, the block of its data
    /// section ([`crate::emit::data_base_label`]).
    data_label: String,
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
    /// The `lea (d,An,Dx),Ar` of element addresses of other units'
    /// arrays or through pointers loaded from memory formed as values
    /// ([`Self::lea_into`]): index in `out` and the statement's span.
    elem_leas: Vec<(usize, Span)>,
    /// Locals assigned from a call, held in D0 and moved to an address
    /// register where dereferenced ([`Park`]).
    parked: Vec<Park>,
    /// Pointer locals assigned once from a call that no call follows,
    /// held in A0 itself from the call to their last dereference
    /// (bead domain-pascal-78n2; [`select_body_with`]).
    a0_locals: Vec<i32>,
    /// The index in `out` of a parked local's `movea.l D0,An` that moves
    /// down to the branch of the `if` being selected ([`Self::park_home`]).
    hoist: Option<usize>,
    /// Those moves: the index of the `movea.l`, of the branch, and the
    /// statement's span.
    hoists: Vec<(usize, usize, Span)>,
    /// The `if` parts a parked local's move goes down into
    /// ([`Self::park_home`]): the local's frame offset and the part's
    /// statements (compared by address).
    park_parts: Vec<(i32, *const Stmt, usize)>,
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
    /// The slots of the routine's parameters passed by address
    /// ([`dpc_ir::Function::address_params`]).
    address_slots: Vec<i32>,
    /// The operands of the references to volatile objects selected so far
    /// ([`Addr::Volatile`]), over virtual registers, with their access.
    volatile_eas: Vec<(Ea, Access)>,
    /// Where [`Self::eval_a`] gave A0 as the base of a pointer local
    /// ([`Self::a0_holds`]): the index in `out` then, checked once the
    /// body is selected (no write of A0 before the base's use).
    a0_bases: Vec<(usize, Span)>,
    /// The registers of the parameters loaded at entry
    /// ([`Self::entry_loads`]), in load order.
    entry_regs: Vec<Reg>,
    /// Those of them that hold a pointer or a by-address slot's address
    /// in a data register (also used as data, [`Self::entry_loads`]).
    data_ptr_regs: Vec<Reg>,
    /// The range of `out` the entry loads take, when any were made: a
    /// test, made right after them, of the register the last data
    /// register load among them loaded branches on that load's flags
    /// ([`Self::tst`]).
    entry_end: Option<std::ops::Range<usize>>,
    /// The index in `out` of the `move` an assignment statement just made
    /// into the data register of a register-held local (`x := y`): a test
    /// of that register right after it branches on the move's flags
    /// ([`Self::tst`]; bead domain-pascal-1gyq).
    assigned_move: Option<usize>,
    /// The moves [`Self::tst`] left the test to (their index in `out`),
    /// with the test's span and whether the move copies a function's
    /// result out of D0: checked once the body is selected
    /// ([`check_assigned_flags`]).
    flag_moves: Vec<(usize, Span, bool)>,
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
    /// The routine's labels ([`StmtKind::Label`]): the local label each
    /// one is, once made.
    user_labels: Vec<Option<u32>>,
    /// Per label, the `goto`s to it in the body, and those selected so
    /// far.
    gotos_total: Vec<u32>,
    gotos_seen: Vec<u32>,
    /// Labels placed whose backward `goto`s are not all selected yet: the
    /// code from such a label on is in a loop whose back edge is not
    /// selected ([`Self::in_loop`]).
    open_back: u32,
    /// Per label, whether its continuation is clean ([`label_tails`]).
    label_clean: Vec<bool>,
    /// The branches `goto` statements emitted, refused when they go to
    /// the next instruction ([`select_body`]).
    goto_jumps: Vec<(usize, Span)>,
}

/// A 4-byte integer local assigned an `a0_return` result
/// ([`integer_call_local`]) held in D0 from its assignment to the statement
/// that first dereferences it or calls a routine, then in an address
/// register (beads domain-pascal-dgb, -yzxr, -78n2; codegen-patterns 3,
/// "Pointer locals held in D0").
///
/// `tools/resultsurvey.py ptrhold` (the first read of A0 after a call of
/// an A0 function, on its fall-through path): of the 34 `move.l A0,D0`
/// copies, 17 are moved to A0 by the first statement that dereferences
/// the value (`movea.l D0,A0`), which later dereferences share: 15
/// TTY_$K_* routines after TTY_$I_GET_DESC (TTY_$K_INQ_FUNC_ENABLED
/// 0xE6763C / 0xE67642), RIP_$UPDATE_INT 0xE159B6, the DIR_ module-local
/// 0xE52394 at 0xE52418; 14 more move to a callee-saved register where a
/// call follows (TTY_$K_FLUSH_INPUT 0xE1C0AE), against 105 `movea.l
/// A0,An` right after the call.  3 locals that no call follows are copied
/// to A1 where D0 and D1 are busy over their range (0xE4D7FC, 0xE528CE,
/// 0xE09C5E): not modelled.  3 stay in A0 (TTY_$I_INQ_RAW 0xE67414,
/// TTY_$K_INQ_FLAGS 0xE674BA, KBD_$INQ_KBD_TYPE 0xE72584), code around
/// which is instruction for instruction that of the D0 sites.  What
/// decides is read from the source's types: the D0 form is an integer
/// local assigned the result and dereferenced through a type transfer,
/// the A0 (or callee-saved) form a pointer local (the integer reading;
/// ../source declares TTY_$I_GET_DESC `integer32; options(a0_return)`
/// and its D0 callers with integer locals).  The D0 holding of pointer
/// locals, by default where no call follows and with `dpc --park-d0`
/// where one does, was retired with that reading (bead
/// domain-pascal-78n2).
#[derive(Clone, Debug)]
struct Park {
    /// The local's frame offset.
    off: i32,
    /// The data register of `move.l A0,Dn` and its index in `out`.
    d: Option<(u8, usize)>,
    /// The address register of `movea.l Dn,An` and its index in `out`.
    home: Option<(u8, usize)>,
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
            data_label: DATA_BASE_LABEL.into(),
            out: Vec::new(),
            vregs: VRegs::new(),
            clobbered: RegSet::EMPTY,
            result_moves: Vec::new(),
            units: Vec::new(),
            colour_after: Vec::new(),
            callee_only: Vec::new(),
            colour_apart: Vec::new(),
            call_stmts: Vec::new(),
            span,
            stmt_start: 0,
            other_elem: None,
            other_syms: Vec::new(),
            split_elems: 0,
            fresh: Vec::new(),
            ptr_copies: Vec::new(),
            pinned: Vec::new(),
            reg_locals: Vec::new(),
            webs: Vec::new(),
            multi: false,
            multi_body: false,
            copied: false,
            copy_blocks: Vec::new(),
            next_label: 1,
            placed: Vec::new(),
            epilogue: None,
            before_bra: Vec::new(),
            call_no_pop: false,
            test_as_value: false,
            in_args: false,
            called: false,
            marith_depth: 0,
            unpopped: false,
            data_absolute: false,
            abs_base: None,
            other_bases: Vec::new(),
            other_base_loads: Vec::new(),
            loops: Vec::new(),
            stepped: Vec::new(),
            loop_jumps: Vec::new(),
            ptr_locals: Vec::new(),
            ptr_defs: Vec::new(),
            elem_leas: Vec::new(),
            parked: Vec::new(),
            a0_locals: Vec::new(),
            hoist: None,
            hoists: Vec::new(),
            park_parts: Vec::new(),
            link: None,
            uplevel: Vec::new(),
            result_web: false,
            web: None,
            web_a: None,
            entry_regs: Vec::new(),
            data_ptr_regs: Vec::new(),
            entry_end: None,
            assigned_move: None,
            flag_moves: Vec::new(),
            stored: Vec::new(),
            escaped: Vec::new(),
            address_slots: Vec::new(),
            volatile_eas: Vec::new(),
            a0_bases: Vec::new(),
            cse_plan: CsePlan::Probe,
            cse: Vec::new(),
            cse_next: 0,
            cse_hits: Vec::new(),
            cse_regs: Vec::new(),
            writes: Vec::new(),
            user_labels: Vec::new(),
            gotos_total: Vec::new(),
            gotos_seen: Vec::new(),
            open_back: 0,
            label_clean: Vec::new(),
            goto_jumps: Vec::new(),
        }
    }

    /// Whether the code being selected is inside a loop, a structured one
    /// or one a backward `goto` not selected yet closes: the analyses of
    /// values held from earlier code ([`Self::forwarded`],
    /// [`Self::a0_holds`], [`Self::cse_valid`]) see only the instructions
    /// selected so far, so not that back edge.
    fn in_loop(&self) -> bool {
        !self.loops.is_empty() || self.open_back > 0
    }

    /// The local label of the routine's label `id`.
    fn user_label(&mut self, id: u32) -> u32 {
        match self.user_labels[id as usize] {
            Some(l) => l,
            None => {
                let l = self.new_label();
                self.user_labels[id as usize] = Some(l);
                l
            }
        }
    }

    /// The routine's label `id` here (`L:` before a statement).  A label
    /// that a `goto` selected later still branches to (backward) heads a
    /// loop: it is aligned like a loop head, with the 0x2048 filler in
    /// aligning routines (codegen-patterns 7, 7.5: of the backward-`goto`
    /// targets of `../source`'s C, those in aligning routines have the
    /// filler, NETWORK_$READ_AHEAD 0xE0FDA0, the REM_FILE module-local
    /// 0xE60FD8 at 0xE61130 and 0xE611E0, or are 4-aligned, BAT_$ALLOCATE
    /// 0xE3B294, FLP_DO_IO 0xE3DE64, PROC2_$WAIT 0xE3FE84; one large outer
    /// loop is not, 0xE0FDE2), and the code from it on is in a loop until
    /// the last such `goto` ([`Self::in_loop`]).  Only forward `goto`s:
    /// a join, not aligned (7, "Other branch targets").
    fn place_user_label(&mut self, id: u32) {
        let l = self.user_label(id);
        let k = id as usize;
        let back = self.gotos_total[k] - self.gotos_seen[k];
        if back > 0 {
            self.out.push(Insn::Align4);
            self.open_back += 1;
        }
        self.place(l);
    }

    /// Account for a `goto` to label `id` just selected.
    fn goto_selected(&mut self, id: u32, at: usize, span: Span) {
        let k = id as usize;
        self.gotos_seen[k] += 1;
        let l = self.user_labels[k].expect("made by the branch");
        if self.placed.contains(&l) && self.gotos_seen[k] == self.gotos_total[k] {
            self.open_back -= 1;
        }
        for i in at..self.out.len() {
            if matches!(self.out[i], Insn::Branch { label, .. } if label == l) {
                self.goto_jumps.push((i, span));
            }
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
            src: Ea::AbsL(self.data_label.clone(), 0),
            dst: a,
        });
        self.abs_base = Some(a);
        Ok(a)
    }

    /// The address register that holds the unit's block of data section
    /// `section`, another than the routine's (codegen-patterns 11, "Data
    /// per section"; survey `tools/sectsurvey.py otherdata`): its address
    /// as an immediate, `movea.l #block,An` (`207c`), where the first
    /// operand needs it, kept for the statements that follow and loaded
    /// again after a call (DXM_$INIT 0xE2FF54 and 0xE2FF66, around `jsr
    /// EC_$INIT`; KBD_$INIT 0xE3336A and 0xE3339E; SUMA_$INIT 0xE33228,
    /// one load for four statements; PROC1_$INIT 0xE2F95C, one for four).
    /// The register is a virtual one, so the allocator gives it the lowest
    /// free (A0 at those sites).  Where a branch target would leave it is
    /// not settled ([`select_body_with`] refuses branches).
    fn other_base(&mut self, section: &str) -> Res<u8> {
        if let Some(&(_, a)) = self.other_bases.iter().find(|(s, _)| s == section) {
            return Ok(a);
        }
        let a = self.alloc_a()?;
        self.other_base_loads.push(self.out.len());
        self.out.push(Insn::Movea {
            size: Size::L,
            src: Ea::ImmSym(crate::emit::data_base_label(section)),
            dst: a,
        });
        self.other_bases.push((section.to_string(), a));
        Ok(a)
    }

    /// Push the address of `a`, an [`Addr::OtherStatic`], as an argument:
    /// the block's start as an immediate, `move.l #block,-(SP)` (`2f3c`;
    /// LOG_$INIT 0xE30054, 0xE3007A, 0xE3010E, 0xE30148; RIP_$INIT
    /// 0xE2FD94; PROC2_$INIT 0xE3070C, 12 sites, none `pea (An)` after a
    /// block load), elsewhere `pea (d,An)` off the block's register
    /// (DXM_$INIT 0xE2FF5A `pea (0x62c,A0)`; 27 sites).
    fn push_other_static(&mut self, a: &Addr) -> Res<()> {
        let Addr::OtherStatic { section, offset } = a else {
            unreachable!("push_other_static of another address")
        };
        if *offset == 0 {
            self.out.push(Insn::Move {
                size: Size::L,
                src: Ea::ImmSym(crate::emit::data_base_label(section)),
                dst: Ea::PreDec(SP),
            });
        } else {
            let ea = self.ea_of(a)?;
            self.out.push(Insn::Pea(ea));
        }
        Ok(())
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
    /// them would be a branch to a `bra`, which compiled code never has;
    /// [`thread_branches`] sends it to the `bra`'s target, and names the
    /// statement in its refusals.
    fn jump(&mut self, cond: Cond, l: u32) {
        if let Some(m) = self.hoist.take() {
            // The branch of the `if` a parked local's move goes down to
            // ([`Self::park_home`]).
            self.hoists.push((m, self.out.len(), self.span));
        }
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
            // The entry copies of value parameters count as earlier
            // statements: after them a one-statement body's slot loads
            // are fresh too (SMD_$SET_TP_CURSOR 0xE6E98C `movea.l
            // (8,A6),A2` after the copies through A0 and A1;
            // [`Self::param_copies`]).
            self.multi = (self.multi_body || self.copied)
                && !(after_call && self.pinned.is_empty() && is_simple(st));
            // What follows this statement up to the epilogue.
            let next = Tail {
                end: only_labels(rest) && tail.end,
                clean: seq_clean(rest, tail.clean, &self.label_clean),
            };
            // A function call in a store's value or a result assignment is
            // followed only by the move of its result: popped unless the
            // statement's continuation is clean (codegen-patterns 1.1;
            // 0xE04700 `bsr.w; move.l D0,(A3); bra.w` with no pop).
            self.call_no_pop =
                matches!(st.kind, StmtKind::SetResult(_) | StmtKind::Store { .. }) && next.clean;
            self.called = false;
            self.unpopped = false;
            self.park_home(f, body, st, rest)?;
            match &st.kind {
                StmtKind::SetResult(e) => self.set_result(f, e)?,
                StmtKind::Store { addr, value } => {
                    self.store(addr, value)?;
                    self.mark_assigned_move(addr);
                    self.wrote(addr, value.ty().bytes());
                }
                StmtKind::StoreBits {
                    addr,
                    field,
                    value,
                    mask,
                } => {
                    self.store_bits(addr, *field, value, *mask)?;
                    self.wrote(addr, field.unit.bytes());
                }
                StmtKind::StoreBitElem {
                    array,
                    bytes,
                    index,
                    lo,
                    value,
                } => {
                    self.store_bit_elem(array, index, *lo, *value)?;
                    self.wrote(array, *bytes);
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
                StmtKind::Repeat { body, until } => {
                    if until.is_none() && !exits_loop(body) && !reached_after_jump(rest) {
                        // Nothing reaches the code after a loop with no
                        // test and no `exit` (bead domain-pascal-pkle
                        // review): dead code, eliminated at `-opt 1` and
                        // above (ref 6.4.24, p. 6-27); the image's 6 such
                        // loops end their routines (codegen-patterns 7.2).
                        return Err(unsupported(
                            rest[0].span,
                            "a statement after a `repeat ... until false` loop that no `exit` \
                             leaves (not reached) is",
                        ));
                    }
                    self.repeat(f, body, until.as_ref())?
                }
                StmtKind::While { cond, body } => self.while_(f, cond, body)?,
                StmtKind::For(l) => self.for_(f, l)?,
                StmtKind::Exit | StmtKind::Next => {
                    let exit = matches!(st.kind, StmtKind::Exit);
                    if !reached_after_jump(rest) {
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
                StmtKind::Label(id) => self.place_user_label(*id),
                // `goto L` (ref "Goto", p. 4-95): a `bra` to the label, as
                // a `return` is one to the epilogue (codegen-patterns 7.5).
                StmtKind::Goto(id) => {
                    if !reached_after_jump(rest) {
                        return Err(unsupported(
                            rest[0].span,
                            "a statement after `goto` (not reached) is",
                        ));
                    }
                    let l = self.user_label(*id);
                    let at = self.out.len();
                    self.jump(Cond::T, l);
                    self.goto_selected(*id, at, st.span);
                }
                StmtKind::Return => {
                    if !reached_after_jump(rest) {
                        return Err(unsupported(
                            rest[0].span,
                            "a statement after `return` (not reached) is",
                        ));
                    }
                    if next.end {
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

    /// Move a parked pointer local ([`Park`]) from its data register to an
    /// address register before the first statement of `body` that refers
    /// to it after its assignment: `movea.l D0,An`, and An holds it from
    /// there on (later dereferences share it, as A0 at
    /// 0xE6C24E..0xE6C284).
    ///
    /// - A statement of the routine's own statement part that is not
    ///   control flow: the move is its first instruction
    ///   (TTY_$K_INQ_INPUT_FLAGS 0xE676DA `movea.l D0,A0; movea.l
    ///   (0xc,A6),A1; move.l (0x14,A0),(A1)`; TTY_$K_SET_SESSION_ID
    ///   0xE679B0, the pointer the destination's base; TTY_$K_SET_DELAY
    ///   0xE67A40, TTY_$K_INQ_PGROUP 0xE6796C, TTY_$K_INQ_FUNC_CHAR
    ///   0xE6759E).
    /// - An `if` of the routine's own statement part whose condition is
    ///   one test that does not read the local and whose two parts both
    ///   refer to it: the move goes before the condition's branch
    ///   ([`select_body_with`]: TTY_$K_SET_INPUT_FLAG 0xE67688).
    /// - An `if` of the routine's own statement part whose condition is
    ///   one test that reads it (and calls nothing): the move is the
    ///   `if`'s first instruction, before the condition's
    ///   (TTY_$K_SET_FUNC_CHAR 0xE67516 `movea.l D0,A2; move.l
    ///   (0x20,A2),D2; btst.l D1,D2; beq.b`, `if func in
    ///   tty^.func_enabled then`).
    /// - An `if` of the routine's own statement part whose condition does
    ///   not read it and calls nothing, with no later statement referring
    ///   to it, and either a `then` part referring to it and no `else`
    ///   part, or an `else` part referring to it after a `then` part that
    ///   ends in a `return` and neither refers to it nor calls: the move
    ///   goes down into the referring part, placed there by these same
    ///   rules (TTY_$K_GET 0xE1C40A `movea.l D0,A2` the first instruction
    ///   of the `then` part of `if status.all = status_$ok then begin ...
    ///   end`, whose false branch 0xE1C406 `bne.w 0xE1C614` goes to the
    ///   result assignment, not the epilogue; the `else` part is the
    ///   same code as the statement after `if ... then begin ...; return
    ///   end`, TTY_$K_SET_FUNC_CHAR 0xE67516 the target of `bls.b`
    ///   0xE6750C).  The parts so entered count as the routine's own
    ///   statement part.
    ///
    /// Elsewhere (inside a statement, a loop or `case`, an `if` whose
    /// condition reads it but is not one test, or calls, one that
    /// dereferences it in one part and after it, a `then` part with an
    /// `else`, an `else` part after a `then` that calls or does not end
    /// in `return`) the place is not seen: refused.
    fn park_home(&mut self, f: &Function, body: &[Stmt], st: &Stmt, rest: &[Stmt]) -> Res<()> {
        for k in 0..self.parked.len() {
            let p = &self.parked[k];
            if p.d.is_none() || p.home.is_some() || !(refers_to(st, p.off) || has_call(st)) {
                continue;
            }
            let (off, (d, _)) = (p.off, p.d.unwrap());
            let top = std::ptr::eq(body, f.body.as_slice())
                || self.park_parts.iter().any(|&(o, ptr, len)| {
                    o == off && std::ptr::eq(ptr, body.as_ptr()) && len == body.len()
                });
            let hoist = match &st.kind {
                _ if !top => None,
                StmtKind::If { cond, then_, else_ } => {
                    let mut in_cond = false;
                    walk_cond(cond, &mut |n| {
                        if let Node::Addr(Addr::Frame(o)) = n {
                            in_cond |= off <= *o && *o < off + 4;
                        }
                    });
                    let simple = matches!(cond, IrCond::Test(_) | IrCond::Compare { .. });
                    let jump = matches!(
                        then_.as_slice(),
                        [Stmt {
                            kind: StmtKind::Return | StmtKind::Exit | StmtKind::Next,
                            ..
                        }]
                    );
                    let in_then = then_.iter().any(|s| refers_to(s, off));
                    let in_else = else_.iter().any(|s| refers_to(s, off));
                    let calls = cond_has_call(cond);
                    if in_cond {
                        (simple && !calls).then_some(false)
                    } else if in_then && in_else {
                        (simple && !jump).then_some(true)
                    } else if in_then != in_else {
                        let (part, other) = if in_then {
                            (then_, else_)
                        } else {
                            (else_, then_)
                        };
                        let later = rest.iter().any(|s| refers_to(s, off));
                        // Only the two shapes seen: the `then` part with
                        // no `else` (TTY_$K_GET), and the `else` part
                        // after a `then` part ending in `return` (the
                        // guard of TTY_$K_SET_FUNC_CHAR); not a `then`
                        // part with an `else` that returns.
                        let other_seen = if in_then {
                            other.is_empty()
                        } else {
                            matches!(
                                other.last(),
                                Some(Stmt {
                                    kind: StmtKind::Return,
                                    ..
                                })
                            )
                        };
                        if !later && !calls && other_seen && !other.iter().any(has_call) {
                            // Placed when the part is selected.
                            self.park_parts.push((off, part.as_ptr(), part.len()));
                            continue;
                        }
                        None
                    } else {
                        None
                    }
                }
                k if is_simple(st) && !matches!(k, StmtKind::Return) => Some(false),
                _ => None,
            };
            let Some(hoist) = hoist else {
                return Err(unsupported(
                    st.span,
                    "the first dereference of a local held in D0 (or the first call after its \
                     assignment) inside a statement, a loop or `case`, or an `if` other than \
                     one test reading it, one test with both parts dereferencing it, a `then` \
                     part alone dereferencing it with no `else`, or an `else` part after a \
                     `then` ending in `return` and calling nothing, with nothing after it \
                     (where the original moves it to an address register is not seen) is",
                ));
            };
            let a = self.alloc_a()?;
            let at = self.out.len();
            self.out.push(Insn::Movea {
                size: Size::L,
                src: Ea::Reg(Reg::D(d)),
                dst: a,
            });
            self.parked[k].home = Some((a, at));
            self.pinned.push((off, Reg::A(a)));
            if hoist {
                self.hoist = Some(at);
            }
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
        // `if c then goto L` likewise: one conditional branch on `c` to
        // the label (codegen-patterns 7.5).
        if let [Stmt {
            kind: StmtKind::Goto(id),
            span: jspan,
        }] = then_
        {
            let l = self.user_label(*id);
            let at = self.out.len();
            self.branch(cond, true, l)?;
            self.goto_selected(*id, at, *jspan);
            return self.seq(f, else_, next);
        }
        // `if c then T else goto L`: a branch on the negated condition to
        // the label, then `T`, as an `if` without `else` part (TTY_$I_RCV
        // 0xE1BBCE..0xE1BBD2 `btst #3,D1; beq.s 0xE1BC38`, the crlf arm's
        // `else goto normal_char`; codegen-patterns 7.5).
        if let [Stmt {
            kind: StmtKind::Goto(id),
            span: jspan,
        }] = else_
        {
            let l = self.user_label(*id);
            let at = self.out.len();
            self.branch(cond, false, l)?;
            self.goto_selected(*id, at, *jspan);
            return self.seq(f, then_, next);
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
    ///
    /// With no test (`until` is `None`: `repeat ... until false`), the
    /// bottom is a `bra` back to the head (codegen-patterns 7.2: 86 loops
    /// closed only by `bra`s; MEM_LITES 0xE0C3D2..0xE0C410, the filler,
    /// the head 0xE0C3D4, `beq.b 0xE0C412` for the `if ... then exit`,
    /// `bra.b 0xE0C3D4`), its head aligned like any other.  `next` goes to
    /// that `bra`, and so to the head ([`thread_branches`]); a body that
    /// ends in an `if c then exit` or a jump leaves no `bra` (also there).
    fn repeat(&mut self, f: &Function, body: &[Stmt], until: Option<&IrCond>) -> Res<()> {
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
        match until {
            Some(c) => self.branch(c, false, head)?,
            None => self.jump(Cond::T, head),
        }
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
        // The count's and the index's set-up instructions (an in-place
        // pointer's `movea.l` goes before the last of them, [`Self::step_in_place`]).
        let mut setup = Vec::new();
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
                setup.push(self.out.len());
                self.out.push(if n == 0 {
                    // One pass: `clr.w`, never `moveq #0` (6 of 6 `dbf`
                    // counters set to 0: SMD_$DM_COND_EVENT_WAIT 0xE6F012
                    // `clr.w D0`, SMD_$FREE_ASID 0xE75262, SMD_$INIT;
                    // codegen-patterns 7.4).
                    Insn::Clr {
                        size: Size::W,
                        dst: Ea::Reg(Reg::D(c)),
                    }
                } else if n <= 127 {
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
                setup.push(self.out.len());
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
            setup.push(self.out.len());
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
        let (arrays, body) = self.step_plan(f, l, arrays, init)?;
        let ptrs = self.stepped_pointers(&arrays, init, &setup)?;
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
            &body,
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
            until: None,
        });
        self.place(exit);
        Ok(())
    }

    /// How each of a `for` loop's stepped arrays gets its pointer
    /// ([`StepForm`]), and the loop's body with its `with a[i] do`
    /// temporaries rebased onto the pointers ([`with_rebase`]); `from` is
    /// the index's initial value (codegen-patterns 7.4, "Forming"; bead
    /// domain-pascal-xm5w; `tools/loopalign.py ptrs`, the `dbf` loops of
    /// compiled code, block copies left out):
    ///
    /// - through a pointer: [`StepForm::New`] (202 `lea (d,An),Ap` from a
    ///   pointer's register; the in-place form, 14 sites, not explained);
    /// - another unit's object: [`StepForm::InPlace`] from `movea.l
    ///   #sym,Ap`, the symbol itself, its offset and the element's in the
    ///   displacements (42 of 42 with an absolute base, none formed with
    ///   `lea`: MAC_$NET_TO_PORT_NUM 0xE0C376 `movea.l #ROUTE_$PORTP,A2`;
    ///   NET_IO_$CREATE_PORT 0xE5A5C4 `movea.l #ROUTE_$USER_STAT,A0; moveq
    ///   #1,D1; lea (144,A0),A0`, the elements at `(-144,A0)`; MMAP_$INIT
    ///   0xE31946 `movea.l #MMAP_$WS_OWNER,A0`, displacement -2);
    /// - the unit's data area or the frame: [`StepForm::New`], `lea
    ///   (from × size,A5|A6),Ap`, when every displacement the body reaches
    ///   an element at through the pointer and that displacement plus
    ///   `from × size` (the element's offset from A5/A6 in the first pass)
    ///   lie in -128..127; [`StepForm::InPlace`] from `movea.l A5|A6,Ap`
    ///   otherwise.  The frame: 52 of 52 (`lea` 35, MSG_$$SEND 0xE0DB7A
    ///   `lea (4,A6),A0` with `(-116,A0)` and `(-20,A0)`; in place 17,
    ///   OS_$INSTALL_DISPLAY_ASTE 0xE6D2BE `movea.l A6,A0; moveq #35,D0;
    ///   addq.l #4,A0` with `(-180,A0)`, then 0xE6D2CE `moveq #7,D0; lea (4,A6),A0`
    ///   with `(-36,A0)` in the next loop; FILE_$FW_PAGES 0xE5E7D2, -132
    ///   but -128 in the first pass).  The unit's data: 57 of 60 (`lea` 28,
    ///   PKT_$RECENTLY_MISSING 0xE128D6 `lea (8,A5),A0`, `(-8,A0)`;
    ///   DBUF_$INVALIDATE 0xE3AA0E; in place 29, EC2_$REGISTER_EC1
    ///   0xE42968, `(2700,A0)`; FLP's module-local 0xE3DC78 at 0xE3DCB6
    ///   `movea.l A5,A1; addq.l #4,A1` with `(124,A0)`..`(127,A0)`, 128..131
    ///   in the first pass).  The three exceptions, ROUTE_$FIND_PORT
    ///   0xE15B12, ROUTE_$FIND_PORTP 0xE15B5E (`(4,A0)`) and the module-
    ///   local 0xE17EAE (`(68,A2)`), all `movea.l A5,An` with `from × size`
    ///   0, are not explained: an array of the unit's data with `from ×
    ///   size` 0 and small displacements (11 `lea (A5),An` against those
    ///   3) is refused.
    ///
    /// Refused besides (not settled): an in-place pointer with any other
    /// stepped pointer in the loop (their order: 0xE6BB2C `movea.l A6,A4;
    /// lea (4,A6),A3; movea.l A6,A2; addq.l #4,A4; lea (32,A2),A2`, but
    /// 0xE0E37A forms a pointer variable's `lea` before the frame's); two
    /// arrays of one element size in the unit's data or the frame (one
    /// pointer for both at MSG_$$SEND 0xE0DB80 `move.l (-116,A0),(-20,A0)`
    /// and DIR_$GET_DEFAULT_ACL 0xE53418, `(5460,A0)` and `(5524,A0)`; two
    /// at RIP_$SERVER 0xE68D64 `lea (A6),A1; lea (A6),A0`); a negative
    /// `from × size` in place (not seen); the unit's data addressed
    /// absolutely (`--a5-absolute`, not seen).
    fn step_plan(
        &self,
        f: &Function,
        l: &ForLoop,
        mut arrays: Vec<SteppedArray>,
        from: i64,
    ) -> Res<(Vec<SteppedArray>, Vec<Stmt>)> {
        let span = self.span;
        let off = l.index.offset;
        let temps = stepped_with_temps(&l.body, &arrays, off, &f.with_temps);
        let body = with_rebase(&l.body, &arrays, &temps);
        let fits = |d: i64| (-128..128).contains(&d);
        for k in 0..arrays.len() {
            let a = &arrays[k];
            let start = from * a.scale as i64;
            let form = match &a.base {
                Addr::Indirect { .. } => StepForm::New,
                Addr::Symbol { .. } => StepForm::InPlace,
                Addr::Static(_) if self.data_absolute => {
                    return Err(unsupported(
                        span,
                        "a `for` loop stepping a pointer through an array of the unit's data \
                         addressed absolutely (`--a5-absolute`; not seen, codegen-patterns \
                         7.4) is",
                    ))
                }
                _ => {
                    let ds = step_displacements(&body, &arrays, k, off, &temps);
                    if ds.iter().all(|&d| fits(d as i64) && fits(d as i64 + start)) {
                        StepForm::New
                    } else {
                        StepForm::InPlace
                    }
                }
            };
            if form == StepForm::New && start == 0 && matches!(a.base, Addr::Static(_)) {
                return Err(unsupported(
                    span,
                    "a `for` loop stepping a pointer from the start of the unit's data area \
                     (`lea (A5),An` 11 times, `movea.l A5,An` 3 times with displacements in a \
                     byte: ROUTE_$FIND_PORT 0xE15B12, 0xE17EAE; codegen-patterns 7.4; bead \
                     domain-pascal-s9tx) is",
                ));
            }
            if form == StepForm::InPlace && start < 0 {
                return Err(unsupported(
                    span,
                    "a `for` loop stepping a pointer formed in place from a negative start (not \
                     seen, codegen-patterns 7.4) is",
                ));
            }
            arrays[k].form = form;
        }
        if arrays.len() > 1 && arrays.iter().any(|a| a.form == StepForm::InPlace) {
            return Err(unsupported(
                span,
                "a `for` loop stepping a pointer formed in place (`movea.l A5|A6|#sym,An`) and \
                 another pointer (their order not settled: 0xE6BB2C, 0xE0E37A; codegen-patterns \
                 7.4; bead domain-pascal-883c) is",
            ));
        }
        for (k, a) in arrays.iter().enumerate() {
            let same = |b: &SteppedArray| {
                b.scale == a.scale
                    && matches!(
                        (&a.base, &b.base),
                        (Addr::Static(_), Addr::Static(_)) | (Addr::Frame(_), Addr::Frame(_))
                    )
            };
            if arrays[..k].iter().any(same) {
                return Err(unsupported(
                    span,
                    "a `for` loop over two arrays of one element size in the unit's data or the \
                     frame (one pointer for both, MSG_$$SEND 0xE0DB80, or two, RIP_$SERVER \
                     0xE68D64: not settled, codegen-patterns 7.4; bead domain-pascal-y1yj) is",
                ));
            }
        }
        Ok((arrays, body))
    }

    /// Form the pointers of a `for` loop's stepped arrays
    /// ([`stepped_arrays`], [`Self::step_plan`], codegen-patterns 7.4) after
    /// its count and index (`setup`, the indices in `out` of their
    /// instructions).  [`StepForm::New`]: each array's pointer is loaded
    /// into an address register (or taken from the one it was loaded into
    /// at entry; A5 for the unit's data, A6 for the frame), all of them
    /// first, then each stepping pointer is formed from it with `lea (from
    /// × size,An),Ap`, `lea (An),Ap` for 0, in the order of first use
    /// (0xE4F010 `movea.l D6,A4; lea (2,A3),A1; lea (2,A4),A0`; 0xE6FCC6
    /// loads its second base before forming the first pointer; LOG_$READ's
    /// worker 0xE177E0 `lea (2,A1),A0; lea (2,A2),A1`; PKT_$DUMP_DATA
    /// 0xE12800 `lea (4,A0),A2` from a pointer loaded at entry; 0xE3E4C6
    /// `lea (A3),A2`; DBUF_$INVALIDATE 0xE3AA0E `lea (36,A5),A2`; VOLX's
    /// FIND_VOLX 0xE6B0DA `moveq #5,D3; moveq #1,D4; lea (32,A5),A0`;
    /// 0xE0E37A `lea (4,A0),A1; lea (4,A6),A0`).  The pointer is the
    /// address of the element at the index *plus the lower bound times the
    /// size* (and less the array's own offset from A5/A6), so the elements
    /// are reached at the displacements the selector forms anyway (the
    /// lower bound and field offsets folded in, section 5.2): 0xE177E8
    /// `move.w (-2,A0),(-2,A1)`.  A new register for each pointer (R2, no
    /// copy coalescing; 0xE177E0 although A1 dies there).  **Not
    /// reproduced:** the forms in place from a pointer (`lea (12,A0),A0`
    /// after `movea.l (12,A6),A0`, 0xE7632C, 0xE76378, 0xE76448; `movea.l
    /// (32,A6),A1; ...; addq.l #2,A1`, 0xE6FCC6), and LOG_$READ's worker's
    /// `movea.l (8,A6),A2` hoisted above the count (0xE177CC; 12.4
    /// R11/R12).  [`StepForm::InPlace`] (the loop's only stepped array):
    /// [`Self::step_in_place`].
    fn stepped_pointers(
        &mut self,
        arrays: &[SteppedArray],
        from: i64,
        setup: &[usize],
    ) -> Res<Vec<u8>> {
        if let [a] = arrays {
            if a.form == StepForm::InPlace {
                return Ok(vec![self.step_in_place(a, from, setup)?]);
            }
        }
        let mut bases = Vec::new();
        for a in arrays {
            bases.push(match &a.base {
                Addr::Indirect { base, .. } => self.eval_a(base)?,
                Addr::Static(_) => self.static_base()?,
                Addr::Frame(_) => FP,
                _ => unreachable!("step_plan forms other arrays' pointers in place"),
            });
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

    /// A stepped pointer formed in place ([`StepForm::InPlace`]): `movea.l
    /// A5|A6|#sym,Ap`, then for a non-zero `from × size` k `addq.l #k,Ap`,
    /// `lea (k,Ap),Ap` above 8 (EC2_$REGISTER_EC1 0xE4296C `addq.l #4,A0`;
    /// RIP_$SERVER 0xE68D94 `addq.l #6,A3`; DISK_$INIT 0xE3C106 `lea
    /// (28,A2),A2`).
    ///
    /// Placement (`tools/loopalign.py ptrs`, the preheaders of the in-place
    /// pointers of A5, A6 and absolute bases): with k = 0 the `movea.l`
    /// comes after the count and the index (AREA_$SHUTDOWN 0xE07F3E `moveq
    /// #10,D3; movea.l A5,A2`; XNS_IDP_$OS_DEMUX 0xE185B6 `moveq #15,D0;
    /// clr.w D1; movea.l A5,A1`; 13 + 7 sites); with an adjustment, the
    /// `movea.l` goes one instruction earlier, before the last set-up
    /// instruction of the count or the index (14 `movea; count; add`:
    /// DISK_$INIT 0xE3C102 `movea.l A5,A2; moveq #63,D2; lea (28,A2),A2`,
    /// MMAP_$INIT 0xE31946, FILE_$PRIV_CREATE 0xE5D63E; 11 `count; movea;
    /// index; add`: TIME_$INIT 0xE2FE94 `moveq #63,D3; movea.l #abs,A2;
    /// moveq #1,D4; lea (12,A2),A2`, PROC2_$PGROUP_INFO 0xE42236,
    /// NET_IO_$CREATE_PORT 0xE5A52C; RIP_$SERVER 0xE68D90 `movea.l A6,A3;
    /// move.w D1,D4; addq.l #6,A3` past the count's copy), but not across
    /// another pointer's `lea` (0xE134D6 `lea (4,A6),A1; movea.l A6,A0;
    /// addq.l #4,A0`) nor a branch (EC2_$WAIT 0xE427D8 `bmi.s; movea.l
    /// A6,A1; addq.l #2,A1`).  Which model this is (a scheduler filling
    /// the `movea`'s stall from above?) is not known; dpc's top-down R11
    /// cannot, so the selector places it.
    fn step_in_place(&mut self, a: &SteppedArray, from: i64, setup: &[usize]) -> Res<u8> {
        let src = match &a.base {
            Addr::Static(_) => Ea::Reg(Reg::A(self.static_base()?)),
            Addr::Frame(_) => Ea::Reg(Reg::A(FP)),
            Addr::Symbol { name, .. } => Ea::ImmSym(name.clone()),
            _ => unreachable!("step_plan forms only constant addresses in place"),
        };
        let k = from * a.scale as i64;
        if !(0..0x8000).contains(&k) {
            return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
        }
        let r = self.alloc_a()?;
        let at = self.out.len();
        self.out.push(Insn::Movea {
            size: Size::L,
            src,
            dst: r,
        });
        if k == 0 {
            return Ok(r);
        }
        self.out.push(if k <= 8 {
            Insn::Addq {
                size: Size::L,
                value: k as u8,
                dst: Ea::Reg(Reg::A(r)),
            }
        } else {
            Insn::Lea {
                src: Ea::Disp(k as i32, r),
                dst: r,
            }
        });
        if at > 0 && setup.contains(&(at - 1)) {
            let m = self.out.remove(at);
            self.out.insert(at - 1, m);
        }
        Ok(r)
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
            // `not (a or b)` on a plain `and`/`or`: the value complemented
            // with `not.b` and the branch on its sign (0xE0AED6..0xE0AEDA
            // `or.b D1,D0; not.b D0; bmi.w`; `tools/boolsurvey.py`, the one
            // `not` read of such a value).
            IrCond::Not(x) if matches!(**x, IrCond::And(..) | IrCond::Or(..)) => {
                let d = self.logic_value(x)?;
                self.out.push(Insn::Unary {
                    op: UnaryOp::Not,
                    size: Size::B,
                    dst: Ea::Reg(Reg::D(d)),
                });
                let cc = if when { Cond::Mi } else { Cond::Pl };
                self.jump(cc, l);
                Ok(())
            }
            IrCond::Not(x) => self.branch(x, !when, l),
            // A plain `and`/`or` whose right operand calls a function: the
            // left operand jumping code, as `and then`/`or else` (ref "Or",
            // p. 4-135: no order of evaluation guaranteed), the right one
            // formed as a boolean and tested ([`Self::call_compare_test`]):
            // TPAD_$SET_UNIT_MODE 0xE6980A..0xE6981A `tst.w (A4); ble.s
            // <then>; jsr SMD_$N_DEVICES; cmp.w (A4),D0; slt D2; tst.b D2;
            // bpl.s <else>`; TPAD_$SET_UNIT 0xE699EE, the `and`, `ble` to
            // the end and `sge D2` (codegen-patterns 7.1, "A plain
            // `or`/`and` whose right operand calls a function"; bead
            // domain-pascal-1rwo; `tools/cmpsurvey.py callcmp`: no and.b/or.b
            // combines a right operand with a call in compiled code).
            IrCond::And(a, b) | IrCond::Or(a, b) if cond_has_call(b) && !cond_has_call(a) => {
                if !matches!(**a, IrCond::Compare { .. }) {
                    // E5DE5A's left operand is a chain of jumping tests:
                    // a nested plain `and`, or `and then`?  Not settled.
                    return Err(unsupported(
                        self.span,
                        "a plain `and`/`or` with a call in its right operand and a left operand \
                         other than a comparison (codegen-patterns 7.1) is",
                    ));
                }
                let short = matches!(c, IrCond::Or(..));
                if when == short {
                    self.branch(a, when, l)?;
                    self.call_compare_test(b, when, l)
                } else {
                    let skip = self.new_label();
                    self.branch(a, short, skip)?;
                    self.call_compare_test(b, when, l)?;
                    self.place(skip);
                    Ok(())
                }
            }
            // A plain `and`/`or` (ref pp. 4-18, 4-134: both operands may be
            // evaluated): the boolean value in a register, the branch on
            // the sign of the `and.b`/`or.b` that formed it (`bpl` 39,
            // `bmi` 31 of the 133 sites of `tools/boolsurvey.py`; TTY_$I_ERR
            // 0xE1BE5E `and.b D3,D1; bmi.b`; 0xE01984 `and.b D0,D1; bpl.s`).
            IrCond::And(..) | IrCond::Or(..) => {
                self.logic_value(c)?;
                let cc = if when { Cond::Mi } else { Cond::Pl };
                self.jump(cc, l);
                Ok(())
            }
            // `odd(x)`: `btst #0` on the integer in a data register, the
            // branch on Z (FIM_$BUILD_DF 0xE0A6F6 `btst #0,D5; beq.s`,
            // 0xE0A8EE `btst #0,D0; bne.s`; [`Self::odd_test`]).
            // A variable in memory: loaded whole, `lsr.b #1` and the carry
            // (tty_$i_put_chars 0xE1B0F6 `move.l (12,A0),D1; lsr.b #1,D1;
            // bcc.s`).
            IrCond::Odd(x) => {
                let (_, _, cc) = self.odd_test(x, false)?;
                self.jump(if when { cc } else { cc.negate() }, l);
                Ok(())
            }
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
                    // longword ([`Self::set_bit_test`]): `btst #k` on the
                    // byte that holds it in memory, or on the set (or its
                    // low word) in a register, the branch on Z; bit 7 of a
                    // byte as its sign (tty_$i_kill_line 0xE1B6B6 `btst
                    // #2,(0x1f,A2); beq.b`; 0xE1B63E `tst.b (9,A2); bmi`;
                    // 0xE031B2 `btst #14,D5; bne.s`).
                    Expr::SetMember {
                        elem,
                        set,
                        set_width,
                    } if matches!(**elem, Expr::Const { .. }) => {
                        let (_, cc) = self.set_bit_test(elem, set, *set_width, false)?;
                        self.jump(if when { cc } else { cc.negate() }, l);
                        return Ok(());
                    }
                    // A variable element of a one-longword set: `btst Dn,Dm`
                    // and the branch on Z ([`Self::set_member_reg`]:
                    // tty_$i_set_funcs 0xE67228 `btst D3,D0; beq.b`;
                    // TTY_$K_SET_DELAY 0xE67A1C `btst D0,D1; bne.b`).
                    Expr::SetMember {
                        elem,
                        set,
                        set_width: Width::Long,
                    } => {
                        self.set_member_reg(elem, set)?;
                        self.jump(if when { Cond::Ne } else { Cond::Eq }, l);
                        return Ok(());
                    }
                    Expr::SetMember { .. } => {
                        return Err(unsupported(
                            self.span,
                            "`in` on a one-word set with an element that is not constant is",
                        ))
                    }
                    // A set larger than a longword: `btst` on the byte
                    // ([`Self::set_member_at`]; 0xE1B750 `btst D1,(0,A0,D2.w);
                    // beq.b`).
                    Expr::SetMemberAt { elem, set, bytes } => {
                        self.set_member_at(elem, set, *bytes)?;
                        self.jump(if when { Cond::Ne } else { Cond::Eq }, l);
                        return Ok(());
                    }
                    // An element of a packed array of booleans: `btst` on
                    // its byte and the branch on Z ([`Self::bit_elem_test`];
                    // PCHIST_$COUNT 0xE1A16C `btst D3,(0x18,A1); beq.b`).
                    Expr::BitElem {
                        array,
                        bytes,
                        index,
                        lo,
                    } => {
                        self.bit_elem_test(array, *bytes, index, *lo)?;
                        self.jump(if when { Cond::Ne } else { Cond::Eq }, l);
                        return Ok(());
                    }
                    // A boolean field of a packed record (codegen-patterns
                    // 5.3): `btst #k` and the branch on Z (0xE0088A `btst
                    // #1,(-47,A6); beq.s`); the field in bit 7 is the sign
                    // of its byte, tested like a whole boolean
                    // (`tst.b`/`bpl`, below; no `btst #7` on memory exists
                    // in compiled code).
                    Expr::Bits { addr, field, ty } if *ty == Scalar::BOOLEAN => {
                        let ea = self.ea_of(addr)?;
                        if field.shift != 7 {
                            self.out.push(Insn::BtstImm {
                                bit: field.shift as u8,
                                dst: ea,
                            });
                            self.jump(if when { Cond::Ne } else { Cond::Eq }, l);
                            return Ok(());
                        }
                        ea
                    }
                    _ => {
                        return Err(unsupported(
                            self.span,
                            "a condition that is a computed boolean value is",
                        ))
                    }
                };
                let cc = if when { Cond::Mi } else { Cond::Pl };
                self.tst(Size::B, ea, cc)?;
                self.jump(cc, l);
                Ok(())
            }
            IrCond::Compare { op, lhs, rhs } => {
                let cc = self.compare(*op, lhs, rhs)?;
                self.jump(if when { cc } else { cc.negate() }, l);
                Ok(())
            }
            IrCond::Block { op, lhs, rhs, size } => {
                self.block_compare(*op, lhs, rhs, *size, when, l)
            }
        }
    }

    /// `lhs = rhs` / `lhs <> rhs` of two records or arrays in memory (ref
    /// Table 4-1, p. 4-2), branching to `l` when the relation is `when`
    /// (codegen-patterns 7.1, "Records"; `tools/cmpsurvey.py`, cmpm).
    /// The left operand's address first, the destination of `cmpm`, then
    /// the right one's, then the count, `moveq #n-1,Dc`:
    ///
    /// - 8 bytes: two `cmpm.l` unrolled, the count loaded but not used,
    ///   the first `bne` leaving as soon as a longword differs
    ///   (TTY_$K_SIMULATE_TERMINAL_INPUT 0xE1C1AC..0xE1C1BC `lea
    ///   (-8,A6),A0; lea (0x4c,A2),A1; moveq #1,D0; cmpm.l (A1)+,(A0)+;
    ///   bne.b; cmpm.l (A1)+,(A0)+; beq.b`; the ACL_ routine 0xE45154;
    ///   all 383 runs of two `cmpm.l` in compiled code have the `moveq #1`);
    /// - 6 bytes: a `cmpm.w` loop of three (TIME_$Q_SCAN_QUEUE 0xE16EC2
    ///   `moveq #2,D0; cmpm.w (A1)+,(A0)+; dbne D0,*-2; beq.s`; 0xE4CEAA);
    /// - 32 bytes and more, a multiple of 4: a `cmpm.l` loop (DISK_IO
    ///   0xE3D84E `moveq #7,D1`, 32 bytes; ACL_$SET_RE_ALL_SIDS 0xE48462
    ///   `moveq #8,D0`, 36 bytes), its head an ordinary loop head
    ///   (codegen-patterns 7).
    ///
    /// Other sizes (12..28 bytes, odd ones) are not seen.
    fn block_compare(
        &mut self,
        op: Rel,
        lhs: &Addr,
        rhs: &Addr,
        size: u32,
        when: bool,
        l: u32,
    ) -> Res<()> {
        // Whether `l` is taken when the two are equal.
        let on_equal = (op == Rel::Eq) == when;
        let cc = if on_equal { Cond::Eq } else { Cond::Ne };
        let skip = self.new_label();
        let (_, unrolled) = self.block_flags(lhs, rhs, size, if on_equal { skip } else { l })?;
        self.jump(cc, l);
        if unrolled && on_equal {
            self.place(skip);
        }
        Ok(())
    }

    /// The comparison of [`Self::block_compare`] up to the flags of its
    /// last `cmpm` (or its `dbne`): Z set when the blocks are equal.  In
    /// the unrolled form the first `bne` goes to `differ`; gives the count
    /// register and whether the form is unrolled.
    fn block_flags(&mut self, lhs: &Addr, rhs: &Addr, size: u32, differ: u32) -> Res<(u8, bool)> {
        let (unit, n) = match size {
            8 => (Size::L, 2),
            6 => (Size::W, 3),
            s if s % 4 == 0 && (32..=512).contains(&s) => (Size::L, s / 4),
            _ => {
                return Err(unsupported(
                    self.span,
                    format!(
                        "comparing records of {size} bytes (only 6, 8 and multiples of 4 from \
                         32 are catalogued, codegen-patterns 7.1) is"
                    ),
                ))
            }
        };
        let dst = self.compared_block(lhs)?;
        let src = self.compared_block(rhs)?;
        let count = self.alloc_d()?;
        self.out.push(Insn::Moveq {
            value: (n - 1) as i8,
            dst: count,
        });
        let cmpm = Insn::Cmpm {
            size: unit,
            src,
            dst,
        };
        if n == 2 {
            self.out.push(cmpm.clone());
            self.jump(Cond::Ne, differ);
            self.out.push(cmpm);
            Ok((count, true))
        } else {
            let head = self.new_label();
            self.out.push(Insn::Align4);
            self.place(head);
            self.out.push(cmpm);
            self.out.push(Insn::Dbf {
                reg: count,
                label: head,
                until: Some(Cond::Ne),
            });
            Ok((count, false))
        }
    }

    /// The address of a compared block ([`Self::block_address`]), or of
    /// another unit's object: `movea.l #sym,An` (201 of the address loads
    /// before a `cmpm` in compiled code, e.g. PACCT_$ON 0xE5A9B2 `movea.l
    /// #UID_$NIL,A1`; `tools/cmpsurvey.py`; no `lea abs.l` of an
    /// operand).
    fn compared_block(&mut self, addr: &Addr) -> Res<u8> {
        if let Addr::Symbol { name, offset: 0 } = addr {
            let a = self.alloc_a()?;
            self.out.push(Insn::Movea {
                size: Size::L,
                src: Ea::ImmSym(name.clone()),
                dst: a,
            });
            return Ok(a);
        }
        self.block_address(addr)
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
    /// tst.l D0; bmi` and 0xE0BEAE do not).
    ///
    /// After a store of a data register into a local's frame slot, a test
    /// of that slot by the next instruction is the store's own flags (bead
    /// domain-pascal-dgb; `tools/resultsurvey.py storetest`, codegen-patterns
    /// 7.1): of the 23 stores `move.<s> Dn,(-k,A6)` the next instruction
    /// tests, 20 are followed by the conditional branch itself
    /// (0xE19BE6 `move.l D0,(-4,A6); bne.s`, a call's result;
    /// 0xE3BCB8 `move.w D4,(-92,A6); bne.s`; 0xE6B180 `seq D1; move.b
    /// D1,(-60,A6); bmi.s`, a boolean), 1 by `tst` of the slot (0xE0B61C,
    /// after `not.b`; not explained) and 2 by `tst` of the register at a
    /// join (0xE554A8; a label between the store and the test is not this
    /// case).  A `move` sets N and Z from the value moved, at its size, and
    /// clears V and C, exactly as `tst` of that value would, so the branch
    /// tests the same relation (codegen-patterns 7.1; the scheduler keeps
    /// the store as the run's last flags setter, `sched.rs`).  After a
    /// store to other memory the memory is tested again (0xE02806 `move.w
    /// D0,(0x412,A0); tst.w (0x412,A0)`; for such stores the survey has 21
    /// branches on the store's flags, 10 tests of the memory and 3 of the
    /// register, by a rule not known).
    ///
    /// `cc` is the condition the caller will branch on.  The survey's
    /// branches on a store's flags are all `beq`/`bne`/`bmi`/`bpl`; no
    /// frame store is followed by an ordered branch (`bgt` .. `bcs`),
    /// with or without a `tst` between them (bead domain-pascal-1vp,
    /// codegen-patterns 7.1), so that shape is refused rather than
    /// extended from the seen ones.
    ///
    /// A data register right after the operation that formed its value
    /// (an expression computed for the test, or a register-held variable
    /// an assignment just formed) is tested as [`Self::flags_test`] says:
    /// the branch on the operation's flags, a `tst`, or a refusal (bead
    /// domain-pascal-qlzu).  Right after the `move` of an assignment to a
    /// register-held local, of the test's size: on the move's flags, or
    /// refused ([`Self::assigned_flags`]; bead domain-pascal-1gyq).  Right
    /// after another load into it of the test's size it is refused (a
    /// load the condition makes: not met in `../source`'s Pascal yet); a
    /// byte widened to a word (`clr.w; move.b`), tested; other sizes,
    /// refused.
    fn tst(&mut self, size: Size, ea: Ea, cc: Cond) -> Res<()> {
        if self.volatile_eas.iter().any(|(v, _)| *v == ea) {
            // A volatile object is read again, whatever the last store
            // left in the flags (ref 3.15.1).
            self.out.push(Insn::Tst { size, dst: ea });
            return Ok(());
        }
        if let Ea::Reg(Reg::D(d)) = ea {
            // Right after the operation that formed the register's value:
            // its flags, a `tst` or a refusal ([`Self::flags_test`]).
            if !self.flags_test(d, size, cc)? {
                return Ok(());
            }
        }
        if let (
            Ea::Disp(k, FP),
            Some(Insn::Move {
                size: s,
                src: Ea::Reg(Reg::D(_)),
                dst: Ea::Disp(d, FP),
            }),
        ) = (&ea, self.out.last())
        {
            if d == k && *k < 0 && *s == size {
                if !matches!(cc, Cond::Eq | Cond::Ne | Cond::Mi | Cond::Pl) {
                    return Err(unsupported(
                        self.span,
                        "an ordered test of a local right after its store (only \
                         `beq`/`bne`/`bmi`/`bpl` on a store's flags are seen, \
                         codegen-patterns 7.1) is",
                    ));
                }
                return Ok(());
            }
        }
        // The parameter the last data-register entry load loaded, tested
        // right after the entry loads: the branch uses the load's flags.
        // In compiled code every `move.<s> (d,A6),Dn` (d >= 8) made before
        // the routine's first branch or call and followed by a test of Dn
        // is followed by the branch itself, 20 of 20, all `beq`/`bne`/
        // `bmi`/`bpl` (tty_$i_set_raw_mode 0xE1BF82 `move.b (0xc,A6),D0;
        // bpl`; TTY_$I_ENABLE_CRASH_FUNC 0xE672A0; AREA_$DELETE 0xE07C34;
        // SIO_$I_INHIBIT_XMIT 0xE1C9D8); none is followed by `tst Dn`.
        // The `movea.l (d,A6),An` entry loads after it leave the flags
        // (MC68000 PRM: MOVEA sets no condition codes) and the branch
        // follows them, 10 of 10 (PROC1_$RESUME 0xE14774..0xE1477C
        // `move.w (8,A6),D0; movea.l (0xa,A6),A0; beq.s`; 0xE02844,
        // 0xE08CF8, 0xE71214), none with a `tst` of that Dn
        // (codegen-patterns 12.2, "One-statement bodies"; bead
        // domain-pascal-s4a5).  Later loads are not settled (below).
        if let (Ea::Reg(r @ Reg::D(_)), Some(range)) = (&ea, &self.entry_end) {
            let last_d = self.out[range.clone()]
                .iter()
                .rev()
                .find(|i| !matches!(i, Insn::Movea { .. }));
            if range.end == self.out.len()
                && matches!(
                    last_d,
                    Some(Insn::Move { size: s, src: Ea::Disp(o, FP), dst: Ea::Reg(d) })
                        if d == r && *s == size && *o >= 8
                )
            {
                if !matches!(cc, Cond::Eq | Cond::Ne | Cond::Mi | Cond::Pl) {
                    return Err(unsupported(
                        self.span,
                        "an ordered test of a parameter right after its entry load (only \
                         `beq`/`bne`/`bmi`/`bpl` on the load's flags are seen, \
                         codegen-patterns 12.2) is",
                    ));
                }
                return Ok(());
            }
        }
        // A load of another size than the test (a byte zero-extended to a
        // word, `clr.w Dn; move.b M,Dn`) is tested: the load's flags are
        // the byte's (`move.b` then `tst.w` of the register 83 times in
        // compiled code, SOCK_$CLOSE 0xE15FB8..0xE15FC0 `clr.w D0; move.b
        // (21,A2),D0; tst.w D0; beq.s`; codegen-patterns 7.1, "A computed
        // value against 0"; bead domain-pascal-qlzu).  Other sizes: a word
        // load tested as a longword (`clr.l Dn; move.w M,Dn; tst.l Dn`) is
        // not seen (no `move.w` then `tst.l` of the register in compiled
        // code; one `move.l; tst.w` at a join, 0xE0EB82, one `move.w; tst.b`,
        // 0xE19106): refused with the same-size load.
        if let (
            Ea::Reg(r),
            Some(Insn::Move {
                size: s,
                src,
                dst: Ea::Reg(d),
            }),
        ) = (&ea, self.out.last())
        {
            if d == r && *s == size && self.assigned_move == Some(self.out.len() - 1) {
                return self.assigned_flags(src.clone(), cc);
            }
            if d == r && !(*s == Size::B && size == Size::W) {
                return Err(unsupported(
                    self.span,
                    "testing a register the previous instruction loaded other than by an \
                     assignment to a local (its flags or a `tst`: not settled, codegen-patterns \
                     7.1) is",
                ));
            }
        }
        self.out.push(Insn::Tst {
            size,
            dst: ea.clone(),
        });
        Ok(())
    }

    /// A test of a register-held local right after the `move` of the
    /// assignment statement before it (`x := y; if x ... then`, bead
    /// domain-pascal-1gyq; `tools/cmpsurvey.py loadflags`, codegen-patterns
    /// 7.1 "Flags of a load"): the branch reads the move's flags, no `tst`.
    /// A `move` sets N and Z from the value moved and clears V and C, as
    /// `tst` does, so every relation with 0 reads the same flags; which of
    /// the two the original emitted is what the survey settles.  The
    /// image cannot tell an assignment from a load made for the test, so
    /// the counts are of every `move.<s> X,Dn` directly followed by a
    /// branch on Dn (or by `tst.<s> Dn` and one), same size, outside a
    /// join:
    ///
    /// - X memory or another data register: 208 on the move's flags and
    ///   none with a `tst` when the branch is the register's only compare
    ///   there (BAT_$FREE's loop 0xE3B590 `move.l (A4),D3; bne.s`;
    ///   0xE072B8 `or.b D1,D0; move.b D0,D4; bmi`): `beq` 78, `bne` 50,
    ///   `bmi` 19, `bpl` 52, `ble` 8 (0xE10104), one `slt`.  When the
    ///   register is compared again right after the branch, both (104 and
    ///   12, the same shapes, no rule found): refused after selection
    ///   ([`check_assigned_flags`]).  Two more `tst` sites are a `for`
    ///   loop's count guard (0xE19020), not an `if`.  `bgt` and `bge` are
    ///   seen neither on a load's flags nor after its `tst`, an `s<cc>`
    ///   once (`slt`, 0xE3358C): refused there too (the branch is known
    ///   only once it is emitted).
    /// - X a function's result, `move.<s> D0,Dn` after the call and its
    ///   pop: 24 on the move's flags, all `beq`/`bne` (XPD_$GET_EC
    ///   0xE5BDDE `move.w D0,D2; beq.s`; VOLX_$DISMOUNT 0xE6B3A4; SEEK
    ///   0xE195DE `move.l D0,D2; bne.s`; 4 compared again, MSG_$WAITI
    ///   0xE59C84), none with a `tst` (one at a join, 0xE16978).  Two of
    ///   the 24 copy a `case` selector into D1; every other is into D2..D7.
    ///   A local in a scratch register is refused after allocation
    ///   ([`check_assigned_flags`]); other branches are refused here.
    fn assigned_flags(&mut self, src: Ea, cc: Cond) -> Res<()> {
        if src == Ea::Reg(Reg::D(0)) {
            if !matches!(cc, Cond::Eq | Cond::Ne) {
                return Err(unsupported(
                    self.span,
                    "an ordered test of a function's result right after its assignment to a \
                     local (only `beq`/`bne` on the copy's flags are seen, codegen-patterns 7.1) \
                     is",
                ));
            }
            self.flag_moves.push((self.out.len() - 1, self.span, true));
            return Ok(());
        }
        if !matches!(
            cc,
            Cond::Eq | Cond::Ne | Cond::Mi | Cond::Pl | Cond::Le | Cond::Gt
        ) {
            return Err(Self::assigned_ordered(self.span));
        }
        self.flag_moves.push((self.out.len() - 1, self.span, false));
        Ok(())
    }

    fn assigned_ordered(span: Span) -> Diagnostic {
        unsupported(
            span,
            "this test of a local right after its assignment (on the move's flags only \
             `beq`/`bne`/`bmi`/`bpl`/`ble` are seen; `bgt`, `bge` and `s<cc>` but one `slt` \
             neither there nor after a `tst`, codegen-patterns 7.1) is",
        )
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
        if (d0_call(lhs) && matches!(rhs, Expr::Load { .. }))
            || (d0_call(rhs) && matches!(lhs, Expr::Load { .. }))
        {
            // Compiled code forms a compare of a function's result with a
            // variable as a boolean and tests it, `jsr; cmp.<s> v,D0;
            // s<cc> Dn; tst.b Dn; bpl/bmi`, at all 14 sites
            // (`tools/cmpsurvey.py callcmp`): 12 in chains of jumping tests,
            // among them the seven TPAD right operands of plain `or`/`and`
            // ([`Self::call_compare_test`]), and 2 alone, DISK_IO 0xE3D86E
            // (an `if`, `sne D2; tst.b D2; bpl`) and 0xE5E590 (ending a
            // `repeat`, `sne D5; tst.b D5; bmi` back to the head: the
            // negated relation?).  None branches on the compare (the
            // branched compares of D0 with a variable are M$ARITH results,
            // 0xE0FD5A, 0xE10AA2, and a routine pointer's kept for a
            // `max`, 0xE3DB34).  Which relation a loop test forms, and the
            // register outside a chain, are not settled (codegen-patterns
            // 7.1; bead domain-pascal-l78t).
            return Err(unsupported(
                self.span,
                "a comparison of a function's result with a variable other than in the right \
                 operand of a plain `and`/`or` (formed as a boolean, the relation and register \
                 not settled: codegen-patterns 7.1) is",
            ));
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
                self.tst(size, ea, cc)?;
                Ok(cc)
            }
            Expr::Const { value, .. } => {
                let mut ea = self.operand(addr)?;
                if let Ea::Reg(Reg::A(_)) = ea {
                    return Err(unsupported(
                        self.span,
                        "comparing an address register with a constant is",
                    ));
                }
                // Right after a store of a data register into a local's
                // frame slot, at the compare's size, the register is
                // compared, not the slot: 23 of the 23 such stores of
                // compiled code that the next instruction compares with a
                // nonzero constant (an ad-hoc scan, codegen-patterns 7.1,
                // "A test right after a store"): the 21 compiled calls of
                // FIM_$CLEANUP that store its `status_$t` into the frame,
                // `move.l D0,(-0x20,A6); cmpi.l #0x120035,D0` (MAC_OS_$SEND
                // 0xE0B604), and two word locals, 0xE1CD8E `move.w
                // D0,(-22,A6); cmpi.w #6,D0`, 0xE63634 `move.w
                // D1,(-0x4c2,A6); cmpi.w #8,D1`.  A volatile local is read
                // again (ref 3.15.1).
                if let (
                    Ea::Disp(k, FP),
                    Some(Insn::Move {
                        size: s,
                        src: src @ Ea::Reg(Reg::D(_)),
                        dst: Ea::Disp(d, FP),
                    }),
                ) = (&ea, self.out.last())
                {
                    if d == k
                        && *k < 0
                        && *s == size
                        && !self.volatile_eas.iter().any(|(v, _)| *v == ea)
                    {
                        ea = src.clone();
                    }
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
                    if held.is_some() {
                        return Err(unsupported(
                            self.span,
                            "comparing two variables when the left one is held in an address \
                             register is",
                        ));
                    }
                    let rheld = match raddr {
                        Addr::Frame(o) => self.pinned(*o),
                        _ => None,
                    };
                    if rheld.is_some() {
                        // `move.<s> lhs,Dn; cmp.<s> Dr,Dn`, or `cmp.<s>
                        // lhs,Dr` with the relation reversed?  Not seen.
                        return Err(unsupported(
                            self.span,
                            "comparing a variable in memory with one held in a register (which \
                             one the original loaded is not known) is",
                        ));
                    }
                    if ty == Scalar::POINTER {
                        // `movea.l p,An; cmpa.l q,An` (20 sites, 0xE01A7E)
                        // or `move.l p,Dn; cmp.l q,Dn`: an integer32 and a
                        // pointer look alike in the image.
                        return Err(unsupported(
                            self.span,
                            "comparing two pointer variables (`cmpa.l` or `cmp.l`: not \
                             catalogued) is",
                        ));
                    }
                    // Two variables in memory: the left one loaded into a
                    // new data register, the right one its operand
                    // (codegen-patterns 7.1: 358 `move.<s> M1,Dn; cmp.<s>
                    // M2,Dn` in compiled code against 4 with both loaded;
                    // TTY_$K_SET_FLAG 0xE67462 `move.w (0x2cc,A2),D0; cmp.w
                    // (0x2ca,A2),D0; beq.b`).  Which operand of the source
                    // was loaded cannot be seen; the left one is.
                    return self.compare_computed(op, lhs, rhs);
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

    /// A pointer parameter held in data register `d` (also used as data,
    /// [`Self::entry_loads`]), copied to an address register to be
    /// dereferenced: `movea.l Dn,An` (codegen-patterns 12.2, "Pointers in
    /// data registers"; EC2_$INIT 0xE42C70 `movea.l D0,A0`; OSINFO_$GET_MMAP
    /// 0xE5C6B0 `movea.l D6,A0`, 0xE5C6B4 `movea.l D3,A1`).  Each
    /// dereference makes a copy, which takes a register as a slot loaded
    /// for one use does (12.4 R4(b), [`fresh_slot_loads`]: 0xE5C6B4 A1
    /// after the copy in A0 of the statement before); after allocation a
    /// copy whose value an address register still holds is dropped and
    /// that register read instead ([`reuse_pointer_copies`]).
    fn copy_to_a(&mut self, d: u8) -> Res<u8> {
        let a = self.alloc_a()?;
        self.ptr_copies.push(self.out.len());
        self.out.push(Insn::Movea {
            size: Size::L,
            src: Ea::Reg(Reg::D(d)),
            dst: a,
        });
        self.fresh.push((Reg::A(a), self.stmt_start, self.span));
        Ok(a)
    }

    /// Whether an argument passed by address at `base` + `offset` is the
    /// object inside one a pointer in memory points to that the push
    /// loads into an address register, `movea.l <pointer>,An; pea
    /// (d,An)` ([`Self::push`]): a pointer in a frame slot not held in a
    /// register (a parameter slot or a pointer local), the enclosing
    /// routine's slot, a pointer in the unit's data, at a displacement
    /// other than 0 (the pointer's own target `p^` is the pointer pushed,
    /// `move.l`; codegen-patterns 1.5).
    fn reloaded_pointer(&self, base: &Expr, offset: i32) -> bool {
        let Expr::Load {
            addr,
            ty: Scalar {
                width: Width::Long, ..
            },
        } = base
        else {
            return false;
        };
        match addr {
            Addr::Frame(o) if self.pinned(*o).is_none() => offset != 0,
            Addr::Enclosing(_) | Addr::Static(_) => offset != 0,
            _ => false,
        }
    }

    /// The register a by-value parameter at frame offset `o` was loaded
    /// into at entry, if it was.
    /// Whether frame offset `o` is the slot of a parameter passed by
    /// address ([`dpc_ir::Function::address_params`]).
    fn is_address_slot(&self, o: i32) -> bool {
        self.address_slots.contains(&o)
    }

    fn pinned(&self, o: i32) -> Option<Reg> {
        self.pinned
            .iter()
            .find(|&&(p, _)| p == o)
            .map(|&(_, r)| r)
            .or_else(|| {
                self.webs
                    .iter()
                    .find(|&&(p, _)| p == o)
                    .map(|&(_, d)| Reg::D(d))
            })
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
    /// against it.  A pointer also used as data is held in a data register
    /// and copied where it is dereferenced ([`Self::copy_to_a`];
    /// codegen-patterns 12.2, "Pointers in data registers").
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
    /// Not so DMA_$INIT_M68450_CHANNEL 0xE0A328, which reloads its
    /// pointer before each of five stores; they are device-register
    /// stores (`dma/init_m68450_channel.c`), and a parameter that is the
    /// base of device references is not loaded at entry
    /// ([`device_param_base`]; codegen-patterns 5.5).  A slot used once is loaded where
    /// it is needed, into a register no earlier statement left a value in
    /// (IIC_$RECEIVE 0xE70AC0..0xE70AD0 A0, A1, A2; PROC2_$ALIGN_CTL
    /// 0xE41D82; [`fresh_slot_loads`]); after a branch or a call, see
    /// [`check_late_slot_loads`].
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
            // A by-address slot is loaded when the body uses it twice or
            // more.  Whether `addr(p)` stored ([`ParamUse::Stored`]) is
            // such a use is split: the module-local 0xE32A60 loads its
            // `(0xc,A6)` at entry (0xE32A6C) for one dereference and one
            // such store, `move.l A0,(0x48,A2)` (0xE32A80), while
            // MAC_OS_$PUT_INFO dereferences `info` once (0xE0C240 `movea.l
            // (0x8,A6),A1`) and stores its address after a call from the
            // slot, 0xE0C322 `move.l (0x8,A6),(-0x8,A6)`.  Where those
            // stores decide it, it is refused (codegen-patterns 12.2; bead
            // domain-pascal-m2w3).  A slot loaded for two other uses is
            // stored from its register (EC_$INIT 0xE15208); one used only
            // by one store is read in place (PKT_$COPY_TO_PA 0xE12538).
            if ty.is_none() && mine.contains(&ParamUse::Stored) {
                let others = mine.iter().filter(|&&u| u != ParamUse::Stored).count();
                if others < 2 && mine.len() >= 2 {
                    return Err(unsupported(
                        f.span,
                        "`addr` of a parameter passed by address stored, its slot used less \
                         than twice besides (loaded at entry at 0xE32A6C, read in place \
                         at MAC_OS_$PUT_INFO 0xE0C322; not settled, codegen-patterns 12.2, bead \
                         domain-pascal-m2w3) is",
                    ));
                }
            }
            if mine.is_empty() || (ty.is_none() && mine.len() < 2) {
                continue;
            }
            // A pointer that is also data is kept in a D register and
            // copied to an address register where it is dereferenced
            // ([`Self::eval_a`]; codegen-patterns 12.2, "Pointers in data
            // registers"): EC2_$INIT 0xE42C64 `move.l (8,A6),D0`, compared
            // at 0xE42C68, `movea.l D0,A0` at 0xE42C70; EC2_$READ
            // 0xE42C82, EC2_$ADVANCE 0xE42CB8, PROC1_$FREE_STACK 0xE15126.
            // A pointer compared with `nil` (or 0) and dereferenced: in
            // an address register at the AREA_ module-local 0xE09E48
            // (`(0xc,A6)` loaded into A3 at entry, 0xE09EAC `cmpa.w
            // #0,A3`, 0xE09EC2 `(4,A3)`), in a data register at the NET_IO
            // module-local 0xE0E238 (0xE0E240 `move.l (8,A6),D3`, 0xE0E270
            // `tst.l D3`, 0xE0E29E `movea.l D3,A3`).  What decides is not
            // known (codegen-patterns 12.2, "Pointers in data registers";
            // bead domain-pascal-nys).
            if mine.contains(&ParamUse::ZeroCompare)
                && mine.contains(&ParamUse::Base)
                && !mine.contains(&ParamUse::Data)
            {
                return Err(unsupported(
                    f.span,
                    "a pointer parameter compared with `nil` or 0 and dereferenced (an address \
                     register at 0xE09EAC, a data register at 0xE0E270; not settled, \
                     codegen-patterns 12.2) is",
                ));
            }
            // A store of the whole pointer is not such a use
            // ([`ParamUse::Stored`]: EC_$INIT 0xE15202 holds its pointer
            // in A0 and stores it from there).
            let class = if mine.contains(&ParamUse::Data) || !mine.contains(&ParamUse::Base) {
                RegClass::Data
            } else {
                RegClass::Addr
            };
            // Each load is a new value; the allocator gives it the lowest
            // register free over its live range (codegen-patterns 12.4 R1):
            // D0, D1, A0, A1 for the sites above, D2/A2 and up for a third
            // value of a class or for one live across a call (R3:
            // module-local 0xE32B76 (`sio/init_dtte.c`), 0xE32B7E `movea.l
            // (8,A6),A2` and 0xE32B82 `move.w (0xc,A6),D2` around three
            // `jsr EC_$INIT`).
            let r = self.alloc(class)?;
            if class == RegClass::Data && (ty.is_none() || mine.contains(&ParamUse::Base)) {
                self.data_ptr_regs.push(r);
            }
            let src = Ea::Disp(offset, FP);
            self.out.push(match r {
                Reg::D(_) => Insn::Move {
                    // A by-address slot holds an address (EC2_$INIT
                    // 0xE42C64 `move.l`).
                    size: ty.map_or(Size::L, |t| size_of(t.width)),
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

    /// Copy the value parameters passed by address into their frame slots
    /// (ref 5.2 Table 5-1, p. 5-5: under the default convention the callee
    /// "Creates local copies of arguments and makes direct references to
    /// them"; codegen-patterns 1.2, "Value parameters passed by address").
    ///
    /// Survey (`tools/framesurvey.py copies`; compiled code,
    /// 0xE00800..0xE88834): the copies are made after the data base and
    /// the entry loads of the other parameters (DIR_$GET_ENTRYU
    /// 0xE4D508..0xE4D51A: `lea abs.l,A5; movea.l (0x18,A6),A2; movea.l
    /// (8,A6),A0; move.l (A0)+,(-8,A6); move.l (A0)+,(-4,A6)`; AST_$SET_DTS
    /// 0xE05540 likewise after a word and a pointer), in parameter order
    /// (FILE_$INVALIDATE 0xE75166..0xE75186), each from its slot loaded into
    /// a register no earlier copy left a value in unless `(An)+` used it up
    /// (12.4 R4(b): A0, A0, A1, A2 at FILE_$INVALIDATE; 133 of the 135
    /// copy loads before a routine's first branch or call, the other two
    /// through a register loaded at entry).  By size:
    ///
    /// - 1, 2, 4 bytes: `move.<s> (An),(d,A6)` (SMD_$SET_TP_CURSOR 0xE6E976
    ///   `move.l`, 0xE6E97E `move.w`; FILE_$INVALIDATE 0xE75186 `move.b`);
    /// - 6 bytes: `move.l (An),(d,A6); move.w (4,An),(d+4,A6)` (all 6
    ///   sites: TONE_$TIME 0xE17304, FILE_$SET_DTM_F 0xE5E208);
    /// - 8 bytes: `move.l (An)+,(d,A6); move.l (An)+,(d+4,A6)` (all 124
    ///   sites: DIR_$GET_ENTRYU 0xE4D516);
    /// - 10 bytes and more: the block copy of codegen-patterns 5.2
    ///   ([`Self::copy`]; XPD_$SET_PTRACE_OPTS 0xE5AFB0..0xE5AFBE, 14
    ///   bytes; OS_$INIT 0xE33808, a `dbf` loop).
    ///
    /// Refused: other sizes (not seen); a copy the routine never refers
    /// to, nor any nested routine (whether the original copied it is not
    /// visible: an empty procedure is a bare `rts`, codegen-patterns 2);
    /// a block copy not from A0 into A1 (checked after allocation, in
    /// [`select_body_with`]: the only form seen); copies in a nested
    /// routine (none catalogued).
    fn param_copies(&mut self, f: &Function) -> Res<()> {
        if f.param_copies.is_empty() {
            return Ok(());
        }
        if f.nesting.is_some() {
            return Err(unsupported(
                f.param_copies[0].span,
                "a value parameter passed by address to a nested routine (its copy is not \
                 catalogued, codegen-patterns 1.2) is",
            ));
        }
        let mut refs: Vec<i32> = Vec::new();
        walk(&f.body, &mut |n| {
            if let Node::Addr(Addr::Frame(o)) = n {
                refs.push(*o);
            }
        });
        let multi = self.multi;
        self.multi = true;
        for pc in &f.param_copies {
            self.span = pc.span;
            let (lo, hi) = (pc.copy, pc.copy + pc.bytes as i32);
            let used = refs.iter().any(|&o| lo <= o && o < hi)
                || f.uplevel_refs
                    .iter()
                    .any(|&(o, n)| o < hi && lo < o + n as i32);
            if !used {
                return Err(unsupported(
                    pc.span,
                    "a value parameter passed by address that the routine never uses (whether \
                     the original copied it is not visible, codegen-patterns 1.2) is",
                ));
            }
            self.stmt_start = self.out.len();
            let from = Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Frame(pc.slot),
                    ty: Scalar::POINTER,
                }),
                offset: 0,
            };
            let to = Addr::Frame(pc.copy);
            match pc.bytes {
                1 | 2 | 4 | 6 | 8 => {
                    let a = self.alloc_a()?;
                    self.out.push(Insn::Movea {
                        size: Size::L,
                        src: Ea::Disp(pc.slot, FP),
                        dst: a,
                    });
                    self.fresh.push((Reg::A(a), self.stmt_start, self.span));
                    let start = self.out.len();
                    let mv = |size, src, d| Insn::Move {
                        size,
                        src,
                        dst: Ea::Disp(pc.copy + d, FP),
                    };
                    match pc.bytes {
                        1 => self.out.push(mv(Size::B, Ea::Ind(a), 0)),
                        2 => self.out.push(mv(Size::W, Ea::Ind(a), 0)),
                        4 => self.out.push(mv(Size::L, Ea::Ind(a), 0)),
                        6 => {
                            self.out.push(mv(Size::L, Ea::Ind(a), 0));
                            self.out.push(mv(Size::W, Ea::Disp(4, a), 4));
                        }
                        _ => {
                            self.out.push(mv(Size::L, Ea::PostInc(a), 0));
                            self.out.push(mv(Size::L, Ea::PostInc(a), 4));
                        }
                    }
                    if self.out.len() - start > 1 {
                        // Like an unrolled block copy (12.4 R10).
                        self.units.push((start, self.out.len()));
                    }
                }
                n if n >= 10 && n % 2 == 0 => {
                    let before = self.out.len();
                    self.copy(&to, &from, n)?;
                    // The slot's register is the source, loaded first.
                    if let Some(Insn::Movea { dst, .. }) = self.out.get(before) {
                        self.fresh.push((Reg::A(*dst), self.stmt_start, self.span));
                    }
                    self.copy_blocks.push((before, pc.span));
                }
                n => {
                    return Err(unsupported(
                        pc.span,
                        format!(
                            "a {n}-byte value parameter passed by address (its copy is not \
                             catalogued, codegen-patterns 1.2) is"
                        ),
                    ))
                }
            }
            self.wrote(&to, pc.bytes);
        }
        self.multi = multi;
        self.copied = true;
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
            src: Ea::AbsL(self.data_label.clone(), 0),
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
    /// - 6 and 8 bytes: [`Self::copy6`], [`Self::copy8`].
    /// - 12 bytes as displacement moves, XNS_IDP_$GET_STATS 0xE18FE6
    ///   `move.l (A5),(A0); move.l (4,A5),(4,A0); move.l (8,A5),(8,A0)`:
    ///   2 sites against 59 `(An)+` copies.  Taken to be three field
    ///   assignments in the source, not one record assignment
    ///   (**Uncertain**: the image cannot tell).
    ///
    /// Address registers: source first, then destination, each the lowest
    /// free (A0, A1 at 0xE14BC6..0xE14BC8; codegen-patterns 12.4 R1).
    fn copy(&mut self, dst: &Addr, src: &Addr, bytes: u32) -> Res<()> {
        match bytes {
            6 => return self.copy6(dst, src),
            8 => return self.copy8(dst, src),
            _ => {}
        }
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
                until: None,
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

    /// `d := s` for a 6-byte record or array (codegen-patterns 5.2,
    /// "6 and 8 bytes"; `tools/storesurvey.py blocks 6`): two moves, memory
    /// to memory, a `move.l` and a `move.w` four bytes on, both operands by
    /// displacement; no 6-byte copy in compiled code post-increments (0 of
    /// 127 runs).  KBD_$RCV 0xE1CDB8 `move.l (-0x10,A6),(0x20,A2); move.w
    /// (-0xc,A6),(0x24,A2)`; TONE_$TIME 0xE17304 `move.l (A0),(-8,A6);
    /// move.w (4,A0),(-4,A6)`; CAL_$SET_DRIFT 0xE68588 `move.l (A0),abs.l;
    /// move.w (4,A0),abs+4.l`.  The operands are those of a longword store
    /// of the first four bytes ([`Self::store`]), so a destination and a
    /// source that both need a register follow its order (the side with
    /// more set-up first, the value on a tie: the source first at
    /// 0xE05104, 0xE1542A, 0xE65184, 0xE747FC, 0xE74828; the destination
    /// at 0xE16DF8, a slot loaded into callee-saved A2).
    fn copy6(&mut self, dst: &Addr, src: &Addr) -> Res<()> {
        if matches!(dst, Addr::Volatile { .. }) || matches!(src, Addr::Volatile { .. }) {
            return Err(unsupported(
                self.span,
                "a 6-byte copy of a volatile object (not seen) is",
            ));
        }
        let start = self.out.len();
        let long = Scalar::signed(Width::Long);
        self.store(
            dst,
            &Expr::Load {
                addr: src.clone(),
                ty: long,
            },
        )?;
        let refused = || {
            unsupported(
                self.span,
                "a 6-byte record or array assignment whose operands are not both in memory is",
            )
        };
        let Some(Insn::Move {
            size: Size::L,
            src: s,
            dst: d,
        }) = self.out.last()
        else {
            return Err(refused());
        };
        let (Some(s2), Some(d2)) = (ea_plus(s, 4), ea_plus(d, 4)) else {
            return Err(refused());
        };
        self.out.push(Insn::Move {
            size: Size::W,
            src: s2,
            dst: d2,
        });
        self.units.push((start, self.out.len()));
        Ok(())
    }

    /// `d := s` for an 8-byte record or array, a UID most often
    /// (codegen-patterns 5.2, "6 and 8 bytes"; `tools/storesurvey.py
    /// blocks 8`): the source's address in an address register, post-
    /// incremented, the destination by displacement, `move.l (Am)+,<d>;
    /// move.l (Am)+,<d+4>` (855 of the 865 8-byte runs in compiled code;
    /// the other 10 move both operands by displacement, adjacent field
    /// assignments such as REM_NAME_$GET_ENTRY 0xE4AD5C, `node :=
    /// curr_node; net := curr_net` in ../source).  The source register
    /// is formed as for a block copy ([`Self::block_address`]; TTY_$K_SET_
    /// INPUT_BREAK_MODE 0xE6784E `lea (A2),A0`, an `in` parameter held in
    /// A2; 0xE0723C `lea (-8,A6),A0`; DIR_$ADD_MOUNT 0xE534CA `movea.l
    /// (8,A6),A0`), and for another unit's object `movea.l #sym,Am` (121
    /// sites name UID_$NIL, 0xE00F8A).  The destination:
    ///
    /// - in the frame, the data area, another unit's data, or through a
    ///   pointer held in a register: its own operand, set up first
    ///   (TTY_$K_SET_PGROUP 0xE6792A `movea.l D0,A0; movea.l (0xc,A6),A1;
    ///   move.l (A1)+,(0x4c,A0); move.l (A1)+,(0x50,A0)`, a pointer local
    ///   parked in D0; 0xE00BF0 `(1144,A5)`; 0xE31C28 `abs.l`);
    /// - the object a pointer slot points to: `movea.l (d,A6),An`, `(An)`
    ///   then `(4,An)`.  The source's register is coloured first (the
    ///   lowest free), the destination's next, at every site where both
    ///   load one instruction.  The source's is loaded first (0xE1A0EA
    ///   `lea (-8,A6),A0; movea.l (8,A6),A1`, 18 sites with a frame
    ///   source; 0xE00F8A `movea.l #UID_$NIL,A0; movea.l (8,A6),A1`, 11),
    ///   but the destination's first when the source is in the data area
    ///   (NAME_$GET_ROOT_UID 0xE589C8 `movea.l (8,A6),A1; lea (0x38,A5),A0`,
    ///   also 0xE589EA, 0xE58A0C: all 3 such sites) or reached through a
    ///   parked pointer local, whose `movea.l D0,An` comes first
    ///   (TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E6 `movea.l D0,A0; movea.l
    ///   (0xc,A6),A3; lea (0x38,A0),A1`, A3 since A1 is the source's and
    ///   A2 holds the status; TTY_$K_INQ_PGROUP 0xE6796C the same).
    ///
    /// Refused: a destination indexed at run time (11 sites, `(d,An,Dn)`,
    /// which side is set up first is not settled), one whose operand takes
    /// other instructions, a source through another pointer expression,
    /// volatile objects (not seen).
    fn copy8(&mut self, dst: &Addr, src: &Addr) -> Res<()> {
        if matches!(dst, Addr::Volatile { .. }) || matches!(src, Addr::Volatile { .. }) {
            return Err(unsupported(
                self.span,
                "an 8-byte copy of a volatile object (not seen) is",
            ));
        }
        let start = self.out.len();
        let slot = match dst {
            Addr::Indirect { base, offset } if (-0x8000..0x7ffc).contains(offset) => {
                match &**base {
                    Expr::Load {
                        addr: Addr::Frame(o),
                        ty:
                            Scalar {
                                width: Width::Long, ..
                            },
                    } if self.pinned(*o).is_none() => Some((*o, *offset)),
                    _ => None,
                }
            }
            _ => None,
        };
        let (from, to) = if let Some((o, k)) = slot {
            // The source through a parked pointer local moved to its
            // address register at this statement's start ([`Self::park_home`]).
            let parked = self.parked.iter().any(|p| {
                let mut hit = false;
                if matches!(p.home, Some((_, at)) if at >= self.stmt_start) {
                    walk_addr(src, &mut |n| {
                        if let Node::Addr(Addr::Frame(o)) = n {
                            hit |= p.off <= *o && *o < p.off + 4;
                        }
                    });
                }
                hit
            });
            let from = self.copy8_source(src)?;
            let s = self.alloc_a()?;
            let d = self.alloc_a()?;
            let load_d = Insn::Movea {
                size: Size::L,
                src: Ea::Disp(o, FP),
                dst: d,
            };
            // Slot loads keep apart from the values of earlier statements
            // (12.4 R4(b), [`fresh_slot_loads`]): A3 at 0xE678E8, A2
            // holding the status pointer loaded at entry.
            self.fresh.push((Reg::A(d), self.stmt_start, self.span));
            if matches!(from, Src8::Slot(_)) {
                self.fresh.push((Reg::A(s), self.stmt_start, self.span));
            }
            let a5 = matches!(&from, Src8::Lea(Ea::Ind(r) | Ea::Disp(_, r)) if *r == DATA_BASE_A);
            if a5 || parked {
                self.out.push(load_d);
                self.copy8_load(from, s);
                self.colour_after.push((Reg::A(s), Reg::A(d)));
            } else {
                self.copy8_load(from, s);
                self.out.push(load_d);
            }
            (s, if k == 0 { Ea::Ind(d) } else { Ea::Disp(k, d) })
        } else {
            let mark = self.out.len();
            let to = self.ea_of(dst)?;
            // Nothing, or a pointer held in a data register copied to an
            // address register (`movea.l Dn,An`, [`Self::copy_to_a`]).
            let simple = matches!(
                &self.out[mark..],
                [] | [Insn::Movea {
                    src: Ea::Reg(Reg::D(_)),
                    ..
                }]
            );
            if !simple || !matches!(to, Ea::Ind(_) | Ea::Disp(..) | Ea::AbsL(..)) {
                return Err(unsupported(
                    self.span,
                    "an 8-byte copy into this kind of destination (indexed, or set up by \
                     several instructions) is",
                ));
            }
            let from = self.copy8_source(src)?;
            let s = self.alloc_a()?;
            if matches!(from, Src8::Slot(_)) {
                self.fresh.push((Reg::A(s), self.stmt_start, self.span));
            }
            self.copy8_load(from, s);
            (s, to)
        };
        let Some(to2) = ea_plus(&to, 4) else {
            return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
        };
        for d in [to, to2] {
            self.out.push(Insn::Move {
                size: Size::L,
                src: Ea::PostInc(from),
                dst: d,
            });
        }
        self.units.push((start, self.out.len()));
        Ok(())
    }

    /// How an 8-byte copy's source register is loaded ([`Self::copy8`]);
    /// a pointer held in a data register is copied to an address register
    /// here ([`Self::copy_to_a`]).
    fn copy8_source(&mut self, src: &Addr) -> Res<Src8> {
        Ok(match src {
            Addr::Static(_) | Addr::Frame(_) => Src8::Lea(self.ea_of(src)?),
            Addr::Symbol { name, offset: 0 } => Src8::Sym(name.clone()),
            Addr::Indirect { base, offset: 0 }
                if matches!(&**base, Expr::Load {
                    addr: Addr::Frame(o),
                    ty: Scalar { width: Width::Long, .. },
                } if self.pinned(*o).is_none()) =>
            {
                let Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } = &**base
                else {
                    unreachable!()
                };
                Src8::Slot(*o)
            }
            Addr::Indirect { .. } => {
                let mark = self.out.len();
                let ea = self.ea_of(src)?;
                let ok = matches!(
                    &self.out[mark..],
                    [] | [Insn::Movea {
                        src: Ea::Reg(Reg::D(_)),
                        ..
                    }]
                );
                if !ok || !matches!(ea, Ea::Ind(_) | Ea::Disp(..)) {
                    return Err(unsupported(
                        self.span,
                        "an 8-byte copy from a pointer not held in an address register is",
                    ));
                }
                Src8::Lea(ea)
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "an 8-byte copy from this kind of address is",
                ))
            }
        })
    }

    /// Load an 8-byte copy's source register `a`: `lea <ea>,An` (`lea
    /// (A5),An` for offset 0 of the data area, 0xE58A10), `movea.l
    /// (d,A6),An` for a pointer slot, `movea.l #sym,An`.
    fn copy8_load(&mut self, from: Src8, a: u8) {
        self.out.push(match from {
            Src8::Lea(src) => Insn::Lea { src, dst: a },
            Src8::Slot(o) => Insn::Movea {
                size: Size::L,
                src: Ea::Disp(o, FP),
                dst: a,
            },
            Src8::Sym(name) => Insn::Movea {
                size: Size::L,
                src: Ea::ImmSym(name),
                dst: a,
            },
        });
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
                        "a block through this pointer expression is",
                    ))
                }
            },
            // A field of the object a pointer held in an address register
            // points to: `lea (d,An),Am` (TTY_$K_SIMULATE_TERMINAL_INPUT
            // 0xE1C1B0 `lea (0x4c,A2),A1`, `tty^.pgroup_uid` with `tty` in
            // A2; codegen-patterns 5.2 lists `lea (d,An)` among the
            // source address forms of copies).  Only when forming the
            // operand needs no instruction: where the original loaded a
            // pointer that is not held is not catalogued.
            Addr::Indirect { .. } => {
                let mark = self.out.len();
                let ea = self.ea_of(addr)?;
                if self.out.len() != mark || !matches!(ea, Ea::Disp(..)) {
                    return Err(unsupported(
                        self.span,
                        "a block through a pointer not held in an address register is",
                    ));
                }
                ea
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "a block (record or array copy or comparison) at this kind of address is",
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
    ///   68000.  How those objects were declared is not known; a store to
    ///   a `device` object ([`Addr::Volatile`]) takes `move.<s> #0`, and
    ///   `move.b #0xff` for `true` (codegen-patterns 5.5).
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
    /// An assignment to a register-held local that is a web
    /// ([`register_locals`]): the value goes into the local's register
    /// `web`, as in TTY_$I_PUT_CHARS (codegen-patterns 12.5; the source in
    /// ../source tty/i_put_chars.pas):
    ///
    /// - `x := x op c` updates the register in place: `addq`/`subq` for
    ///   1..8 (0xE1B07E `addq.w #1,D2`), `addi`/`subi`/`andi`/`ori`/`eori`
    ///   otherwise (0xE1B04A `addi.w #256,D1`);
    /// - `x := x op y` applies the operation to it (0xE1B04E `sub.w
    ///   D0,D1`);
    /// - any other value is computed into it (0xE1B03E..0xE1B046 `move.w
    ///   (978,A0),D1; sub.w (980,A0),D1; subq.w #1,D1`; TTY_$I_BUF_INSERT
    ///   0xE1AF20 `moveq #1,D1`), a constant as [`Self::const_into`] loads
    ///   it and `true` with `st` (TTY_$I_NEWLINE 0xE1B45E `st D0`).  When
    ///   the computation reads the local itself after it starts writing
    ///   its result, the result is computed apart and moved (not seen).
    fn store_web(&mut self, o: i32, web: u8, value: &Expr) -> Res<()> {
        let ty = value.ty();
        let w = ty.width;
        let size = size_of(w);
        let me = Addr::Frame(o);
        let dst = Ea::Reg(Reg::D(web));
        match value {
            // `x := min(x, y)` / `max`: the select in x's register
            // ([`Self::min_max`]; DISK_$SPIN_DOWN 0xE3DB34 `cmp.w D0,D2;
            // bge.b; move.w D0,D2`, as the `if` the image cannot tell from
            // it).
            Expr::Binary {
                op: op @ (BinOp::Min | BinOp::Max),
                lhs,
                rhs,
                ty: bty,
            } if matches!(&**lhs, Expr::Load { addr, ty } if *addr == me && ty == bty)
                && !expr_reads_addr(rhs, &me) =>
            {
                self.min_max(*op, lhs, rhs, *bty, Some(web))?;
                return Ok(());
            }
            // A plain `and`/`or`: formed in a register of its own and moved
            // (0xE072B6..0xE072B8 `or.b D1,D0; move.b D0,D4`; 0xE11CA0..
            // 0xE11CA2 `or.b D1,D6; move.b D6,D5`; 0xE66024..0xE66028 `or.b
            // (-722,A6),D3; move.b D3,D2`).
            Expr::Test(c) if matches!(**c, IrCond::And(..) | IrCond::Or(..)) => {
                let d = self.logic_value(c)?;
                self.out.push(Insn::Move {
                    size: Size::B,
                    src: Ea::Reg(Reg::D(d)),
                    dst,
                });
                return Ok(());
            }
            // A comparison: `s<cc>` into the local's register when the
            // compare computes nothing into a data register (TTY_$K_GET
            // 0xE1C42E `tst.w (0x38,A2); seq D5`, break_mode_flag held in
            // D5, ../source tty/k_get.pas).  When the compare computes a
            // value into a data register first (an index, a loaded word),
            // whether the `s<cc>` goes into that register and is moved, or
            // the whole computation into the local's register, is not
            // seen: refused.
            Expr::Test(c) => {
                let start = self.out.len();
                let d = self.test_value(c)?;
                let last = self.out.len() - 1;
                let computes = self.out[start..last]
                    .iter()
                    .flat_map(|i| crate::regs::access(i).defs)
                    .any(|r| matches!(r, Reg::D(_)));
                if computes
                    || !crate::regs::is_virtual(Reg::D(d))
                    || !matches!(self.out[last], Insn::Scc { .. })
                {
                    return Err(unsupported(
                        self.span,
                        "storing a computed boolean whose compare computes into a data register \
                         to a register-held local (the `s<cc>` register not seen, \
                         codegen-patterns 3) is",
                    ));
                }
                crate::regs::rename(&mut self.out[last], &|r| {
                    if r == Reg::D(d) {
                        Reg::D(web)
                    } else {
                        r
                    }
                });
                return Ok(());
            }
            Expr::Const { value: -1, ty } if *ty == Scalar::BOOLEAN => {
                self.out.push(Insn::Scc { cond: Cond::T, dst });
                return Ok(());
            }
            Expr::Const { value: c, .. } => {
                let signed = imm(*c, w);
                let signed = if w == Width::Byte {
                    signed as i8 as i64
                } else {
                    signed
                };
                if (-128..128).contains(&signed) && !(-128..128).contains(c) {
                    // 0xE149A4 `move.w #-1,D2` (PROC1_$SET_VT): a word
                    // `moveq` cannot tell from `move.w`; 12 such `move.w`
                    // of -1 in the image against 57 negative `moveq`s.
                    return Err(unsupported(
                        self.span,
                        "a constant outside -128..127 that `moveq` could load (`move.w #-1` at                          0xE149A4, `moveq` elsewhere; not settled) assigned to a register-held                          local is",
                    ));
                }
                self.const_into(*c, w, web);
                return Ok(());
            }
            _ => {}
        }
        if let Expr::Binary {
            op,
            lhs,
            rhs,
            ty: bty,
        } = value
        {
            let alu = match op {
                BinOp::Add => Some(AluOp::Add),
                BinOp::Sub => Some(AluOp::Sub),
                BinOp::And => Some(AluOp::And),
                BinOp::Or => Some(AluOp::Or),
                BinOp::Xor => Some(AluOp::Eor),
                _ => None,
            };
            let is_me =
                |e: &Expr| matches!(e, Expr::Load { addr, ty } if *addr == me && *ty == *bty);
            let other = if is_me(lhs) {
                Some(&**rhs)
            } else if is_me(rhs) && op.commutative() {
                Some(&**lhs)
            } else {
                None
            };
            if let (Some(alu), Some(y)) = (alu, other) {
                if !expr_reads_addr(y, &me) {
                    if let Expr::Const { value: c, .. } = y {
                        let c = imm(*c, w);
                        self.out.push(match alu {
                            AluOp::Add | AluOp::Sub if c < 0 => {
                                return Err(unsupported(
                                    self.span,
                                    "adding or subtracting a negative constant is",
                                ))
                            }
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
                        return Ok(());
                    }
                    let v = self.eval(y)?;
                    let v = match v {
                        Val::Mem(_) | Val::Imm(_) if alu == AluOp::Eor => {
                            Val::Tmp(self.in_tmp(v, w)?)
                        }
                        v => v,
                    };
                    self.out.push(Insn::Alu {
                        op: alu,
                        size,
                        src: Self::ea_val(&v),
                        dst: web,
                    });
                    return Ok(());
                }
            }
        }
        let start = self.out.len();
        let v = match a0_call(value) {
            Some(c) => Val::Reg(Reg::A(self.call_a0(c)?)),
            None => self.eval(value)?,
        };
        let src = match v {
            // A virtual register the computation wrote; not M$ARITH's
            // result, `Val::Tmp(0)` (physical D0, written by the `jsr`
            // rather than by an instruction to rename): that one is moved.
            Val::Tmp(t) if crate::regs::is_virtual(Reg::D(t)) => {
                let reads = self.out[start..]
                    .iter()
                    .any(|i| crate::regs::access(i).uses.contains(&Reg::D(web)));
                if !reads {
                    // Computed into the local's register.
                    for i in &mut self.out[start..] {
                        crate::regs::rename(i, &|r| if r == Reg::D(t) { Reg::D(web) } else { r });
                    }
                    return Ok(());
                }
                Ea::Reg(Reg::D(t))
            }
            Val::Reg(r) if r == Reg::D(web) => return Ok(()),
            Val::Imm(c) => {
                self.const_into(c, w, web);
                return Ok(());
            }
            v => Self::ea_val(&v),
        };
        self.out.push(Insn::Move { size, src, dst });
        Ok(())
    }

    fn store(&mut self, addr: &Addr, value: &Expr) -> Res<()> {
        let ty = value.ty();
        let size = size_of(ty.width);
        // `integer32(addr(x))` (or another 4-byte transfer of it) stored
        // as `addr(x)` ([`addr_value`]: NETBUF_$INIT 0xE2F63C, SOCK_$INIT
        // 0xE2FE4C).
        let peeled = self.addr_peeled(value);
        let value: &Expr = &peeled;
        let (plain, access) = addr.unwrap_volatile();
        if matches!(plain, Addr::Frame(o) | Addr::Enclosing(o) if *o > 0) {
            return Err(unsupported(self.span, "assigning to a parameter is"));
        }
        let device = access.is_some_and(|a| a.device);
        if access.is_some_and(|a| a.atomic) {
            // ref 3.15.2 p. 3-57: an assignment to an atomic variable
            // whose right side contains it is handled so that "the value
            // of x is not changed by other processes while it completes"
            // it; Table 3-6 p. 3-55: "perform updates with single
            // instruction when possible".  The single instruction is the
            // in-place update (`addq`, `addi`, `subi`, `andi`, `ori` to
            // memory); what the compiler did when there is none is not
            // known, nor how `device` combines with it (no image site
            // known).
            if expr_reads_addr(value, addr) {
                if device {
                    return Err(unsupported(
                        self.span,
                        "updating an atomic device object (one instruction, ref 3.15.2, or no \
                         read-modify-write, ref 3.15.3) is",
                    ));
                }
                if self.update_in_place(addr, value)? {
                    return Ok(());
                }
                return Err(unsupported(
                    self.span,
                    "an update of an atomic variable that is not one instruction (`x := x + c`, \
                     `-`, `&`, `!` with a constant; ref 3.15.2) is",
                ));
            }
        }
        if let Addr::Frame(o) = addr {
            if self.a0_locals.contains(o) {
                // Held in A0 itself: no copy ([`Self::a0_locals`]).
                let c = a0_call(value).expect("an A0-held local is assigned a call's A0");
                let a = self.call_a0(c)?;
                self.pinned.push((*o, Reg::A(a)));
                return Ok(());
            }
            if let Some(k) = self.parked.iter().position(|p| p.off == *o) {
                // A pointer local that no call follows: `move.l A0,D0`
                // right after the call ([`Park`]; TTY_$K_INQ_FUNC_ENABLED
                // 0xE6763C).
                let c = a0_call(value).expect("a parked local is assigned a call's A0");
                let a = self.call_a0(c)?;
                let d = self.alloc_d()?;
                self.parked[k].d = Some((d, self.out.len()));
                self.out.push(Insn::Move {
                    size: Size::L,
                    src: Ea::Reg(Reg::A(a)),
                    dst: Ea::Reg(Reg::D(d)),
                });
                return Ok(());
            }
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
                if let Expr::AddrOf(x) = value {
                    // `p := addr(x)`, or a `with` statement's record address
                    // ([`dpc_ir::Function::with_temps`]): `lea` straight into
                    // the register (NETLOG_$LOG_IT 0xE71BCA `lea
                    // (0,A0,D1.w),A0`, the element address its eleven field
                    // stores go through).
                    self.lea_into(x, a)?;
                    self.ptr_defs.push((*o, self.out.len() - 1, self.span));
                    if self.pinned(*o).is_none() {
                        self.pinned.push((*o, Reg::A(a)));
                    }
                    return Ok(());
                }
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
                        // `nil` (the pointer 0): `suba.l An,An` at 24
                        // sites; of the 17 `movea.l #0,An`, 15 are an
                        // address the binder relocates (a symbol at 0,
                        // EC_$NIL_EC or TRAP_PAGE in the map), only TIME_$
                        // RTE_INT 0xE163D6 and TIME_$VT_INT 0xE16426 a
                        // literal 0, both pointer results
                        // (`tools/nilsurvey.py areg`).
                        Val::Imm(0) => {
                            return Err(unsupported(
                                self.span,
                                "`nil` (the pointer 0) assigned to a pointer held in an \
                                 address register (`suba.l An,An`, `tools/nilsurvey.py \
                                 areg`; bead domain-pascal-bdi0) is",
                            ))
                        }
                        Val::Imm(_) => {
                            return Err(unsupported(
                                self.span,
                                "a constant assigned to a pointer held in an address \
                                 register is",
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
            if let Some(&(_, web)) = self.webs.iter().find(|&&(p, _)| p == *o) {
                return self.store_web(*o, web, value);
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
                // Not M$ARITH's result in physical D0 (`Val::Tmp(0)`),
                // which the next call or M$ARITH call overwrites: copied
                // into a register of the local's own.
                let d = match v {
                    Val::Tmp(d) if crate::regs::is_virtual(Reg::D(d)) => d,
                    Val::Tmp(d) => self.in_tmp(Val::Reg(Reg::D(d)), ty.width)?,
                    v => self.in_tmp(v, ty.width)?,
                };
                self.pinned.push((*o, Reg::D(d)));
                return Ok(());
            }
        }
        // A device object is not updated in place: ref 3.15.3 p. 3-57,
        // "specifying a variable as device causes the compiler to avoid
        // using instructions that do unnecessary reads" (`bset`/`bclr`,
        // `addq`, `ori` ... on memory read it first).  The value is
        // computed in a register and stored (Uncertain: no image site of
        // an update of a device register is catalogued).
        if !device && self.set_update(addr, value)? {
            return Ok(());
        }
        if let Expr::SetOf { .. } = value {
            // AST_$GET_SEG_MAP 0xE06E44..0xE06E54 builds `[k]` in the set's
            // memory: `clr.l (-124,A6)`, the byte offset guarded by `bcs`,
            // `bset D0,(0,A1,D5.w)` (also FILE_$GET_SEG_MAP 0xE740CE,
            // REM_FILE_$GET_SEG_MAP 0xE62044); when the guard is left out is
            // not catalogued (codegen-patterns 5.1).
            return Err(unsupported(
                self.span,
                "assigning `[x]` with a run-time member to a set (built in memory, \
                 codegen-patterns 5.1) is",
            ));
        }
        if !device && self.update_in_place(addr, value)? {
            return Ok(());
        }
        // A destination and a value that both need a register: the side
        // with more instructions first, the value on a tie (`tools/
        // storesurvey.py summary`: of 210 such stores in compiled code, the
        // side with more set-up instructions first at 116 of 126, the
        // value first at 71 of the 84 ties; codegen-patterns 12.4,
        // "Stores whose destination and value both need a register").
        // TTY_$K_SET_DELAY 0xE67A40 `movea.l D0,A0; move.w (A2),D1;
        // movea.l (0x10,A6),A1; add.w D1,D1; move.w (A1),(0x40,A0,D1.w)`
        // (the element first, interleaved by the scheduler, R11);
        // TTY_$K_INQ_DELAY 0xE67A96 the same with the element as the value;
        // TTY_$K_GET 0xE1C4D6 `lea (0,A2,D2.w),A0; movea.l (-4,A6),A1;
        // move.b (0x2d1,A0),(A1)` (a tie).  A call on either side is not
        // seen (the survey leaves calls out): refused.
        // An element of another unit's array and another object of the
        // same variable in one store: the original reaches the other
        // through a register loaded with the same symbol (0xE19F7A
        // `movea.l #0xE7AEF4,A1; ...; movea.l #0xE7AEF4,A2; ...; lea
        // (0,A1,D0.w),A1; move.l (236,A1),(300,A2)`), `dpc` absolutely
        // (review of bead domain-pascal-rlnm; codegen-patterns 5.2).
        let mut syms: Vec<String> = Vec::new();
        let mut elems: Vec<String> = Vec::new();
        let mut note = |n: Node| match n {
            Node::Addr(Addr::Index { base, .. }) => {
                if let Addr::Symbol { name, .. } = &**base {
                    elems.push(name.clone());
                }
            }
            Node::Addr(Addr::Symbol { name, .. }) => syms.push(name.clone()),
            _ => {}
        };
        walk_addr(addr, &mut note);
        walk_expr(value, &mut note);
        if elems
            .iter()
            .any(|e| syms.iter().filter(|s| *s == e).count() > 1)
        {
            return Err(unsupported(
                self.span,
                "a store reaching an element of another unit's array and another part of the \
                 same variable (through a register loaded with the same symbol, 0xE19F82; \
                 codegen-patterns 5.2; bead domain-pascal-lnzy) is",
            ));
        }
        let both = self.addr_needs_reg(addr) && self.value_needs_reg(value);
        let mut select = false;
        walk_expr(value, &mut |n| {
            if let Node::Expr(Expr::Binary {
                op: BinOp::Min | BinOp::Max,
                ..
            }) = n
            {
                select = true;
            }
        });
        if both && select {
            // The compare-and-select's branch (codegen-patterns 10.1)
            // before or after the destination's register: not seen.
            return Err(unsupported(
                self.span,
                "a store whose destination needs a register of a `max` or `min` value (the \
                 destination's register before or after the branch: not seen) is",
            ));
        }
        if both && (expr_has_call(value) || addr_has_call(addr)) {
            return Err(unsupported(
                self.span,
                "a store whose destination and value both need a register, with a call on \
                 either side, is",
            ));
        }
        let start = self.out.len();
        enum Src {
            Zero,
            True,
            Ea(Ea),
        }
        let src = match value {
            // A device object is cleared with `move.<s> #0`, not `clr`,
            // which reads its operand on the 68000/68010 (MC68000 PRM; ref
            // 3.15.3 p. 3-57, no "gratuitous read-modify-write
            // references"): WIN_ 0xE19112 `move.b #0,(14,A0)`, RING
            // 0xE758E2 `move.b #0,(0x29,A0)`, 0xE758F2, the M68450
            // registers of 0xE0A328 (codegen-patterns 13).  `st` (`Scc`
            // reads its operand too) likewise becomes `move.b #0xff`
            // (0xE0A33E `move.b #-1,(A0)`, `10bc 00ff`).
            Expr::Const { value: 0, ty } if device => Src::Ea(Ea::Imm(imm(0, ty.width))),
            Expr::Const { value: -1, ty } if device && *ty == Scalar::BOOLEAN => {
                Src::Ea(Ea::Imm(imm(-1, ty.width)))
            }
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
            // The low bytes of a parameter loaded at entry: stored from
            // its register (module-local 0xE0A328 at 0xE0A35A `move.b
            // D0,(0x2d,A0)`, the word loaded into D0 at 0xE0A32C).
            Expr::Convert { value: x, ty: t }
                if t.width.bytes() < x.ty().width.bytes()
                    && matches!(&**x, Expr::Load { addr: Addr::Frame(o), .. }
                        if matches!(self.pinned(*o), Some(Reg::D(_)))) =>
            {
                let Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } = &**x
                else {
                    unreachable!()
                };
                Src::Ea(Ea::Reg(self.pinned(*o).unwrap()))
            }
            // The address of another unit's object or of a routine, as an
            // immediate: `move.l #sym,<dst>` (ACL_ 0xE465C6 `move.l
            // #UID_$NIL,(-136,A6)`; MAC_OS 0xE0B9A8 `move.l
            // #MAC_$DEMUX,(-8,A6)`; PROC1_ 0xE14B7A `move.l
            // #PROC1_$TS_END_CALLBACK,(24,A2)`; codegen-patterns 1.6).
            Expr::AddrOf(Addr::Symbol { name, offset: 0 }) => Src::Ea(Ea::ImmSym(name.clone())),
            // `addr(p)` of a parameter passed by address (any mode), the
            // whole object: the address its slot holds, moved from the
            // frame, `move.l (d,A6),<dst>` (codegen-patterns 5.2, "`addr`
            // of a parameter"; bead domain-pascal-mbtx): MAC_OS_$PUT_INFO
            // 0xE0C322 `move.l (0x8,A6),(-0x8,A6)` and PKT_$COPY_TO_PA
            // 0xE12538 (`in`), TTY_$I_INIT 0xE33302 `move.l
            // (0x8,A6),(0x5c,A3)`, SIO2681_$INIT 0xE3344A, SIO6509_$INIT
            // 0xE33526 (`var`), as ../source reconstructs them.  A slot
            // held in a register (12.2) is stored from it, as a by-value
            // pointer is ([`ParamUse::Stored`]): EC_$INIT 0xE15208 `move.l
            // A0,(0x4,A0)`, `addr(ec)` of its `out` parameter.
            Expr::AddrOf(a) if by_address_slot(a).is_some_and(|o| self.is_address_slot(o)) => {
                let o = by_address_slot(a).unwrap();
                match self.pinned(o) {
                    Some(r) => Src::Ea(Ea::Reg(r)),
                    None => Src::Ea(Ea::Disp(o, FP)),
                }
            }
            // `addr(p)` of an enclosing routine's parameter passed by
            // address: the slot's value through the static link; the `lea
            // (An),Am` of the generic form below has no `addr` site in
            // compiled code (codegen-patterns 5.2, "`addr` of a
            // parameter"; bead domain-pascal-ucss).
            Expr::AddrOf(Addr::Indirect { base, offset: 0 }) if matches!(&**base, Expr::Load { addr: Addr::Enclosing(o), .. } if *o > 0) =>
            {
                return Err(unsupported(
                    self.span,
                    "`addr` of an enclosing routine's parameter passed by address, stored (no \
                     site, codegen-patterns 5.2; bead domain-pascal-ucss) is",
                ));
            }
            // `addr(x)` of an object of this unit's data or frame, or one
            // reached through a pointer or an index: `lea` into an address
            // register, then `move.l An,<dst>` ([`Self::addr_into_a`]).
            Expr::AddrOf(a) => Src::Ea(Ea::Reg(Reg::A(self.addr_into_a(a)?))),
            // A plain `and`/`or` stored: the value formed in a data register,
            // then `move.b Dn,<dst>` (`tools/boolsurvey.py`, 11 stores:
            // 0xE38630..0xE38632 `and.b D4,D1; move.b D1,(631,A3)`; 0xE0AF56
            // `or.b D2,D1; move.b D1,(A5)`; 0xE34C5E..0xE34C60).
            Expr::Test(c) if matches!(**c, IrCond::And(..) | IrCond::Or(..)) => {
                Src::Ea(Ea::Reg(Reg::D(self.logic_value(c)?)))
            }
            // A comparison stored as a boolean: `s<cc>` into a data
            // register by the rule of a boolean value ([`Self::test_value`]),
            // then `move.b Dn,<dst>`.  Compiled code never sets a byte in
            // memory with `s<cc>` other than `st`/`sf` (`tools/boolsurvey.py
            // scc`: of the `s<cc>` writes, none to memory; 56 into a data
            // register followed by `move.b Dn,<mem>`): TTY_$K_GET 0xE1C506
            // `cmp.w (A3),D3; seq D1; move.b D1,(-0x1e,A6)`; 0xE5CF30
            // `cmpi.w #3,(A0); seq D1; move.b D1,(294,A5)`; 0xE650CC
            // `cmpi.w #1,abs.l; sgt D3; move.b D3,(10,A1)` (codegen-patterns
            // 3, "Comparisons as boolean results").
            Expr::Test(c) => Src::Ea(Ea::Reg(Reg::D(self.test_value(c)?))),
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
                // The low bytes of a variable, moved from memory
                // ([`Self::convert`]; APP_$DEMUX 0xE00AF6 `move.w
                // (0x1a,A0),(-0x14,A6)`).
                Val::Mem(ea) => Src::Ea(ea),
                v => Src::Ea(Ea::Reg(Reg::D(self.in_tmp(v, ty.width)?))),
            },
        };
        let mid = self.out.len();
        let dst = self.ea_of(addr)?;
        if both
            && self.split_base(plain)
            && (self.out.len() - mid > mid - start
                || self.out[start..mid].iter().any(|i| {
                    crate::regs::access(i)
                        .defs
                        .iter()
                        .any(|r| matches!(r, Reg::A(_)))
                }))
        {
            // An element of another unit's array or through a loaded
            // pointer as the destination, the value needing an address
            // register: the original sets the value up before the
            // destination's base (TERM_$CONTROL 0xE66B40..0xE66B56, the ×56
            // index, `movea.l (16,A6),A0`, then `movea.l #DTTE,A1`;
            // NET_IO_$SEND 0xE0E8A6..0xE0E8B2; PROC2_$FORK 0xE731EA..0xE731FA
            // `lea (-228,A3),A0` between the index load and its shift, the
            // base last; 0xE6AA1C the value's pointer first), where `dpc`
            // puts the destination first; with the value first, an
            // address register's set-up is not seen (review of bead
            // domain-pascal-rlnm; codegen-patterns 5.2).
            return Err(unsupported(
                self.span,
                "a store into an element of another unit's array or through a pointer loaded \
                 from memory whose value needs a register set up (the value before the base, \
                 TERM_$CONTROL 0xE66B4C; codegen-patterns 5.2; bead domain-pascal-lnzy) is",
            ));
        }
        if both && self.out.len() - mid > mid - start {
            // The destination's instructions first, unless they read what
            // the value's wrote (a shared base: not seen).
            let defs: Vec<Reg> = self.out[start..mid]
                .iter()
                .flat_map(|i| crate::regs::access(i).defs)
                .collect();
            if self.out[mid..]
                .iter()
                .flat_map(|i| crate::regs::access(i).uses)
                .any(|r| defs.contains(&r))
            {
                return Err(unsupported(
                    self.span,
                    "a store whose destination, set up by more instructions than the value, \
                     reads a register the value's instructions wrote is",
                ));
            }
            let (n_src, end) = (mid - start, self.out.len());
            self.out[start..].rotate_left(n_src);
            // The indices into `out` recorded while the two sides were
            // selected move with their instructions (an index equal to
            // `mid` is taken as the destination's first instruction: for
            // a position after the value's last one, [`Cse::at`], that
            // is earlier than it now is, which only makes
            // [`Self::cse_valid`] check more).
            let moved = |p: &mut usize| {
                if (start..mid).contains(p) {
                    *p += end - mid;
                } else if (mid..end).contains(p) {
                    *p -= n_src;
                }
            };
            self.ptr_copies.iter_mut().for_each(moved);
            self.a0_bases.iter_mut().for_each(|(p, _)| moved(p));
            self.cse.iter_mut().for_each(|c| moved(&mut c.at));
        }
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
            if *o < 0 && !self.in_loop() && !self.escaped.contains(o) && *r != Reg::A(0) {
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
    /// no call there, no write of the register, no assignment of a
    /// register-held local ([`register_locals`]), no other store to the
    /// slot ([`may_write_frame`]: none to the frame at a run-time index,
    /// none through a pointer when any frame object's address escapes),
    /// and the local's address never taken ([`escaped_frame_objects`]).
    fn forwarded(&self, o: i32, size: Size) -> Option<Reg> {
        if self.in_loop() {
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
        // The registers of the register-held locals (12.5): the original
        // gives variables their registers first, so one assigned in
        // between may take the stored value's register (TTY_$I_BUF_PUT:
        // the token stored from D0 at 0xE1AF60 is pushed from its slot at
        // 0xE1AF88, after `new_tail` took D0 at 0xE1AF6C).
        let locals: Vec<Reg> = self
            .webs
            .iter()
            .map(|&(_, d)| Reg::D(d))
            .chain(
                self.reg_locals
                    .iter()
                    .filter_map(|&l| self.pinned.iter().find(|&&(p, _)| p == l).map(|&(_, r)| r)),
            )
            .collect();
        let clobbered = region.iter().any(|i| {
            let defs = crate::regs::access(i).defs;
            locals.iter().any(|l| defs.contains(l))
                || matches!(
                    i,
                    Insn::Jsr(_)
                        | Insn::Bsr { .. }
                        | Insn::Pea(Ea::Disp(_, FP))
                        | Insn::Lea {
                            src: Ea::Disp(_, FP),
                            ..
                        }
                )
                || defs.contains(&r)
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
        if self.in_loop() {
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

    /// After an assignment to `addr`: when it is a local held in a data
    /// register and the statement ended with a `move` into that register
    /// (a load, a copy of another register or of a call's D0), remember
    /// the move for a test of the register right after it ([`Self::tst`]).
    fn mark_assigned_move(&mut self, addr: &Addr) {
        self.assigned_move = None;
        let Addr::Frame(o) = addr.unwrap_volatile().0 else {
            return;
        };
        let Some(Insn::Move {
            dst: Ea::Reg(Reg::D(d)),
            ..
        }) = self.out.last()
        else {
            return;
        };
        let held = self.webs.contains(&(*o, *d))
            || (self.reg_locals.contains(o) && self.pinned.contains(&(*o, Reg::D(*d))));
        if held {
            self.assigned_move = Some(self.out.len() - 1);
        }
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
        if !self.in_loop() {
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
        if self.in_loop() || c.at > self.out.len() {
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
            // Another data section's block: a register loaded with its
            // address unless one still holds it ([`Self::other_base`]).
            Addr::OtherStatic { section, .. } => {
                !self.other_bases.iter().any(|(s, _)| s == section)
            }
            Addr::Index {
                base, index, scale, ..
            } => self.stepped_pointer(base, index, *scale).is_none(),
            Addr::Indirect { base, .. } => !matches!(
                &**base,
                Expr::Load { addr: Addr::Frame(o), .. } if self.pinned(*o).is_some()
            ),
            a @ Addr::Volatile { addr, .. } => {
                device_param_base(a).is_some() || self.addr_needs_reg(addr)
            }
        }
    }

    /// Whether `value` as a store's source takes a register of its own.
    fn value_needs_reg(&self, value: &Expr) -> bool {
        match value {
            Expr::Const { .. } | Expr::AddrOf(Addr::Symbol { offset: 0, .. }) => false,
            // `addr(p)` of a parameter passed by address: its slot, read
            // in place ([`Self::store`]).
            Expr::AddrOf(a) if by_address_slot(a).is_some_and(|o| self.is_address_slot(o)) => false,
            Expr::Load { addr, .. } => self.addr_needs_reg(addr),
            // The low bytes of a parameter loaded at entry into a data
            // register: stored from that register (module-local 0xE0A328
            // at 0xE0A35A `move.b D0,(0x2d,A0)`, a word loaded at entry
            // into D0 by 0xE0A32C).
            Expr::Convert { value: x, ty }
                if ty.width.bytes() < x.ty().width.bytes()
                    && matches!(&**x, Expr::Load { addr: Addr::Frame(o), .. }
                        if matches!(self.pinned(*o), Some(Reg::D(_)))) =>
            {
                false
            }
            // The low bytes of a variable in memory, read in place
            // ([`Self::convert`]).
            Expr::Convert { value: x, ty } if narrows_in_place(x, *ty) => self.value_needs_reg(x),
            Expr::SetMember { .. }
            | Expr::SetMemberAt { .. }
            | Expr::BitElem { .. }
            | Expr::SetOp { .. }
            | Expr::SetOf { .. }
            | Expr::Bits { .. }
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
    ///    (0xE38B42/0xE38B44): every such pair was merged.  `false`,
    ///    cleared as a word, counts as a word 0 (0xE73FFC).  A run of
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
            // A byte 0 (`false`) is a word 0 too: [`Self::push`] clears
            // its whole slot with `clr.w -(SP)`, which compiled code never
            // follows with another word constant (above), and FILE_$CHECK_
            // PROT's boolean `ignore_super` (read as a byte by ACL_$RIGHTS,
            // `move.b (A3),-(SP)` at 0xE46A54, through `pea (0x12,A6)` at
            // 0xE5D226) and the word after it are pushed as one `clr.l
            // -(SP)` at both call sites (FILE_$TRUNCATE_D 0xE73FFC,
            // 0xE43DAC).
            Arg::Value {
                value:
                    Expr::Const {
                        value: 0,
                        ty:
                            Scalar {
                                width: Width::Byte, ..
                            },
                    },
                slot: 2,
            } => Some(0),
            _ => None,
        };
        // A pointer an argument passed by address is reached through and
        // that the pushes load ([`Self::reloaded_pointer`]), read again by
        // the argument list: the original loads it once for both
        // (0xE74706 `movea.l (-0xbc,A6),A1; pea (-0x38,A1); pea
        // (-0x34,A1)`; 0xE3CA4E, 0xE748A4), `dpc` would load it twice.
        for a in &c.args {
            let Arg::Address {
                addr: Addr::Indirect { base, offset },
                ..
            } = a
            else {
                continue;
            };
            if !self.reloaded_pointer(base, *offset) {
                continue;
            }
            let mut n = 0;
            let mut v = |node: Node| {
                if matches!(node, Node::Expr(e) if e == &**base) {
                    n += 1;
                }
            };
            for a in &c.args {
                match a {
                    Arg::Value { value, .. } | Arg::Temp { value, .. } => walk_expr(value, &mut v),
                    Arg::Address { addr, .. } => walk_addr(addr, &mut v),
                }
            }
            if n > 1 {
                return Err(unsupported(
                    self.span,
                    "an argument list that reads twice a pointer it loads to pass an object by \
                     address (one load for both, 0xE74706; not modelled) is",
                ));
            }
        }
        let mut k = c.args.len();
        let split0 = self.split_elems;
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
            // A byte constant other than 0 and `true` is a word push
            // ([`Self::push`]), so it may merge with a word constant next
            // to it as `false` does; no such pair is in compiled code (the
            // 26 nonzero byte constants have a pad, a pointer, `st` or a
            // non-constant next to them), so how is not known.
            let other_byte = |a: &Arg| {
                matches!(a, Arg::Value {
                    value: Expr::Const { value, ty: ty @ Scalar { width: Width::Byte, .. } },
                    slot: 2,
                } if *value & 0xff != 0 && !(*value == -1 && *ty == Scalar::BOOLEAN))
            };
            if other_byte(a)
                && [k.checked_sub(2).map(|i| &c.args[i]), c.args.get(k)]
                    .into_iter()
                    .flatten()
                    .any(|b| word_const(b).is_some() || other_byte(b))
            {
                return Err(unsupported(
                    self.span,
                    "a byte constant argument other than `true` and 0 next to a word or byte \
                     constant (merged into one longword push?) is",
                ));
            }
            self.push(a)?;
            if k == c.args.len() && c.pad == 2 && self.split_elems > split0 {
                // The first pushed argument formed an element of another
                // unit's array or through a loaded pointer, after the pad:
                // the original sets it up before the pad (NAME_$FREE_ASID
                // 0xE74DD8 `movea.l #NAME_$NODE_DATA_UID,A1; subq.l #2,SP;
                // pea (40,A1,D2.w)`; MSG_$RCV_CONTIGI 0xE59900 `movea.l
                // (44,A6),A3; move.w (A4),D1; subq.l #2,SP; pea
                // (0,A3,D1.w)`; review of bead domain-pascal-rlnm).
                return Err(unsupported(
                    self.span,
                    "a call with a pad whose first pushed argument is an element of another \
                     unit's array or through a pointer loaded from memory (set up before the \
                     pad, NAME_$FREE_ASID 0xE74DDE; codegen-patterns 1.4, 5.2; bead \
                     domain-pascal-lnzy) is",
                ));
            }
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
        let insn = match &c.callee {
            // codegen-patterns 1.6: another unit's routine, or one of
            // this unit in another code section (PROC1_$INIT 0xE2F9EC
            // `jsr 0xE14B12`).
            Callee::External(sym) | Callee::Section(sym) => Insn::Jsr(Ea::AbsL(sym.clone(), 0)),
            Callee::Local(sym) => Insn::Bsr {
                target: sym.clone(),
                short: true,
            },
            Callee::Pointer(x) => Insn::Jsr(Ea::Ind(self.call_target(c, x)?)),
        };
        self.out.push(insn);
        self.call_stmts.push((start, self.out.len() - 1, self.span));
        // The block of another data section is loaded again after a call
        // (DXM_$INIT 0xE2FF66, KBD_$INIT 0xE3339E; [`Self::other_base`]),
        // and so is another unit's array (27 reloads of a `movea.l #`
        // after a call in a run without branch targets, 3 bases held
        // across one; [`Self::other_unit_index`]).
        self.other_bases.clear();
        self.other_syms.clear();
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

    /// The address register a call through a routine pointer jumps through
    /// (codegen-patterns 1.6, survey of the 74 `jsr (An)` sites in
    /// compiled code with compiler frames, `tools/indcallsurvey.py`): the
    /// pointer is read into a new address register right after the pushes,
    /// in all 69 sites whose register is defined in the straight-line code
    /// before the call:
    /// - from memory, `movea.l <ea>,An` (KBD_$RCV 0xE1CE5C `movea.l
    ///   (A2),A0`; RING_INTERRUPT 0xE0AB1C `movea.l (A5),A0`; TTY_$I_ERR
    ///   0xE1BE1A `movea.l (0x2c0,A0),A1`, A0 being the base the
    ///   arguments use);
    /// - from a data register holding it, `movea.l Dn,An` (DISK_$REVALID
    ///   0xE3DB8E `movea.l D2,A1`, IO_$INIT 0xE329D0 `movea.l D3,A0`).
    ///
    /// The register is the allocator's (codegen-patterns 12.4), kept apart
    /// from the values of the argument list like theirs
    /// ([`argument_list_loads`]): A0 at 35 sites, A1 at 33, A2-A4 at 6.
    /// Uncertain: the image's A1 where A0 looks free (DISK_$REVALID,
    /// MAC_OS_$OPEN 0xE0B3EC).  A pointer held in an address register
    /// (a pinned parameter or local, a pointer function's A0) is refused:
    /// the only such sites are the 5 NET_ calls through the A0 a pointer
    /// function returned in an earlier statement (NET_$OPEN 0xE5A1D8 ..
    /// 0xE5A1F4), which `dpc` does not model.
    ///
    /// Refused too: a pointer reached through a slot (a by-address or
    /// pointer parameter not loaded at entry) that the arguments also
    /// read.  The original loads the slot once for both (TTY_$I_ERR
    /// 0xE1BE14..0xE1BE1A `movea.l (8,A6),A0; move.l (A0),-(SP); movea.l
    /// (0x2c0,A0),A1`; also 0xE1B368, SIO_$I_TSTART 0xE1C932), which
    /// `dpc` does not do.
    fn call_target(&mut self, c: &Call, x: &Expr) -> Res<u8> {
        if let Expr::Load {
            addr: Addr::Indirect { base, .. },
            ..
        } = x
        {
            if let Expr::Load {
                addr: Addr::Frame(o),
                ..
            } = &**base
            {
                let mut shared = false;
                if self.pinned(*o).is_none() {
                    let mut v = |n: Node| {
                        if let Node::Expr(Expr::Load {
                            addr: Addr::Frame(p),
                            ..
                        }) = n
                        {
                            shared |= p == o;
                        }
                    };
                    for a in &c.args {
                        match a {
                            Arg::Value { value, .. } | Arg::Temp { value, .. } => {
                                walk_expr(value, &mut v)
                            }
                            Arg::Address { addr, .. } => walk_addr(addr, &mut v),
                        }
                    }
                }
                if shared {
                    return Err(unsupported(
                        self.span,
                        "a call through a routine pointer reached through a parameter slot the \
                         arguments also read (one load for both, TTY_$I_ERR 0xE1BE14) is",
                    ));
                }
            }
        }
        if let Expr::Load {
            addr: Addr::Frame(o),
            ..
        } = x
        {
            match self.pinned(*o) {
                Some(Reg::D(d)) => {
                    let a = self.alloc_a()?;
                    self.out.push(Insn::Movea {
                        size: Size::L,
                        src: Ea::Reg(Reg::D(d)),
                        dst: a,
                    });
                    return Ok(a);
                }
                Some(Reg::A(_)) => {
                    return Err(unsupported(
                        self.span,
                        "a call through a routine pointer held in an address register is",
                    ))
                }
                None if *o < 0 && self.a0_holds(*o) => {
                    return Err(unsupported(
                        self.span,
                        "a call through a routine pointer still held in A0 is",
                    ))
                }
                None => {}
            }
        }
        self.eval_a(x)
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
    ///   No `clr.b -(SP)` or `move.b #n,-(SP)` exists in compiled code:
    ///   any other byte constant is a word with the byte in the high half,
    ///   `move.w #n<<8,-(SP)` (26 sites whose callee reads a byte slot,
    ///   TTY 0xE1AFC0 `move.w #-0x200` for chr(254)).  Refused: a byte
    ///   with all bits set other than `true` (no evidence), and a nonzero
    ///   byte next to a word or byte constant (merged? no evidence);
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
    ///   compiled code push a map symbol's own address, the others a
    ///   data block's or an overlay section's start).  An address inside
    ///   another unit's object, or a variable of an overlay section after
    ///   its first: `pea (sym+d).l` (all 21 such pushes: 19 into overlay
    ///   sections, PCHIST_$CNTL 0xE5CF00 `pea (0xe85c18).l`; 0xE0F5DC
    ///   ROUTE_$PORT+0x30; PEB_$INIT 0xE31D12, PEB_$INFO+8);
    /// - the object a pointer in the frame points to (a by-address
    ///   parameter passed on, `p^` of a pointer local): the pointer,
    ///   `move.l (d,A6),-(SP)` (LOG_$READ 0xE1780C; 0xE53DB0 to
    ///   FILE_$FW_FILE);
    /// - the target of a pointer field of a record held in an address
    ///   register, `p.f^`: the pointer, `move.l (d,An),-(SP)` (0xE1CC52
    ///   in KBD_$RCV's nested 0xE1CC10);
    /// - an object inside one a pointer the push loads points to (a slot
    ///   not held, the enclosing routine's slot, a pointer local in the
    ///   frame, a pointer in the unit's data), at a displacement other
    ///   than 0: `movea.l <pointer>,An; pea (d,An)`
    ///   (KBD_$OUTPUT_BUFFER_DRAINED 0xE1CE9E, AST_$PURIFY 0xE0599E);
    ///   other objects reached through a pointer are not catalogued yet.
    fn push(&mut self, a: &Arg) -> Res<()> {
        let dst = Ea::PreDec(SP);
        match a {
            Arg::Value { value, slot } => {
                if value.ty().bytes().max(2) != *slot {
                    return Err(unsupported(self.span, "an argument slot of this size is"));
                }
                // `integer32(addr(x))` by value pushed as `addr(x)`
                // ([`addr_value`]: FLP_$DINIT 0xE3E15E `pea (0x80,A5)` for
                // MMU_$VTOP's `va`, mmu/mmu.ins.pas).
                let peeled = self.addr_peeled(value);
                let value: &Expr = &peeled;
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
                        // Any other byte: the byte in the high half of its
                        // word slot, the low half 0, pushed as a word
                        // (codegen-patterns 1.5): all 26 nonzero byte
                        // constants that reach a callee reading a byte
                        // slot are `move.w #n<<8,-(SP)` (TTY 0xE1AFC0
                        // `move.w #-0x200,-(SP)` for chr(254) to
                        // tty_$i_buf_put, which reads `move.b (8,A6),D2`
                        // at 0xE1AF4A; SIO_$I_TSTART 0xE1C7D6 #0x1300;
                        // SIO_$INIT 0xE32E62 #0x1b00).  A byte whose bits
                        // are all ones other than `true` (chr(255), a
                        // signed -1) would be `move.w #-0x100` or `st`:
                        // the first is not in compiled code and an `st`
                        // cannot be told from `true` without the callee's
                        // type, so not known (bead domain-pascal-3f6h).
                        Width::Byte if *v & 0xff == 0xff => {
                            return Err(unsupported(
                                self.span,
                                "a byte constant argument with all bits set other than `true` \
                                 (st or move.w #-0x100?) is",
                            ))
                        }
                        Width::Byte => self.out.push(Insn::Move {
                            size: Size::W,
                            src: Ea::Imm(imm((*v & 0xff) << 8, Width::Word)),
                            dst,
                        }),
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
                    // `addr(x)` by value of an object of this unit's data or
                    // frame: `pea`, the push of the same address by address
                    // (codegen-patterns 1.5: `pea (d,A6)`, PROC1_$BIND 0xE14DF4;
                    // `pea (d,A5)`, 0xE02884; `pea (d,An)`, 0xE32B86).
                    Expr::AddrOf(a @ (Addr::Frame(_) | Addr::Static(_))) => {
                        let ea = self.ea_of(a)?;
                        self.out.push(Insn::Pea(ea));
                    }
                    Expr::AddrOf(a @ Addr::OtherStatic { .. }) => self.push_other_static(a)?,
                    // `addr(p)` by value of a parameter passed by address
                    // (any mode), or of a field of one: the push of the
                    // same address by address, below (the whole object,
                    // the slot's value, `move.l (d,A6),-(SP)`, LOG_$READ
                    // 0xE1780C; a field, `movea.l (d,A6),An; pea (k,An)`,
                    // KBD_$OUTPUT_BUFFER_DRAINED 0xE1CE9E; from the
                    // register that holds the slot, 12.2;
                    // codegen-patterns 5.2, bead domain-pascal-mbtx).  The
                    // image cannot tell the two apart.
                    Expr::AddrOf(a) if slot_root(a).is_some_and(|o| self.is_address_slot(o)) => {
                        return self.push(&Arg::Address {
                            addr: a.clone(),
                            slot: 4,
                            writable: false,
                        });
                    }
                    Expr::AddrOf(_) => {
                        return Err(unsupported(self.span, "this argument value is"))
                    }
                    // A boolean computed for the argument (a comparison, a
                    // set member) is formed as a value ([`Self::test_value`])
                    // right before its push and pushed from its register,
                    // as below: all 35 `s<cc> Dn; move.b Dn,-(SP)` of
                    // compiled code have the test right before the `s<cc>`
                    // (`tools/boolsurvey.py scc`; TTY_$K_GET 0xE1C54E
                    // `btst.b #1,(0x17,A2); sne D1; move.b D1,-(SP)`;
                    // AREA_$DEACTIVATE_ASTE 0xE0A196 `tst.l D0; seq D1;
                    // move.b D1,-(SP)`; VTOCE_$TRUNCATE 0xE3A246 `cmpi.l
                    // #32,(-52,A6); sls D0; move.b D0,-(SP)`).
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
                            // The low bytes of a variable, pushed from
                            // memory ([`Self::convert`]; AREA_$COPY
                            // 0xE092DE `move.w (-10,A6),-(SP)` for a
                            // longword at -12).
                            Val::Mem(ea) => ea,
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
                Addr::OtherStatic { .. } => self.push_other_static(addr)?,
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
                // An address inside another unit's object, or a variable
                // of an overlay section after its first: `pea (sym+d).l`
                // (codegen-patterns 1.5, 11: all 21 such pushes of
                // compiled code, 19 into overlay sections, PCHIST_$CNTL
                // 0xE5CF00 `pea (0xe85c18).l`, PCHIST_UNWIRED_D+0x500;
                // also PEB_$INIT 0xE31D12, PEB_$INFO+8, CAL_$GET_LOCAL_TIME
                // 0xE685E2, CAL_$TIMEZONE+6; never `move.l #sym+d`).
                Addr::Symbol { name, offset } => {
                    self.out.push(Insn::Pea(Ea::AbsL(name.clone(), *offset)))
                }
                // The object a pointer in the frame points to, e.g. a
                // by-address parameter passed on by address: the pointer
                // itself, `move.l (d,A6),-(SP)` (LOG_$READ 0xE1780C and
                // 0xE17818 pass its `actual_len` and `buffer` slots on to
                // the worker 0xE17778).  A pointer local's target `p^`
                // likewise (codegen-patterns 1.5): 0xE53D32, 0xE53DD6
                // `move.l (-0x180,A6),-(SP)` to FILE_$TRUNCATE and 0xE53DB0,
                // 0xE53DEE, 0xE53E26 to FILE_$FW_FILE, the uid at the start
                // of the record the local points to (module-local
                // 0xE53A18, `dir/do_op_fix_dir.c` `orig_handle`).
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
                // A by-address parameter held in a data register
                // (codegen-patterns 12.2, "Pointers in data registers")
                // passed on: `move.l Dn,-(SP)` (EC2_$ADVANCE 0xE42CDA
                // `move.l D0,-(SP)` to EC2_$WAKEUP; OSINFO_$GET_MMAP
                // 0xE5C72C, 0xE5C860, 0xE5C862).  Inside the object: not
                // seen.
                Addr::Indirect { base, offset }
                    if matches!(
                        &**base,
                        Expr::Load { addr: Addr::Frame(o), .. }
                            if matches!(self.pinned(*o), Some(Reg::D(_)))
                    ) =>
                {
                    let Expr::Load {
                        addr: Addr::Frame(o),
                        ..
                    } = &**base
                    else {
                        unreachable!()
                    };
                    if *offset != 0 {
                        return Err(unsupported(
                            self.span,
                            "passing by address a part of an object reached through a pointer \
                             held in a data register (not seen, codegen-patterns 12.2) is",
                        ));
                    }
                    let r = self.pinned(*o).unwrap();
                    self.out.push(Insn::Move {
                        size: Size::L,
                        src: Ea::Reg(r),
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
                // The target of a pointer field of a record reached
                // through a pointer held in an address register since the
                // entry, `p.f^`: the pointer itself, `move.l (d,An),-(SP)`
                // (0xE1CC52 `move.l (0x48,A2),-(SP)` in KBD_$RCV's nested
                // 0xE1CC10 for `kbd.key_ec^` to EC_$ADVANCE_WITHOUT_DISPATCH, a `var`
                // parameter; TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B3A4,
                // TTY_$I_DELETE_CHAR 0xE1B588, TTY_$I_FLUSH_INPUT 0xE1B7DA
                // `move.l (0x2a8,A2),-(SP)`, TTY_$I_BREAK_CHAR 0xE1B8EC
                // `move.l (0x2a4,A2),-(SP)`).  With the record's pointer not
                // held: not seen (TIME_$ADVANCE_CALLBACK 0xE16440 `movea.l
                // (8,A1),A2; pea (A2)` is a chain through a slot, but its
                // source is not known), refused below.
                Addr::Indirect { base, offset: 0 }
                    if field_pointer(base)
                        .is_some_and(|(o, _)| matches!(self.pinned(o), Some(Reg::A(_)))) =>
                {
                    let (_, field) = field_pointer(base).unwrap();
                    let src = self.ea_of(field)?;
                    self.out.push(Insn::Move {
                        size: Size::L,
                        src,
                        dst,
                    });
                }
                // An object inside one a pointer in memory points to, the
                // pointer loaded into an address register for the push:
                // `movea.l <pointer>,An; pea (d,An)` (ad-hoc survey of
                // compiled code, codegen-patterns 1.5: 34 `movea.l
                // (d,A6),An; pea (d',An)` of a parameter slot, 25 of a
                // frame slot, 35 through another address register):
                // - a parameter slot not held: KBD_$OUTPUT_BUFFER_DRAINED
                //   0xE1CE9E `movea.l (8,A6),A0; pea (0x4c,A0)`;
                //   TTY_$I_PUT_CHARS 0xE1B0BC `movea.l (8,A6),A0; pea
                //   (0x3d2,A0)`, reloaded for each call (0xE1B104, ...);
                // - the enclosing routine's slot, through the static link:
                //   0xE1AFB8 `movea.l (8,A2),A0; pea (0x3d2,A0)`, again for
                //   each call (0xE1AFCC, 0xE1AFDE, 0xE1AFF2), also
                //   0xE1BD16, 0xE1BDF0;
                // - a pointer local in the frame: AST_$PURIFY 0xE0599E
                //   `movea.l (-0xb8,A6),A0; pea (0x28,A0)` (its own target
                //   `p^` is the pointer pushed, `move.l (-k,A6),-(SP)`,
                //   above; the image's 7 `movea.l (-k,A6),An; pea (An)` all
                //   go to routines that take arguments by value, where the
                //   same position also gets `move.l (-k,A6),-(SP)`, 6 with
                //   the register holding the pointer from an earlier
                //   statement: codegen-patterns 1.5);
                // - a pointer in the unit's data: AUDIT_$LOG_EVENT_S
                //   0xE70E88 `movea.l (0x198,A5),A0; pea (0xc,A0)` (13
                //   such in AUDIT).
                // The register is the allocator's, as for any pointer
                // loaded as a base (codegen-patterns 12.4: kept apart from
                // the argument list's other address values, a slot read
                // once in a register no earlier statement left a value in).
                // The pointer's own target through the unit's data, and
                // pointers reached through other pointers: not catalogued,
                // refused below.
                Addr::Indirect { base, offset } if self.reloaded_pointer(base, *offset) => {
                    let ea = self.ea_of(addr)?;
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
                // `integer32(addr(x))` as `addr(x)` ([`addr_value`]).
                let peeled = self.addr_peeled(value);
                let value: &Expr = &peeled;
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
            until: None,
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
        // An element of a packed array of booleans: `btst`, then `sne`
        // (ACL_$USED_SUSER 0xE492FA..0xE492FE `btst D2,(-15568,A1); sne
        // D0`, its one statement; codegen-patterns 5.2).
        if let Expr::BitElem {
            array,
            bytes,
            index,
            lo,
        } = e
        {
            self.bit_elem_test(array, *bytes, index, *lo)?;
            let d = self.alloc_d()?;
            self.out.push(Insn::Scc {
                cond: Cond::Ne,
                dst: Ea::Reg(Reg::D(d)),
            });
            self.result_moves.push(self.out.len());
            self.out.push(Insn::Move {
                size: Size::B,
                src: Ea::Reg(Reg::D(d)),
                dst: Ea::Reg(Reg::D(0)),
            });
            return Ok(());
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

    /// The data register holding `addr(x)` when `x` is the object of a
    /// by-address slot held in one since the entry ([`Self::entry_loads`];
    /// EC2_$INIT 0xE42C68 `cmpi.l #0x3e8,D0`).
    fn held_addr(&self, a: &Addr) -> Option<Reg> {
        let Addr::Indirect { base, offset: 0 } = a else {
            return None;
        };
        match &**base {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } => self.pinned(*o).filter(|r| matches!(r, Reg::D(_))),
            _ => None,
        }
    }

    /// `e` with a 4-byte type transfer of `addr(x)` read as `addr(x)`
    /// ([`addr_value`]), for a store or an argument; an address held in a
    /// data register ([`Self::held_addr`]) is left to [`Self::eval`].
    fn addr_peeled<'e>(&self, e: &'e Expr) -> std::borrow::Cow<'e, Expr> {
        match addr_value(e) {
            Some(x) if !matches!(e, Expr::AddrOf(_)) && self.held_addr(x).is_none() => {
                std::borrow::Cow::Owned(Expr::AddrOf(x.clone()))
            }
            _ => std::borrow::Cow::Borrowed(e),
        }
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
            Expr::Bits { addr, field, ty } => Ok(Val::Tmp(self.bits_value(addr, *field, *ty)?)),
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
                    let (r, cc) = self.set_bit_test(elem, set, *set_width, true)?;
                    let r = match r {
                        Some(r) => r,
                        None => self.alloc_d()?,
                    };
                    self.out.push(Insn::Scc {
                        cond: cc,
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
                if let Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } = &**set
                {
                    if self.pinned(*o).is_some() {
                        return Err(unsupported(
                            self.span,
                            "`in` on a set held in a register, as a value, is",
                        ));
                    }
                }
                let r = self.set_member_reg(elem, set)?;
                self.out.push(Insn::Scc {
                    cond: Cond::Ne,
                    dst: Ea::Reg(Reg::D(r)),
                });
                Ok(Val::Tmp(r))
            }
            Expr::SetMemberAt { .. } => Err(unsupported(
                self.span,
                "`in` on a set larger than a longword, as a value (`s<cc>` after the byte \
                 `btst`: not catalogued), is",
            )),
            // As a function result: [`Self::set_result`].
            Expr::BitElem { .. } => Err(unsupported(
                self.span,
                "an element of a packed array of booleans as a value other than a condition or \
                 the function result of a one-statement body (not seen, codegen-patterns 5.2) is",
            )),
            Expr::SetOf { elem, ty } => Ok(Val::Tmp(self.set_of(elem, ty.width)?)),
            Expr::SetOp { op, lhs, rhs, ty } => {
                Ok(Val::Tmp(self.set_op_value(*op, lhs, rhs, *ty)?))
            }
            // The address a by-address slot holds, held in a data register
            // since the entry ([`Self::entry_loads`]; EC2_$INIT 0xE42C68
            // `cmpi.l #0x3e8,D0`).
            Expr::AddrOf(a) if self.held_addr(a).is_some() => {
                Ok(Val::Reg(self.held_addr(a).unwrap()))
            }
            // Elsewhere (a store, an argument, `+`/`-`: [`Self::store`],
            // [`Self::push`], [`Self::addr_arith`]) the address has the
            // forms of codegen-patterns 5.2, "`addr(x)` as a value"; in
            // other expressions (a comparison, `*`, a shift, `&`, ...) no
            // form is catalogued (`cmp.l #sym,Dn` and `cmpa.l #sym,An`
            // are seen for comparisons, codegen-patterns 5.2; bead
            // domain-pascal-awzk).
            Expr::AddrOf(_) => Err(unsupported(
                self.span,
                "an address (`addr`) in a comparison, in an expression other than `+`/`-`, or \
                 into or with a register-held local (no form catalogued, codegen-patterns 5.2) \
                 is",
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
                // The low bytes of a constant outside the object's range
                // (an integer subrange's, ref Appendix D, ISO 6.4.6,
                // p. D-3; clarification Q38), as a store to memory keeps.
                src: Ea::Imm(imm(c, w)),
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
        if matches!(addr.unwrap_volatile().0, Addr::Index { .. }) {
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
            BinOp::Min | BinOp::Max => return self.min_max(op, lhs, rhs, ty, None),
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
        if [lhs, rhs]
            .iter()
            .any(|e| addr_value(e).is_some_and(|a| self.held_addr(a).is_none()))
        {
            return self.addr_arith(alu, lhs, rhs, ty);
        }
        // A constant operand: on the right for `-`; either side otherwise.
        let (x, c) = match (konst(lhs), konst(rhs)) {
            (_, Some(c)) => (Some(lhs), Some(c)),
            (Some(c), None) if op.commutative() => (Some(rhs), Some(c)),
            _ => (None, None),
        };
        if let (Some(x), Some(c)) = (x, c) {
            let c = imm(c, w);
            // `x & c` of an unsigned byte widened to a word, `c` in
            // 0..255: the constant loaded as a word, then `and.b` of the
            // byte, whose high byte the constant's clears (TPAD_$DATA
            // 0xE691EE `moveq #0x70,D0; and.b (11,A1),D0; lsr.w #4,D0`,
            // 0xE693EC `move.w #240,D3; ... and.b (12,A1),D3`; the same
            // form as a packed field's, codegen-patterns 5.3, "Wider
            // field, value", RIP 0xE158B6).  The bytes are `[byte] 0..255`
            // record fields in `../source`'s reconstruction (ref 3.15.5).
            if let (AluOp::And, Width::Word, Expr::Convert { value: inner, .. }) = (alu, w, x) {
                let it = inner.ty();
                if matches!(**inner, Expr::Load { .. })
                    && it.width == Width::Byte
                    && it.sign == Sign::Unsigned
                    && (0..=0xff).contains(&c)
                {
                    let xv = self.eval(inner)?;
                    // (Evaluated once: falling through would evaluate the
                    // byte again.)
                    if !matches!(xv, Val::Mem(_) | Val::Reg(Reg::D(_)) | Val::Tmp(_)) {
                        return Err(unsupported(self.span, "this byte operand of `&` is"));
                    }
                    let d = self.load_const(c, w)?;
                    self.out.push(Insn::Alu {
                        op: alu,
                        size: Size::B,
                        src: Self::ea_val(&xv),
                        dst: d,
                    });
                    return Ok(d);
                }
            }
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
                    // A value held in a data register and a constant
                    // `moveq` cannot load: the value copied, then `andi`
                    // (TTY_$I_RCV 0xE1BACE `move.w D4,D1; andi.w #0x80,D1`;
                    // survey: 24 such copies and `andi` with a constant
                    // outside -128..127, no `move #c,Dn; and Dm,Dn` with
                    // one; 32 `moveq #c,Dn; and Dm,Dn`, codegen-patterns 8).
                    let xv = match xv {
                        Val::Reg(Reg::D(_)) if !(-128..128).contains(&c) => {
                            Val::Tmp(self.in_tmp(xv, w)?)
                        }
                        v => v,
                    };
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
            // #0x400,D2; sub.l (-372,A6),D2`; 0xE0E964).  A widened word
            // is extended first, then the constant loaded and the word
            // subtracted; R11 (12.4) then moves the constant's load up
            // between the word's load and its extension: NETWORK_$READ_AHEAD
            // 0xE10000 `move.w D4,D0; move.l #0x400,D1; ext.l D0; sub.l
            // D0,D1`, XNS_ERROR_$SEND 0xE17B22, AREA_$INVALIDATE 0xE08EF4
            // `clr.l D0; moveq #32,D1; move.w (0xe,A6),D0; sub.l D0,D1`;
            // `min`'s operands, NETBUF_$ADD_PAGES 0xE0E95C..0xE0E96A,
            // REM_FILE_$SERVER 0xE637AE..0xE637BC.  Other computed values
            // are refused (not seen).
            if matches!(rhs, Expr::Convert { value, ty: t }
                if value.ty().width == Width::Word && t.width == Width::Long)
            {
                let xv = self.eval(rhs)?;
                let d = self.load_const(imm(c, w), w)?;
                self.out.push(Insn::Alu {
                    op: alu,
                    size,
                    src: Self::ea_val(&xv),
                    dst: d,
                });
                return Ok(d);
            }
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

    /// `lhs op rhs` with an operand `addr(x)` (or `integer32(addr(x))`,
    /// [`addr_value`]; ref "Addr", p. 4-13, "Type Transfer Functions",
    /// p. 4-197) into a new data register (codegen-patterns 5.2, "`addr(x)`
    /// as a value"; bead domain-pascal-j28d):
    ///
    /// - the address of another unit's object (a map symbol's own
    ///   address) as the right operand: the symbol an immediate operand,
    ///   `add.l #sym,Dn` / `sub.l #sym,Dn` in the immediate-EA encodings
    ///   `d0bc` / `90bc` (all 13 `add.l`/`sub.l` with an immediate that is
    ///   an address in compiled code, against no `addi`/`subi` of one:
    ///   NET_$OPEN 0xE5A1C0 `sub.l #NET_IO_$NIL_DRIVER,D0` and its four
    ///   neighbours, ACL_$INIT 0xE310B2, PROC2_$GET_CR_REC 0xE40188,
    ///   0xE078CE, 0xE07A72, 0xE09E6E; `add.l` 0xE0F14E, 0xE0F15E,
    ///   0xE339FA), the left operand in a data register first as for any
    ///   sum (a variable loaded, 0xE09E6A `move.l (8,A2),D3`; a computed
    ///   value in place, 0xE0F148 `andi.l #1023,D1; add.l #AST_$COPY_BUFF,
    ///   D1`; a pointer held in an address register copied, 0xE078CC
    ///   `move.l A3,D0`);
    /// - an address as the left operand of `-`, in a data register
    ///   ([`Self::addr_into_d`]), then the right operand subtracted: an
    ///   immediate as above (NET_$OPEN 0xE5A1B8..0xE5A1C0 `lea
    ///   (NET_IO_$NIL_DRIVER+0x28).l,A0; move.l A0,D0; sub.l
    ///   #NET_IO_$NIL_DRIVER,D0`, NET_$CLOSE 0xE5A228, NET_$IOCTL
    ///   0xE5A284, NET_$RCV 0xE5A2E0, NET_$SEND 0xE5A348; ACL_$INIT
    ///   0xE310AA), or the address of an object that needs a `lea`, formed
    ///   first, before the left one, and subtracted from its register
    ///   (`sub.l An,Dn`, 6 of 6 sites: the DIR module-local 0xE4BD48 at
    ///   0xE4BDCE `lea (-1304,A6),A0; lea (0,A3,D2.l),A1; move.l A1,D0;
    ///   sub.l A0,D0`, 0xE4BE7A, 0xE4BF4A, 0xE4BFE4; ASKNODE_$SERVER
    ///   0xE65DA0 `lea (-592,A6),A0; lea (-582,A6),A1; move.l A1,D1; sub.l
    ///   A0,D1`; ASKNODE_$INTERNET_INFO 0xE6519C).
    ///
    /// Refused, no form catalogued: an address with a constant (`addr(x)
    /// + c`), with a variable or a computed value other than a symbol's
    /// immediate on the right (NETBUF_$GETVA 0xE0ECC8..0xE0ECE2 forms its
    /// element address first, then the masked value, `add.l A1,D3`: one
    /// site; its element address is read once, bead domain-pascal-1ibr), the
    /// sum of two addresses, a constant minus an address, `&`, `!`,
    /// `xor`, a width other than a longword (bead domain-pascal-awzk).
    fn addr_arith(&mut self, alu: AluOp, lhs: &Expr, rhs: &Expr, ty: Scalar) -> Res<u8> {
        let refuse = |s: &Self, what: &str| {
            Err(unsupported(
                s.span,
                format!(
                    "{what} (codegen-patterns 5.2, \"`addr(x)` as a value\": no form \
                     catalogued) is"
                ),
            ))
        };
        if !matches!(alu, AluOp::Add | AluOp::Sub) || ty.width != Width::Long {
            return refuse(
                self,
                "an address (`addr`) in an operation other than a longword `+` or `-`",
            );
        }
        let la = addr_value(lhs).filter(|a| self.held_addr(a).is_none());
        let ra = addr_value(rhs).filter(|a| self.held_addr(a).is_none());
        let d = match (la, ra) {
            (Some(_), Some(_)) if alu == AluOp::Add => {
                return refuse(self, "the sum of two addresses")
            }
            (Some(a), Some(b)) => {
                let src = match b {
                    Addr::Symbol { name, offset: 0 } => Ea::ImmSym(name.clone()),
                    b => Ea::Reg(Reg::A(self.addr_into_a(b)?)),
                };
                let d = self.addr_into_d(a)?;
                self.out.push(Insn::Alu {
                    op: alu,
                    size: Size::L,
                    src,
                    dst: d,
                });
                d
            }
            (None, Some(Addr::Symbol { name, offset: 0 }))
            | (Some(Addr::Symbol { name, offset: 0 }), None)
                if alu == AluOp::Add || ra.is_some() =>
            {
                let x = if ra.is_some() { lhs } else { rhs };
                if matches!(x, Expr::Const { .. }) {
                    return refuse(self, "an address and a constant in a sum or difference");
                }
                let v = self.eval(x)?;
                let d = self.in_tmp(v, Width::Long)?;
                self.out.push(Insn::Alu {
                    op: alu,
                    size: Size::L,
                    src: Ea::ImmSym(name.clone()),
                    dst: d,
                });
                d
            }
            (Some(_), None) if matches!(rhs, Expr::Const { .. }) => {
                return refuse(self, "an address and a constant in a sum or difference")
            }
            (None, Some(_)) if matches!(lhs, Expr::Const { .. }) => {
                return refuse(
                    self,
                    if alu == AluOp::Add {
                        "an address and a constant in a sum or difference"
                    } else {
                        "a constant minus an address"
                    },
                );
            }
            // The other operand the address a by-address slot holds in a
            // data register since the entry ([`Self::held_addr`]): with an
            // immediate on the right it is a value like any other (the arm
            // above); with an address formed here no form is seen.
            _ if [lhs, rhs]
                .iter()
                .any(|e| addr_value(e).is_some_and(|a| self.held_addr(a).is_some())) =>
            {
                return refuse(
                    self,
                    "an address formed here with the address a parameter's slot holds in a \
                     data register",
                )
            }
            (Some(_), None) if alu == AluOp::Sub => {
                return refuse(self, "an address minus a value that is not an address")
            }
            _ => {
                return refuse(
                    self,
                    "an address that needs an address register and a value that is not one \
                     (the order is not settled: NETBUF_$GETVA 0xE0ECC8, one site; bead \
                     domain-pascal-awzk)",
                )
            }
        };
        Ok(d)
    }

    /// `addr(x)` in a new data register, the left operand of a difference
    /// ([`Self::addr_arith`]): another unit's object, `move.l #sym,Dn`
    /// (OS_$INIT 0xE3397C `move.l #VM_TABLES_END,D0`, compared; the
    /// immediate of its stores and pushes, codegen-patterns 1.5); an
    /// address inside one, `lea (sym+d).l,An; move.l An,Dn` (NET_$OPEN
    /// 0xE5A1B8 `lea (0xe2451c).l,A0; move.l A0,D0`, the record's field at
    /// +0x28, and NET's four other entries; ACL_$INIT 0xE310AA); any other
    /// object, its `lea` ([`Self::addr_into_a`]), then `move.l An,Dn` (the
    /// DIR module-local 0xE4BD48 at 0xE4BDD2 `lea (0,A3,D2.l),A1; move.l
    /// A1,D0`).  The address register and the data register are new ones
    /// (A0 and D0 at NET's five sites; D1 at ASKNODE_$SERVER 0xE65DA8 and
    /// at FLP_$DINIT 0xE3E17E, where D0 looks free: Uncertain, bead
    /// domain-pascal-i2na).
    fn addr_into_d(&mut self, a: &Addr) -> Res<u8> {
        let src = match a {
            Addr::Symbol { name, offset: 0 } => Ea::ImmSym(name.clone()),
            Addr::Symbol { name, offset } => {
                let r = self.alloc_a()?;
                self.out.push(Insn::Lea {
                    src: Ea::AbsL(name.clone(), *offset),
                    dst: r,
                });
                Ea::Reg(Reg::A(r))
            }
            a => Ea::Reg(Reg::A(self.addr_into_a(a)?)),
        };
        let d = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: Size::L,
            src,
            dst: Ea::Reg(Reg::D(d)),
        });
        Ok(d)
    }

    /// The word that a 32-bit operand of sign `sign` stands for: the operand
    /// of a widening from a word of that sign, or a constant that fits a word
    /// of that sign.  These are the "W" operands of M$ARITH and the operands
    /// of `mul<s>.w` with a longword product (codegen-patterns 8).
    fn word_part(e: &Expr, sign: Sign) -> Option<Expr> {
        let wty = Scalar::integer(Width::Word, sign);
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
    ///   at a word's width also `mulu.w #c`, or for a constant of 1..=127
    ///   and a variable `moveq #c,Dn; mul<s>.w x,Dn`,
    ///   [`Self::mul_const_first`], the type's sign: module-local 0xE4519C
    ///   at 0xE458CC..0xE458D2 `moveq #44,D3; muls.w D2,D3`, a signed word;
    ///   an unsigned one is refused, no site), at either width; at a
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
    ///   type (0xE30A9A `muls.w D3,D0; move.w D0,(A0)+`; **Uncertain:**
    ///   the mnemonic of a word product is the same either way).
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
            if w == Width::Word && c < 128 && by_mul_insn(c) && const_first(x) {
                if !signed || !x.ty().is_signed() {
                    return Err(unsupported(
                        self.span,
                        format!(
                            "a word product of a `0..65535` variable and {c} (`moveq #{c},Dn; \
                             mul<s>.w x,Dn`: the one such word product, module-local 0xE4519C \
                             at 0xE458D2, is `muls.w` of a signed word; codegen-patterns 8) is"
                        ),
                    ));
                }
                return self.mul_const_first(x, c, true, &mut |_| Ok(()));
            }
            if w == Width::Word && (1..128).contains(&c) && by_mul_insn(c) && !in_place(x) {
                return Err(computed_factor(self.span, c));
            }
            if w == Width::Word || by_shifts(c) {
                let d = self.eval_d(x)?;
                self.mul_const(d, c, w)?;
                return Ok(d);
            }
            if (1..=0xffff).contains(&c) && !by_mul_insn(c) {
                return Err(three_bit_odd_part(self.span, c));
            }
            let b = Expr::Const { value: c, ty };
            if let (Some(xw), Some(_)) = (Self::word_part(x, ty.sign), Self::word_part(&b, ty.sign))
            {
                if (1..128).contains(&c) && const_first(&xw) {
                    // The constant first, as for a `.l` index (20 of 20,
                    // all `muls.w` of signed words: RIP_$NET_LOOKUP
                    // 0xE15504 `moveq #44,D0; muls.w D1,D0`; TPAD_$DATA
                    // 0xE691D0; codegen-patterns 5.2).  An unsigned word:
                    // no site.
                    if !signed {
                        return Err(unsupported(
                            self.span,
                            format!(
                                "a longword product of a `0..65535` variable and {c} (`moveq \
                                 #{c},Dn; mul<s>.w x,Dn`: the 20 such products, `.l` indices, \
                                 are `muls.w` of signed words; codegen-patterns 5.2) is"
                            ),
                        ));
                    }
                    return self.mul_const_first(&xw, c, signed, &mut |_| Ok(()));
                }
                if (1..128).contains(&c) && !in_place(&xw) {
                    return Err(computed_factor(self.span, c));
                }
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
                    let wty = Scalar::integer(Width::Word, ty.sign);
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
                if let Some(b) = bw {
                    let name = self.marith_name("OI", signed, "WLW")?;
                    self.marith(name, lhs, &b)?;
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
                let name = self.marith_name("OI", signed, "LLL")?;
                self.marith(name, lhs, rhs)?;
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

    /// `min(a, b)` / `max(a, b)` of type `ty` (`dpc-ir` `min_max`;
    /// codegen-patterns 10.1; **Reported**: the image's compare-and-select,
    /// read as these built-ins by ../source's Pascal):
    ///
    /// ```text
    ///         <a into Dn>             ; the first operand in the result's register
    ///         cmp.<s> b,Dn            ; cmpi.<s> #c,Dn for a constant
    ///         b<keep>.s L             ; min: ble / bls; max: bge
    /// L':     move.<s> b,Dn           ; clr, moveq or move #c for a constant
    /// L:
    /// ```
    ///
    /// - `a` is evaluated into a new register (a copy of a register-held
    ///   variable, a load, a constant: `moveq #2,D3`, PKT_$PING_SERVER
    ///   0xE12C8C; `move.w #0x400,D0`, NET_IO_$COPY_PACKET 0xE0E612), or
    ///   is the register it was computed in (0xE17DF2 `move.l
    ///   (0x38,A2),D7; sub.l (-0x2c,A6),D7; cmpi.l #0x400,D7`), or is
    ///   `into`, a register-held local `x` in `x := min(x, b)` (the select
    ///   in its register, as the `if` the image cannot tell from it:
    ///   DISK_$SPIN_DOWN 0xE3DB34 `cmp.w D0,D2; bge.b; move.w D0,D2`).
    /// - `b` is an operand used twice, in memory (MMAP_$ALLOC_PURE
    ///   0xE0D7CE `cmp.l (0x30,A2),D6; bls.b; move.l (0x30,A2),D6`), a
    ///   register (NETBUF_$ADD_PAGES 0xE0E96C `cmp.l D6,D3; ble.b; move.l
    ///   D6,D3`, D6 computed), or an immediate (PKT_$DAT_COPY 0xE12852
    ///   `cmpi.w #0x400,D5; ble.b; move.w #0x400,D5`; module-local at
    ///   0xE17D46 (../source's xns_$get_checksum) at 0xE17DFA; the move as a constant load, `moveq #1,D5` at 0xE08B56,
    ///   `moveq #17,D0` at 0xE1594A).  **Uncertain against:** six sites
    ///   hold the constant in a register, `move.w #c,Dn; cmp.w Dn,Dx;
    ///   ble.s; move.w Dn,Dx` (OS_INIT_START 0xE2F08A, 0xE66008, 0xE874D6,
    ///   0xE41B04, 0xE1CAA0, 0xE15230), against 22 with the immediate:
    ///   unexplained (bead domain-pascal-avnn).
    /// - The branch keeps `a` when `a <= b` (min: `ble`, `bls`) or `a >= b`
    ///   (max: `bge`): of the 109 compare-and-selects of compiled code
    ///   (`tools/cmpsurvey.py select`), `ble` 43, `bls` 41, `bge` 9 and
    ///   `bpl` over a `clr` 6, against `bhi` 7, `bcc` 1, `bcs` 2, and no
    ///   `blt` or `bgt`.  Against 0, `max(x,
    ///   0)` tests the flags of the operation that formed `x` or a `tst`
    ///   and clears (NETBUF_$DEL_PAGES 0xE0EB64 `sub.l D4,D1; bpl.b; clr.l
    ///   D1`; also 0xE0FD64, 0xE6E282; `tst.w D2; bpl.s; clr.w D2`
    ///   0xE4BD78), as `div_pow2` does.
    ///
    /// Refused: an unsigned `max` (`bhi` 7 times, among them module-local
    /// at 0xE6C2BC, read as `max(1, num_parts)` at 0xE6C4CA by ../source's
    /// disk/pv_mount_internal.c, against
    /// `bcc` once, 0xE5C7C6: not settled, bead domain-pascal-ckzr),
    /// `min(x, 0)` (not seen), `b` in an address register.
    fn min_max(&mut self, op: BinOp, a: &Expr, b: &Expr, ty: Scalar, into: Option<u8>) -> Res<u8> {
        let w = ty.width;
        let size = size_of(w);
        let keep = match (op, ty.sign == Sign::Signed) {
            (BinOp::Min, true) => Cond::Le,
            (BinOp::Min, false) => Cond::Ls,
            (_, true) => Cond::Ge,
            (_, false) => {
                return Err(unsupported(
                    self.span,
                    "an unsigned `max` (`bhi` or `bcc`: not settled, codegen-patterns 10.1) is",
                ))
            }
        };
        let d = match into {
            Some(r) => r,
            None => {
                let v = self.eval(a)?;
                self.in_tmp(v, w)?
            }
        };
        let dst = Ea::Reg(Reg::D(d));
        let (src, cond) = match b {
            Expr::Const { value: 0, .. } if op == BinOp::Max => {
                if !self.sets_flags(d, w) {
                    self.out.push(Insn::Tst {
                        size,
                        dst: dst.clone(),
                    });
                }
                (Val::Imm(0), Cond::Pl)
            }
            Expr::Const { value: 0, .. } => {
                return Err(unsupported(
                    self.span,
                    "`min(x, 0)` (not seen, codegen-patterns 10.1) is",
                ))
            }
            Expr::Const { value, .. } => {
                let c = imm(*value, w);
                self.out.push(Insn::Cmpi {
                    size,
                    imm: c,
                    dst: dst.clone(),
                });
                (Val::Imm(c), keep)
            }
            _ => {
                let v = self.eval(b)?;
                let src = Self::ea_val(&v);
                if matches!(src, Ea::Reg(Reg::A(_)) | Ea::Imm(_)) {
                    return Err(unsupported(
                        self.span,
                        "a `max` or `min` operand in an address register is",
                    ));
                }
                self.out.push(Insn::Cmp { size, src, dst: d });
                (v, keep)
            }
        };
        if self.unpopped {
            return Err(unsupported(
                self.span,
                "a conditional branch after a call whose pop was skipped (codegen-patterns 1.1) is",
            ));
        }
        let l = self.new_label();
        self.jump(cond, l);
        match src {
            Val::Imm(c) => self.const_into(c, w, d),
            v => self.out.push(Insn::Move {
                size,
                src: Self::ea_val(&v),
                dst,
            }),
        }
        self.place(l);
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
        let signed = ty.is_signed();
        match Self::word_part(b, ty.sign) {
            Some(bw) => {
                let name = self.marith_name(kind, signed, "LLW")?;
                self.marith(name, a, &bw)
            }
            None => {
                let name = self.marith_name(kind, signed, "LLL")?;
                self.marith(name, a, b)
            }
        }
    }

    /// The M$ARITH entry `M$<kind><S|U>$<form>` from the shipped module's
    /// list ([`crate::runtime::marith_entry`]); an operation the module
    /// has no routine for is refused, never given an invented name.
    fn marith_name(&self, kind: &str, signed: bool, form: &str) -> Res<&'static str> {
        crate::runtime::marith_entry(kind, signed, form).ok_or_else(|| {
            unsupported(
                self.span,
                format!(
                    "an operation with no {} routine (M${kind}{}${form}) is",
                    crate::runtime::MODULE,
                    if signed { 'S' } else { 'U' }
                ),
            )
        })
    }

    /// A call of the M$ARITH entry `name` (one of the shipped module's,
    /// [`Self::marith_name`]; a `'static` name, so never one formed here)
    /// with argument 1 `a` (a longword)
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
    fn marith(&mut self, name: &'static str, a: &Expr, b: &Expr) -> Res<()> {
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
        self.other_bases.clear();
        self.other_syms.clear();
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
    /// - other odd parts ([`by_mul_insn`]): `mulu.w #c` in place
    ///   (BAT_$GET_BAT_STEP 0xE3BB6A `mulu.w #0x234`; `tools/arithsurvey.py
    ///   scale`: every constant multiplied with `mulu.w`/`muls.w` (44, 46,
    ///   92, 100, 150, 153, 204, 228, 244, 250, 268, 300, 336, 564, 580,
    ///   1083, 1244, 1624, 5240, 12345) has an odd part that is not 2^k ± 1,
    ///   and every one done with shifts (2^s, 6, 10, 12, 17, 18, 20, 24, 28,
    ///   36, 40, 48, 56, 60, 72, 96, 120, 144, 248, 257) has one that is,
    ///   except ×26 (0xE71BBA, odd part 13 done bit by bit, refused).  A
    ///   constant of 1..=127 multiplies a variable with the constant first
    ///   ([`Self::mul_const_first`]); here it multiplies a zero-extended byte
    ///   (RIP_$BROADCAST 0xE872F8 `mulu.w #92,D5`; other computed values
    ///   refused, [`in_place`]).  `mulu.w` at a word
    ///   width only (the low word of the product is the same for both
    ///   signs; 94 `mulu.w #c` feed a word index, 118 `muls.w #c` a
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
            if !by_mul_insn(c as i64) {
                return Err(three_bit_odd_part(self.span, c as i64));
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

    /// `x × c` for a constant `c` in 1..=127 done with a multiply
    /// ([`by_mul_insn`]), `x` a word variable (the value `v`: memory or a
    /// held register, not a computed value): the constant first, `moveq
    /// #c,Dn; mul<s>.w x,Dn`, into a new register (codegen-patterns 5.2, 8;
    /// `tools/arithsurvey.py scale`, bead domain-pascal-r3yy).  `between` is
    /// called after the `moveq` (another unit's array base: RIP_$TABLE
    /// 0xE6904C..0xE69054 `moveq #92,D1; movea.l #sym,A0; mulu.w D0,D1`, 5
    /// of the 6 such bases).
    ///
    /// Of the 44 products by such constants with a multiply in compiled
    /// code, 43 have this form: the variable in memory (TPAD_$SET_UNIT_MODE
    /// 0xE69824..0xE69826 `moveq #44,D2; mulu.w (A4),D2`, a by-address
    /// parameter's object, and the 5 other TPAD unit routines; TPAD_$DATA
    /// 0xE691D0 `muls.w (380,A5),D0`; RINGLOG_$CNTL 0xE722A0 `moveq #46,D1;
    /// muls.w (-2,A6),D1`, a local) or in a register (VTOC_$MOUNT 0xE38612
    /// `moveq #100,D0; mulu.w D2,D0`; a by-value parameter loaded first,
    /// module-local 0xE383B0 at 0xE383C8..0xE383CE `move.w (12,A6),D2;
    /// moveq #100,D0; mulu.w D2,D0`) or a mask (HINT_$GET_HINTS 0xE4998E
    /// ..0xE4999A, refused: [`in_place`]).  The other multiplies a
    /// zero-extended byte in place, RIP_$BROADCAST 0xE872F8 `mulu.w #92,D5`
    /// ([`Self::mul_const`]).  Constants of 128 and more: the value
    /// in a register, then `mul<s>.w #c` (261 of 261; no `move.w #c,Dn`
    /// multiplied by the value).  A multiplicand at an absolute or indexed
    /// address is refused (no site).
    fn mul_const_first(
        &mut self,
        x: &Expr,
        c: i64,
        signed: bool,
        between: &mut dyn FnMut(&mut Self) -> Res<()>,
    ) -> Res<u8> {
        let held = match x {
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } => *o > 0 || self.pinned(*o).is_some(),
            Expr::Load {
                addr: Addr::Static(_) | Addr::Indirect { .. },
                ..
            } => false,
            _ => {
                return Err(unsupported(
                    self.span,
                    format!(
                        "a multiplication by {c} of another unit's variable, an array element or \
                         an enclosing routine's variable (`moveq #{c},Dn; mul<s>.w x,Dn` with \
                         such an operand: no site, codegen-patterns 8) is"
                    ),
                ))
            }
        };
        let (d, src) = if held {
            // A by-value parameter in a register first (loaded now, or at
            // the entry), then the constant.
            let v = match self.eval(x)? {
                Val::Mem(ea) => Val::Reg(Reg::D(self.in_tmp(Val::Mem(ea), Width::Word)?)),
                v => v,
            };
            let d = self.alloc_d()?;
            self.out.push(Insn::Moveq {
                value: c as i8,
                dst: d,
            });
            (d, v)
        } else {
            // The constant, then the operand's address (TPAD_$INQUIRE_UNIT
            // 0xE6999C..0xE699A2 `moveq #44,D0; movea.l (8,A6),A0; mulu.w
            // (A0),D0`; PKT_$BLD_INTERNET_HDR 0xE121E6..0xE121F2).
            let d = self.alloc_d()?;
            self.out.push(Insn::Moveq {
                value: c as i8,
                dst: d,
            });
            (d, self.eval(x)?)
        };
        let src = match src {
            Val::Reg(r @ Reg::D(_)) => Ea::Reg(r),
            Val::Mem(ea @ (Ea::Ind(_) | Ea::Disp(..))) => ea,
            _ => {
                return Err(unsupported(
                    self.span,
                    format!(
                        "a multiplication by {c} of a value at an absolute or indexed address \
                         (`moveq #{c},Dn; mul<s>.w x,Dn` with such an operand: no site, \
                         codegen-patterns 8) is"
                    ),
                ))
            }
        };
        between(self)?;
        self.out.push(Insn::Mul {
            signed,
            src,
            dst: d,
        });
        Ok(d)
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
            // `x & c` of a longword held in a register and a `moveq`
            // constant, used as a word: a word `and` on the register's low
            // word, the constant first (DBUF_$GET_BLOCK 0xE3A5EC `moveq
            // #31,D0; and.w D3,D0`; 0xE0630E..0xE06312, 0xE064CC..0xE064D0,
            // RIP_$NET_LOOKUP 0xE154F6..0xE154FC `moveq #63,D1; and.w
            // D3,D1`, D3 loaded `move.l (8,A6),D3`).  A longword in memory
            // is read whole and masked at its width (0xE4F25E `move.l
            // (10,A0),D1; andi.l #1023,D1; move.w D1,-(SP)`), as below.
            if let Some(n) = self.narrow_held_and(value, ty) {
                return self.eval(&n);
            }
            // The low bytes of a value (codegen-patterns 8, "Narrowing and
            // widening"; `dpc-ir`'s `assign_convert`): of a register, the
            // register read at the narrow width (LOG_$ADD 0xE1775E `move.w
            // D5,-(SP)`, a longword held in D5; SOCK 0xE15ECE `move.b
            // D4,(0x1b,A2)`); of a variable in memory, the bytes at the
            // higher address, big-endian (APP_$DEMUX 0xE00AF6 `move.w
            // (0x1a,A0),(-0x14,A6)` for a longword at 0x18; PACCT_$LOG
            // 0xE5AB26 `move.b (1,A0),...` for a word); of a computed
            // value, its register (DBUF_$GET_BLOCK 0xE3A5F6 `move.w
            // D1,-(SP)` after `lsr.l #5,D1`).
            // A volatile object is read whole, at its own size (ref
            // 3.15.1-3.15.3: an access of another size is another
            // access; `chr(exec_$counter)`), and a longword narrowed to
            // a byte (`chr(l)`, not catalogued) too, as before.
            let v = self.eval(value)?;
            let k = (from.bytes() - ty.bytes()) as i32;
            return Ok(match v {
                Val::Reg(Reg::D(d)) => Val::Reg(Reg::D(d)),
                Val::Reg(Reg::A(a)) if ty.width == Width::Word => Val::Reg(Reg::A(a)),
                Val::Mem(ea) if narrows_in_place(value, ty) => match displace(&ea, k) {
                    Some(ea) => Val::Mem(ea),
                    None => Val::Tmp(self.in_tmp(Val::Mem(ea), from.width)?),
                },
                v => Val::Tmp(self.in_tmp(v, from.width)?),
            });
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
        // (`dpc-ir` reads a shift of an unsigned operand back as the
        // `integer` result, a same-width [`Expr::Convert`].)
        let shifted = match value {
            Expr::Convert { value: inner, .. } if inner.ty().width == from.width => inner,
            v => v,
        };
        if let Expr::Binary { op: BinOp::Shl, .. } = shifted {
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

    /// `value` (a longword `x & c`, `c` a `moveq` constant, `x` held in a
    /// data register) narrowed to the word `ty` as a word `and`
    /// ([`Self::convert`]); `None` for any other value.
    fn narrow_held_and(&self, value: &Expr, ty: Scalar) -> Option<Expr> {
        let Expr::Binary {
            op: BinOp::And,
            lhs,
            rhs,
            ty: t,
        } = value
        else {
            return None;
        };
        if t.width != Width::Long || ty.width != Width::Word {
            return None;
        }
        let (x, c) = match (&**lhs, &**rhs) {
            (x, Expr::Const { value: c, .. }) | (Expr::Const { value: c, .. }, x) => (x, *c),
            _ => return None,
        };
        let held = matches!(x, Expr::Load { addr: Addr::Frame(o), .. }
            if matches!(self.pinned(*o), Some(Reg::D(_))));
        if !held || !(-128..=127).contains(&c) {
            return None;
        }
        Some(Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(Expr::Convert {
                value: Box::new(x.clone()),
                ty,
            }),
            rhs: Box::new(Expr::Const { value: c, ty }),
            ty,
        })
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
        if let IrCond::Block { op, lhs, rhs, size } = c {
            // `sne`/`seq` into the count register, the first `bne` of the
            // unrolled form going to it (PACCT_$ON 0xE5A9B8..0xE5A9C0
            // `moveq #1,D0; cmpm.l; bne.b *+4; cmpm.l; sne D0`; 0xE57786
            // `seq D0`, 0xE56C20 `seq D1`, 0xE60F76 `seq D4`, each the
            // count's register).
            let differ = self.new_label();
            let (count, unrolled) = self.block_flags(lhs, rhs, *size, differ)?;
            if unrolled {
                self.place(differ);
            }
            self.out.push(Insn::Scc {
                cond: if *op == Rel::Eq { Cond::Eq } else { Cond::Ne },
                dst: Ea::Reg(Reg::D(count)),
            });
            return Ok(count);
        }
        if let IrCond::Odd(x) = c {
            // `btst #0; sne`: into the tested register when it held a value
            // computed for the test, as `in` does (REM_NAME_SERVER_LOCAL
            // 0xE4A412 `btst #13,D0; sne D0`), into a new one when it holds
            // a variable (AST_$GET_LOCATION 0xE0472E `btst #0,D2; sne D0`;
            // DISK_$ADD_QUE 0xE3C744 `btst #0,D2; sne D4`).
            let (d, held, _) = self.odd_test(x, true)?;
            let d = if held { self.alloc_d()? } else { d };
            self.out.push(Insn::Scc {
                cond: Cond::Ne,
                dst: Ea::Reg(Reg::D(d)),
            });
            return Ok(d);
        }
        if let IrCond::And(..) | IrCond::Or(..) = c {
            return self.logic_value(c);
        }
        let IrCond::Compare { op, lhs, rhs } = c else {
            return Err(unsupported(
                self.span,
                "a condition other than a comparison as a value (`not`, `and then`, a boolean \
                 variable) is",
            ));
        };
        let start = self.out.len();
        let outer = std::mem::replace(&mut self.test_as_value, true);
        let cc = self.compare(*op, lhs, rhs);
        self.test_as_value = outer;
        let cc = cc?;
        // The data registers the compare's operands used, last allocated
        // first.
        // Only a register the compare itself defined holds a value
        // computed for the test: one it merely reads may be a variable's
        // (a register-held local's web, read by `cmpi.w #3,Dm` in `(m > 3)
        // or b`, live after the test).
        let defined: Vec<Reg> = self.out[start..]
            .iter()
            .flat_map(|i| crate::regs::access(i).defs)
            .collect();
        let mut used: Vec<u8> = self.out[start..]
            .iter()
            .flat_map(|i| crate::regs::access(i).uses)
            .filter_map(|r| match r {
                Reg::D(n) if crate::regs::is_virtual(r) && defined.contains(&r) => Some(n),
                _ => None,
            })
            .collect();
        used.sort_unstable();
        // A register held for the whole body (a parameter loaded at
        // entry, a register-held local, a value held for reuse) is not
        // overwritten.
        let held: Vec<u8> = self
            .pinned
            .iter()
            .filter_map(|&(_, r)| match r {
                Reg::D(n) => Some(n),
                _ => None,
            })
            .chain(self.webs.iter().map(|&(_, d)| d))
            .chain(self.cse_regs.iter().copied())
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

    /// The address of `a` in a new address register, for `addr(x)` as a
    /// value stored into memory: `lea <ea>,An` (`lea; move.l An,<mem>` in
    /// compiled code: `(d,A5)` 19 sites, 0xE00964 `lea (24,A5),A0; move.l
    /// A0,(-108,A6)`; `(d,A6)` 18, 0xE11B1A; `(d,An)` 97, 0xE012CC; an
    /// element `(d,An,Dx)` 122, 0xE03576 `lea (-48,A1,D0.l),A2; move.l
    /// A2,(-76,A6)`, the lower bound folded in), or `movea.l #sym,An` for
    /// another unit's object (0xE00ECC).
    fn addr_into_a(&mut self, a: &Addr) -> Res<u8> {
        let r = self.alloc_a()?;
        self.lea_into(a, r)?;
        Ok(r)
    }

    /// The address of `a` into address register `r` ([`Self::addr_into_a`]):
    /// `lea <ea>,Ar`; `movea.l #sym,Ar` for another unit's object; for an
    /// element of an array a pointer in memory points to, the scaled index
    /// first, then the pointer, then the `lea` (NETLOG_$LOG_IT
    /// 0xE71BB6..0xE71BCA: the index times 26 into D1, `movea.l
    /// (116,A5),A0; lea (0,A0,D1.w),A0`).
    fn lea_into(&mut self, a: &Addr, r: u8) -> Res<()> {
        if let Addr::Index {
            base,
            index,
            scale,
            offset,
        } = a
        {
            if let Some((p, o)) = self.stepped_array(base, index, *scale) {
                // A stepped element's address: a `with a[i] do` record
                // (codegen-patterns 7.4, "`with`"): the pointer copied, the
                // array's offset and the lower bound in the references
                // ([`with_rebase`]; DBUF_$INVALIDATE 0xE3AA12 `movea.l
                // A2,A3`, then `(-8,A3)`, `(A3)`; DISK_$REGISTER 0xE3D9EC,
                // XNS_IDP_$OS_DEMUX 0xE185BC, PROC2_$PGROUP_INFO 0xE42244).
                if o + offset != 0 {
                    return Err(unsupported(
                        self.span,
                        "the address of a stepped element other than a `with` record's \
                         (`lea (d,Ap)` not catalogued, codegen-patterns 7.4) is",
                    ));
                }
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src: Ea::Reg(Reg::A(p)),
                    dst: r,
                });
                return Ok(());
            }
        }
        match a {
            // `lea (d,An),Ar` off the block's register (KBD_$INIT 0xE3339E
            // `movea.l #0xE2DDE4,A0; lea (0x58,A0),A1`, through the generic
            // case below); the block's start itself is not seen.
            Addr::OtherStatic { offset: 0, .. } => {
                return Err(unsupported(
                    self.span,
                    "the address of the first variable of another data section's block as a \
                     value (not seen, codegen-patterns 11) is",
                ))
            }
            Addr::Symbol { name, offset: 0 } => self.out.push(Insn::Movea {
                size: Size::L,
                src: Ea::ImmSym(name.clone()),
                dst: r,
            }),
            Addr::Symbol { .. } | Addr::Const(_) => {
                // 6 `lea abs.l,An; move.l An,Dn` (0xE310AA), and `move.l
                // #sym+d` / `pea abs.l` for arguments (codegen-patterns 1.5):
                // not settled.
                return Err(unsupported(
                    self.span,
                    "the address of a constant or of a part of another unit's object as a value \
                     is",
                ));
            }
            Addr::Index {
                base,
                index,
                scale,
                offset,
            } if matches!(&**base, Addr::Indirect { base: p, .. }
                if matches!(&**p, Expr::Load { addr: Addr::Frame(_) | Addr::Static(_), .. })
                    && !matches!(&**p, Expr::Load { addr: Addr::Frame(f), .. }
                        if self.pinned(*f).is_some())) =>
            {
                let Addr::Indirect { base: p, offset: o } = &**base else {
                    unreachable!()
                };
                let Expr::Load { addr: pa, .. } = &**p else {
                    unreachable!()
                };
                let long = index.ty().width == Width::Long;
                let x = self.index_reg(index, *scale)?;
                let b = self.alloc_a()?;
                let src = self.operand(pa)?;
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src,
                    dst: b,
                });
                let disp = o + offset;
                if !(-128..128).contains(&disp) {
                    return Err(unsupported(self.span, "a displacement beyond 8 bits is"));
                }
                self.out.push(Insn::Lea {
                    src: Ea::Idx(disp, b, x, long),
                    dst: r,
                });
            }
            // Anything else: `lea <ea>,Ar`.  The element of another unit's
            // array, or of a byte array through a pointer loaded from memory
            // ([`Self::split_base`]), takes the base and the index as an
            // operand does ([`Self::index_ea`]: the index's value, the base,
            // the scaling; for a byte element the index, then the pointer),
            // then `lea (d,An,Dx),Ar` (codegen-patterns 5.2, "The element's
            // address as a value").  Whether `Ar` is the base's register or
            // another is the allocator's choice: the dying base's register
            // counts as free for an indexed `lea` (12.4 R2 does not apply to
            // it), so the address takes it where that is the lowest register
            // free over the address's range (XPD_$FIND_INDEX 0xE5AF56..
            // 0xE5AF60 `movea.l #0xEA551C,A0; muls.w #228,D1; lea
            // (0,A0,D1.l),A0`, a `with` record read twice) and another where
            // it is not (MMAP_$AVAIL 0xE0CC76..0xE0CC80 `movea.l #MMAP,A0;
            // ...; lea (0,A0,D0.l),A2`, a callee-saved A2 across the call).
            // Only an address read twice or more, or live across a call, is
            // settled; one read once in no call's range is refused after
            // selection ([`select_body_with`], `elem_leas`).
            _ => {
                let split = self.split_base(a);
                let src = self.ea_of(a)?;
                self.out.push(Insn::Lea { src, dst: r });
                if split {
                    self.elem_leas.push((self.out.len() - 1, self.span));
                }
            }
        }
        Ok(())
    }

    /// The value of a plain `a and b` / `a or b` (ref "And", p. 4-18; "Or",
    /// p. 4-134: both operands may be evaluated, in any order) in a data
    /// register, its flags those of the `and.b`/`or.b` that formed it
    /// (codegen-patterns 7.1, "Plain `and` and `or`"; `tools/boolsurvey.py
    /// andor`, 133 sites): the left operand's boolean into a register of
    /// its own ([`Self::bool_into_reg`]), then `and.b`/`or.b` of the right
    /// one into it ([`Self::bool_operand`]).  0xE4BC2A..0xE4BC4A `cmpi.l
    /// #..,(8,A6); seq D0; cmpi.l #..,(8,A6); seq D1; or.b D1,D0; cmpi.l;
    /// seq D1; or.b D1,D0` (`a or b or c`, left to right); TTY_$I_ERR
    /// 0xE1BE48..0xE1BE5E `seq D1b; ...; sne D3b; and.b D3b,D1b`.
    fn logic_value(&mut self, c: &IrCond) -> Res<u8> {
        let (op, a, b) = match c {
            IrCond::And(a, b) => (AluOp::And, a, b),
            IrCond::Or(a, b) => (AluOp::Or, a, b),
            _ => unreachable!("logic_value of a plain and/or"),
        };
        if cond_has_call(b) {
            // The right operand calls: compiled code forms such an operand
            // only after a left one that is jumping code ([`Self::branch`];
            // the TPAD sites of codegen-patterns 7.1), never combined with
            // `and.b`/`or.b` (`tools/cmpsurvey.py callcmp`).  As a value,
            // or under `not`: not seen.
            return Err(unsupported(
                self.span,
                "a plain `and`/`or` with a call in its right operand outside a condition, or \
                 under `not` (codegen-patterns 7.1; bead domain-pascal-1rwo) is",
            ));
        }
        let d = self.bool_into_reg(a)?;
        let src = self.bool_operand(b)?;
        self.out.push(Insn::Alu {
            op,
            size: Size::B,
            src,
            dst: d,
        });
        Ok(d)
    }

    /// The right operand of a plain `and`/`or` that calls a function, in
    /// a condition ([`Self::branch`]): a comparison of the function's
    /// result, formed as a boolean and tested, the branch on its sign.
    /// The call first, its result in D0 compared there (`tst`, `cmpi
    /// #c,D0`, `cmp.<s> v,D0`: the call's result is the left operand, the
    /// relation reversed when the source has it on the right), `s<cc>`
    /// into a data register as if live across the call, the lowest
    /// callee-saved one free, `tst.b` of it (codegen-patterns 7.1; bead
    /// domain-pascal-i1by; `tools/cmpsurvey.py callcmp`).  Image: the five
    /// TPAD compares of a word through a held parameter address
    /// (TPAD_$SET_UNIT_MODE 0xE69814 `cmp.w (A4),D0; slt D2; tst.b D2`;
    /// TPAD_$SET_UNIT 0xE699F8 `sge D2`; 0xE69A4C, 0xE69AFC, 0xE69B52),
    /// and the other compares of a call's result formed so: through a
    /// pointer, D2 (XNS_IDP_$OS_DEMUX 0xE184FA `cmp.w (A2),D0; sne D2`;
    /// DISK_IO 0xE3D7F4, 0xE3D86E); with a register-held variable, D5
    /// with D2..D4 live (0xE5E590 `cmp.l D3,D0; sne D5`; 0xE68B0E); with
    /// 0 or a constant, a callee-saved register at all 20 (FILE_$SET_PROT_INT
    /// 0xE5DE5A `tst.w D0; sne D2; tst.b D2`; 0xE6EBE2 `sne D3`, D2 live;
    /// 0xE12B6A `seq D7`).
    ///
    /// Refused, the register not explained: a static variable
    /// (TPAD_$SET_UNIT_CURSOR 0xE698E0 and TPAD_$INQUIRE_UNIT 0xE6998A
    /// `cmp.w (0x17c,A5),D0; slt D0`, the call's own register); an
    /// enclosing routine's variable (the module-locals 0xE10628 at
    /// 0xE10D0C `cmp.l (-0x22e,A2),D0; sne D1` and 0xE75DE4 at 0xE75ECE,
    /// A2 the dynamic link).  Refused, not seen: a variable in the frame
    /// (no compare of one with a function's result; 0xE0FD5A `cmp.l
    /// (d,A6),D0; ble` and 0xE30CD2 `bls` compare M$ARITH results), another
    /// unit's, an element, one whose address is loaded for the compare;
    /// widened operands; a pointer function.
    fn call_compare_test(&mut self, c: &IrCond, when: bool, l: u32) -> Res<()> {
        const OTHER: &str = "a comparison with a function's result in the right operand of a \
             plain `and`/`or` other than with a constant or with a variable held in a register \
             or read through a held address (codegen-patterns 7.1) is";
        let IrCond::Compare { op, lhs, rhs } = c else {
            return Err(unsupported(
                self.span,
                "a right operand of a plain `and`/`or` with a call other than a comparison of \
                 the function's result is",
            ));
        };
        let (op, call, other) = if d0_call(lhs) {
            (*op, lhs, rhs)
        } else if d0_call(rhs) {
            (swap_rel(*op), rhs, lhs)
        } else {
            return Err(unsupported(self.span, OTHER));
        };
        if expr_has_call(other) {
            return Err(unsupported(self.span, OTHER));
        }
        let ty = call.ty();
        let size = size_of(ty.width);
        self.eval(call)?;
        let mut held = None;
        match other {
            Expr::Const { value: 0, .. } => self.out.push(Insn::Tst {
                size,
                dst: Ea::Reg(Reg::D(0)),
            }),
            Expr::Const { value, .. } => self.out.push(Insn::Cmpi {
                size,
                imm: imm(*value, ty.width),
                dst: Ea::Reg(Reg::D(0)),
            }),
            Expr::Load { addr, .. } => {
                match addr {
                    Addr::Static(_) | Addr::OtherStatic { .. } => {
                        return Err(unsupported(
                            self.span,
                            "a comparison of a function's result with a static variable in the \
                             right operand of a plain `and`/`or` (`s<cc>` into D0, the call's \
                             register, at the two sites, not explained; codegen-patterns 7.1) is",
                        ))
                    }
                    Addr::Enclosing(_) => {
                        return Err(unsupported(
                            self.span,
                            "a comparison of a function's result with an enclosing routine's \
                             variable in the right operand of a plain `and`/`or` (`s<cc>` into \
                             D1 at the two sites, not explained; codegen-patterns 7.1) is",
                        ))
                    }
                    Addr::Indirect { .. } => {}
                    Addr::Frame(o) if matches!(self.pinned(*o), Some(Reg::D(_))) => {}
                    _ => return Err(unsupported(self.span, OTHER)),
                }
                let start = self.out.len();
                let src = self.operand(addr)?;
                let seen = match src {
                    Ea::Ind(a) | Ea::Disp(_, a) => crate::regs::is_virtual(Reg::A(a)),
                    Ea::Reg(Reg::D(_)) => true,
                    _ => false,
                };
                if !seen || self.out.len() != start {
                    return Err(unsupported(self.span, OTHER));
                }
                held = match src {
                    Ea::Reg(Reg::D(r)) => Some(r),
                    _ => None,
                };
                self.out.push(Insn::Cmp { size, src, dst: 0 });
            }
            _ => return Err(unsupported(self.span, OTHER)),
        }
        let d = self.alloc_d()?;
        self.callee_only.push(d);
        if let Some(r) = held {
            // Not the compared register either, even where the compare is
            // its last use: the value is coloured as if live from the call
            // on, where the register is live (0xE5E590 `cmp.l D3,D0; sne
            // D5`, D3 live after; no site where it dies).
            self.colour_apart.push((Reg::D(d), Reg::D(r)));
        }
        self.out.push(Insn::Scc {
            cond: rel_cond(op, ty.sign == Sign::Signed),
            dst: Ea::Reg(Reg::D(d)),
        });
        self.out.push(Insn::Tst {
            size: Size::B,
            dst: Ea::Reg(Reg::D(d)),
        });
        self.jump(if when { Cond::Mi } else { Cond::Pl }, l);
        Ok(())
    }

    /// The boolean value of `c` in a data register of its own, as the left
    /// operand of a plain `and`/`or` (`tools/boolsurvey.py andor`):
    ///
    /// - a comparison, `odd`, a constant member of a register-sized set:
    ///   `s<cc>` ([`Self::test_value`]; 0xE4BC32 `seq D0`; 0xE01978
    ///   `btst #13,D0; sne D1`);
    /// - a boolean variable in memory: `move.b <mem>,Dn` (0xE6E3D8 `move.b
    ///   (-28,A6),D4; and.b D0,D4`); one held in a register, copied:
    ///   `move.b Dh,Dn` (0xE01A20 `move.b D2,D0; and.b D3,D0`; 0xE39ED8;
    ///   0xE6973A);
    /// - `not v` of such a variable: the same, then `not.b Dn` (0xE1009C
    ///   `move.b (26,A6),D3; not.b D3`; 0xE3BAAE; 0xE51482 `move.b D4,D7;
    ///   not.b D7`);
    /// - a function's boolean result: its D0 (0xE072AA `or.b (-18,A6),D0`
    ///   right after the call);
    /// - a nested plain `and`/`or`: its register (0xE4BC4A, a chain).
    fn bool_into_reg(&mut self, c: &IrCond) -> Res<u8> {
        match c {
            IrCond::And(..) | IrCond::Or(..) => self.logic_value(c),
            IrCond::Not(x) if matches!(&**x, IrCond::Test(Expr::Load { .. })) => {
                let d = self.bool_into_reg(x)?;
                self.out.push(Insn::Unary {
                    op: UnaryOp::Not,
                    size: Size::B,
                    dst: Ea::Reg(Reg::D(d)),
                });
                Ok(d)
            }
            IrCond::Test(e @ Expr::Load { .. }) => {
                let src = match self.eval(e)? {
                    Val::Mem(ea) => ea,
                    Val::Reg(r) => Ea::Reg(r),
                    // Bead domain-pascal-1dn: a refusal, not a crash, if
                    // a variable's value is ever anything else.
                    _ => {
                        return Err(unsupported(
                            self.span,
                            "this boolean variable operand of a plain `and`/`or` is",
                        ))
                    }
                };
                let d = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: Size::B,
                    src,
                    dst: Ea::Reg(Reg::D(d)),
                });
                Ok(d)
            }
            IrCond::Test(e @ (Expr::Call { .. } | Expr::SetMember { .. } | Expr::Bits { .. })) => {
                match self.eval(e)? {
                    Val::Tmp(d) => Ok(d),
                    // A function's result register.
                    Val::Reg(Reg::D(d)) if matches!(e, Expr::Call { .. }) => Ok(d),
                    _ => Err(unsupported(
                        self.span,
                        "this operand of a plain `and`/`or` is",
                    )),
                }
            }
            IrCond::Compare { .. } | IrCond::Block { .. } | IrCond::Odd(_) => self.test_value(c),
            _ => Err(unsupported(
                self.span,
                "this operand of a plain `and`/`or` (`not` of a computed boolean, `and then`, \
                 `or else`: not catalogued) is",
            )),
        }
    }

    /// The right operand of a plain `and`/`or`: a boolean variable as it
    /// is, in memory or in its register (0xE39EDA `and.b (20,A6),D6`;
    /// 0xE072AA `or.b (-18,A6),D0`; 0xE0F554 `or.b (844,A5),D0`; 0xE01A22
    /// `and.b D3,D0`, D3 a register-held boolean), anything else in a
    /// register of its own ([`Self::bool_into_reg`]).
    fn bool_operand(&mut self, c: &IrCond) -> Res<Ea> {
        if let IrCond::Test(e @ Expr::Load { .. }) = c {
            return match self.eval(e)? {
                Val::Mem(ea) => Ok(ea),
                Val::Reg(r) => Ok(Ea::Reg(r)),
                _ => Err(unsupported(
                    self.span,
                    "this boolean variable operand of a plain `and`/`or` is",
                )),
            };
        }
        Ok(Ea::Reg(Reg::D(self.bool_into_reg(c)?)))
    }

    /// `odd(x)` (ref "Odd", p. 4-128), the instructions that test it and
    /// the condition that holds when `x` is odd.  The register and whether
    /// it holds the value for longer (a register-held variable) are
    /// returned too.  Codegen-patterns 8, `odd`; bead domain-pascal-3xwx.
    ///
    /// - `x` in a data register (a register-held variable, or a value
    ///   computed for the test): `btst #0` on it, Z (FIM_$BUILD_DF
    ///   0xE0A6F6 `btst #0,D5`, 0xE0A8EE `btst #0,D0`, both register-held
    ///   longwords).
    /// - A word or longword variable in memory: loaded whole into a data
    ///   register, then `lsr.b #1` and the carry (`tools/setsurvey.py
    ///   onebit`: all 9 `lsr.b #1` in compiled code, each on a word or
    ///   longword just loaded, 7 of them followed by `bcc`/`bcs`:
    ///   tty_$i_put_chars 0xE1B0F6 `move.l (12,A0),D1; lsr.b #1,D1;
    ///   bcc.s`; AREA_$COPY 0xE0905A `move.w (46,A1),D0; lsr.b #1,D0`).
    ///   The reading: these are `odd` of an integer, bit 0 of the same
    ///   fields being otherwise tested with `btst #k` on the loaded value
    ///   (`&`, [`Self::one_bit_test`]); `btst #0` after a whole load (4
    ///   sites, 0xE05C5E with the loaded value reused, DTTY_$INIT
    ///   0xE34C4C, PEB_$GET_STATUS 0xE5ADEA, TERM_$INQUIRE 0xE66E9E) is
    ///   `(x & 1) <> 0`.
    ///   **Uncertain**: the image cannot show the source.
    ///
    /// Refused: a byte variable in memory (no `lsr.b #1` after a byte
    /// load); a variable in memory as a value (`scs` into a new register
    /// at both sites, 0xE052FE and 0xE70E9C, the latter with the load in
    /// D3: not modelled; bead domain-pascal-uwfe); a constant.
    fn odd_test(&mut self, x: &Expr, value: bool) -> Res<(u8, bool, Cond)> {
        let (d, held) = match self.eval(x)? {
            Val::Reg(Reg::D(r)) => (r, true),
            Val::Tmp(r) => (r, false),
            Val::Mem(ea) if x.ty().width != Width::Byte && !value => {
                let d = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(x.ty().width),
                    src: ea,
                    dst: Ea::Reg(Reg::D(d)),
                });
                self.out.push(Insn::Shift {
                    op: ShiftOp::Lsr,
                    size: Size::B,
                    count: ShiftCount::Imm(1),
                    dst: d,
                });
                return Ok((d, false, Cond::Cs));
            }
            Val::Mem(_) if value => {
                return Err(unsupported(
                    self.span,
                    "`odd` of a variable in memory as a boolean value (`lsr.b #1; scs` into \
                     another register at 0xE052FE and 0xE70E9C: not settled, bead \
                     domain-pascal-uwfe) is",
                ))
            }
            Val::Mem(_) => {
                return Err(unsupported(
                    self.span,
                    "`odd` of a byte variable in memory (no `lsr.b #1` after a byte load in \
                     compiled code) is",
                ))
            }
            _ => return Err(unsupported(self.span, "`odd` of a constant here is")),
        };
        self.out.push(Insn::BtstImm {
            bit: 0,
            dst: Ea::Reg(Reg::D(d)),
        });
        Ok((d, held, Cond::Ne))
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
            if let (Expr::SetOp { ty, .. }, Rel::Eq | Rel::Ne) = (lhs, op) {
                // `x * y <> []`, `x - y = []` (ref p. 4-176): the branch
                // reads the flags of the `and` that formed the set (TTY
                // 0xE1BFFE `moveq #7,D1; and.w (8,A2),D1; beq.s`, TTY_$K_PUT
                // 0xE67D04; PROC2_$SIGPAUSE 0xE3FACC `and.l (-100,A4),D1;
                // beq.s`; codegen-patterns 5.1).
                let d = match self.eval(lhs)? {
                    Val::Tmp(d) => d,
                    _ => unreachable!("a set operation's value is a new register"),
                };
                if !self.sets_flags(d, ty.width) {
                    return Err(unsupported(
                        self.span,
                        "comparing this set value with `[]` is",
                    ));
                }
                return Ok(if op == Rel::Eq { Cond::Eq } else { Cond::Ne });
            }
            if let (Expr::Bits { ty, .. }, Rel::Eq | Rel::Ne) = (lhs, op) {
                if *ty != Scalar::BOOLEAN {
                    // A bit field against 0: the branch reads the flags of
                    // the `and` or the shift that formed it (0xE1BFFE
                    // `moveq #7,D1; and.w (8,A2),D1; beq.s`; 0xE5BFBE
                    // `move.w #480,D1; and.w (16,A0),D1; lsr.w #5,D1;
                    // beq.s`).
                    self.eval(lhs)?;
                    return Ok(if op == Rel::Eq { Cond::Eq } else { Cond::Ne });
                }
            }
            return self.compare_computed_zero(op, lhs);
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
            // Two widened words (a signed and an unsigned one, `dpc-ir`
            // `widen_operands`): the right one widened into a register of
            // its own after the left one, `cmp.l Dr,Dl` (TTY_$K_PUT
            // 0xE67C42..0xE67C4A `move.w D2,D0; clr.l D1; ext.l D0; move.w
            // (A4),D1; cmp.l D1,D0`, the two widenings interleaved by the
            // scheduler, 12.4 R11; tty_$i_put_chars 0xE1B050..0xE1B058).
            Expr::Convert { value, ty: t }
                if t.width == Width::Long && value.ty().width == Width::Word =>
            {
                let r = match self.eval(rhs)? {
                    Val::Tmp(r) | Val::Reg(Reg::D(r)) => r,
                    _ => unreachable!("a widening is in a data register"),
                };
                self.out.push(Insn::Cmp {
                    size,
                    src: Ea::Reg(Reg::D(r)),
                    dst: d,
                });
            }
            _ => return Err(unsupported(self.span, "comparing two computed values is")),
        }
        Ok(rel_cond(op, signed))
    }

    /// `lhs op 0` with a left operand that is not a variable (bead
    /// domain-pascal-qlzu; codegen-patterns 7.1, "A computed value against
    /// 0"; `tools/cmpsurvey.py opflags`):
    ///
    /// - `(x & 2^k) = 0` or `<> 0` (or `(2^k & x)`): `btst #k` on `x` in a
    ///   data register, a word or longword in memory loaded whole first
    ///   ([`Self::one_bit_test`]; pacct_$compress 0xE5A9DE `btst #2,D0;
    ///   sne D2`, ../source `(value & 4) <> 0`).  No `and` with a one-bit
    ///   mask is followed by a branch or an `s<cc>` in compiled code,
    ///   against 92 with other masks, immediate or loaded.
    /// - Anything else: the value in a data register, then the test
    ///   [`Self::tst`] gives (through [`Self::flags_test`]): the branch on
    ///   the flags of the operation that formed it (SOCK_$ALLOCATE
    ///   0xE15EFA `or.w D4,D0; beq.s`; SOCK_$CLOSE 0xE16004;
    ///   SIO6509_$INQ_LINE 0xE7267C `andi.l #0x180,D1; beq.s`), or `tst.<s>
    ///   Dn` (a function's result, SOCK_$PUT_INT 0xE161EA `tst.w D0; seq
    ///   D0`, 31 branches on `tst` of a call's D0; SMD_$SIGNAL 0xE6F25C
    ///   `sub.l D0,D1; tst.l D1; bgt.s`).
    ///
    /// The relations map to conditions as for a variable ([`Self::compare`]):
    /// unsigned `<`, `<=`, `>`, `>=` against 0 are refused.
    fn compare_computed_zero(&mut self, op: Rel, lhs: &Expr) -> Res<Cond> {
        let ty = lhs.ty();
        let size = size_of(ty.width);
        let cc = match (op, ty.sign == Sign::Signed) {
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
        if let (
            Expr::Binary {
                op: BinOp::And,
                lhs: a,
                rhs: b,
                ..
            },
            Rel::Eq | Rel::Ne,
        ) = (lhs, op)
        {
            // The mask on either side (`4 & x` as `x & 4`): the source
            // order cannot be seen in the image, and the `and` with a
            // one-bit mask the other order would give is not seen.
            let (x, m) = match (&**a, &**b) {
                (Expr::Const { .. }, _) => (b, a),
                _ => (a, b),
            };
            if let Expr::Const { value, .. } = **m {
                let mask = match ty.width {
                    Width::Byte => value & 0xff,
                    Width::Word => value & 0xffff,
                    Width::Long => value & 0xffff_ffff,
                } as u64;
                if mask.is_power_of_two() {
                    self.one_bit_test(x, mask.trailing_zeros() as u8)?;
                    return Ok(cc);
                }
            }
        }
        let d = match self.eval(lhs)? {
            Val::Tmp(d) | Val::Reg(Reg::D(d)) => d,
            v => self.in_tmp(v, ty.width)?,
        };
        self.tst(size, Ea::Reg(Reg::D(d)), cc)?;
        Ok(cc)
    }

    /// `(x & 2^k) <> 0` or `= 0` (the mask on either side): `btst #k` on
    /// `x` in a data register, the branch on Z (bead domain-pascal-3xwx;
    /// codegen-patterns 7.1, "One bit"; `tools/setsurvey.py onebit`).
    /// Compiled code never tests a one-bit `and` mask (0 of the 92 masks
    /// followed by a branch or an `s<cc>`, `tools/cmpsurvey.py opflags`;
    /// none in `tools/setsurvey.py onebit` either):
    ///
    /// - `x` held in a data register: `btst #k` on it (pacct_$compress
    ///   0xE5A9DE `btst #2,D0; sne D2`, ../source `(value & 4) <> 0`).
    /// - `x` a word or longword variable in memory: loaded whole, at its
    ///   own size, into a data register, then `btst #k` on it (the 80
    ///   `whole` tests of `tools/setsurvey.py onebit`, codegen-patterns
    ///   7.1, 78 in 5.1's `ctest` count, which read more than the part
    ///   holding bit k: tty_$i_put_chars 0xE1B01E `move.w (8,A0),D1;
    ///   btst #5,D1; beq.s`, 0xE1B0E6 `move.l (12,A0),D1; btst #3,D1`;
    ///   AST 0xE013BA `move.w (190,A2),D0; btst #4,D0`; TTY_$I_ERR
    ///   0xE1BE3C; FILE_$PRIV_LOCK 0xE5F7B8 on an indexed word).  The
    ///   reading (5.1): `in` on a set in memory is narrowed to the byte or
    ///   word holding the bit, an integer is read whole.  **Uncertain**:
    ///   the image cannot show the source.
    /// - `x` computed (an operation, not a call or a widening): `btst #k`
    ///   on the register that holds it (DIR_$SET_PROTECTION 0xE522D4
    ///   `lsr.w #4,D1; btst #4,D1; bne`, DIR_$SET_DEFAULT_ACL 0xE530D6,
    ///   FILE_$SET_PROT 0xE5DF5C).
    ///
    /// Not reproduced: a loaded copy tested again for another bit later
    /// (TTY_$I_ERR 0xE1BE62 `btst #11,D2` on the longword 0xE1BE54
    /// loaded; AREA_ 0xE07C80): each test loads again (bead
    /// domain-pascal-gwpz).
    ///
    /// Refused: bit 7, 15 or 31 (no `btst #7`, `#15` or `#31` on any
    /// operand among the 926 one-bit tests of compiled code; a lead for
    /// bit 7 of a word: 0xE19106..0xE1910C `move.w (6,A0),D3; tst.b D3;
    /// bpl`; bead domain-pascal-fhd4);
    /// a byte variable in memory (`move.b` then `btst` at 4 sites, each
    /// explained otherwise, against `btst #k` on 66 even-addressed bytes:
    /// not settled; bead domain-pascal-kqa2); a variable in memory as a
    /// boolean value (the `s<cc>` register after a whole load: the loaded
    /// one at 0xE04C3C and 0xE5EC4A, another at 0xE02CD8: not settled;
    /// bead domain-pascal-uwfe); a call's result (not
    /// seen); a byte or a word widened for the `&` (an integer mask on a
    /// `[byte]` or `0..65535` variable: the widening is not seen before a
    /// `btst`, `clr.w; move.b` only into register-held locals, 0xE0A3BC;
    /// beads domain-pascal-kqa2, domain-pascal-9js).
    fn one_bit_test(&mut self, x: &Expr, k: u8) -> Res<()> {
        if matches!(k, 7 | 15 | 31) {
            return Err(unsupported(
                self.span,
                "testing bit 7, 15 or 31 alone (no `btst #7`/`#15`/`#31` exists in compiled \
                 code; lead: 0xE1910A `tst.b` of a loaded word; bead domain-pascal-fhd4) is",
            ));
        }
        match x {
            Expr::Call { .. } => {
                return Err(unsupported(
                    self.span,
                    "testing one bit of a function's result (`btst` on D0 after a call is not \
                     seen in compiled code) is",
                ))
            }
            Expr::Convert { value, .. } if value.ty().width == Width::Byte => {
                return Err(unsupported(
                    self.span,
                    "testing one bit of a byte widened for the `&` (`btst` on the byte in \
                     memory, or `clr.w; move.b` first: not settled, bead domain-pascal-kqa2) is",
                ))
            }
            Expr::Convert { .. } => {
                return Err(unsupported(
                    self.span,
                    "testing one bit of a word widened to a longword for the `&` (an unsigned \
                     word with an integer mask: `clr.l; move.w; btst` is not seen in compiled \
                     code; beads domain-pascal-kqa2, domain-pascal-9js) is",
                ))
            }
            _ => {}
        }
        let r = match self.eval(x)? {
            Val::Reg(Reg::D(r)) | Val::Tmp(r) => r,
            Val::Mem(_) if x.ty().width == Width::Byte => {
                return Err(unsupported(
                    self.span,
                    "testing one bit of a byte variable in memory (`btst` on the byte, or the \
                     byte loaded first: not settled, bead domain-pascal-kqa2) is",
                ))
            }
            Val::Mem(_) if self.test_as_value => {
                return Err(unsupported(
                    self.span,
                    "one bit of a variable in memory as a boolean value (which register takes \
                     the `s<cc>` after the load is not settled, bead domain-pascal-uwfe) is",
                ))
            }
            Val::Mem(ea) => {
                let d = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(x.ty().width),
                    src: ea,
                    dst: Ea::Reg(Reg::D(d)),
                });
                d
            }
            _ => return Err(unsupported(self.span, "testing one bit of this operand is")),
        };
        self.out.push(Insn::BtstImm {
            bit: k,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok(())
    }

    /// Whether a test of data register `d` at `size`, for a branch on
    /// `cc` right after the instruction that last wrote it, needs a `tst`
    /// (`true`) or reads that instruction's flags (`false`), whether the
    /// value was computed for the test or is a register-held variable an
    /// assignment just formed (bead domain-pascal-qlzu; codegen-patterns
    /// 7.1, "A computed value against 0"; `tools/cmpsurvey.py opflags`
    /// over compiled code: of 1156 branches and `s<cc>`s after an
    /// operation on a data register, 993 read its flags, 144 follow a
    /// `tst`, none a `cmpi #0`).  The rule found: the original trusts the
    /// N and Z flags of an add, sub, and, or, shift, `not` or multiply,
    /// not their V flag, and none of a divide.
    ///
    /// - `=`, `<>`, `<`, `>=` (`beq`/`bne`/`bmi`/`bpl`) after an add, sub,
    ///   and, or, `lsr`, `asr`, `lsl`, `not` or `mul` of `d` at `size`: the
    ///   operation's flags (eq/ne: and 113, lsr 36, sub 15, add 4, or 2;
    ///   SOCK_$ALLOCATE 0xE15EFA `or.w D4,D0; beq.s`; MAC_OS_$SEND's nested
    ///   routine 0xE0B522 at 0xE0B57C `sub.w D5,D3; bne.b`, `remaining := remaining - chunk;
    ///   if remaining = 0`.  mi/pl: tty_$i_put_chars 0xE1B046 `subq.w
    ///   #1,D1; bpl.s`, `free_space := head - tail - 1; if free_space <
    ///   0`; TTY_$K_PUT 0xE67C1A; TTY_$K_GET 0xE1C516 `sub.w (716,A2),D0;
    ///   bpl.s`; netbuf_rtnva_locked 0xE0E8E0 `asr.l #2,D2; bmi.s`, `slot
    ///   := (va - base) div 1024; if slot < 0`, NETBUF_$RTN_HDR 0xE0EED6;
    ///   `and.b`/`or.b` of booleans 70, `not.b` 1, `lsl.l` 1, `muls.w` 2).
    ///   Of the same-size `tst`s after these with `beq`/`bne`/`bmi`/`bpl`,
    ///   all are at a join, a label between (0xE08F80, 0xE3C930, 0xE4445E,
    ///   0xE3DF66, 0xE3EF72): a label is the last instruction
    ///   there, and the `tst` is kept.  N and Z are those `tst` would
    ///   give, so the relation tested is the same.
    /// - `>`, `<=` (`bgt`/`ble`, which read V) after an add or sub: `tst`
    ///   (7 of 7, none on the flags: SMD_$SIGNAL 0xE6F25C `sub.l D0,D1;
    ///   tst.l D1; bgt.s`, 0xE6F270; NETWORK_$WRITE 0xE0FBF6;
    ///   PKT_$COPY_TO_PA 0xE125E0; SHAKE 0xE3E4CC; LOG_$ADD 0xE1766A).
    ///   On the operation's flags the overflow would make them test the
    ///   exact sum or difference, not the 16- or 32-bit one Pascal keeps.
    /// - The operation at another size than the test (and.b of a byte
    ///   tested as a word, 0xE0A544 `moveq #7,D1; and.b (-64,A6),D1; tst.w
    ///   D1; bne.s`; and.w as a longword, 0xE3EE6E; lsr.l, asr.l or `swap`
    ///   tested as a word, 0xE1701C, 0xE130AA, NETWORK_$PAGE_SERVER
    ///   0xE11822), a divide (0xE11BF4 `divs.w #60,D1; tst.w D1; beq.s`;
    ///   0xE6966E `divs.w D1,D0; tst.w D0; bpl.s`: N and Z are undefined
    ///   after an overflowing divide), a call (31 `tst` of a call's D0
    ///   then a branch; 7 after an M$ARITH call, 0xE14C02 `tst.l D0`),
    ///   anything else: `tst` (a load into `d`: refused by [`Self::tst`]).
    ///
    /// Refused (no site either way): `>`, `<=` after an and, or, shift,
    /// `not` or multiply (their V is clear, the original's choice is not
    /// seen); a `muls.w`/`mulu.w` product tested as a word (the two
    /// branches after a `muls.w`, 0xE69626 and 0xE69630, are `abs` of a
    /// word product reading the 32-bit flags, no `tst.w`); any test after
    /// an eor, a `neg`, an `ext` or a `swap` tested
    /// as a longword (no branch or `tst` follows one in compiled code;
    /// bead domain-pascal-bkoz).
    fn flags_test(&self, d: u8, size: Size, cc: Cond) -> Res<bool> {
        #[derive(PartialEq)]
        enum Op {
            /// Add or subtract: N and Z trusted, V not.
            AddSub,
            /// And, or, shift, `not`, multiply: N and Z trusted.
            Nz,
            /// No branch after one seen.
            Unseen,
        }
        let alu = |op: AluOp| match op {
            AluOp::Add | AluOp::Sub => Op::AddSub,
            AluOp::And | AluOp::Or => Op::Nz,
            AluOp::Eor => Op::Unseen,
        };
        let reg = Ea::Reg(Reg::D(d));
        let (op, s) = match self.out.last() {
            Some(Insn::Alu {
                op, size: s, dst, ..
            }) if *dst == d => (alu(*op), *s),
            Some(Insn::AluImm {
                op, size: s, dst, ..
            }) if *dst == reg => (alu(*op), *s),
            Some(Insn::Addq { size: s, dst, .. } | Insn::Subq { size: s, dst, .. })
                if *dst == reg =>
            {
                (Op::AddSub, *s)
            }
            Some(Insn::Shift { size: s, dst, .. }) if *dst == d => (Op::Nz, *s),
            Some(Insn::Unary { op, size: s, dst }) if *dst == reg => (
                if *op == UnaryOp::Not {
                    Op::Nz
                } else {
                    Op::Unseen
                },
                *s,
            ),
            // `mulu.w`/`muls.w` and `swap` set the flags from all 32 bits.
            // A word product tested as a word: not seen either way (the
            // two branches after a `muls.w`, 0xE69626 and 0xE69630, read
            // its 32-bit flags for `abs` of a word product, `muls.w;
            // bpl.s; neg.w`, no `tst.w`): refused, not re-tested.
            Some(Insn::Mul { dst, .. }) if *dst == d => {
                if size != Size::L {
                    return Err(self.flags_refusal());
                }
                (Op::Nz, Size::L)
            }
            Some(Insn::Swap(r)) if *r == d => (Op::Unseen, Size::L),
            Some(Insn::Ext { size: s, dst }) if *dst == d => (Op::Unseen, *s),
            _ => return Ok(true),
        };
        if s != size {
            return Ok(true);
        }
        match (op, cc) {
            (Op::AddSub | Op::Nz, Cond::Eq | Cond::Ne | Cond::Mi | Cond::Pl) => Ok(false),
            (Op::AddSub, Cond::Le | Cond::Gt) => Ok(true),
            _ => Err(self.flags_refusal()),
        }
    }

    fn flags_refusal(&self) -> Diagnostic {
        unsupported(
            self.span,
            "this test of a value right after the operation that formed it (the flags of the \
             operation or a `tst`: not seen for this operation and relation, codegen-patterns \
             7.1) is",
        )
    }

    /// A bit field that fills its unit: an 8-bit field of a word type
    /// (`0..255`) on a byte boundary.  The general forms would read it with
    /// `move.w #255,Dn; and.b M,Dn` and clear it with `andi.b #0,M`; the
    /// image has no `andi #0` on memory at all, and how the original read
    /// and wrote such a field (`move.b` with a zero-extension?) is not
    /// catalogued.
    const FILLS_UNIT: &'static str =
        "a packed field of 8 bits on a byte boundary whose type is wider than a byte (its \
         access is not catalogued) is";

    /// The value of a bit field of a packed record in a new data register
    /// (ref 3.10.7; codegen-patterns 5.3).  `M` is the field's unit
    /// (`field.unit`, the byte, word or longword that holds it):
    ///
    /// - a boolean field (one bit): `btst #k,M; sne Dn` (0xE16F0C `btst
    ///   #4,(19,A2); sne D0`; 0xE03368); in bit 7 `tst.b M; smi Dn`
    ///   (0xE066B2 `tst.b (185,A0); smi D0`): no `btst #7` on memory
    ///   exists in compiled code;
    /// - a field that fills the top of a unit as wide as its value: the
    ///   unit, shifted down, the zeros shifted in clearing the rest
    ///   (0xE08434 `move.l (8,A3),D6; lsr.l #4,D6`; 0xE0A232);
    /// - any other: the field's mask in the register at the value's width
    ///   or the unit's, whichever is wider, `and` of the unit, and the
    ///   shift down (0xE158B6 `move.w #192,D0; and.b (16,A0),D0; lsr.w
    ///   #6,D0`, a 2-bit enumeration field of a byte; 0xE5BFBE `move.w
    ///   #480,D1; and.w (16,A0),D1; lsr.w #5,D1`; 0xE15868 `move.l
    ///   #0xfffff,D2; ... and.l (6,A1),D2`, the low 20 bits of a longword,
    ///   no shift; 0xE106A0 `move.l #0x7f8000,D0; and.l (-10,A2),D0; lsr.l
    ///   #8,D0; lsr.l #7,D0`).  A mask of -128..127 is a `moveq`
    ///   (0xE0E00A `moveq #124,D1; and.b (-1012,A4),D1; lsr.w #2,D1`;
    ///   0xE4CBD6 `moveq #7,D0; and.b (A0),D0`), the shift split at 8
    ///   (0xE106A0).
    ///
    /// A field narrower than its value's width leaves the rest of the
    /// register zero: the value is zero-extended (an unsigned field; a
    /// field of negative values is refused when the IR is built).
    fn bits_value(&mut self, addr: &Addr, field: BitField, ty: Scalar) -> Res<u8> {
        let ea = self.ea_of(addr)?;
        let d = self.alloc_d()?;
        if ty == Scalar::BOOLEAN && field.width == 1 {
            if field.shift == 7 {
                self.out.push(Insn::Tst {
                    size: Size::B,
                    dst: ea,
                });
                self.out.push(Insn::Scc {
                    cond: Cond::Mi,
                    dst: Ea::Reg(Reg::D(d)),
                });
            } else {
                self.out.push(Insn::BtstImm {
                    bit: field.shift as u8,
                    dst: ea,
                });
                self.out.push(Insn::Scc {
                    cond: Cond::Ne,
                    dst: Ea::Reg(Reg::D(d)),
                });
            }
            return Ok(d);
        }
        if ty == Scalar::BOOLEAN {
            return Err(unsupported(
                self.span,
                "a boolean packed field of more than one bit is",
            ));
        }
        let unit = field.unit;
        if field.width == 8 * unit.bytes() {
            return Err(unsupported(self.span, Self::FILLS_UNIT));
        }
        let top = field.shift + field.width == 8 * unit.bytes();
        if top && ty.width == unit {
            self.out.push(Insn::Move {
                size: size_of(unit),
                src: ea,
                dst: Ea::Reg(Reg::D(d)),
            });
            self.shift_by(ShiftOp::Lsr, d, field.shift, unit);
            return Ok(d);
        }
        let w = if ty.bytes() > unit.bytes() {
            ty.width
        } else {
            unit
        };
        self.const_into(field.mask(), w, d);
        self.out.push(Insn::Alu {
            op: AluOp::And,
            size: size_of(unit),
            src: ea,
            dst: d,
        });
        self.shift_by(ShiftOp::Lsr, d, field.shift, w);
        Ok(d)
    }

    /// `d := v` for a bit field `d` of a packed record (ref 3.10.7;
    /// codegen-patterns 5.3), `M` its unit.  Image forms:
    ///
    /// - A boolean constant: `bset #k,M` / `bclr #k,M` (PROC2_$RESUME
    ///   0xE41444 `bset #7,(-0x4,A6)`; 0xE00E02 `bclr #6,(-128,A2)`).
    /// - Another constant: the field cleared, `andi #~mask,M`, then its
    ///   bits set, `ori #c<<k,M` (0xE158CE..0xE158D4 `andi.b #63,(16,A0);
    ///   ori.b #-128,(16,A0)`); only the `andi` for 0 (0xE1565E `andi.b
    ///   #63,(16,A2)`), only the `ori` when every bit of the field is set
    ///   (0xE15656 `ori.b #-64,(16,A2)`).  The `ori` is as wide as the unit
    ///   (0xE0A1DE `ori.l #16,(8,A0)`, 0xE33E96 `ori.l #1,(-444,A6)`),
    ///   except that a word's `ori` whose low byte is 0 is an `ori.b` of
    ///   its high byte (0xE008CE `ori.b #1,(40,A2)` after `andi.w
    ///   #-0x7f81,(40,A2)`); a longword's with a low byte of 0 is not
    ///   catalogued.
    /// - A boolean value in Dn (computed: `s<cc>`, or loaded; a held one
    ///   copied): for bit 7, `andi.b #0x7f,M; andi.b #0x80,Dn; or.b Dn,M`
    ///   (0xE00892..0xE008A4 `btst #2,(-47,A6); sne D0; andi.b
    ///   #127,(39,A2); andi.b #-128,D0; or.b D0,(39,A2)`), a boolean
    ///   variable loaded after the `andi` (0xE5BFA2 `andi.b #127,(16,A0);
    ///   move.b (A2),D1; andi.b #-128,D1; or.b D1,(16,A0)`); for bit k <
    ///   7, made 0 or 1 first, `lsr.b #7,Dn`, then `andi.b #~(1<<k),M`,
    ///   `lsl.b #k,Dn` (`add.b Dn,Dn` for 1, nothing for 0), `or.b Dn,M`
    ///   (0xE04D0E `sne D1; lsr.b #7,D1; andi.b #-17,(14,A2); lsl.b
    ///   #4,D1; or.b D1,(14,A2)`; 0xE1D4B2 `add.b D1,D1`; 0xE04A96 bit 0;
    ///   a variable 0xE04C5C `move.b (A1),D1; lsr.b #7,D1; ...`).
    /// - Another value: `andi #~mask,M`, the value shifted up to the field
    ///   (`lsl` split at 8, `add` for 1), masked when its type admits
    ///   values outside the field's (`mask`, decided by `lower.rs`), and
    ///   `or Dn,M`, at the unit's width.  A variable of the unit's width,
    ///   or a value held in a register, is read after the `andi` (0xE0DFB0
    ///   `andi.w #-512,(-1014,A4); move.w (14,A3),D0; or.w D0,(-1014,A4)`;
    ///   0xE0B372 `andi.b #3,(1970,A0); move.b 0xe2060b,D1; lsl.b #2,D1;
    ///   or.b D1,(1970,A0)`; 0xE04F34 `andi.b #-65,(113,A2); move.b
    ///   D0,D6; lsl.b #6,D6; or.b D6,(113,A2)`; masked, 0xE38C72 `andi.b
    ///   #-16,(-9,A6); move.b (-53,A6),D0; andi.b #15,D0; or.b
    ///   D0,(-9,A6)`); a narrower variable is zero-extended to the unit
    ///   first (0xE00994 `clr.w D0; move.b (12,A1),D0; andi.w
    ///   #-0x7f81,(40,A2); lsl.w #7,D0; or.w D0,(40,A2)`; 0xE0086E
    ///   `move.b (21,A4),D0; andi.l #255,D0; andi.l #-0x7f8001,(38,A2);
    ///   lsl.l #8,D0; lsl.l #7,D0; or.l D0,(38,A2)`; 0xE0E346 `clr.l D0;
    ///   move.w (-80,A6),D0; andi.l #-0xffff01,(4,A2); lsl.l #8,D0; or.l
    ///   D0,(4,A2)`), and so is any computed value, at its own width or
    ///   wider (0xE106A0..0xE106B6: a longword value merged into a byte,
    ///   `andi.b #15,(-223,A6); lsl.b #4,D0; or.b D0,(-223,A6)`).
    fn store_bits(&mut self, addr: &Addr, field: BitField, value: &Expr, mask: bool) -> Res<()> {
        if matches!(addr, Addr::Frame(o) | Addr::Enclosing(o) if *o > 0) {
            return Err(unsupported(self.span, "assigning to a parameter is"));
        }
        if self.addr_needs_reg(addr) && self.value_needs_reg(value) {
            return Err(unsupported(
                self.span,
                "a store whose destination and value both need a register is",
            ));
        }
        let unit = field.unit;
        let usize_ = size_of(unit);
        let m = field.mask();
        let boolean = value.ty() == Scalar::BOOLEAN;
        if boolean && field.width != 1 {
            return Err(unsupported(
                self.span,
                "a boolean packed field of more than one bit is",
            ));
        }
        if !boolean && field.width == 8 * unit.bytes() {
            return Err(unsupported(self.span, Self::FILLS_UNIT));
        }
        let and_mem = |s: &mut Self, dst: &Ea, keep: i64| {
            s.out.push(Insn::AluImm {
                op: AluOp::And,
                size: usize_,
                imm: imm(keep, unit),
                dst: dst.clone(),
            });
        };
        if let Expr::Const { value: c, .. } = value {
            let dst = self.ea_of(addr)?;
            if boolean {
                self.out.push(Insn::BitImm {
                    op: if *c != 0 { BitOp::Set } else { BitOp::Clr },
                    bit: field.shift as u8,
                    dst,
                });
                return Ok(());
            }
            if field.width == 1 {
                // A one-bit field of another type: `bset`/`bclr` or
                // `andi`/`ori`, not catalogued.
                return Err(unsupported(
                    self.span,
                    "a constant stored into a one-bit packed field that is not a boolean is",
                ));
            }
            let v = (c << field.shift) & m;
            if v != m {
                and_mem(self, &dst, !m);
            }
            if v != 0 {
                let (size, imm_v, dst) = match unit {
                    Width::Byte => (Size::B, imm(v, Width::Byte), dst),
                    Width::Word if v & 0xff == 0 => (Size::B, v >> 8, dst),
                    Width::Word => (Size::W, imm(v, Width::Word), dst),
                    Width::Long if v & 0xff != 0 => (Size::L, imm(v, Width::Long), dst),
                    // Low byte 0, but bits in both halves: no narrower `ori`
                    // could set them, and the image has the `ori.l`
                    // (0xE4A834 `andi.l #255,(50,A0)`, 0xE4A83C `ori.l
                    // #0x8001e00,(50,A0)`).
                    Width::Long if v & 0xffff != 0 && v >> 16 != 0 => {
                        (Size::L, imm(v, Width::Long), dst)
                    }
                    Width::Long => {
                        return Err(unsupported(
                            self.span,
                            "a constant stored into a packed field of a longword whose low \
                             byte it leaves 0 and whose bits lie in one half of it (the width \
                             of its `ori` is not catalogued) is",
                        ))
                    }
                };
                self.out.push(Insn::AluImm {
                    op: AluOp::Or,
                    size,
                    imm: imm_v,
                    dst,
                });
            }
            return Ok(());
        }
        // A variable (in memory or held in a register), possibly widened
        // from an unsigned narrower one: (its address, its width, the
        // sign of the widening).
        let var = |e: &Expr| -> Option<(Addr, Width, bool)> {
            match e {
                Expr::Load { addr: a, ty } => Some((a.clone(), ty.width, false)),
                Expr::Convert { value: x, ty } if ty.width.bytes() > x.ty().width.bytes() => {
                    match &**x {
                        Expr::Load { addr: a, ty: xt } => {
                            Some((a.clone(), xt.width, xt.sign == Sign::Signed))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        };
        let src = if boolean { None } else { var(value) };
        let held = |s: &Self, a: &Addr| matches!(a, Addr::Frame(o) if s.pinned(*o).is_some());
        let d;
        let dst;
        if boolean {
            if field.shift == 7 {
                // Masked in place, no shift: the value as it is; a boolean
                // variable is read after the `andi` (0xE5BFA2), a held one
                // copied then (0xE1D1EE `andi.b #127,(4,A1); move.b D0,D1`).
                match value {
                    Expr::Load { addr: a, .. } => {
                        dst = self.ea_of(addr)?;
                        and_mem(self, &dst, 0x7f);
                        let v = if held(self, a) {
                            Val::Reg(self.pinned_of(a))
                        } else {
                            Val::Mem(self.ea_of(a)?)
                        };
                        d = self.in_tmp(v, Width::Byte)?;
                    }
                    _ => {
                        d = self.eval_d(value)?;
                        dst = self.ea_of(addr)?;
                        and_mem(self, &dst, 0x7f);
                    }
                }
                self.out.push(Insn::AluImm {
                    op: AluOp::And,
                    size: Size::B,
                    imm: 0x80,
                    dst: Ea::Reg(Reg::D(d)),
                });
            } else {
                d = self.eval_d(value)?;
                self.out.push(Insn::Shift {
                    op: ShiftOp::Lsr,
                    size: Size::B,
                    count: ShiftCount::Imm(7),
                    dst: d,
                });
                dst = self.ea_of(addr)?;
                and_mem(self, &dst, !m);
                self.shift_up(d, field.shift, Width::Byte);
            }
        } else {
            // A variable is read only as wide as the field needs: a byte
            // for up to 8 bits, a word up to 16, at the variable's low end
            // (big-endian: its last bytes).  0xE18192 `clr.w D1; move.b
            // 0xe2060b,D1` reads the low byte of the word 0xE2060A (compared
            // as a word at 0xE18A0C `cmp.w 0xe2060a,D1`) for a 6-bit field;
            // the sources of the byte merges above are all at odd
            // addresses (0xE1D220 `move.b (133,A3),D1`; 0xE38C72 `move.b
            // (-53,A6),D0`).  Read at the unit's width it is read after
            // the `andi`; narrower, it is zero-extended to the unit first.
            let need = match field.width {
                0..=8 => Width::Byte,
                9..=16 => Width::Word,
                _ => Width::Long,
            };
            match src {
                Some((a, w, signed)) if !(signed && w.bytes() < need.bytes()) => {
                    let k = if w.bytes() < need.bytes() { w } else { need };
                    let reg = held(self, &a);
                    let read = |s: &mut Self| -> Res<Ea> {
                        Ok(if reg {
                            Ea::Reg(s.pinned_of(&a))
                        } else {
                            let a = match &a {
                                Addr::Const(_) => a.clone(),
                                _ => a.clone().offset_by((w.bytes() - k.bytes()) as i32),
                            };
                            s.ea_of(&a)?
                        })
                    };
                    if k == unit {
                        dst = self.ea_of(addr)?;
                        and_mem(self, &dst, !m);
                        let ea = read(self)?;
                        d = self.alloc_d()?;
                        self.out.push(Insn::Move {
                            size: size_of(k),
                            src: ea,
                            dst: Ea::Reg(Reg::D(d)),
                        });
                    } else {
                        let ea = read(self)?;
                        d = self.alloc_d()?;
                        let r = Ea::Reg(Reg::D(d));
                        match (k, unit) {
                            // 0xE0086E `move.b (21,A4),D0; andi.l #255,D0`.
                            (Width::Byte, Width::Long) => {
                                self.out.push(Insn::Move {
                                    size: Size::B,
                                    src: ea,
                                    dst: r.clone(),
                                });
                                self.out.push(Insn::AluImm {
                                    op: AluOp::And,
                                    size: Size::L,
                                    imm: 0xff,
                                    dst: r,
                                });
                            }
                            // 0xE00994 `clr.w D0; move.b (12,A1),D0`;
                            // 0xE0F216 `clr.w D4; move.b D3,D4` from a
                            // register; 0xE0E346 `clr.l D0; move.w
                            // (-80,A6),D0`.
                            _ => {
                                self.out.push(Insn::Clr {
                                    size: usize_,
                                    dst: r.clone(),
                                });
                                self.out.push(Insn::Move {
                                    size: size_of(k),
                                    src: ea,
                                    dst: r,
                                });
                            }
                        }
                        dst = self.ea_of(addr)?;
                        and_mem(self, &dst, !m);
                    }
                }
                Some(_) => {
                    return Err(unsupported(
                        self.span,
                        "storing a signed variable narrower than the field into a packed field \
                         (its widening is not catalogued) is",
                    ))
                }
                None => {
                    if matches!(value, Expr::Bits { .. }) {
                        // The image copies a field into one at the same
                        // place through the unit, masked in place
                        // (0xE19DE6 `andi.b #-97,(2,A1); move.b (2,A0),D0;
                        // andi.b #96,D0; or.b D0,(2,A1)`); the general
                        // form of a copy is not catalogued.
                        return Err(unsupported(
                            self.span,
                            "copying a packed field that is not a boolean into another is",
                        ));
                    }
                    if value.ty().bytes() < unit.bytes() {
                        return Err(unsupported(
                            self.span,
                            "storing a computed value narrower than the field's unit into a \
                             packed field (its zero-extension is not catalogued) is",
                        ));
                    }
                    d = self.eval_d(value)?;
                    dst = self.ea_of(addr)?;
                    and_mem(self, &dst, !m);
                }
            }
            self.shift_up(d, field.shift, unit);
            if mask {
                self.out.push(Insn::AluImm {
                    op: AluOp::And,
                    size: usize_,
                    imm: imm(m, unit),
                    dst: Ea::Reg(Reg::D(d)),
                });
            }
        }
        self.out.push(Insn::AluToMem {
            op: AluOp::Or,
            size: if boolean { Size::B } else { usize_ },
            src: d,
            dst,
        });
        Ok(())
    }

    /// The register a held frame object is in ([`Self::pinned`]).
    fn pinned_of(&self, a: &Addr) -> Reg {
        match a {
            Addr::Frame(o) => self.pinned(*o).expect("a held frame object"),
            _ => unreachable!("only frame objects are held"),
        }
    }

    /// Dd shifted left by `n` for a bit field: `add.<w> Dd,Dd` for 1
    /// (0xE1D4B2 `add.b D1,D1`; 0xE5BD6E), else `lsl` split at 8
    /// (0xE0086E `lsl.l #8,D0; lsl.l #7,D0`).
    fn shift_up(&mut self, d: u8, n: u32, w: Width) {
        if n == 1 {
            self.out.push(Insn::Alu {
                op: AluOp::Add,
                size: size_of(w),
                src: Ea::Reg(Reg::D(d)),
                dst: d,
            });
        } else {
            self.shift_by(ShiftOp::Lsl, d, n, w);
        }
    }

    /// `k in s` for a constant `k` and a set of one word or one longword
    /// (codegen-patterns 5.1, "`in` on sets in memory, constant member";
    /// `tools/setsurvey.py`, kind `ctest`).  Returns the register the test
    /// left its value in, if any, and the condition that is true when `k`
    /// is a member.
    ///
    /// - A set in memory, at an `(An)`, `(d,An)` or absolute address and
    ///   not volatile, is read only as far as the low-order part that
    ///   holds bit `k` *at the same bit number*: the byte at the set's
    ///   last address for k < 8, `btst #k` on it (tty_$i_kill_line
    ///   0xE1B6B6 `btst #2,(0x1f,A2)`, TTY_$I_RCV 0xE1BAB6 `btst
    ///   #0,(23,A2)`; as a value `btst; sne Dn`, TTY_$K_GET 0xE1C54E), bit
    ///   7 as the byte's sign (`tst.b (9,A2); bmi`, 0xE1B63E: no `btst #7`
    ///   exists in compiled code); the word at its last two bytes for
    ///   member 8..15 of a longword, loaded (TTY_$I_RCV 0xE1B972 `move.w
    ///   (22,A2),D0; btst #12,D0`).  The bit is never renumbered: member
    ///   8..15 of a word set is tested in the loaded word (PROC2's word at
    ///   -186: members 8..13 `move.w` in 31 routines, members 2..6 `btst
    ///   (-185,An)`), member 16..30 of a longword in the loaded longword
    ///   (PROC2_$SET_SIG_MASK 0xE3F94A `move.l; btst #17`).
    /// - Otherwise the set's value in a data register, then `btst #k` on
    ///   it: a set held in a register (0xE031B2 `btst #14,D5`), an
    ///   indexed operand (`(d,An,Dn)`: the four k < 8 sites are whole,
    ///   0xE4437E, 0xE5F4C8, 0xE5F7B8, 0xE5FB70), a volatile set (not
    ///   catalogued).  The boolean value takes `sne` in that register
    ///   (REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A41A `move.w (22,A0),D0; btst
    ///   #13,D0; sne D0`).
    ///
    /// Member 7, 15 or 31 tested in a register is refused (member 15 of a
    /// word set or of a longword's low word, 31 of a longword, 7 of a set
    /// read whole: volatile, indexed or held): compiled code has no `btst
    /// #7`, `#15` or `#31` on any operand (916 `btst #k` sites), so the
    /// original tested such a bit some other way, not catalogued for sets
    /// (a lead for 7: 0xE19106..0xE1910C `move.w (6,A0),D3; tst.b D3;
    /// bpl` on the WIN_ controller's status word).  The whole-value
    /// tests of k < 8 in memory that the image also has (0xE013BA `move.w
    /// (190,A2),D0; btst #4,D0`; tty_$i_put_chars 0xE1B0E6) are read as
    /// integer bit tests (`&` of one bit, `odd`), not as `in`
    /// (codegen-patterns 5.1).
    fn set_bit_test(
        &mut self,
        elem: &Expr,
        set: &Expr,
        w: Width,
        value: bool,
    ) -> Res<(Option<u8>, Cond)> {
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
        let k = *k as u32;
        let volatile = matches!(
            set,
            Expr::Load {
                addr: Addr::Volatile { .. },
                ..
            }
        );
        let v = self.eval(set)?;
        let r = match v {
            Val::Reg(Reg::D(r)) if !value => r,
            Val::Tmp(r) => r,
            Val::Mem(ea) => {
                let narrow = !volatile && matches!(ea, Ea::Ind(_) | Ea::Disp(..) | Ea::AbsL(..));
                if narrow && k < 8 {
                    let byte = self.byte_of(ea, w.bytes() - 1)?;
                    if k == 7 {
                        self.out.push(Insn::Tst {
                            size: Size::B,
                            dst: byte,
                        });
                        return Ok((None, Cond::Mi));
                    }
                    self.out.push(Insn::BtstImm {
                        bit: k as u8,
                        dst: byte,
                    });
                    return Ok((None, Cond::Ne));
                }
                let (ea, lw) = if narrow && w == Width::Long && k < 16 {
                    (self.byte_of(ea, 2)?, Width::Word)
                } else {
                    (ea, w)
                };
                if matches!(k, 7 | 15 | 31) {
                    return Err(Self::sign_member(self.span));
                }
                let d = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(lw),
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
        if matches!(k, 7 | 15 | 31) {
            return Err(Self::sign_member(self.span));
        }
        self.out.push(Insn::BtstImm {
            bit: k as u8,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok((Some(r), Cond::Ne))
    }

    /// The refusal of [`Self::set_bit_test`] for bit 7, 15 or 31 of a
    /// set tested in a register.
    fn sign_member(span: Span) -> Diagnostic {
        unsupported(
            span,
            "`in` testing member 7, 15 or 31 of a set in a register (no `btst #7`/`#15`/`#31` \
             exists in compiled code; the original's form for a set is not catalogued) is",
        )
    }

    /// `elem in set` for a variable element and a one-longword set: the
    /// `btst Dn,Dm` that sets Z, in the register that holds the set, which
    /// is returned (a boolean value takes `sne` there, a condition
    /// branches on Z).  The register form takes the element mod 32 (no
    /// range check: TTY_$K_SET_DELAY 0xE67A1C, a word of 32 and above tests
    /// a member of 0..31; ref 3.9.3 would give false for it).
    ///
    /// - A set variable in memory, PROC1_$TST_LOCK 0xE148CE..0xE148DE:
    ///
    ///   ```text
    ///   movea.l PROC1_$CURRENT_PCB.l,A0   ; the set's address first
    ///   move.w  (8,A6),D0                 ; then the element
    ///   move.l  (0x40,A0),D1              ; then the set's value
    ///   btst    D0,D1                     ; register btst: bit mod 32
    ///   sne     D1                        ; boolean in the set's register
    ///   ```
    ///
    ///   (the order seen at this one site only, codegen-patterns 5.1); in a
    ///   condition tty_$i_set_funcs 0xE67230 `move.l (0x20,A0),D4; btst
    ///   D3,D4; beq.b`.
    /// - A set held in a register (a parameter loaded at entry): `btst` on
    ///   it (tty_$i_set_funcs 0xE67228 `btst D3,D0; beq.b`).
    /// - A constant set: the element, then the constant in a register
    ///   (TTY_$K_SET_DELAY 0xE67A18..0xE67A1E `move.w (A2),D0; moveq
    ///   #0x1f,D1; btst D0,D1; bne.b`; tty_$i_wait 0xE1C264 `move.w
    ///   (0x38,A2),D0; moveq #3,D5; btst D0,D5; bne.b`).
    fn set_member_reg(&mut self, elem: &Expr, set: &Expr) -> Res<u8> {
        let bit_of = |s: &mut Self, e: &Expr| -> Res<u8> {
            match e {
                // Not copied: `btst` only reads it.
                Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } if matches!(s.pinned(*o), Some(Reg::D(_))) => match s.pinned(*o) {
                    Some(Reg::D(n)) => Ok(n),
                    _ => unreachable!(),
                },
                e => s.eval_d(e),
            }
        };
        let r = match set {
            Expr::Const { value, .. } => {
                let bit = bit_of(self, elem)?;
                let r = self.load_const(*value, Width::Long)?;
                self.out.push(Insn::Btst {
                    bit,
                    dst: Ea::Reg(Reg::D(r)),
                });
                return Ok(r);
            }
            Expr::Load {
                addr: Addr::Frame(o),
                ty: Scalar {
                    width: Width::Long, ..
                },
            } if matches!(self.pinned(*o), Some(Reg::D(_))) => {
                let Some(Reg::D(r)) = self.pinned(*o) else {
                    unreachable!()
                };
                r
            }
            Expr::Load {
                addr,
                ty: Scalar {
                    width: Width::Long, ..
                },
            } => {
                let ea = self.ea_of(addr)?;
                let bit = bit_of(self, elem)?;
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
                return Ok(r);
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "`in` on a set that is not a variable or a constant is",
                ))
            }
        };
        let bit = bit_of(self, elem)?;
        self.out.push(Insn::Btst {
            bit,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok(r)
    }

    /// `elem in s` for a set of `bytes` bytes in memory at `set`, more
    /// than a register holds (ref 3.9.3, Figure 3-6: member k is bit k mod
    /// 8 of the byte `(bits-1 - k) div 8`), setting Z for a branch.  The
    /// element's type lies within the set's bits (lowering refuses the
    /// guarded form).  Image (codegen-patterns 5.1), the order: the
    /// constant bits-1, the element, the set's address, the byte offset,
    /// the `btst` on the byte:
    ///
    /// ```text
    /// clr.w   D1              ; a char element widened...
    /// move.w  #0xff,D2        ; bits-1 (`moveq` up to 127)
    /// move.b  (0x2d1,A0),D1   ; ...loaded after the constant
    /// lea     (A5),A0         ; the set's address, always an `lea`
    /// sub.w   D1,D2
    /// lsr.w   #3,D2           ; the byte
    /// btst    D1,(0,A0,D2.w)  ; the bit, mod 8
    /// ```
    ///
    /// (the TTY module-local 0xE1B740..0xE1B750, and again at 0xE1B77E;
    /// MST_$ALLOC_ASID 0xE42D5C..0xE42D64 `moveq #63,D1; lea (A0),A2;
    /// sub.w D2,D1; lsr.w #3,D1; btst D2,(0,A2,D1.w)` with the element held
    /// in D2; MSG_$OPENI 0xE59250..0xE59260 `moveq #63,D0; move.w
    /// abs.w,D1; lea (-20,A6),A0; sub.w D1,D0; lsr.w #3,D0` for a word
    /// element in memory).  **Uncertain:** the `lea` of a set in a record
    /// reached through a register comes after the `lsr` at MSG_$CLOSEI
    /// 0xE59432 and MSG_$FORK 0xE73F38 (`lea (472,A0),A1`).
    fn set_member_at(&mut self, elem: &Expr, set: &Addr, bytes: u32) -> Res<()> {
        let top = bytes as i64 * 8 - 1;
        let (k, c) = match elem {
            // A byte element zero-extended around the constant's load.
            Expr::Convert { value, ty } if *ty == Scalar::unsigned(Width::Word) => {
                let Expr::Load {
                    ty:
                        Scalar {
                            width: Width::Byte, ..
                        },
                    ..
                } = &**value
                else {
                    return Err(unsupported(
                        self.span,
                        "`in` on a large set with a computed byte element is",
                    ));
                };
                // A parameter held since the entry is copied from its
                // register (**Uncertain**: no site; codegen-patterns 8,
                // held values are never modified).
                let src = match self.eval(value)? {
                    Val::Mem(ea) => ea,
                    Val::Reg(r @ Reg::D(_)) => Ea::Reg(r),
                    _ => unreachable!("a byte load is memory or a data register"),
                };
                let k = self.alloc_d()?;
                self.out.push(Insn::Clr {
                    size: Size::W,
                    dst: Ea::Reg(Reg::D(k)),
                });
                let c = self.load_const(top, Width::Word)?;
                self.out.push(Insn::Move {
                    size: Size::B,
                    src,
                    dst: Ea::Reg(Reg::D(k)),
                });
                (k, c)
            }
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if matches!(self.pinned(*o), Some(Reg::D(_))) => {
                let Some(Reg::D(k)) = self.pinned(*o) else {
                    unreachable!()
                };
                (k, self.load_const(top, Width::Word)?)
            }
            Expr::Load { addr, .. } => {
                let c = self.load_const(top, Width::Word)?;
                let src = self.ea_of(addr)?;
                let k = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: Size::W,
                    src,
                    dst: Ea::Reg(Reg::D(k)),
                });
                (k, c)
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "`in` on a large set with a computed element is",
                ))
            }
        };
        let src = self.ea_of(set)?;
        let a = self.alloc_a()?;
        self.out.push(Insn::Lea { src, dst: a });
        self.out.push(Insn::Alu {
            op: AluOp::Sub,
            size: Size::W,
            src: Ea::Reg(Reg::D(k)),
            dst: c,
        });
        self.out.push(Insn::Shift {
            op: ShiftOp::Lsr,
            size: Size::W,
            count: ShiftCount::Imm(3),
            dst: c,
        });
        self.out.push(Insn::Btst {
            bit: k,
            dst: Ea::Idx(0, a, c, false),
        });
        Ok(())
    }

    /// The byte of the array at `array` (the unit's data) that the bit
    /// `btst`/`or.b`/`and.b` reaches: `lea (0,A5,Do.w),Aa`, the element's
    /// displacement in the data area kept for the operand, `(d,Aa)`
    /// (codegen-patterns 5.2, "Packed arrays of booleans": PCHIST_$COUNT
    /// 0xE1A160 `lea (0,A5,D1.w),A1` then `btst D3,(0x18,A1)`, though
    /// `(0x18,A5,D1.w)` would do).  Only the unit's own data through A5:
    /// another unit's array is `movea.l #sym,An; lea (0,An,Dx.w),...` into
    /// the base's register or another (ACL_$ALLOC_ASID 0xE73CAE A3 to A3,
    /// ACL_$USED_SUSER 0xE492EE A0 to A1: which one is not settled), and a
    /// frame or pointer base is not seen.
    fn bit_elem_byte(&mut self, array: &Addr, offset: u8) -> Res<Ea> {
        let o = match array {
            Addr::Static(o) if !self.data_absolute && (0..0x8000).contains(o) => *o,
            _ => {
                return Err(unsupported(
                    self.span,
                    "an element of a packed array of booleans that is not in the unit's data \
                     reached through A5 (other units\' arrays: the `lea` register not settled, \
                     codegen-patterns 5.2) is",
                ))
            }
        };
        let base = self.static_base()?;
        let a = self.alloc_a()?;
        self.out.push(Insn::Lea {
            src: Ea::Idx(0, base, offset, false),
            dst: a,
        });
        Ok(if o == 0 { Ea::Ind(a) } else { Ea::Disp(o, a) })
    }

    /// The low byte (`byte`) or the word of a word index of a packed array
    /// of booleans, as the source of the `move` that starts the element's
    /// computation: a register-held value itself (PCHIST_$COUNT 0xE1A156
    /// `move.b D2,D0`; ACL_$INIT 0xE310E4 `move.w D3,D0`), a variable in
    /// memory read in place, its low byte at the next address
    /// (PCHIST_$UNIX_PROFIL_CNTL 0xE5CB3A `move.b (-0x21,A6),D0` for the
    /// word at -0x22; PCHIST_$UNIX_PROFIL_FORK 0xE5CC44 `move.b
    /// (0x00e20609).l,D0` for PROC1_$CURRENT, 0xE5CC9E `move.b (0x1,A2),D0`
    /// through an `in` parameter's address held in A2).  A computed index
    /// is not seen.
    fn bit_elem_index(&mut self, index: &Expr, byte: bool) -> Res<Ea> {
        let bad = |s: &Selector| {
            unsupported(
                s.span,
                "a computed index into a packed array of booleans (not seen, codegen-patterns \
                 5.2) is",
            )
        };
        if !matches!(index, Expr::Load { .. }) {
            return Err(bad(self));
        }
        match self.eval(index)? {
            Val::Reg(r @ Reg::D(_)) => Ok(Ea::Reg(r)),
            Val::Mem(ea) if !byte => Ok(ea),
            Val::Mem(ea) => ea_plus(&ea, 1).ok_or_else(|| bad(self)),
            _ => Err(bad(self)),
        }
    }

    /// `btst` of element `index` of the packed array of booleans at
    /// `array` (ref 3.11.4.2 Table 3-5, 3.15.5 p. 3-62; [`Expr::BitElem`]):
    /// Z is set when the element is false.  The image's one form
    /// (codegen-patterns 5.2, "Packed arrays of booleans"; PCHIST_$COUNT
    /// 0xE1A156..0xE1A16C, PCHIST_$UNIX_PROFIL_CNTL 0xE5CB3A..0xE5CB52 and
    /// 0xE5CBA8..0xE5CBC0, PCHIST_$UNIX_PROFIL_FORK 0xE5CC44..0xE5CC5E and
    /// 0xE5CC9E..0xE5CCB6; with another base ACL_$SET_ACL_CHECK 0xE47384,
    /// ACL_$ALLOC_ASID 0xE73C9A, ACL_$USED_SUSER 0xE492DA), computed on the
    /// index's low byte:
    ///
    /// ```text
    /// move.b <index's low byte>,Dp
    /// clr.w Do
    /// subq.b #lo,Dp           ; the position from 0
    /// move.b Dp,Do
    /// lsr.w #3,Do             ; its byte, zero-extended
    /// lea (0,A5,Do.w),Aa
    /// moveq #7,Dm             ; Do's register again
    /// moveq #7,Dn
    /// and.b Dp,Dm
    /// sub.b Dm,Dn             ; 7 - position mod 8
    /// btst Dn,(d,Aa)
    /// ```
    ///
    /// The sequence is one scheduling unit (`sched.rs`): R11's window
    /// would place `moveq #7,Dn` before the `lea`, which no site does.
    /// Byte arithmetic covers 256 elements, and a larger array is refused;
    /// every site's array has 64 elements and a lower bound of 1, so other
    /// sizes up to 256 and lower bounds 2..=8 (`subq`) are extrapolated
    /// (unverified).  Not reproduced: ACL_$USED_SUSER has Dp and Do the other
    /// way round (D1, D0), and the module-local 0xE464B8 (0xE4655C) forms
    /// the bit number as `andi.b #7,Dp; ...; moveq #7,Dm; sub.b Dp,Dm`.
    fn bit_elem_test(&mut self, array: &Addr, bytes: u32, index: &Expr, lo: i64) -> Res<()> {
        self.bit_elem_bound(lo)?;
        if bytes > 32 {
            return Err(unsupported(
                self.span,
                "testing an element of a packed array of more than 256 booleans (the byte \
                 arithmetic of the 64-element sites would not reach it, codegen-patterns 5.2) is",
            ));
        }
        let src = self.bit_elem_index(index, true)?;
        let start = self.out.len();
        let dp = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: Size::B,
            src,
            dst: Ea::Reg(Reg::D(dp)),
        });
        let dof = self.alloc_d()?;
        self.out.push(Insn::Clr {
            size: Size::W,
            dst: Ea::Reg(Reg::D(dof)),
        });
        self.out.push(Insn::Subq {
            size: Size::B,
            value: lo as u8,
            dst: Ea::Reg(Reg::D(dp)),
        });
        self.out.push(Insn::Move {
            size: Size::B,
            src: Ea::Reg(Reg::D(dp)),
            dst: Ea::Reg(Reg::D(dof)),
        });
        self.out.push(Insn::Shift {
            op: ShiftOp::Lsr,
            size: Size::W,
            count: ShiftCount::Imm(3),
            dst: dof,
        });
        let ea = self.bit_elem_byte(array, dof)?;
        let dm = self.alloc_d()?;
        let dn = self.alloc_d()?;
        self.out.push(Insn::Moveq { value: 7, dst: dm });
        self.out.push(Insn::Moveq { value: 7, dst: dn });
        self.out.push(Insn::Alu {
            op: AluOp::And,
            size: Size::B,
            src: Ea::Reg(Reg::D(dp)),
            dst: dm,
        });
        self.out.push(Insn::Alu {
            op: AluOp::Sub,
            size: Size::B,
            src: Ea::Reg(Reg::D(dm)),
            dst: dn,
        });
        self.out.push(Insn::Btst { bit: dn, dst: ea });
        self.units.push((start, self.out.len()));
        Ok(())
    }

    /// The lower bound of a packed array of booleans whose element is
    /// formed at run time: 1..=8, subtracted with `subq` (every site's is
    /// 1, `subq.b #0x1` / `subq.w #0x1`).
    fn bit_elem_bound(&self, lo: i64) -> Res<()> {
        if (1..=8).contains(&lo) {
            Ok(())
        } else {
            Err(unsupported(
                self.span,
                "a packed array of booleans whose lower bound is not 1..8 (only `subq` of 1 is \
                 seen, codegen-patterns 5.2) is",
            ))
        }
    }

    /// `a[index] := true` / `false` for an element of a packed array of
    /// booleans ([`StmtKind::StoreBitElem`]; codegen-patterns 5.2,
    /// "Packed arrays of booleans"): the mask `0x80 >> (position mod 8)`
    /// (`#-0x81` for false: the low byte has all but that bit) `or`ed
    /// (`and`ed) into the element's byte, computed on the index's word
    /// (PCHIST_$UNIX_PROFIL_FORK 0xE5CCDC..0xE5CCF4, ACL_$INIT
    /// 0xE310E4..0xE310FC, ACL_$SET_LOCAL_LOCKSMITH 0xE49204..0xE49226;
    /// PCHIST_$UNIX_PROFIL_CNTL 0xE5CB5C..0xE5CB76 and the clear
    /// 0xE5CBF2..0xE5CC0C):
    ///
    /// ```text
    /// move.w <index>,Dp
    /// moveq #7,Dc
    /// subq.w #lo,Dp
    /// move.l #0x80,Dm         ; #-0x81 for false
    /// and.w Dp,Dc
    /// lsr.l Dc,Dm
    /// move.w Dp,Do
    /// lsr.w #3,Do
    /// lea (0,A5,Do.w),Aa
    /// or.b Dm,(d,Aa)          ; and.b for false
    /// ```
    ///
    /// The selector writes the instructions in that order and the
    /// scheduler (R11) keeps it; the allocator gives Do the count's
    /// register, free after the `lsr.l` (FORK, ACL_$INIT, LOCKSMITH: D1).
    /// **Not reproduced:** PCHIST_$UNIX_PROFIL_CNTL, the module-local
    /// 0xE463E4 (0xE4646C) and ACL_$SET_ACL_CHECK 0xE473AC take a new
    /// register for Do, `move.w Dp,Do` then placed before the `lsr.l`
    /// (with a new register R11 gives that order); why is not known.
    fn store_bit_elem(&mut self, array: &Addr, index: &Expr, lo: i64, value: bool) -> Res<()> {
        self.bit_elem_bound(lo)?;
        let src = self.bit_elem_index(index, false)?;
        let dp = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: Size::W,
            src,
            dst: Ea::Reg(Reg::D(dp)),
        });
        let dc = self.alloc_d()?;
        self.out.push(Insn::Moveq { value: 7, dst: dc });
        self.out.push(Insn::Subq {
            size: Size::W,
            value: lo as u8,
            dst: Ea::Reg(Reg::D(dp)),
        });
        let dm = self.alloc_d()?;
        self.const_into(if value { 0x80 } else { -0x81 }, Width::Long, dm);
        self.out.push(Insn::Alu {
            op: AluOp::And,
            size: Size::W,
            src: Ea::Reg(Reg::D(dp)),
            dst: dc,
        });
        self.out.push(Insn::Shift {
            op: ShiftOp::Lsr,
            size: Size::L,
            count: ShiftCount::Reg(dc),
            dst: dm,
        });
        let dof = self.alloc_d()?;
        self.out.push(Insn::Move {
            size: Size::W,
            src: Ea::Reg(Reg::D(dp)),
            dst: Ea::Reg(Reg::D(dof)),
        });
        self.out.push(Insn::Shift {
            op: ShiftOp::Lsr,
            size: Size::W,
            count: ShiftCount::Imm(3),
            dst: dof,
        });
        let ea = self.bit_elem_byte(array, dof)?;
        self.out.push(Insn::AluToMem {
            op: if value { AluOp::Or } else { AluOp::And },
            size: Size::B,
            src: dm,
            dst: ea,
        });
        Ok(())
    }

    /// The register-sized set `[elem]` in a new data register: the
    /// register cleared, then `bset` of the element (codegen-patterns 5.1;
    /// survey, 30 `clr; bset Dk,Dn` in compiled code).  The element is
    /// evaluated first when it is computed (DBUF_$GET_BLOCK 0xE3A726
    /// `moveq #15,D0; and.b (12,A4),D0; clr.w D1; bset D0,D1`;
    /// PROC2_$DELIVER_FIM 0xE3EE22 `subq.w #1,D0; clr.l D1; bset D0,D1`),
    /// loaded after the `clr` when it is a variable (TTY_$K_SET_INPUT_FLAG
    /// 0xE6768E `clr.l D1; move.w (A2),D2; bset D2,D1`; 0xE3EE64), used
    /// in place when it is held in a register (EC2_$RELEASE_EC1 0xE42BA4
    /// `clr.l D1; bset D2,D1`).  `bset` on a register takes the element
    /// mod 32 (no range check; on a word set a member above 15 is lost
    /// when the word is used, one of 32 and above wraps).
    fn set_of(&mut self, elem: &Expr, w: Width) -> Res<u8> {
        let v = self.eval(elem)?;
        let d = self.alloc_d()?;
        self.out.push(Insn::Clr {
            size: size_of(w),
            dst: Ea::Reg(Reg::D(d)),
        });
        let k = match v {
            Val::Reg(Reg::D(r)) | Val::Tmp(r) => r,
            Val::Imm(_) => unreachable!("a constant member is part of a set constant"),
            v => self.in_tmp(v, elem.ty().width)?,
        };
        self.out.push(Insn::Bit {
            op: BitOp::Set,
            bit: k,
            dst: Ea::Reg(Reg::D(d)),
        });
        Ok(d)
    }

    /// A register-sized set operation as a value in a new data register
    /// (codegen-patterns 5.1).
    ///
    /// - `x * c`, a constant: as the integer `x & c` ([`Self::binary`],
    ///   codegen-patterns 8): the constant loaded first and `and` of a
    ///   variable (TTY 0xE1BFFE `moveq #7,D1; and.w (8,A2),D1`; TTY_$K_PUT
    ///   0xE67D04), a held value copied and masked with `andi` when the
    ///   constant needs more than `moveq` (TTY_$I_RCV 0xE1BACE `move.w
    ///   D4,D1; andi.w #0x80,D1`).
    /// - `x * y`: the right operand loaded, the left one `and`ed in (TTY
    ///   module-local 0xE1BF92 `move.l (0x24,A5),D1; and.l (0x14,A2),D1`;
    ///   **Uncertain:** which source operand was the right one cannot be
    ///   seen).
    /// - `x - y`: the right operand loaded, complemented, the left one
    ///   `and`ed in (PROC2_$SIGPAUSE 0xE3FAC6 `move.l (-108,A4),D1; not.l
    ///   D1; and.l (-100,A4),D1`; PROC2_$SET_SIG_MASK 0xE3F9C6;
    ///   PROC2_$SIGSETMASK 0xE3F70E; 0xE3EC54 with the left operand in D4).
    ///
    /// `x + y` and `x - c` as values (not stored back into `x`) are not
    /// catalogued.
    fn set_op_value(&mut self, op: SetOp, lhs: &Expr, rhs: &Expr, ty: Scalar) -> Res<u8> {
        let w = ty.width;
        let konst = |e: &Expr| matches!(e, Expr::Const { .. });
        match op {
            SetOp::Inter if konst(lhs) || konst(rhs) => self.binary(BinOp::And, lhs, rhs, ty),
            SetOp::Inter | SetOp::Diff if !konst(rhs) => {
                let d = self.eval_d(rhs)?;
                if op == SetOp::Diff {
                    self.out.push(Insn::Unary {
                        op: UnaryOp::Not,
                        size: size_of(w),
                        dst: Ea::Reg(Reg::D(d)),
                    });
                }
                let src = match self.eval(lhs)? {
                    Val::Mem(ea) => ea,
                    Val::Reg(r @ Reg::D(_)) => Ea::Reg(r),
                    _ => {
                        return Err(unsupported(
                            self.span,
                            "an intersection or difference whose left operand is computed is",
                        ))
                    }
                };
                self.out.push(Insn::Alu {
                    op: AluOp::And,
                    size: size_of(w),
                    src,
                    dst: d,
                });
                Ok(d)
            }
            SetOp::Diff => Err(unsupported(
                self.span,
                "a set minus a constant as a value (not stored back into the set) is",
            )),
            SetOp::Union => Err(unsupported(
                self.span,
                "the union of sets as a value (not `s := s + x`) is",
            )),
            SetOp::Inter => unreachable!("a constant operand is the first arm's"),
        }
    }

    /// `s := s + x` and `s := s - x` for a register-sized set `s` in memory,
    /// done in place (codegen-patterns 5.1).  Returns whether the statement
    /// was this form.
    ///
    /// - One constant member: `bset`/`bclr #k mod 8` on the byte that
    ///   holds it, `size-1 - k div 8` (ref 3.9.3, Figure 3-6):
    ///   TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E `bclr #0,(9,A2)`, TTY_$K_SET_FLAG
    ///   0xE6745C `bset #4,(9,A2)` and 0xE6747C `bclr #4,(9,A2)` on the word
    ///   at 8; TTY_$I_RCV 0xE1B952 `bset #1,(0xb,A2)` on the word at 0xA;
    ///   TTY_$K_INQ_FLAGS 0xE674C2 `bset #0,(1,A1)`.  **Uncertain:** no
    ///   site with a member of 8 and above is known to be a set (PROC2_$FORK
    ///   0xE732B4 `bclr #0,(-186,A2)` on the high byte of a flag word whose
    ///   type is not known).
    /// - Several constant members: `ori`/`andi` of the mask (its
    ///   complement for `-`) on the set (TTY_$I_RCV 0xE1B98E `andi.w
    ///   #-0xe1,(8,A2)` for `- [5, 6, 7]`; `ori.w #0x180,(42,A2)` at
    ///   PROC2_$INIT 0xE3065A on a flag word, type not known).
    /// - `[k]`, one run-time member ([`Self::set_of`]), then `or` (for `-`
    ///   `not` and `and`) into the set: TTY_$K_SET_INPUT_FLAG 0xE6768E..
    ///   0xE676A2 `clr.l D1; move.w (A2),D2; bset D2,D1; or.l D1,(0x14,A0)`
    ///   and `...; not.l D1; and.l D1,(0x14,A0)`; on words AST_$DISMOUNT
    ///   0xE069F4 `clr.w D3; bset D2,D3; or.w D3,(1056,A5)`,
    ///   DBUF_$INVALIDATE 0xE3AA8E `clr.w D0; bset D3,D0; not.w D0; and.w
    ///   D0,(2332,A5)`.
    /// - Any other set value: computed into a register, then `or` (`not`
    ///   and `and`) into the set (TTY module-local 0xE1C034 `move.l
    ///   (0x18,A2),D1; or.l D1,(0x14,A2)`, 0xE1BF9E `move.l (0x24,A5),D1;
    ///   not.l D1; and.l D1,(0x14,A2)`; TTY_$I_RCV 0xE1BACE `move.w D4,D1;
    ///   andi.w #0x80,D1; or.w D1,(8,A2)` for `s + (t * [7])`).
    ///
    /// `s := s * x` is refused: the image has `andi` to memory, but also
    /// the mask in a register `and`ed into the set (0xE1C006 `moveq #-8,D1;
    /// and.w D1,(8,A2)`), and which source gave which is not settled.
    fn set_update(&mut self, addr: &Addr, value: &Expr) -> Res<bool> {
        let Expr::SetOp { op, lhs, rhs, ty } = value else {
            return Ok(false);
        };
        let Expr::Load { addr: a, .. } = &**lhs else {
            return Ok(false);
        };
        if a != addr {
            return Ok(false);
        }
        if *op == SetOp::Inter {
            return Err(unsupported(
                self.span,
                "`s := s * x` in place (`andi` to memory, or the mask in a register and `and` to \
                 memory as at 0xE1C006: not settled) is",
            ));
        }
        if matches!(addr, Addr::Index { .. }) {
            return Err(unsupported(
                self.span,
                "updating a set that is an array element in place is",
            ));
        }
        if let Addr::Frame(o) = addr {
            if self.pinned(*o).is_some() {
                return Err(unsupported(
                    self.span,
                    "updating a set held in a register in place is",
                ));
            }
        }
        let w = ty.width;
        let size = size_of(w);
        let all: u64 = if w == Width::Long {
            0xffff_ffff
        } else {
            0xffff
        };
        if let Expr::Const { value: c, .. } = &**rhs {
            let mask = (*c as u64) & all;
            if mask == 0 {
                return Err(unsupported(
                    self.span,
                    "adding or removing the empty set `[]` is",
                ));
            }
            let dst = self.ea_of(addr)?;
            if mask.count_ones() == 1 {
                let k = mask.trailing_zeros();
                let dst = self.byte_of(dst, w.bytes() - 1 - k / 8)?;
                self.out.push(Insn::BitImm {
                    op: if *op == SetOp::Union {
                        BitOp::Set
                    } else {
                        BitOp::Clr
                    },
                    bit: (k % 8) as u8,
                    dst,
                });
            } else {
                let (alu, m) = if *op == SetOp::Union {
                    (AluOp::Or, mask)
                } else {
                    (AluOp::And, !mask & all)
                };
                self.out.push(Insn::AluImm {
                    op: alu,
                    size,
                    imm: imm(m as i64, w),
                    dst,
                });
            }
            return Ok(true);
        }
        let d = match &**rhs {
            Expr::SetOf { elem, .. } => self.set_of(elem, w)?,
            e => match (op, self.eval(e)?) {
                (SetOp::Union, Val::Reg(Reg::D(r))) => r,
                (_, v) => self.in_tmp(v, w)?,
            },
        };
        if *op == SetOp::Diff {
            self.out.push(Insn::Unary {
                op: UnaryOp::Not,
                size,
                dst: Ea::Reg(Reg::D(d)),
            });
        }
        let dst = self.ea_of(addr)?;
        self.out.push(Insn::AluToMem {
            op: if *op == SetOp::Union {
                AluOp::Or
            } else {
                AluOp::And
            },
            size,
            src: d,
            dst,
        });
        Ok(true)
    }

    /// The byte `off` bytes into the object at `ea`.
    fn byte_of(&self, ea: Ea, off: u32) -> Res<Ea> {
        let off = off as i32;
        Ok(match ea {
            Ea::Ind(a) if off == 0 => Ea::Ind(a),
            Ea::Ind(a) => Ea::Disp(off, a),
            Ea::Disp(d, a) if (-0x8000..0x8000).contains(&(d + off)) => Ea::Disp(d + off, a),
            Ea::AbsL(s, o) => Ea::AbsL(s, o + off),
            _ => {
                return Err(unsupported(
                    self.span,
                    "a bit of a set at this kind of address is",
                ))
            }
        })
    }

    /// The operand of an array element (codegen-patterns 5.2): the element
    /// is at `base + index × scale + offset`.
    ///
    /// - The base: the unit's data area (A5), the frame (A6), or the object
    ///   a pointer held in an address register since the entry points to:
    ///   no instruction.  Another unit's array: `movea.l #sym,An`, the
    ///   immediate the variable's own symbol and every offset (field, lower
    ///   bound) in the displacement (`tools/boolsurvey.py ebase`: 958 of the
    ///   1043 `movea.l #` bases of indexed operands are a map symbol or a
    ///   data block's start, the other 85 a section's start + 60000, ACL_
    ///   0xE470AA; module-local 0xE4649C `movea.l #PROC1_$TYPE,A0; ...;
    ///   cmpi.w #2,(-2,A0,D1.w)`, a 1-based array), loaded after the index's
    ///   value and before its scaling ([`Self::other_unit_index`]).  An
    ///   element of 1 byte through a pointer loaded from memory: the index,
    ///   then the pointer ([`Self::pointer_index`]).  Other bases are refused
    ///   (codegen-patterns 5.2, "Element operand").
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
        if let Some((p, o)) = self.stepped_array(base, index, scale) {
            // Through the `for` loop's stepped pointer ([`Self::for_`]):
            // `(d,Ap)`, the lower bound and field offsets in `d` (0xE177E8
            // `move.w (-2,A0),(-2,A1)`; PKT_$RECENTLY_MISSING 0xE128DC
            // `cmp.l (-8,A0),D1`), and the array's own offset from the
            // unit's data area, the frame or the symbol (DBUF_$INVALIDATE
            // 0xE3AA16 `(-8,A3)`, the array at 0x10; EC2_$REGISTER_EC1
            // 0xE42972 `cmpa.l (2700,A0),A1`).  An access at displacement 0
            // is made with `(An)+` where it is the pointer's last use
            // (0xE00ED8 `clr.l (A0)+`; AST_ 0xE0287A `bclr #7,(A2)+; addq.l
            // #3,A2`), which `dpc` does not do.
            let offset = o + offset;
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
            Addr::Symbol { name, offset: o } => {
                self.split_disp(o + offset)?;
                let (x, a) = self.other_unit_index(name, index, scale)?;
                return self.element_ea(a, x, index, o + offset);
            }
            Addr::Indirect { base: p, offset: o } if scale == 1 => {
                self.split_disp(o + offset)?;
                let (x, a) = self.pointer_index(p, index)?;
                return self.element_ea(a, x, index, o + offset);
            }
            Addr::Indirect { .. } => {
                return Err(unsupported(
                    self.span,
                    "an element of more than 1 byte of an array through a pointer loaded from \
                     memory (the pointer's load between the index's value and its scaling or \
                     after the scaling, no rule found: tools/boolsurvey.py ebase, \
                     codegen-patterns 5.2; bead domain-pascal-nqg1) is",
                ))
            }
            Addr::OtherStatic { .. } => {
                return Err(unsupported(
                    self.span,
                    "an element of an array in another data section's block (each element's \
                     block load, NAME_$FORK 0xE73EAC, and the block's register across branch \
                     targets: not settled, codegen-patterns 11; bead domain-pascal-hxwq) is",
                ))
            }
            _ => {
                return Err(unsupported(
                    self.span,
                    "an element of an array at this kind of address (codegen-patterns 5.2) is",
                ))
            }
        };
        let x = self.index_reg(index, scale)?;
        self.element_ea(areg, x, index, disp0 + offset)
    }

    /// The operand of an element at `disp` off base register `areg` with
    /// the scaled index in `x` ([`Self::index_ea`]): `(d,An,Dx)` within
    /// -128..127, otherwise `lea (0,An,Dx),Am` and `(d,Am)`.
    fn element_ea(&mut self, areg: u8, x: u8, index: &Expr, disp: i32) -> Res<Ea> {
        let long = index.ty().width == Width::Long;
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
        self.stepped_array(base, index, scale).map(|(r, _)| r)
    }

    /// The pointer a `for` loop steps through the array at `base`
    /// ([`Self::stepped_pointer`]) and the array's own offset from it
    /// ([`SteppedArray::base_offset`]).
    fn stepped_array(&self, base: &Addr, index: &Expr, scale: u32) -> Option<(u8, i32)> {
        self.stepped
            .iter()
            .rev()
            .find(|(o, a, _)| is_index(index, *o) && a.scale == scale && a.base == *base)
            .map(|(_, a, r)| (*r, a.base_offset()))
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
    ///   [`Self::mul_const`], except an element size of 1..=127 done with a
    ///   multiply ([`by_mul_insn`]) and a word index read from a variable:
    ///   the size first, `moveq #size,Dn; mulu.w x,Dn`
    ///   ([`Self::mul_const_first`]; TPAD_$SET_UNIT_MODE 0xE69824..0xE69828
    ///   `moveq #44,D2; mulu.w (A4),D2; lea (0,A5,D2.w),A1`).
    fn index_reg(&mut self, index: &Expr, scale: u32) -> Res<u8> {
        self.index_reg_with(index, scale, &mut |_| Ok(()))
    }

    /// [`Self::index_reg`], calling `between` once the index's value is in
    /// its register and before the scaling (right after the value when
    /// there is none to do, or when the scaled index is a computed value
    /// held from earlier, 12.4 R7).
    fn index_reg_with(
        &mut self,
        index: &Expr,
        scale: u32,
        between: &mut dyn FnMut(&mut Self) -> Res<()>,
    ) -> Res<u8> {
        let w = index.ty().width;
        let mut called = false;
        let x = {
            let mut once = |s: &mut Self| -> Res<()> {
                called = true;
                between(s)
            };
            if scale == 1 {
                let v = self.eval(index)?;
                let d = match v {
                    Val::Tmp(d) | Val::Reg(Reg::D(d)) => d,
                    v => self.in_tmp(v, w)?,
                };
                once(self)?;
                d
            } else if let Some(leaf) = self.cse_leaf(index) {
                // A scaled index is a reusable computation (codegen-patterns
                // 12.4 R7: FIM_$ACKNOWLEDGE's `PROC1_$AS_ID * 12`,
                // 0xE0A972..0xE0A97E, indexes FIM_$QUIT_EC at 0xE0A994 and
                // FIM_$DELIV_EC at 0xE0A9B0).
                let key = CseKey::Scaled(index.clone(), scale as i64);
                match self.cse_value(key, leaf, |s| s.scaled_index(index, scale, &mut once))? {
                    Val::Tmp(d) | Val::Reg(Reg::D(d)) => d,
                    _ => unreachable!("a scaled index is in a data register"),
                }
            } else {
                self.scaled_index(index, scale, &mut once)?
            }
        };
        if !called {
            between(self)?;
        }
        Ok(x)
    }

    /// The scaling of [`Self::index_reg`], `between` called before it.
    fn scaled_index(
        &mut self,
        index: &Expr,
        scale: u32,
        between: &mut dyn FnMut(&mut Self) -> Res<()>,
    ) -> Res<u8> {
        let w = index.ty().width;
        if w == Width::Word && scale < 128 && by_mul_insn(scale as i64) && const_first(index) {
            // `.w` indices: `mulu.w` (21 of 21 such scalings by a constant
            // in a register; `muls.w` feeds `.l` indices only,
            // codegen-patterns 5.2).
            return self.mul_const_first(index, scale as i64, false, between);
        }
        if w == Width::Word && scale < 128 && by_mul_insn(scale as i64) && !in_place(index) {
            return Err(computed_factor(self.span, scale as i64));
        }
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
        between(self)?;
        self.mul_const(d, scale as i64, w)?;
        Ok(d)
    }

    /// The scaled index and the base register of an element of another
    /// unit's array `sym` ([`Self::index_ea`]; codegen-patterns 5.2,
    /// "Another unit's array").
    ///
    /// The base is `movea.l #sym,An`, the immediate the symbol itself, the
    /// offsets in the displacement.  It is generated after the index's
    /// value (a load, a copy, a zero-extension) and before the scaling
    /// (`tools/boolsurvey.py ebase`, the place R11 gives from that order,
    /// 12.4: 518 of the 641 such bases it can judge; module-local 0xE4649C
    /// `move.w (8,A6),D0; movea.l #PROC1_$TYPE,A0; move.w D0,D1; add.w
    /// D1,D1`; TTY_$I_WAIT 0xE1C22E `move.w PROC1_$AS_ID,D0; movea.l
    /// #FIM_$QUIT_EC,A0; lsl.w #2,D0; ...` for ×12).  For a scaling with
    /// `neg` (2^s (2^k - 1), [`Self::mul_const`]) it is generated after the
    /// scaling, which R11 then moves above its last `add` (the ×56 of
    /// 0xE1AA5A, the ×28 of 0xE1975C, 37 sites).  Refused in a loop: the
    /// image loads such a base in the loop (202 indexed operands, KBD_$FETCH_KEY
    /// 0xE1CBAA) and also holds it in a register from before the loop
    /// (TTY_$K_DRAIN_OUTPUT 0xE67AF0 `move.l #FIM_$QUIT_VALUE,D7`, then
    /// `movea.l D7,A1` in the loop); which one is not settled.  Restricted
    /// to element operands and `pea`s outside loops, 173 of 210 fit and
    /// one misfit is not explained by a refused shape (PKT_$SAR_INTERNET
    /// 0xE71F36; review of bead domain-pascal-rlnm, codegen-patterns 5.2).
    fn other_unit_index(&mut self, sym: &str, index: &Expr, scale: u32) -> Res<(u8, u8)> {
        if self.in_loop() {
            return Err(self.other_unit_in_loop());
        }
        let here = (self.stmt_start, self.span);
        if self.other_elem == Some(here) {
            return Err(unsupported(
                self.span,
                "a statement with two elements of other units' arrays (FIM_$ACKNOWLEDGE \
                 0xE0A972..0xE0A994 and 10 like it: the value's index in full, the \
                 destination's index load, the value's base, then the destination's scaling \
                 and base; not modelled, codegen-patterns 5.2; bead domain-pascal-0r1v) is",
            ));
        }
        self.other_elem = Some(here);
        if self.other_syms.iter().any(|s| s == sym) {
            // Within a run without calls the image keeps the base for the
            // next statement: of 78 `movea.l #` bases of indexed operands
            // used again in their block, none is loaded a second time while
            // a register holds it (MMAP_$FREE_PAGES 0xE0CF5C..0xE0CF74,
            // `(0,A1,D2.l)` in two statements; module-local 0xE5B744
            // ..0xE5B756 `clr.l (0,A1,D2.w); ...; clr.l (0,A1,D3.w)`).
            // `dpc` loads it again; across a branch target not seen.
            return Err(unsupported(
                self.span,
                "an element of another unit's array that an earlier statement since the last \
                 call reached too (the original keeps the base's register for it, \
                 MMAP_$FREE_PAGES 0xE0CF74, 0xE5B756; codegen-patterns 5.2; bead \
                 domain-pascal-lnzy) is",
            ));
        }
        let mut sym_read = false;
        walk_expr(index, &mut |n| {
            if let Node::Addr(Addr::Symbol { name, .. }) = n {
                sym_read |= name == sym;
            }
        });
        if sym_read {
            // SIO_$INIT 0xE32C9C..0xE32CB4 `movea.l #0xE2C9F0,A1; move.w
            // (5000,A1),D0; ...; lea (0,A1,D0.w),A1`: the base first, the
            // index read through it (8 operands); `dpc` reads it absolutely.
            return Err(unsupported(
                self.span,
                "an element of another unit's array whose index is read from the same variable \
                 (through the base's register, SIO_$INIT 0xE32CA2; codegen-patterns 5.2; bead \
                 domain-pascal-lnzy) is",
            ));
        }
        self.other_syms.push(sym.to_string());
        self.split_elems += 1;
        let mut base = None;
        let mut load = |s: &mut Self| -> Res<()> {
            let a = s.alloc_a()?;
            s.out.push(Insn::Movea {
                size: Size::L,
                src: Ea::ImmSym(sym.to_string()),
                dst: a,
            });
            base = Some(a);
            Ok(())
        };
        let x = if neg_form(scale) {
            let x = self.index_reg(index, scale)?;
            load(self)?;
            x
        } else {
            self.index_reg_with(index, scale, &mut load)?
        };
        Ok((x, base.expect("the base is loaded")))
    }

    /// The refusal of an element of another unit's array in a loop
    /// ([`Self::other_unit_index`]).
    fn other_unit_in_loop(&self) -> Diagnostic {
        unsupported(
            self.span,
            "an element of another unit's array in a loop (its address loaded in the loop or \
             held from before it, TTY_$K_DRAIN_OUTPUT 0xE67AF0: not settled, codegen-patterns \
             5.2; bead domain-pascal-dw25) is",
        )
    }

    /// The index and the base register of a 1-byte element of an array
    /// through the pointer `p`, loaded from memory ([`Self::index_ea`];
    /// codegen-patterns 5.2, "Arrays through a pointer"): the index into
    /// its register, then the pointer (KBD_$FETCH_KEY 0xE1CB8A `clr.w D0;
    /// movea.l (0x34,A4),A1; move.b (A2),D0; move.b (0,A1,D0.w),(A2)`, the
    /// `movea.l` in the zero-extension's stall by R11; 0xE4D13A `move.w
    /// (A2),D0; movea.l (8,A6),A0; cmpi.b #47,(-1,A0,D0.w)`).  Of the 1-byte
    /// elements through such a pointer with the index formed in the block
    /// (`tools/boolsurvey.py ebase`, base kinds `slot`, `local`,
    /// `field-ptr`, `unit-ptr` with no scaling; review of bead
    /// domain-pascal-rlnm), 10 fit (also 0xE4D180, 0xE4D1C6, 0xE4D292,
    /// 0xE4D2EC, VFMT_$MAIN 0xE6AB5E, 0xE6AB84, 0xE6AC98, 0xE6AF62), all in
    /// a loop's range; the other two are a call's first argument with a pad
    /// (MSG_$RCV_CONTIGI 0xE59900, [`Self::call`]) and a store whose value
    /// needs a register (0xE6AA1C, [`Self::store`]), both refused.  An index
    /// read through the same pointer, loaded once for both (0xE1B2E0
    /// `movea.l (8,A6),A0; move.w (980,A0),D1; lea (0,A0,D1.w),A1`;
    /// 0xE4DE58), is refused.
    fn pointer_index(&mut self, p: &Expr, index: &Expr) -> Res<(u8, u8)> {
        if shares_pointer(index, p) {
            return Err(unsupported(
                self.span,
                "an element of an array through a pointer that its index is read through too \
                 (the pointer loaded once for both, 0xE1B2E0; codegen-patterns 5.2) is",
            ));
        }
        self.split_elems += 1;
        let x = self.index_reg(index, 1)?;
        let a = self.eval_a(p)?;
        Ok((x, a))
    }

    /// Refuse an element of another unit's array or through a pointer
    /// loaded from memory ([`Self::index_ea`]) at a displacement beyond a
    /// byte: `lea (0,An,Dx),Am` into the base's own register or another,
    /// for an element used once with the base dead after it 84 against 105
    /// (`movea.l #` bases; own: SIO_$INIT 0xE32CB0 `lea (0,A1,D0.w),A1`,
    /// MMAP_ 0xE02912; another: 0xE0CC80 `lea (0,A0,D0.l),A2`, 0xE3E7A4),
    /// not settled (codegen-patterns 5.2, 12.4 R2).
    fn split_disp(&self, disp: i32) -> Res<()> {
        if (-128..128).contains(&disp) {
            return Ok(());
        }
        Err(unsupported(
            self.span,
            "an element of another unit's array or through a pointer loaded from memory at a \
             displacement beyond a byte (the `lea` into the base's register or another, 84 \
             against 105 sites: not settled, codegen-patterns 5.2, 12.4 R2; bead \
             domain-pascal-1ibr, domain-pascal-lnzy) is",
        ))
    }

    /// Whether `a` is an element of another unit's array or of an array
    /// through a pointer not held in an address register since the entry
    /// (the bases [`Self::index_ea`] loads itself; [`Self::store`],
    /// [`Self::lea_into`]).
    fn split_base(&self, a: &Addr) -> bool {
        match a {
            Addr::Volatile { addr, .. } => self.split_base(addr),
            // Through a `for` loop's stepped pointer: no base to load.
            Addr::Index {
                base, index, scale, ..
            } if self.stepped_pointer(base, index, *scale).is_some() => false,
            Addr::Index { base, .. } => match &**base {
                Addr::Symbol { .. } => true,
                Addr::Indirect { base: p, .. } => !matches!(
                    &**p,
                    Expr::Load { addr: Addr::Frame(f), .. }
                        if matches!(self.pinned(*f), Some(Reg::A(_)))
                ),
                _ => false,
            },
            _ => false,
        }
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
                    Reg::D(d) => self.copy_to_a(d),
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
            // A simple statement right after a call in a body of several
            // statements takes the lowest free register ([`Self::seq`]);
            // after a call that is the rule above too, and its slot loads
            // are checked like the others ([`check_late_slot_loads`]).
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if (self.multi || self.multi_body) && *o > 0 && !self.is_uplevel(*o) => {
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
            // Another data section's block: off a register holding its
            // address ([`Self::other_base`]; KBD_$INIT 0xE33374 `clr.l
            // (0x58,A0)`; EC2_$INIT_S 0xE309A8 `clr.l (A0)`).
            Addr::OtherStatic { section, offset } => {
                if !(0..0x8000).contains(offset) {
                    return Err(unsupported(
                        self.span,
                        "a static variable beyond 32K into the data area is",
                    ));
                }
                let base = self.other_base(section)?;
                Ok(if *offset == 0 {
                    Ea::Ind(base)
                } else {
                    Ea::Disp(*offset, base)
                })
            }
            Addr::Index {
                base,
                index,
                scale,
                offset,
            } => self.index_ea(base, index, *scale, *offset),
            // A reference to a volatile object: its address as usual,
            // recorded so that the instruction that references it is not
            // moved past another volatile reference (`sched.rs`), its
            // value not taken from a store's flags ([`Self::tst`]), and,
            // for a device, not referenced by a read-modify-write
            // instruction ([`Self::check_device_references`]).
            a @ Addr::Volatile { addr, access } => {
                let ea = match device_param_base(a) {
                    Some(o) => {
                        if self.pinned(o).is_some() {
                            return Err(unsupported(
                                self.span,
                                "a parameter that is both the pointer to a device object and \
                                 otherwise used (reloaded for each device reference at \
                                 0xE0A328; not seen together) is",
                            ));
                        }
                        let Addr::Indirect { offset, .. } = &**addr else {
                            unreachable!("device_param_base is a dereference")
                        };
                        if !(-0x8000..0x8000).contains(offset) {
                            return Err(unsupported(self.span, "a displacement beyond 16 bits is"));
                        }
                        let r = self.alloc_a()?;
                        self.out.push(Insn::Movea {
                            size: Size::L,
                            src: Ea::Disp(o, FP),
                            dst: r,
                        });
                        if *offset == 0 {
                            Ea::Ind(r)
                        } else {
                            Ea::Disp(*offset, r)
                        }
                    }
                    None => self.ea_of(addr)?,
                };
                if matches!(ea, Ea::Reg(_)) {
                    // Not expected (volatile locals and parameters are not
                    // register candidates), but refused rather than
                    // compiled with the object held in a register.
                    return Err(unsupported(
                        self.span,
                        "a volatile object held in a register (ref 3.15.1) is",
                    ));
                }
                self.volatile_eas.push((ea.clone(), *access));
                Ok(ea)
            }
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

    /// [`on_every_path`], [`after_a_branch`] (codegen-patterns 12.4
    /// R4(c), bead domain-pascal-9j1z): a call in an `if` part is not on
    /// every path; one after the `if`'s join is, and comes after a branch;
    /// one before the `if` is on every path but before any branch.
    #[test]
    fn call_positions_for_held_arguments() {
        let call = || Insn::Jsr(Ea::AbsL("T".into(), 0));
        let bne = Insn::Branch {
            cond: Cond::Ne,
            label: 1,
            short: true,
            back: false,
        };
        // 0: call; 1: bne 1f; 2: call; 3: 1:; 4: call
        let out = vec![call(), bne, call(), Insn::Label(1), call()];
        assert!(on_every_path(&out, 0) && !after_a_branch(&out, 0));
        assert!(!on_every_path(&out, 2) && after_a_branch(&out, 2));
        assert!(on_every_path(&out, 4) && after_a_branch(&out, 4));
        // An `rts` in the middle (no epilogue join) is a way out too.
        let out = vec![
            Insn::Branch {
                cond: Cond::Ne,
                label: 1,
                short: true,
                back: false,
            },
            Insn::Rts,
            Insn::Label(1),
            call(),
        ];
        assert!(!on_every_path(&out, 3));
    }

    fn span() -> Span {
        let f = SourceMap::new().add("t.pas", String::new());
        Span::new(f, 0, 0)
    }

    fn func(frame_size: u32, result: Option<ResultSpec>, body: Vec<StmtKind>) -> Function {
        Function {
            symbol: "F".into(),
            global: true,
            code_section: ".text".into(),
            data_section: ".data".into(),
            frame_size,
            result,
            loads_data_base: false,
            data_absolute: false,
            value_params: Vec::new(),
            address_params: Vec::new(),
            scalar_locals: Vec::new(),
            nesting: None,
            uplevel_refs: Vec::new(),
            param_copies: Vec::new(),
            with_temps: Vec::new(),
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

    /// One-byte integers (`[byte]` subranges, ref 3.15.5) in word
    /// operations: `x & c` with c in 0..255 loads the constant as a word
    /// and `and.b`s the byte (TPAD_$DATA 0xE691EE `moveq #0x70,D0; and.b
    /// (11,A1),D0`, 0xE693EC `move.w #240,D3; ... and.b (12,A1),D3`); a
    /// signed byte is sign-extended, `move.b; ext.w` (0xE69228..0xE6922E),
    /// an unsigned one zero-extended, `clr.w; move.b` (0xE691D8..0xE691DC).
    #[test]
    fn byte_integers_in_word_operations() {
        let ub = Scalar::unsigned(Width::Byte);
        let sb = Scalar::signed(Width::Byte);
        let uw = Scalar::unsigned(Width::Word);
        let widen = |e: Expr, ty: Scalar| Expr::Convert {
            value: Box::new(e),
            ty,
        };
        let and = |c: i64| Expr::Binary {
            op: BinOp::And,
            lhs: Box::new(widen(load(through(8, 11), ub), uw)),
            rhs: Box::new(konst(c, uw)),
            ty: uw,
        };
        let mut f = func(
            0,
            None,
            vec![
                store(Addr::Static(0), and(0x70)),
                store(Addr::Static(4), and(240)),
                store(Addr::Static(8), widen(load(through(8, 12), sb), word())),
                store(Addr::Static(12), widen(load(through(8, 10), ub), uw)),
            ],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        let f = with_data(f);
        let t = text(&f);
        let body: Vec<&str> = t
            .iter()
            .map(String::as_str)
            .skip_while(|i| !i.starts_with("movea.l"))
            .take(13)
            .collect();
        assert_eq!(
            body,
            [
                "movea.l\t(8,%fp),%a0",
                "moveq\t#0x70,%d0",
                "and.b\t(0xb,%a0),%d0",
                "move.w\t%d0,(%a5)",
                "move.w\t#0xf0,%d0",
                "and.b\t(0xb,%a0),%d0",
                "move.w\t%d0,(4,%a5)",
                "move.b\t(0xc,%a0),%d0",
                "ext.w\t%d0",
                "move.w\t%d0,(8,%a5)",
                "clr.w\t%d0",
                "move.b\t(0xa,%a0),%d0",
                "move.w\t%d0,(0xc,%a5)",
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
        // Not adjacent (a gap), too big together (two longwords), or 3
        // bytes (a byte above a word: module-local 0xE32B76,
        // 0xE32BA6/0xE32BAA): accepted.  A word and a longword: below.
        for (first, o, ty) in [
            (word(), 4, word()),
            (Scalar::signed(Width::Long), 4, Scalar::signed(Width::Long)),
            (word(), 2, Scalar::BOOLEAN),
        ] {
            let mut f = func(
                0,
                None,
                vec![
                    store(through(8, 0), konst(0, first)),
                    store(through(8, o), konst(0, ty)),
                ],
            );
            vparams(&mut f, &[(8, Scalar::POINTER)]);
            assert!(select(&f).is_ok(), "{o}: {:?}", select(&f));
        }
    }

    /// Codegen-patterns 13, "Store merging": only a pair that one
    /// reference could store is a candidate (bead domain-pascal-2m0).
    #[test]
    fn adjacent_stores_that_cannot_merge_are_accepted() {
        let ptrs = [
            (8, Scalar::POINTER),
            (12, Scalar::POINTER),
            (16, Scalar::POINTER),
        ];
        // Words from two other pointers (TPAD_$SET_UNIT_MODE 0xE69830..
        // 0xE69840): no one reference.
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), load(through(12, 0), word())),
                store(through(8, 2), load(through(16, 0), word())),
            ],
        );
        vparams(&mut f, &ptrs);
        assert!(select(&f).is_ok(), "{:?}", select(&f));
        // A constant next to a load: accepted.
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), load(through(12, 0), word())),
                store(through(8, 2), konst(0, word())),
            ],
        );
        vparams(&mut f, &ptrs);
        assert!(select(&f).is_ok(), "{:?}", select(&f));
        // Adjacent memory copied to adjacent memory (could be one
        // `move.l`): rejected, in either order.
        for (a, b) in [(0, 2), (2, 0)] {
            let mut f = func(
                0,
                None,
                vec![
                    store(through(8, a), load(through(12, a), word())),
                    store(through(8, b), load(through(12, b), word())),
                ],
            );
            vparams(&mut f, &ptrs);
            assert!(err(&f).starts_with("a store adjacent to the previous"));
        }
        // The same memory pattern mirrored the other way: no one reference.
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), load(through(12, 2), word())),
                store(through(8, 2), load(through(12, 0), word())),
            ],
        );
        vparams(&mut f, &ptrs);
        assert!(select(&f).is_ok(), "{:?}", select(&f));
        // Two frame bytes, the lower at an odd address (0xE04CAC,
        // 0xE19F1A): no word there on the 68010.  At an even one: rejected.
        let b = Scalar::BOOLEAN;
        let f = func(
            8,
            None,
            vec![
                store(Addr::Frame(-3), konst(0, b)),
                store(Addr::Frame(-2), konst(0, b)),
            ],
        );
        assert!(select(&f).is_ok(), "{:?}", select(&f));
        let f = func(
            8,
            None,
            vec![
                store(Addr::Frame(-4), konst(0, b)),
                store(Addr::Frame(-3), konst(0, b)),
            ],
        );
        assert!(err(&f).starts_with("a store adjacent to the previous"));
        // Through a pointer the parity is not known: rejected.
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 5), konst(0, b)),
                store(through(8, 6), konst(0, b)),
            ],
        );
        vparams(&mut f, &ptrs);
        assert!(err(&f).starts_with("a store adjacent to the previous"));
        // A word and a longword of constants, the word lower
        // (FIM_$BUILD_DF 0xE0A896, merged and split again): rejected in
        // either order; the longword lower (0xE70B5E): accepted in either
        // order; a word and a longword copied (0xE0A478, apart): accepted.
        let l = Scalar::signed(Width::Long);
        for (w, lo, ok) in [(0, 2, false), (4, 0, true)] {
            let mut f = func(
                0,
                None,
                vec![
                    store(through(8, w), konst(0, word())),
                    store(through(8, lo), konst(0, l)),
                ],
            );
            vparams(&mut f, &ptrs);
            assert_eq!(select(&f).is_ok(), ok, "{w}: {:?}", select(&f));
        }
        for (lo, w, ok) in [(2, 0, false), (0, 4, true)] {
            let mut f = func(
                0,
                None,
                vec![
                    store(through(8, lo), konst(0, l)),
                    store(through(8, w), konst(0, word())),
                ],
            );
            vparams(&mut f, &ptrs);
            assert_eq!(select(&f).is_ok(), ok, "{lo}: {:?}", select(&f));
        }
        let mut f = func(
            0,
            None,
            vec![
                store(through(8, 0), load(through(12, 0), word())),
                store(through(8, 2), load(through(12, 2), l)),
            ],
        );
        vparams(&mut f, &ptrs);
        assert!(select(&f).is_ok(), "{:?}", select(&f));
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

    /// [`reuse_pointer_copies`]: a copy whose value an address register
    /// still holds on every path is dropped (the same register) or renamed
    /// to the holder (another one); a holder written in between, or a
    /// call, keeps the copy (OSINFO_$GET_MMAP 0xE5C870 after the `jsr` at
    /// 0xE5C864; reads of the copy at 0xE5C8C8 past an `if`).
    #[test]
    fn pointer_copies_are_reused_while_held() {
        let copy = |a: u8| Insn::Movea {
            size: Size::L,
            src: Ea::Reg(Reg::D(0)),
            dst: a,
        };
        let clr = |size: Size, d: i32, a: u8| Insn::Clr {
            size,
            dst: if d == 0 { Ea::Ind(a) } else { Ea::Disp(d, a) },
        };
        // Another register: renamed to the holder.
        let mut out = vec![copy(0), clr(Size::L, 0, 0), copy(1), clr(Size::W, 4, 1)];
        assert_eq!(reuse_pointer_copies(&mut out, &[0, 2]), vec![2]);
        assert_eq!(out[3], clr(Size::W, 4, 0));
        // The same register past an `if` and its join: dropped.
        let mut out = vec![
            copy(1),
            Insn::Tst {
                size: Size::L,
                dst: Ea::Ind(1),
            },
            Insn::Branch {
                cond: Cond::Ne,
                label: 1,
                short: true,
                back: false,
            },
            clr(Size::W, 4, 0),
            Insn::Label(1),
            copy(1),
            clr(Size::L, 0, 1),
        ];
        assert_eq!(reuse_pointer_copies(&mut out, &[0, 5]), vec![5]);
        // The holder written in between, or a call: kept.
        let mut out = vec![
            copy(0),
            clr(Size::L, 0, 0),
            Insn::Movea {
                size: Size::L,
                src: Ea::Disp(8, FP),
                dst: 0,
            },
            clr(Size::L, 0, 0),
            copy(1),
            clr(Size::W, 4, 1),
        ];
        assert!(reuse_pointer_copies(&mut out, &[0, 4]).is_empty());
        let mut out = vec![
            copy(0),
            clr(Size::L, 0, 0),
            Insn::Jsr(Ea::AbsL("X".into(), 0)),
            copy(0),
            clr(Size::W, 4, 0),
        ];
        assert!(reuse_pointer_copies(&mut out, &[0, 3]).is_empty());
        // Two registers hold the copy: the copy's own is read, though
        // a label keeps the other from being renamed in.
        let mut out = vec![
            copy(0),
            clr(Size::L, 0, 0),
            copy(1),
            Insn::Label(1),
            clr(Size::L, 0, 1),
            copy(1),
            Insn::Label(2),
            clr(Size::W, 4, 1),
        ];
        assert_eq!(reuse_pointer_copies(&mut out, &[0, 2, 5]), vec![5]);
    }

    /// A pointer parameter also used as data is held in a data register
    /// and copied to an address register to be dereferenced
    /// (codegen-patterns 12.2, "Pointers in data registers"; EC2_$INIT
    /// 0xE42C64..0xE42C74; PROC1_$FREE_STACK 0xE15126..0xE15136, the
    /// pointer stepped by `subq.l #4,D0` at 0xE15130).  Only stored whole,
    /// it stays in an address register (EC_$INIT 0xE15202..0xE1520C; the
    /// AST module-local 0xE00F7C, 0xE00FA0 `move.l A1,(0x3ec,A5)`).
    #[test]
    fn pointer_used_as_data_is_held_in_a_data_register() {
        let x = || Addr::Symbol {
            name: "X".into(),
            offset: 0,
        };
        let body = |value: Expr| {
            let mut f = func(
                0,
                None,
                vec![store(through(8, 0), konst(0, word())), store(x(), value)],
            );
            vparams(&mut f, &[(8, Scalar::POINTER)]);
            f
        };
        let f = body(Expr::Binary {
            op: BinOp::Sub,
            lhs: Box::new(param(8, Scalar::POINTER)),
            rhs: Box::new(konst(4, Scalar::POINTER)),
            ty: Scalar::POINTER,
        });
        assert_eq!(
            text(&f)[1..4],
            ["move.l\t(8,%fp),%d0", "movea.l\t%d0,%a0", "clr.w\t(%a0)",]
        );
        let f = body(param(8, Scalar::POINTER));
        assert_eq!(
            text(&f)[1..4],
            ["movea.l\t(8,%fp),%a0", "clr.w\t(%a0)", "move.l\t%a0,(X).l",]
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
        // Both sides need an address register: one instruction each, the
        // value first (codegen-patterns 12.4, "Stores whose destination
        // and value both need a register"; bead domain-pascal-k1sv).
        let f = func(
            0,
            None,
            vec![store(through(8, 0), load(through(12, 0), word()))],
        );
        let t = text(&f);
        let i = |s: &str| t.iter().position(|l| l == s).expect(s);
        assert!(
            i("movea.l\t(0xc,%fp),%a0") < i("movea.l\t(8,%fp),%a1"),
            "{t:?}"
        );
        assert!(t.contains(&"move.w\t(%a0),(%a1)".to_string()), "{t:?}");
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

    /// 8 bytes: the source post-incremented, the destination by
    /// displacement (codegen-patterns 5.2, bead domain-pascal-fbg).
    #[test]
    fn eight_byte_copies() {
        // Into the frame from a pointer slot: DIR_$ADD_MOUNT 0xE534CA.
        let f = func(8, None, vec![copy(Addr::Frame(-8), through(8, 0), 8)]);
        assert_eq!(
            &text(&f)[1..4],
            [
                "movea.l\t(8,%fp),%a0",
                "move.l\t(%a0)+,(-8,%fp)",
                "move.l\t(%a0)+,(-4,%fp)"
            ]
        );
        // From the data area into the data area: `lea (A5),A0` (UID_$GEN
        // 0xE1A0B6 into the frame; 0xE00BF0 into `(1144,A5)`).
        let f = with_data(func(
            0,
            None,
            vec![copy(Addr::Static(0x478), Addr::Static(0), 8)],
        ));
        assert_eq!(
            &text(&f)[3..6],
            [
                "lea\t(%a5),%a0",
                "move.l\t(%a0)+,(0x478,%a5)",
                "move.l\t(%a0)+,(0x47c,%a5)"
            ]
        );
        // Into a pointer slot's object from the frame: the source first
        // (UID_$GEN 0xE1A0E2..0xE1A0EC).
        let f = func(8, None, vec![copy(through(8, 0), Addr::Frame(-8), 8)]);
        assert_eq!(
            &text(&f)[1..5],
            [
                "lea\t(-8,%fp),%a0",
                "movea.l\t(8,%fp),%a1",
                "move.l\t(%a0)+,(%a1)",
                "move.l\t(%a0)+,(4,%a1)"
            ]
        );
        // Another unit's object as the source: `movea.l #sym` (0xE00F80
        // `movea.l #UID_$NIL,A0; movea.l (8,A6),A1; move.l (A0)+,(16,A1)`).
        let nil = Addr::Symbol {
            name: "UID_$NIL".into(),
            offset: 0,
        };
        let f = func(4, None, vec![copy(through(8, 16), nil, 8)]);
        assert_eq!(
            &text(&f)[1..5],
            [
                "movea.l\t#UID_$NIL,%a0",
                "movea.l\t(8,%fp),%a1",
                "move.l\t(%a0)+,(0x10,%a1)",
                "move.l\t(%a0)+,(0x14,%a1)"
            ]
        );
        // From the data area into a pointer slot's object: the slot first,
        // coloured after the source (NAME_$GET_ROOT_UID 0xE589C8).
        let f = with_data(func(
            0,
            None,
            vec![copy(through(8, 0), Addr::Static(0x38), 8)],
        ));
        assert_eq!(
            &text(&f)[3..7],
            [
                "movea.l\t(8,%fp),%a1",
                "lea\t(0x38,%a5),%a0",
                "move.l\t(%a0)+,(%a1)",
                "move.l\t(%a0)+,(4,%a1)"
            ]
        );
        // Into another unit's object (0xE31C28 `move.l (A0)+,abs.l`).
        let sym = Addr::Symbol {
            name: "X".into(),
            offset: 0,
        };
        let f = func(4, None, vec![copy(sym, through(8, 0), 8)]);
        assert_eq!(
            &text(&f)[1..4],
            [
                "movea.l\t(8,%fp),%a0",
                "move.l\t(%a0)+,(X).l",
                "move.l\t(%a0)+,(X+4).l"
            ]
        );
    }

    /// 6 bytes: `move.l` and `move.w` by displacement on both sides
    /// (TONE_$TIME 0xE17304; KBD_$RCV 0xE1CDB8; CAL_$SET_DRIFT 0xE68588).
    #[test]
    fn six_byte_copies() {
        let f = func(0x10, None, vec![copy(Addr::Frame(-0x10), through(8, 0), 6)]);
        assert_eq!(
            &text(&f)[1..4],
            [
                "movea.l\t(8,%fp),%a0",
                "move.l\t(%a0),(-0x10,%fp)",
                "move.w\t(4,%a0),(-0xc,%fp)"
            ]
        );
        let f = with_data(func(
            0x10,
            None,
            vec![copy(Addr::Static(0x20), Addr::Frame(-0x10), 6)],
        ));
        assert_eq!(
            &text(&f)[3..5],
            [
                "move.l\t(-0x10,%fp),(0x20,%a5)",
                "move.w\t(-0xc,%fp),(0x24,%a5)"
            ]
        );
    }

    #[test]
    fn unsettled_block_copies_are_rejected() {
        let f = func(
            0x10,
            None,
            vec![copy(Addr::Frame(-0x10), through(8, 0), 11)],
        );
        assert_eq!(
            err(&f),
            "a 11-byte record or array assignment is not supported yet"
        );
        // 8 bytes into an element indexed at run time (0xE38CA2, 11
        // sites, order not settled).
        let idx = Addr::Index {
            base: Box::new(Addr::Frame(-0x10)),
            index: Box::new(load(Addr::Frame(-0x12), word())),
            scale: 8,
            offset: 0,
        };
        let f = func(0x12, None, vec![copy(idx, through(8, 0), 8)]);
        assert!(err(&f).starts_with("an 8-byte copy into this kind of destination"));
        // Another unit's object; a field through a pointer.
        let sym = Addr::Symbol {
            name: "X".into(),
            offset: 0,
        };
        let f = func(0x10, None, vec![copy(Addr::Frame(-0x10), sym, 12)]);
        assert!(err(&f).starts_with("a block (record or array copy or comparison) at this kind"));
        let f = func(
            0x10,
            None,
            vec![copy(Addr::Frame(-0x10), through(8, 4), 12)],
        );
        assert!(err(&f).starts_with("a block through a pointer not held in an address register"));
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
        // Other bytes: in the high half of a word push (0xE1AFC0 chr(254)
        // `move.w #-0x200`; 0xE1C7D6 0x13 `move.w #0x1300`).
        let byte = |v| konst(v, Scalar::unsigned(Width::Byte));
        for (v, want) in [
            (254, "move.w\t#-0x200,-(%sp)"),
            (0x13, "move.w\t#0x1300,-(%sp)"),
            (0x7f, "move.w\t#0x7f00,-(%sp)"),
        ] {
            let f = proc_(
                0,
                vec![ext("P", vec![val(byte(v))], 2), ext("Q", vec![], 0)],
            );
            assert_eq!(body_text(&f)[1], want);
        }
        // A signed byte -2: the same bits as 254.
        let f = proc_(
            0,
            vec![
                ext("P", vec![val(konst(-2, Scalar::signed(Width::Byte)))], 2),
                ext("Q", vec![], 0),
            ],
        );
        assert_eq!(body_text(&f)[1], "move.w\t#-0x200,-(%sp)");
        // All bits set other than `true`: st or move.w #-0x100?  Not known.
        let f = proc_(0, vec![ext("P", vec![val(byte(255))], 2)]);
        assert!(err(&f).starts_with("a byte constant argument with all bits set"));
        // Next to a word constant or another such byte: merged?  Not known.
        let f = proc_(
            0,
            vec![ext("P", vec![val(byte(0x41)), val(konst(1, word()))], 0)],
        );
        assert!(err(&f).contains("next to a word or byte constant"));
        let f = proc_(
            0,
            vec![ext("P", vec![val(konst(0, word())), val(byte(0x41))], 0)],
        );
        assert!(err(&f).contains("next to a word or byte constant"));
        let f = proc_(0, vec![ext("P", vec![val(byte(0x41)), val(byte(0x42))], 0)]);
        assert!(err(&f).contains("next to a word or byte constant"));
        // Next to `true` (`st`): separate pushes (SIO_$INIT 0xE32E60 `st
        // -(SP)` then 0xE32E62 `move.w #0x1b00,-(SP)`).
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(byte(0x1b)), val(konst(-1, Scalar::BOOLEAN))],
                0,
            )],
        );
        assert_eq!(body_text(&f)[..2], ["st\t-(%sp)", "move.w\t#0x1b00,-(%sp)"]);
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
        // `false` (a byte 0, cleared as a word) merges with a word constant
        // next to it (FILE_$TRUNCATE_D 0xE73FFC `clr.l -(SP)`); `true`
        // (`st -(SP)`) does not.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(0, Scalar::BOOLEAN)), val(konst(5, word()))],
                0,
            )],
        );
        assert_eq!(body_text(&f)[0], "pea\t(5).w");
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![val(konst(-1, Scalar::BOOLEAN)), val(konst(0, word()))],
                0,
            )],
        );
        assert_eq!(body_text(&f)[..2], ["clr.w\t-(%sp)", "st\t-(%sp)"]);
    }

    /// The low bytes of a big-endian object in memory are at the higher
    /// address ([`displace`]; APP_$DEMUX 0xE00AF6 `(0x1a,A0)` for a
    /// longword at 0x18, PACCT_$LOG 0xE5AB26 `(1,A0)` for a word at
    /// `(A0)`); a mode without room for the displacement is left alone.
    #[test]
    fn displace_low_bytes() {
        assert_eq!(displace(&Ea::Disp(0x18, 0), 2), Some(Ea::Disp(0x1a, 0)));
        assert_eq!(displace(&Ea::Ind(0), 1), Some(Ea::Disp(1, 0)));
        assert_eq!(displace(&Ea::Disp(-20, 6), 2), Some(Ea::Disp(-18, 6)));
        assert_eq!(
            displace(&Ea::AbsL("X".into(), 4), 2),
            Some(Ea::AbsL("X".into(), 6))
        );
        assert_eq!(
            displace(&Ea::Idx(-4, 0, 1, true), 2),
            Some(Ea::Idx(-2, 0, 1, true))
        );
        assert_eq!(displace(&Ea::Disp(0x7fff, 0), 2), None);
        assert_eq!(displace(&Ea::Idx(126, 0, 1, false), 2), None);
        assert_eq!(displace(&Ea::PostInc(0), 2), None);
    }

    /// Only a non-volatile object one step wider, not an element with a
    /// computed index, is narrowed by reading its low part in place
    /// ([`narrows_in_place`]); a volatile word (`chr(exec_$counter)`)
    /// is read whole (review of bead domain-pascal-8td).
    #[test]
    fn narrows_in_place_forms() {
        let load = |addr, ty| Expr::Load { addr, ty };
        let byte = Scalar::unsigned(Width::Byte);
        let w = Scalar::signed(Width::Word);
        let l = Scalar::signed(Width::Long);
        assert!(narrows_in_place(&load(Addr::Frame(-4), l), w));
        assert!(narrows_in_place(&load(Addr::Static(2), w), byte));
        assert!(!narrows_in_place(&load(Addr::Frame(-4), l), byte));
        let vol = Addr::Volatile {
            addr: Box::new(Addr::Static(2)),
            access: Access::default(),
        };
        assert!(!narrows_in_place(&load(vol, w), byte));
        let elem = Addr::Index {
            base: Box::new(Addr::Frame(-8)),
            index: Box::new(load(Addr::Frame(-10), w)),
            scale: 4,
            offset: 0,
        };
        assert!(!narrows_in_place(&load(elem, l), w));
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

    /// An object inside one a pointer in memory points to, passed by
    /// address: the pointer loaded, then `pea` (codegen-patterns 1.5;
    /// KBD_$OUTPUT_BUFFER_DRAINED 0xE1CE9E, AST_$PURIFY 0xE0599E).
    #[test]
    fn reloaded_pointer_pushes() {
        let addr = |a: Addr| Arg::Address {
            addr: a,
            slot: 4,
            writable: true,
        };
        let local = |offset| Addr::Indirect {
            base: Box::new(load(Addr::Frame(-4), Scalar::POINTER)),
            offset,
        };
        // A field through a slot not held.
        let f = proc_(0, vec![ext("P", vec![addr(through(8, 0x4c))], 0)]);
        assert_eq!(
            body_text(&f),
            ["movea.l\t(8,%fp),%a0", "pea\t(0x4c,%a0)", "jsr\t(P).l"]
        );
        // The slot's own target is still the slot pushed (LOG_$READ).
        let f = proc_(0, vec![ext("P", vec![addr(through(8, 0))], 0)]);
        assert_eq!(body_text(&f), ["move.l\t(8,%fp),-(%sp)", "jsr\t(P).l"]);
        // A pointer local in the frame: loaded for a field; its own
        // target `p^` is the pointer pushed (0xE53DB0 FILE_$FW_FILE).
        let f = proc_(4, vec![ext("P", vec![addr(local(0x28))], 0)]);
        assert_eq!(
            body_text(&f),
            ["movea.l\t(-4,%fp),%a0", "pea\t(0x28,%a0)", "jsr\t(P).l"]
        );
        let f = proc_(4, vec![ext("P", vec![addr(local(0))], 0)]);
        assert_eq!(body_text(&f), ["move.l\t(-4,%fp),-(%sp)", "jsr\t(P).l"]);
        // The same pointer read twice by one argument list: refused (the
        // original loads it once, 0xE74706).
        let f = proc_(4, vec![ext("P", vec![addr(local(4)), addr(local(8))], 0)]);
        assert!(err(&f).starts_with("an argument list that reads twice a pointer"));
        // The target of a pointer in the unit's data: not catalogued.
        let f = proc_(
            0,
            vec![ext(
                "P",
                vec![addr(Addr::Indirect {
                    base: Box::new(load(Addr::Static(8), Scalar::POINTER)),
                    offset: 0,
                })],
                0,
            )],
        );
        assert!(err(&f).starts_with("passing this kind of object by address"));
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
        // An address inside another unit's object (or an overlay
        // section's variable after its first): `pea (sym+d).l`
        // (codegen-patterns 1.5, 11: PCHIST_$CNTL 0xE5CF00).
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
        assert_eq!(body_text(&f), ["pea\t(X+4).l", "jsr\t(P).l"]);
        // `addr(x)` of a frame object by value: `pea`, as by address
        // (codegen-patterns 1.5).
        let f = proc_(
            4,
            vec![ext("P", vec![val(Expr::AddrOf(Addr::Frame(-4)))], 0)],
        );
        assert_eq!(body_text(&f), ["pea\t(-4,%fp)", "jsr\t(P).l"]);
        let f = proc_(
            4,
            vec![ext(
                "P",
                vec![val(Expr::AddrOf(Addr::Symbol {
                    name: "X".into(),
                    offset: 4,
                }))],
                0,
            )],
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

    /// The facts `crate::locals::body_uses` gives (codegen-patterns 12.5,
    /// `dpc --dump-locals`): where each assignment and read is, whether a
    /// read pushes the local as an argument, and whether a call lies
    /// between an assignment and a read or in a loop around a read.
    #[test]
    fn body_uses_facts() {
        let w = word();
        let x = || load(Addr::Frame(-2), w);
        let set = |v: Expr| store(Addr::Frame(-2), v);
        let st = |k: StmtKind| Stmt {
            kind: k,
            span: span(),
        };
        let call = |value: Expr| {
            StmtKind::Call(Call {
                callee: Callee::External("Q".into()),
                args: vec![Arg::Value { value, slot: 2 }],
                pad: 2,
                link: None,
            })
        };
        let plus1 = || Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(x()),
            rhs: Box::new(konst(1, w)),
            ty: w,
        };
        let body = vec![
            set(load(sym("V"), w)),
            StmtKind::If {
                cond: IrCond::Test(load(sym("B"), Scalar::BOOLEAN)),
                then_: vec![st(set(konst(1, w)))],
                else_: vec![],
            },
            StmtKind::While {
                cond: IrCond::Compare {
                    op: dpc_ir::Rel::Lt,
                    lhs: x(),
                    rhs: konst(9, w),
                },
                body: vec![st(call(x())), st(set(plus1()))],
            },
            call(plus1()),
        ];
        let mut f = proc_(2, body);
        locals(&mut f, &[(-2, w)]);
        let u = crate::locals::body_uses(&f);
        let l = &u.locals[0];
        let stores: Vec<(usize, usize, bool, bool)> = l
            .stores
            .iter()
            .map(|s| (s.at.top, s.at.seq, s.at.nested, s.konst))
            .collect();
        assert_eq!(
            stores,
            [
                (0, 0, false, false),
                (1, 2, true, true),
                (2, 5, true, false)
            ]
        );
        assert_eq!(l.stores[2].at.lp, Some((3, 5)));
        // The loop test (numbered as the loop statement, walked after the
        // body), the pushed argument, `x + 1` in the loop and as the last
        // call's argument.
        let mut loads: Vec<(usize, bool, bool, bool)> = l
            .loads
            .iter()
            .map(|r| (r.at.seq, r.arg, r.whole, r.cond))
            .collect();
        loads.sort();
        assert_eq!(
            loads,
            [
                (3, false, false, true),
                (4, true, true, false),
                (5, false, false, false),
                (6, true, false, false)
            ]
        );
        assert_eq!(u.calls.len(), 2);
        assert!(u.across_call(l));
        assert!(!l.other && !l.deref && !l.for_index);
    }

    /// Which locals [`register_locals`] holds in a register (every local
    /// read and assigned only as a whole, assigned at least once, unless
    /// each read pushes it as an argument) and which of them are webs (all
    /// but one assignment of a non-constant at the top level, reads after
    /// it).
    #[test]
    fn register_local_rule() {
        let w = word();
        let x = || load(Addr::Frame(-2), w);
        let set = |v: Expr| store(Addr::Frame(-2), v);
        let use_ = || store(sym("W"), x());
        let push = || {
            StmtKind::Call(Call {
                callee: Callee::External("Q".into()),
                args: vec![Arg::Value {
                    value: x(),
                    slot: 2,
                }],
                pad: 2,
                link: None,
            })
        };
        // None: in the frame; Some(web).
        let cases: Vec<(&str, Vec<StmtKind>, Option<bool>)> = vec![
            (
                "one store, then reads",
                vec![set(load(sym("V"), w)), use_(), use_()],
                Some(false),
            ),
            ("a constant", vec![set(konst(5, w)), use_()], Some(true)),
            (
                "two stores",
                vec![set(load(sym("V"), w)), set(load(sym("U"), w)), use_()],
                Some(true),
            ),
            (
                "a read first",
                vec![use_(), set(load(sym("V"), w))],
                Some(true),
            ),
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
                Some(true),
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
                None,
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
                None,
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
                Some(false),
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
                None,
            ),
        ];
        let more: Vec<(&str, Vec<StmtKind>, Option<bool>)> = vec![
            (
                "pushed and also read otherwise",
                vec![set(load(sym("V"), w)), push(), use_()],
                Some(false),
            ),
            (
                "pushed twice, nothing else",
                vec![set(load(sym("V"), w)), push(), push()],
                None,
            ),
            ("never assigned", vec![use_()], None),
            (
                "assigned, never read",
                vec![set(load(sym("V"), w))],
                Some(false),
            ),
        ];
        for (what, body, held) in cases.into_iter().chain(more) {
            let mut f = proc_(2, body);
            locals(&mut f, &[(-2, w)]);
            let got = register_locals(&f);
            let want: Vec<(i32, bool)> = held.map(|web| (-2, web)).into_iter().collect();
            assert_eq!(got, want, "{what}");
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
        // One pass: the count cleared (SMD_$DM_COND_EVENT_WAIT 0xE6F012).
        let f = proc_(
            4,
            vec![for_loop(
                konst(5, word()),
                konst(5, word()),
                false,
                vec![store(sym("W"), konst(1, word()))],
            )],
        );
        assert_eq!(body_text(&f)[0], "clr.w\t%d0");
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

    /// Stepped pointers through arrays at constant addresses
    /// (codegen-patterns 7.4, "Forming"; bead domain-pascal-xm5w): another
    /// unit's always in place from `movea.l #sym`; the frame's with `lea
    /// (k,A6)` when every displacement fits in a byte in the first pass,
    /// in place from `movea.l A6` otherwise; the in-place `movea.l` one
    /// instruction before the last set-up instruction when it is adjusted.
    #[test]
    fn constant_address_stepped_loops() {
        let elem = |base: Addr, scale: u32, offset: i32| Addr::Index {
            base: Box::new(base),
            index: Box::new(index()),
            scale,
            offset,
        };
        // NET_IO_$CREATE_PORT 0xE5A52C: `moveq; movea.l #sym; moveq; addq`.
        let f = proc_(
            4,
            vec![
                for_loop(
                    konst(1, word()),
                    konst(4, word()),
                    false,
                    vec![
                        store(elem(sym("TAB"), 2, -2), konst(0, word())),
                        store(sym("W"), index()),
                    ],
                ),
                ext("Q", vec![], 0),
            ],
        );
        assert_eq!(
            body_text(&f),
            [
                "moveq\t#3,%d0",
                "movea.l\t#TAB,%a0",
                "moveq\t#1,%d1",
                "addq.l\t#2,%a0",
                ".balignw\t4,0x2048",
                "2:",
                "clr.w\t(-2,%a0)",
                "move.w\t%d1,(W).l",
                "addq.w\t#1,%d1",
                "addq.l\t#2,%a0",
                "dbf\t%d0,2b",
                "jsr\t(Q).l",
            ]
        );
        // A frame array in reach of a byte: `lea (2,A6)` after the count.
        let f = proc_(
            48,
            vec![for_loop(
                konst(1, word()),
                konst(8, word()),
                false,
                vec![store(elem(Addr::Frame(-40), 2, -2), load(sym("W"), word()))],
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "moveq\t#7,%d0",
                "lea\t(2,%fp),%a0",
                ".balignw\t4,0x2048",
                "2:",
                "move.w\t(W).l,(-0x2a,%a0)",
                "addq.l\t#2,%a0",
                "dbf\t%d0,2b",
            ]
        );
        // Out of reach: `movea.l A6,A0` before the count, `addq.l` after
        // (FILE_$PRIV_CREATE 0xE5D63E).
        let f = proc_(
            208,
            vec![for_loop(
                konst(1, word()),
                konst(36, word()),
                false,
                vec![store(
                    elem(Addr::Frame(-200), 4, -4),
                    load(sym("L"), long()),
                )],
            )],
        );
        assert_eq!(
            body_text(&f),
            [
                "movea.l\t%fp,%a0",
                "moveq\t#0x23,%d0",
                "addq.l\t#4,%a0",
                ".balignw\t4,0x2048",
                "2:",
                "move.l\t(L).l,(-0xcc,%a0)",
                "addq.l\t#4,%a0",
                "dbf\t%d0,2b",
            ]
        );
        // From 0, nothing to adjust: after the count (AREA_$REMOVE_SEG
        // 0xE099E2 `moveq #31,D0; movea.l A6,A0`).
        let f = proc_(
            208,
            vec![for_loop(
                konst(0, word()),
                konst(31, word()),
                false,
                vec![store(elem(Addr::Frame(-200), 4, 4), load(sym("L"), long()))],
            )],
        );
        assert_eq!(
            body_text(&f)[..3],
            ["moveq\t#0x1f,%d0", "movea.l\t%fp,%a0", ".balignw\t4,0x2048"]
        );
        let one = |body: Vec<StmtKind>| {
            err(&proc_(
                208,
                vec![for_loop(konst(1, word()), konst(4, word()), false, body)],
            ))
        };
        // Two frame arrays of one element size: shared or not, unsettled.
        assert!(one(vec![store(
            elem(Addr::Frame(-40), 2, -2),
            load(elem(Addr::Frame(-20), 2, -2), word())
        )])
        .contains("two arrays of one element size"));
        // An in-place pointer and another.
        assert!(one(vec![store(
            elem(Addr::Frame(-200), 4, -4),
            load(elem(sym("TAB"), 2, -2), word())
        )])
        .contains("formed in place"));
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
            vec![store(
                elem(
                    Addr::OtherStatic {
                        section: "S".into(),
                        offset: 0
                    },
                    index()
                ),
                konst(0, word())
            )]
        )
        .contains("another data section's block"));
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

    /// Bead domain-pascal-s4a5: a body of one `if` (or `case` or loop)
    /// loads the by-value parameters it reads at entry, as a body of
    /// several statements does, and a test of the parameter loaded last
    /// branches on that load's flags (TTY_$I_ENABLE_CRASH_FUNC
    /// 0xE67298..0xE672A4 `movea.l (8,A6),A0; move.b (0xc,A6),D0; move.b
    /// (0xe,A6),D1; bpl.s`; codegen-patterns 12.2, "One-statement
    /// bodies").
    #[test]
    fn one_statement_if_loads_value_parameters_at_entry() {
        let arm = |v: Expr| {
            vec![Stmt {
                kind: store(through(8, 0x10), v),
                span: span(),
            }]
        };
        let mut f = proc_(
            0,
            vec![StmtKind::If {
                cond: IrCond::Test(param(14, Scalar::BOOLEAN)),
                then_: arm(param(12, Scalar::unsigned(Width::Byte))),
                else_: arm(konst(0, Scalar::unsigned(Width::Byte))),
            }],
        );
        vparams(
            &mut f,
            &[
                (8, Scalar::POINTER),
                (12, Scalar::unsigned(Width::Byte)),
                (14, Scalar::BOOLEAN),
            ],
        );
        let t = body_text(&f);
        assert_eq!(
            t[..5],
            [
                "movea.l\t(8,%fp),%a0",
                "move.b\t(0xc,%fp),%d0",
                "move.b\t(0xe,%fp),%d1",
                "bpl.s\t1f",
                "move.b\t%d0,(0x10,%a0)",
            ],
            "{t:?}"
        );
        // A parameter loaded before the last entry load: `tst` of its
        // register (PROC1_$SET_TYPE 0xE152F2..0xE152FA `move.w (8,A6),D2;
        // move.w (0xa,A6),D3; tst.w D2`; 12 such sites in compiled code).
        let mut f = proc_(
            0,
            vec![StmtKind::If {
                cond: IrCond::Test(param(12, Scalar::BOOLEAN)),
                then_: arm(param(14, Scalar::unsigned(Width::Byte))),
                else_: vec![],
            }],
        );
        vparams(
            &mut f,
            &[
                (8, Scalar::POINTER),
                (12, Scalar::BOOLEAN),
                (14, Scalar::unsigned(Width::Byte)),
            ],
        );
        let t = body_text(&f);
        assert_eq!(
            t[..4],
            [
                "movea.l\t(8,%fp),%a0",
                "move.b\t(0xc,%fp),%d0",
                "move.b\t(0xe,%fp),%d1",
                "tst.b\t%d0",
            ],
            "{t:?}"
        );
        // A pointer loaded after the tested word: `movea` leaves the
        // word's flags and the branch follows it (PROC1_$RESUME
        // 0xE14774..0xE1477C `move.w (8,A6),D0; movea.l (0xa,A6),A0;
        // beq.s`); a byte boolean likewise.
        let ptr_arm = |v: i64| {
            vec![Stmt {
                kind: store(through(10, 0), konst(v, Scalar::signed(Width::Long))),
                span: span(),
            }]
        };
        for (ty, size) in [(word(), "w"), (Scalar::BOOLEAN, "b")] {
            let cond = if ty == word() {
                IrCond::Compare {
                    op: Rel::Eq,
                    lhs: param(8, ty),
                    rhs: konst(0, ty),
                }
            } else {
                IrCond::Test(param(8, ty))
            };
            let mut f = proc_(
                0,
                vec![StmtKind::If {
                    cond,
                    then_: ptr_arm(1),
                    else_: ptr_arm(2),
                }],
            );
            vparams(&mut f, &[(8, ty), (10, Scalar::POINTER)]);
            let t = body_text(&f);
            assert_eq!(
                t[..3],
                [
                    format!("move.{size}\t(8,%fp),%d0").as_str(),
                    "movea.l\t(0xa,%fp),%a0",
                    if size == "w" {
                        "bne.s\t1f"
                    } else {
                        "bpl.s\t1f"
                    },
                ],
                "{t:?}"
            );
        }
        // A word compared with a constant: loaded, then compared
        // (DBUF_$UPDATE_VOL 0xE3AAB0 `move.w (8,A6),D2`, its `for` loop's
        // `tst.w D2w` at 0xE3AAD2).
        let mut f = proc_(
            0,
            vec![StmtKind::If {
                cond: IrCond::Compare {
                    op: Rel::Eq,
                    lhs: param(8, word()),
                    rhs: konst(3, word()),
                },
                then_: vec![Stmt {
                    kind: store(sym("X"), konst(0, word())),
                    span: span(),
                }],
                else_: vec![],
            }],
        );
        vparams(&mut f, &[(8, word())]);
        let t = body_text(&f);
        assert_eq!(
            t[..3],
            ["move.w\t(8,%fp),%d0", "cmpi.w\t#3,%d0", "bne.s\t1f"],
            "{t:?}"
        );
        // An ordered test against 0 right after the load: no branch of
        // that kind is seen on a load's flags (all 20 are `beq`/`bne`/
        // `bmi`/`bpl`).
        let mut f = proc_(
            0,
            vec![StmtKind::If {
                cond: IrCond::Compare {
                    op: Rel::Gt,
                    lhs: param(8, word()),
                    rhs: konst(0, word()),
                },
                then_: vec![Stmt {
                    kind: store(sym("X"), konst(0, word())),
                    span: span(),
                }],
                else_: vec![],
            }],
        );
        vparams(&mut f, &[(8, word())]);
        assert!(err(&f).starts_with("an ordered test of a parameter right after its entry load"));
        // A function of one `if` assigning the parameter to its result:
        // refused (0xE1CCBA against 0xE15222; bead domain-pascal-0zo8).
        let mut f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: word(),
            }),
            vec![StmtKind::If {
                cond: IrCond::Compare {
                    op: Rel::Eq,
                    lhs: param(8, word()),
                    rhs: konst(3, word()),
                },
                then_: vec![Stmt {
                    kind: StmtKind::SetResult(konst(0, word())),
                    span: span(),
                }],
                else_: vec![Stmt {
                    kind: StmtKind::SetResult(param(8, word())),
                    span: span(),
                }],
            }],
        );
        vparams(&mut f, &[(8, word())]);
        assert!(err(&f).starts_with("a function body of one `if`, `case` or loop statement"));
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
