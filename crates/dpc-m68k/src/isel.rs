//! Instruction selection: IR → m68k instructions (CLAUDE.md 4.4).
//!
//! Two phases per routine:
//!
//! 1. **Body.**  Pattern-directed, one IR node at a time, over physical
//!    registers handed out by [`Regs`] (`regs.rs`).  The default policy is
//!    scratch only, D0, D1 / A0, A1 in that order (codegen-patterns 2,
//!    12); an expression that needs more is reported as not supported
//!    yet.  A pattern that must keep a value across a call asks for a
//!    callee-saved register with [`Selector::hold`]; one that loads A5
//!    reports it with [`Regs::clobber`].
//! 2. **Frame.**  The callee-saved registers the body wrote decide the
//!    save set; [`crate::frame::Frame`] then gives the prologue (`link.w`,
//!    the save) and the epilogue (the restore from the frame, `unlk`,
//!    `rts`) around the body.
//!
//! Each pattern cites the image address(es) it reproduces.  Patterns the
//! image has not shown are rejected, not guessed.

use dpc_ir::{
    Addr, Arg, Call, Callee, Cond as IrCond, Expr, Function, Rel, ResultClass, Scalar, Sign, Stmt,
    StmtKind, Width,
};
use dpc_syntax::{Diagnostic, Span};

use crate::emit::DATA_BASE_LABEL;
use crate::frame::{frame_for, FP, SP};
use crate::insn::{Cond, Ea, Insn, Reg, RegSet, Size};
use crate::regs::{RegClass, Regs, DATA_BASE};

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

fn size_of(w: Width) -> Size {
    match w {
        Width::Byte => Size::B,
        Width::Word => Size::W,
        Width::Long => Size::L,
    }
}

/// Select the instructions of one routine, prologue to `rts`.
pub fn select(f: &Function) -> Result<Vec<Insn>, Diagnostic> {
    let body = select_body(f)?;
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
/// 0xE16C62..0xE16C6A), and its statements may then use only those
/// registers: what the original did with further registers in such a body
/// (fresh registers per statement, IIC_$RECEIVE 0xE70ABA; scheduling of
/// entry loads with other work, 0xE758C4) is not modelled yet
/// (codegen-patterns 12.2).  The one exception is a simple statement right
/// after a call in a body that loaded nothing at entry: it takes the lowest
/// free scratch registers (0xE00ECC, after the `jsr` at 0xE00EC2).  A body
/// that loads parameters at entry may not call (the values would not
/// survive the call).
///
/// A body that is one `if` or `repeat` statement is treated like a body of
/// several statements (NAME_CONVERT_ACL_STATUS 0xE58620: one `if ... else
/// if` chain, its by-address status loaded into A0 once, at the top), but
/// may not read by-value parameters: whether the original loaded them at
/// entry there is not known (codegen-patterns 12.2; the sites that load
/// them at entry have several statements).
pub fn select_body(f: &Function) -> Result<Body, Diagnostic> {
    let mut s = Selector::new(f.span);
    if f.loads_data_base {
        s.load_data_base();
    }
    let control_only = f.body.len() == 1 && !is_simple(&f.body[0]);
    s.multi_body = f.body.len() >= 2 || control_only;
    if s.multi_body {
        if f.result.is_some() {
            // PARITY_$CHK_IO 0xE0B180..0xE0B18A loads its first parameter
            // into D1 and its second into D0: functions do not follow the
            // procedure order, and the rule is not known.
            return Err(unsupported(
                f.body[0].span,
                "a function body of more than one statement is",
            ));
        }
        let reads_value_param = param_uses(f)
            .iter()
            .any(|(o, _)| f.value_params.iter().any(|p| p.offset == *o));
        if control_only && reads_value_param {
            return Err(unsupported(
                f.body[0].span,
                "a body of one `if` or `repeat` statement that reads by-value parameters \
                 (whether they are loaded at entry is not settled, codegen-patterns 12.2) is",
            ));
        }
        reject_mergeable_stores(&f.body)?;
        s.entry_loads(f)?;
        if !s.pinned.is_empty() {
            if let Some(st) = find_stmt(&f.body, &|st| matches!(st.kind, StmtKind::Call(_))) {
                // A parameter loaded at entry into a scratch register does
                // not survive the call; the original then holds it in a
                // callee-saved register (PROC1_$SET_TYPE 0xE152F2, D2/D3
                // around a `jsr`), by a rule not catalogued
                // (codegen-patterns 12.1, 12.2).
                return Err(unsupported(
                    st.span,
                    "a call in a body that loads parameters at entry is",
                ));
            }
        }
    }
    s.seq(f, &f.body, Tail::END)?;
    if let Some(l) = s.epilogue {
        s.place(l);
    }
    // Labels nothing branches to (the join after a `then` part that ends
    // in `return`, ...) are dropped; they emit no bytes either way.
    let used: Vec<u32> = s
        .out
        .iter()
        .filter_map(|i| match i {
            Insn::Branch { label, .. } | Insn::Dbf { label, .. } => Some(*label),
            _ => None,
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
    s.out
        .retain(|i| !matches!(i, Insn::Label(l) if !used.contains(l)));
    Ok(Body {
        insns: s.out,
        saves: s.regs.to_save(),
    })
}

/// A statement that is not control flow.
fn is_simple(st: &Stmt) -> bool {
    !matches!(
        st.kind,
        StmtKind::If { .. } | StmtKind::Repeat { .. } | StmtKind::Return
    )
}

/// The first statement, in body order and depth first, for which `pred`
/// holds.
fn find_stmt<'a>(body: &'a [Stmt], pred: &dyn Fn(&Stmt) -> bool) -> Option<&'a Stmt> {
    for st in body {
        if pred(st) {
            return Some(st);
        }
        let inner = match &st.kind {
            StmtKind::If { then_, else_, .. } => {
                find_stmt(then_, pred).or_else(|| find_stmt(else_, pred))
            }
            StmtKind::Repeat { body, .. } => find_stmt(body, pred),
            _ => None,
        };
        if inner.is_some() {
            return inner;
        }
    }
    None
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
            StmtKind::SetResult(_) | StmtKind::Store { .. } => {}
            // Up to 18 bytes the copy is unrolled; from 20 it is a `dbf`
            // loop (codegen-patterns 5.2).
            StmtKind::Copy { bytes, .. } if *bytes <= 18 => {}
            StmtKind::Copy { .. }
            | StmtKind::Fill { .. }
            | StmtKind::Call(_)
            | StmtKind::If { .. }
            | StmtKind::Repeat { .. } => return false,
        }
    }
    tail_clean
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
/// longword are rejected rather than guessed at.  TIME_$Q_INIT_QUEUE's
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
            StmtKind::Repeat { body, .. } => reject_mergeable_stores(body)?,
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
                if k1 == k2 && adjacent && pb + bytes <= 4 {
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
    let mut u = Uses { f, out: Vec::new() };
    u.stmts(&f.body);
    u.out
}

/// The walk of [`param_uses`].
struct Uses<'a> {
    f: &'a Function,
    out: Vec<(i32, ParamUse)>,
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
            Expr::Const { .. } => {}
        }
    }

    fn address(&mut self, a: &Addr) {
        if let Addr::Indirect { base, .. } = a {
            match &**base {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ..
                } if self.is_param(*o) || self.is_slot(*o) => self.out.push((*o, ParamUse::Base)),
                b => self.expr(b),
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
                StmtKind::Repeat { body, until } => {
                    self.stmts(body);
                    self.cond(until);
                }
                StmtKind::Return => {}
                StmtKind::SetResult(e) => self.expr(e),
                StmtKind::Store { addr, value } => {
                    self.address(addr);
                    self.expr(value);
                }
                StmtKind::Copy { dst, src, .. } => {
                    self.address(src);
                    self.address(dst);
                }
                StmtKind::Call(c) => {
                    for a in &c.args {
                        match a {
                            // A by-value parameter that is only pushed as
                            // an argument is pushed from the frame, not
                            // loaded at entry: AST_$SET_ATTRIBUTE 0xE053DA
                            // `move.l (0xe,A6),-(SP)`; 0xE00EBE and
                            // 0xE00EDE push (8,A6) twice (codegen-patterns
                            // 12.2).
                            Arg::Value {
                                value:
                                    Expr::Load {
                                        addr: Addr::Frame(_),
                                        ..
                                    },
                                ..
                            } => {}
                            Arg::Value { value, .. } => self.expr(value),
                            Arg::Address { addr, .. } => self.address(addr),
                        }
                    }
                }
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
    regs: Regs,
    /// The statement being selected, for diagnostics.
    span: Span,
    /// By-value parameters loaded at entry: frame offset and the register
    /// that holds the value for the whole body.
    pinned: Vec<(i32, Reg)>,
    /// The statement being selected may use only the pinned registers
    /// (see [`select_body`]).
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
}

impl Selector {
    fn new(span: Span) -> Selector {
        Selector {
            out: Vec::new(),
            regs: Regs::new(),
            span,
            pinned: Vec::new(),
            multi: false,
            multi_body: false,
            next_label: 1,
            placed: Vec::new(),
            epilogue: None,
            before_bra: Vec::new(),
        }
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
            let rest = &body[i + 1..];
            // In a body of several statements a statement may use only the
            // registers loaded at entry (see `select_body`), except a
            // simple statement right after a call: the call leaves no value
            // in a scratch register, and the next statement takes the
            // lowest free ones (0xE00ECC..0xE00ED2, A0 and D0 after the
            // `jsr` at 0xE00EC2).
            self.multi =
                self.multi_body && !(after_call && self.pinned.is_empty() && is_simple(st));
            // What follows this statement up to the epilogue.
            let next = Tail {
                end: rest.is_empty() && tail.end,
                clean: seq_clean(rest, tail.clean),
            };
            match &st.kind {
                StmtKind::SetResult(e) => self.set_result(f, e)?,
                StmtKind::Store { addr, value } => self.store(addr, value)?,
                StmtKind::Copy { dst, src, bytes } => self.copy(dst, src, *bytes)?,
                StmtKind::Call(c) => self.call(c, next.clean)?,
                StmtKind::Fill {
                    start,
                    count,
                    value,
                } => self.fill(start, *count, value)?,
                StmtKind::If { cond, then_, else_ } => self.if_(f, cond, then_, else_, next)?,
                StmtKind::Repeat { body, until } => self.repeat(f, body, until)?,
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
                        let l = match self.epilogue {
                            Some(l) => l,
                            None => {
                                let l = self.new_label();
                                self.epilogue = Some(l);
                                l
                            }
                        };
                        self.jump(Cond::T, l);
                    }
                }
            }
            after_call = matches!(st.kind, StmtKind::Call(_));
            debug_assert!(
                self.regs
                    .live_scratch()
                    .iter()
                    .all(|r| self.pinned.iter().any(|&(_, p)| p == r)),
                "a statement left a register busy"
            );
        }
        Ok(())
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
        if !matches!(then_.last().map(|s| &s.kind), Some(StmtKind::Return)) {
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
        // the body is popped (codegen-patterns 1.1).
        self.seq(
            f,
            body,
            Tail {
                end: false,
                clean: false,
            },
        )?;
        (self.span, self.multi) = (span, multi);
        self.branch(until, false, head)
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
                // 37).
                let Expr::Load { addr, .. } = e else {
                    return Err(unsupported(
                        self.span,
                        "a condition that is a computed boolean value is",
                    ));
                };
                let ea = self.operand(addr)?;
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
    /// into at entry, else the memory operand (an address register it
    /// needs stays allocated until [`Self::free_ea`]).
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
        self.free_ea(&ea);
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
        let Expr::Load { addr, .. } = lhs else {
            return Err(unsupported(
                self.span,
                "a comparison whose left operand is not a variable is",
            ));
        };
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
                if let Ea::Reg(Reg::A(_)) = ea {
                    // `cmpa.w #0,An` (151 sites, e.g. 0xE0243C); `entry_loads`
                    // never pins a value that is compared.
                    return Err(unsupported(self.span, "testing an address register is"));
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
                self.free_ea(&ea);
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
                self.free_ea(&src);
                Ok(rel_cond(op, signed))
            }
            _ => Err(unsupported(
                self.span,
                "a comparison with this kind of value is",
            )),
        }
    }

    fn alloc(&mut self, class: RegClass) -> Res<Reg> {
        if self.multi {
            return Err(unsupported(
                self.span,
                "in a body of more than one statement, a statement that needs a register \
                 of its own is",
            ));
        }
        self.regs.alloc_scratch(class).ok_or_else(|| {
            unsupported(
                self.span,
                "an expression needing more than two scratch registers of a kind is",
            )
        })
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

    /// A callee-saved register of `class` for a value that must survive
    /// a call (`regs.rs`, [`Regs::alloc_held`]); it joins the prologue's
    /// save set.  Free it with [`Self::free`].
    pub fn hold(&mut self, class: RegClass) -> Res<Reg> {
        self.regs.alloc_held(class).ok_or_else(|| {
            unsupported(
                self.span,
                "holding more values than there are callee-saved registers is",
            )
        })
    }

    /// Release a register from [`Self::hold`] or the scratch pools.
    pub fn free(&mut self, r: Reg) {
        self.regs.free(r);
    }

    fn free_d(&mut self, r: u8) {
        self.regs.free(Reg::D(r));
    }

    fn free_a(&mut self, r: u8) {
        self.regs.free(Reg::A(r));
    }

    /// Release the register an effective address uses.  A5, A6 and A7
    /// are bases the allocator never hands out (`regs.rs`), so they are
    /// not released; nor are registers pinned by [`Self::entry_loads`].
    fn free_ea(&mut self, ea: &Ea) {
        match ea {
            Ea::Ind(a) | Ea::Disp(_, a)
                if Reg::A(*a) != DATA_BASE
                    && *a != FP
                    && *a != SP
                    && !self.is_pinned(Reg::A(*a)) =>
            {
                self.free_a(*a)
            }
            Ea::Reg(r) if !self.is_pinned(*r) => self.regs.free(*r),
            _ => {}
        }
    }

    /// The register a by-value parameter at frame offset `o` was loaded
    /// into at entry, if it was.
    fn pinned(&self, o: i32) -> Option<Reg> {
        self.pinned.iter().find(|&&(p, _)| p == o).map(|&(_, r)| r)
    }

    fn is_pinned(&self, r: Reg) -> bool {
        self.pinned.iter().any(|&(_, p)| p == r)
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
    /// free scratch register of the class, data values in D registers,
    /// pointers the body dereferences in A registers.  A pointer also used
    /// as data, more than two parameters of a class (the original then
    /// takes callee-saved registers even without calls: MST_$VA_TO_SEGNO
    /// 0xE0DCD6, ROUTE_$FIND_PORT 0xE15B06) and parameters that must
    /// survive a call (PROC1_$SET_TYPE 0xE152F2) are not supported yet.
    ///
    /// **By-address parameters** (the slot holds the argument's address)
    /// that the body reaches through more than once are loaded the same
    /// way, in parameter order with the by-value ones, into an address
    /// register that holds the address for the whole body (bead
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
    /// it is needed, into a register no earlier load used (IIC_$RECEIVE
    /// 0xE70AC0..0xE70AD0 A0, A1, A2; PROC2_$ALIGN_CTL 0xE41D82): not
    /// supported yet in a body of several statements.
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
            let r = self.regs.alloc_scratch(class).ok_or_else(|| {
                unsupported(
                    f.span,
                    "loading more than two parameters of a register class at entry is",
                )
            })?;
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
        self.regs.clobber(DATA_BASE);
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
    /// Address registers: source first, then destination, the lowest free
    /// scratch register each (A0, A1).
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
            self.free_d(count);
        }
        if bytes % 4 == 2 {
            self.out.push(mv(Size::W));
        }
        self.free_a(from);
        self.free_a(to);
        Ok(())
    }

    /// The address of a block in a fresh scratch address register.
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
        if matches!(addr, Addr::Frame(o) if *o > 0) {
            return Err(unsupported(self.span, "assigning to a parameter is"));
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
            } if *o > 0 => match self.pinned(*o) {
                Some(r) => Src::Ea(Ea::Reg(r)),
                None => {
                    return Err(unsupported(
                        self.span,
                        "storing a parameter that was not loaded at entry is",
                    ))
                }
            },
            Expr::Load { addr: a, .. } => Src::Ea(self.ea_of(a)?),
            e => Src::Ea(Ea::Reg(Reg::D(self.eval_d(e)?))),
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
        if let Src::Ea(ea) = &src {
            self.free_ea(ea);
        }
        self.free_ea(&dst);
        Ok(())
    }

    /// Whether forming `addr` takes a register that is not already
    /// holding the base.
    fn addr_needs_reg(&self, addr: &Addr) -> bool {
        match addr {
            Addr::Frame(_) | Addr::Static(_) | Addr::Symbol { .. } => false,
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
            Expr::SetMember { .. } | Expr::AddrOf(_) => true,
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
        if !self.regs.live_scratch().is_empty() {
            return Err(unsupported(
                self.span,
                "a call while a scratch register holds a value is",
            ));
        }
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
            self.push(a)?;
            k -= 1;
        }
        self.out.push(match &c.callee {
            Callee::External(sym) => Insn::Jsr(Ea::AbsL(sym.clone(), 0)),
            Callee::Local(sym) => Insn::Bsr {
                target: sym.clone(),
                short: true,
            },
        });
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
                            _ => self.ea_of(addr)?,
                        };
                        self.out.push(Insn::Move {
                            size: size_of(ty.width),
                            src: src.clone(),
                            dst,
                        });
                        self.free_ea(&src);
                    }
                    Expr::AddrOf(Addr::Symbol { name, offset: 0 }) => self.out.push(Insn::Move {
                        size: Size::L,
                        src: Ea::ImmSym(name.clone()),
                        dst,
                    }),
                    _ => return Err(unsupported(self.span, "this argument value is")),
                }
            }
            Arg::Address { addr, slot: 4 } => match addr {
                Addr::Frame(_) | Addr::Static(_) => {
                    let ea = self.ea_of(addr)?;
                    self.out.push(Insn::Pea(ea));
                }
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
        self.free_d(d);
        self.free_a(a);
        Ok(())
    }

    /// `f := e` as the last statement: the value goes straight to the
    /// result register (PROC1_$TST_LOCK 0xE148E0 `move.b D1,D0`; the
    /// frame slot of the result variable is not used).
    fn set_result(&mut self, f: &Function, e: &Expr) -> Res<()> {
        let res = f.result.expect("SetResult in a function");
        match res.class {
            ResultClass::Data => {}
            _ => return Err(unsupported(self.span, "address-register results are")),
        }
        let r = self.eval_d(e)?;
        if r != 0 {
            // The value is moved at the result's width; the upper bytes
            // of D0 are left as they are (codegen-patterns 3: 0xE148E0).
            self.out.push(Insn::Move {
                size: size_of(res.ty.width),
                src: Ea::Reg(Reg::D(r)),
                dst: Ea::Reg(Reg::D(0)),
            });
        }
        self.free_d(r);
        Ok(())
    }

    /// Evaluate `e` into a scratch data register.
    fn eval_d(&mut self, e: &Expr) -> Res<u8> {
        match e {
            // A value held in a register for the whole body is copied before
            // it is used as a temporary (BAT_$GET_BAT_STEP 0xE3BB68 `move.w
            // D0,D1`); the copy rule has not been settled.
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if self.pinned(*o).is_some() => Err(unsupported(
                self.span,
                "computing with a parameter loaded at entry is",
            )),
            // Load a scalar: `move.<w> <ea>,Dn` (0xE148D4 `move.w
            // (8,A6),D0`; 0xE148D8 `move.l (0x40,A0),D1`).  The address is
            // formed first, its register released, then Dn allocated.
            Expr::Load { addr, ty } => {
                let ea = self.ea_of(addr)?;
                self.free_ea(&ea);
                let r = self.alloc_d()?;
                self.out.push(Insn::Move {
                    size: size_of(ty.width),
                    src: ea,
                    dst: Ea::Reg(Reg::D(r)),
                });
                Ok(r)
            }
            Expr::SetMember {
                elem,
                set,
                set_width: Width::Long,
            } => self.set_member_long(elem, set),
            Expr::SetMember { .. } => Err(unsupported(
                self.span,
                "`in` on a set that is not one longword is",
            )),
            // `moveq` / `clr` / `move #imm` choice not catalogued yet.
            Expr::Const { .. } => Err(unsupported(self.span, "loading a constant is")),
            Expr::AddrOf(_) => Err(unsupported(
                self.span,
                "an address (`addr`) as a value other than an argument is",
            )),
        }
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
        let bit = self.eval_d(elem)?;
        self.free_ea(&ea);
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
        self.free_d(bit);
        self.out.push(Insn::Scc {
            cond: Cond::Ne,
            dst: Ea::Reg(Reg::D(r)),
        });
        Ok(r)
    }

    /// Evaluate a pointer into a scratch address register, or give the
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
        }
        match e {
            // `movea.l <ea>,An`: 0xE148CE `movea.l abs.l,A0` (an extern
            // pointer variable); LOG_$READ 0xE17810 `movea.l (0xc,A6),A0`
            // (a by-address parameter).
            Expr::Load {
                addr: Addr::Frame(o),
                ..
            } if self.multi && *o > 0 => Err(unsupported(
                self.span,
                "in a body of several statements, a pointer parameter or by-address parameter \
                 that is used once (loaded where needed into a fresh register: IIC_$RECEIVE \
                 0xE70AC0, codegen-patterns 12.2) is",
            )),
            Expr::Load {
                addr,
                ty: Scalar {
                    width: Width::Long, ..
                },
            } => {
                let ea = self.ea_of(addr)?;
                // TODO(codegen): reusing the base register for a pointer
                // chain (`movea.l (d,A0),A0`) follows from releasing it
                // first; no chain has been checked against the image yet.
                self.free_ea(&ea);
                let a = self.alloc_a()?;
                self.out.push(Insn::Movea {
                    size: Size::L,
                    src: ea,
                    dst: a,
                });
                Ok(a)
            }
            _ => Err(unsupported(self.span, "this pointer expression is")),
        }
    }

    /// The effective address of `addr`; any address register it needs is
    /// loaded now and stays allocated until [`Self::free_ea`].
    fn ea_of(&mut self, addr: &Addr) -> Res<Ea> {
        match addr {
            // Frame objects and arguments: `(d,A6)` (0xE148D4).
            Addr::Frame(o) => Ok(Ea::Disp(*o, FP)),
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
                Ok(if *o == 0 {
                    Ea::Ind(DATA_BASE_A)
                } else {
                    Ea::Disp(*o, DATA_BASE_A)
                })
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
            value_params: Vec::new(),
            address_params: Vec::new(),
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

    #[test]
    fn empty_procedure() {
        assert_eq!(
            text(&func(0, None, vec![])),
            ["link.w\t%fp,#0", "unlk\t%fp", "rts"]
        );
    }

    #[test]
    fn link_rounds_frame_up_to_4() {
        for (raw, disp) in [(1, "#-4"), (4, "#-4"), (5, "#-8"), (12, "#-0xc")] {
            assert_eq!(
                text(&func(raw, None, vec![]))[0],
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

    #[test]
    fn constants_are_rejected() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: Scalar::signed(Width::Word),
            }),
            vec![StmtKind::SetResult(Expr::Const {
                value: 0,
                ty: Scalar::signed(Width::Word),
            })],
        );
        let d = select(&f).unwrap_err();
        assert_eq!(d.message, "loading a constant is not supported yet");
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let d = select(&func(0x8001, None, vec![])).unwrap_err();
        assert!(d.message.ends_with("not supported yet"), "{}", d.message);
    }

    /// Phase 2 puts the save the body needs between `link` and the body,
    /// and the restore from the frame before `unlk`.
    #[test]
    fn two_phase_frame_around_a_body_with_held_registers() {
        let f = func(16, None, vec![]);
        // A synthetic body: a value held across a (pretend) call.
        let mut s = Selector::new(span());
        let d = s.hold(RegClass::Data).unwrap();
        let a = s.hold(RegClass::Addr).unwrap();
        let d2 = s.hold(RegClass::Data).unwrap();
        assert_eq!((d, a, d2), (Reg::D(2), Reg::A(2), Reg::D(3)));
        s.out.push(Insn::Move {
            size: Size::L,
            src: Ea::Disp(0xc, FP),
            dst: Ea::Reg(d),
        });
        s.free(d);
        s.free(a);
        s.free(d2);
        let body = Body {
            insns: s.out,
            saves: s.regs.to_save(),
        };
        let text: Vec<String> = wrap(&f, body)
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

    /// Effective addresses based on A5, A6 or A7 release nothing (a
    /// module-data operand `(d,A5)` must not free a register the
    /// allocator never gave out).
    #[test]
    fn fixed_base_registers_are_not_released() {
        let mut s = Selector::new(span());
        for a in [5, FP, SP] {
            s.free_ea(&Ea::Disp(4, a));
            s.free_ea(&Ea::Ind(a));
        }
        assert!(s.regs.live_scratch().is_empty());
        let a = s.alloc_a().unwrap();
        s.free_ea(&Ea::Disp(4, a));
        assert!(!s.regs.is_busy(Reg::A(a)));
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

    #[test]
    fn multi_statement_functions_are_rejected() {
        let f = func(
            0,
            Some(ResultSpec {
                class: ResultClass::Data,
                ty: word(),
            }),
            vec![
                store(through(8, 0), konst(0, word())),
                StmtKind::SetResult(param(12, word())),
            ],
        );
        assert_eq!(
            err(&f),
            "a function body of more than one statement is not supported yet"
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
        // Not adjacent (a gap), or too big together: accepted.
        for (o, ty) in [(4, word()), (2, Scalar::signed(Width::Long))] {
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

    #[test]
    fn too_many_entry_loads_are_rejected() {
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
            err(&f),
            "loading more than two parameters of a register class at entry is not supported yet"
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

    /// In a body of several statements, a statement may use only the
    /// registers loaded at entry.
    #[test]
    fn statement_registers_in_multi_statement_bodies_are_rejected() {
        let ext = load(
            Addr::Symbol {
                name: "P".into(),
                offset: 0,
            },
            Scalar::POINTER,
        );
        let f = func(
            0,
            None,
            vec![
                store(
                    Addr::Indirect {
                        base: Box::new(ext),
                        offset: 0,
                    },
                    konst(0, word()),
                ),
                store(through(8, 4), konst(0, word())),
            ],
        );
        assert!(err(&f).starts_with("in a body of more than one statement"));
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
        let addr = |a: Addr| Arg::Address { addr: a, slot: 4 };
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

    /// In a body of several statements, a statement gets registers of its
    /// own only right after a call (0xE00ECC after the `jsr` at
    /// 0xE00EC2); a call is refused where parameters are loaded at entry.
    #[test]
    fn registers_after_calls() {
        let fill = || StmtKind::Fill {
            start: sym("BUF"),
            count: 4,
            value: konst(0, long()),
        };
        let f = proc_(0, vec![fill(), ext("P", vec![], 0)]);
        assert!(err(&f).starts_with("in a body of more than one statement"));
        let f = proc_(0, vec![ext("P", vec![], 0), fill()]);
        assert!(select(&f).is_ok());
        let f = proc_(0, vec![ext("P", vec![], 0), fill(), fill()]);
        assert!(err(&f).starts_with("in a body of more than one statement"));
        let mut f = proc_(
            0,
            vec![store(through(8, 0), konst(0, long())), ext("P", vec![], 0)],
        );
        vparams(&mut f, &[(8, Scalar::POINTER)]);
        assert!(err(&f).starts_with("a call in a body that loads parameters at entry"));
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
}
