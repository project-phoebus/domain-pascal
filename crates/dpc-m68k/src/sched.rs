//! Instruction scheduling inside basic blocks (codegen-patterns 12.4
//! R9-R12; bead domain-pascal-g5q.8).
//!
//! The original reorders instructions inside a basic block, after the
//! registers are chosen (R9 Verified that it happens; R12 supported).
//! This pass runs on a routine body after allocation (`isel.rs`,
//! `select_body_with`): it splits the body into runs of instructions
//! between labels, branches and barriers, builds the dependence graph of
//! each run, and lists the run again under a [`Schedule`] policy.
//!
//! **Dependences** (a later instruction is never placed before an earlier
//! one it depends on; the graph is conservative):
//!
//! - registers: read after write, write after read, write after write,
//!   over D0-D7 and A0-A7 (A7 orders pushes, pads and pops).  A byte or
//!   word write to a data register keeps the rest of it, so it also reads
//!   it (`clr.l D1; move.w D0,D1`).
//! - memory: a store other than a push is ordered against every
//!   instruction of its run (R10: a load through a pointer is not hoisted
//!   above the store before it, 0xE3F2DE/0xE3F2E2; a call's pad stays
//!   below the previous statement's store, 0xE00C12/0xE00C16).  Among
//!   loads and pushes, a push is ordered against every memory access (an
//!   argument list is not interleaved with later loads: 0xE00D26..
//!   0xE00D2A); `alias` would let distinct frame slots, module-data
//!   offsets, absolute ranges and fields through one unchanged address
//!   register pass each other, which the store rule makes moot today.
//!   `pea`/`lea` of a variable count as reading it (TERM_$SEND_KBD_STRING:
//!   a frame temporary's address pushed after the store to it).
//! - condition codes: a reader (`s<cc>`) after the last setter before it,
//!   that setter after every earlier setter, and later setters after the
//!   reader.  Where the flags are live at the end of a run (liveness over
//!   the body's control flow: a `b<cc>` after it, or a join that tests
//!   them, PARITY_$CHK_IO 0xE0B1AC `beq` after `moveq`/`clr.w`), the last
//!   setter of the run stays last.
//! - barriers (R10, codegen-patterns 10): calls (`jsr`, `bsr`), the
//!   status-register instructions (`disable`, `enable`, `set_sr`), writes
//!   of A5/A6 (the data base load), a call's pop (0xE1AFC8 `addq.w #8,SP;
//!   subq.l #2,SP; movea.l (8,A2),A0`) and a nested routine's static link
//!   `movea.l An,A1` before its `bsr` (all 34 calls that pass one in A1)
//!   end a run; nothing moves across them.
//! - volatile references (ref 3.15.1-3.15.3): two instructions that
//!   reference volatile objects (marked by the selector) keep their
//!   order, so that every reference is made as written; other loads may
//!   still pass them (WIN_'s module-local 0xE190C8 reads TIME_$CLOCKH
//!   among the entry loads; whether the original ordered more is not
//!   seen).
//! - units (R10): an unrolled `(An)+` block copy is one node ([`Unit`],
//!   marked by the selector).  So is a zero-extended argument, `clr;
//!   move; push` (acl_$prim_create_internal 0xE4535A..0xE45366: the `clr`
//!   after the pad and the push before it; `isel.rs`,
//!   `zero_extension_unit`).  A shift-and-add multiplication is not: the
//!   ×56 at 0xE1AA50..0xE1AA60 has a `movea.l #` before its last `add`.
//!
//! **Policy** (R11, Uncertain): see [`Schedule`].

use crate::insn::{Cond, Ea, Insn, Reg, RegSet, Size};

/// Which scheduling policy to apply.  The default is R11 with a window
/// of 3 (codegen-patterns 12.4 R11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// The selector's order (no reordering).
    Off,
    /// Codegen-patterns 12.4 R11, the candidate list rule: top-down, take
    /// the first instruction in the selector's order whose predecessors
    /// are placed and that does not read a register the instruction just
    /// placed wrote; if there is none, the first whose predecessors are
    /// placed.
    R11 {
        /// Only the first `window` instructions not yet placed, in the
        /// selector's order, are candidates (0: all of them).
        window: u8,
    },
}

impl Default for Schedule {
    fn default() -> Schedule {
        Schedule::R11 { window: 3 }
    }
}

/// A range `start..end` of a body's instructions that is scheduled as one
/// node (the selector's indices, before any reordering).
pub type Unit = (usize, usize);

/// A memory location an operand may touch.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Loc {
    /// `(d,A6)`: bytes `lo..hi` of the frame (`None`: unknown, indexed).
    Frame(Option<(i32, i32)>),
    /// `(d,A5)`: bytes of the module data area.
    Static(Option<(i32, i32)>),
    /// `(sym+off).l` / `(n).w`: bytes of an absolute symbol.
    Abs(String, i32, i32),
    /// The stack: pushes, `(SP)`.
    Stack,
    /// Through address register `An` (A0-A4), bytes `lo..hi` from it
    /// (`None`: indexed or stepped).
    Ptr(u8, Option<(i32, i32)>),
    /// A constant in the code section: never stored to.
    Const,
}

fn overlap(a: Option<(i32, i32)>, b: Option<(i32, i32)>) -> bool {
    match (a, b) {
        (Some((a0, a1)), Some((b0, b1))) => a0 < b1 && b0 < a1,
        _ => true,
    }
}

fn bytes(size: Size) -> i32 {
    match size {
        Size::B => 1,
        Size::W => 2,
        Size::L => 4,
    }
}

const SP: u8 = 7;

/// The location `ea` names when it addresses memory with an access of
/// `size`, or `None` for a register or an immediate.
fn loc(ea: &Ea, size: Size) -> Option<Loc> {
    let n = bytes(size);
    let r = |d: i32| Some((d, d + n));
    Some(match ea {
        Ea::Reg(_) | Ea::Imm(_) | Ea::ImmSym(_) => return None,
        Ea::Ind(6) => Loc::Frame(r(0)),
        Ea::Disp(d, 6) => Loc::Frame(r(*d)),
        Ea::Idx(_, 6, ..) => Loc::Frame(None),
        Ea::Ind(5) => Loc::Static(r(0)),
        Ea::Disp(d, 5) => Loc::Static(r(*d)),
        Ea::Idx(_, 5, ..) => Loc::Static(None),
        Ea::Ind(SP) | Ea::Disp(_, SP) | Ea::Idx(_, SP, ..) | Ea::PreDec(SP) | Ea::PostInc(SP) => {
            Loc::Stack
        }
        Ea::Ind(a) => Loc::Ptr(*a, r(0)),
        Ea::Disp(d, a) => Loc::Ptr(*a, r(*d)),
        Ea::Idx(_, a, ..) | Ea::PreDec(a) | Ea::PostInc(a) => Loc::Ptr(*a, None),
        Ea::AbsL(s, o) => Loc::Abs(s.clone(), *o, *o + n),
        Ea::AbsW(v) => Loc::Abs(String::new(), *v, *v + n),
        Ea::Pool(_) | Ea::PoolLabel(_) => Loc::Const,
    })
}

/// What one instruction does, for the dependence graph.
#[derive(Clone, Debug, Default)]
struct Fx {
    uses: RegSet,
    defs: RegSet,
    loads: Vec<Loc>,
    stores: Vec<Loc>,
    cc_set: bool,
    cc_use: bool,
    /// Nothing moves across it: a call, an SR instruction, a write of
    /// A5/A6.
    barrier: bool,
    /// Ends a run without being part of one: labels, branches, `dbf`,
    /// `case` tables, alignment and data.
    boundary: bool,
    /// References a volatile object (marked by the selector).
    volatile: bool,
}

impl Fx {
    fn use_(&mut self, r: Reg) {
        self.uses.insert(r);
    }

    fn def(&mut self, r: Reg) {
        self.defs.insert(r);
    }

    /// The registers an operand reads to form its address; `-(An)` and
    /// `(An)+` also write An.
    fn address(&mut self, ea: &Ea) {
        match ea {
            Ea::Ind(a) | Ea::Disp(_, a) => self.use_(Reg::A(*a)),
            Ea::Idx(_, a, x, _) => {
                self.use_(Reg::A(*a));
                self.use_(Reg::D(*x));
            }
            Ea::PreDec(a) | Ea::PostInc(a) => {
                self.use_(Reg::A(*a));
                self.def(Reg::A(*a));
            }
            _ => {}
        }
    }

    /// `pea`/`lea` of a variable: its address escapes, which is taken as
    /// a read of it (the address of a frame temporary is pushed after the
    /// store to it: TERM_$SEND_KBD_STRING, codegen-patterns 1.5).
    /// Pointer arithmetic (`lea (d,An),Am`) is not a read.
    fn address_taken(&mut self, ea: &Ea) {
        match loc(ea, Size::L) {
            Some(Loc::Ptr(..) | Loc::Stack | Loc::Const) | None => {}
            Some(l) => self.loads.push(match l {
                Loc::Frame(_) => Loc::Frame(None),
                Loc::Static(_) => Loc::Static(None),
                other => other,
            }),
        }
    }

    /// A source operand of `size`.
    fn src(&mut self, ea: &Ea, size: Size) {
        if let Ea::Reg(r) = ea {
            self.use_(*r);
            return;
        }
        self.address(ea);
        if let Some(l) = loc(ea, size) {
            self.loads.push(l);
        }
    }

    /// A destination operand of `size` that is only written.  A byte or
    /// word write to a data register keeps its other bits: a read too.
    fn dst(&mut self, ea: &Ea, size: Size) {
        if let Ea::Reg(r) = ea {
            if matches!(r, Reg::D(_)) && size != Size::L {
                self.use_(*r);
            }
            self.def(*r);
            return;
        }
        self.address(ea);
        if let Some(l) = loc(ea, size) {
            self.stores.push(l);
        }
    }

    /// A destination operand that is read and written.
    fn rmw(&mut self, ea: &Ea, size: Size) {
        if let Ea::Reg(r) = ea {
            self.use_(*r);
            self.def(*r);
            return;
        }
        self.address(ea);
        if let Some(l) = loc(ea, size) {
            self.loads.push(l.clone());
            self.stores.push(l);
        }
    }
}

fn is_reg_a(ea: &Ea, n: u8) -> bool {
    matches!(ea, Ea::Reg(Reg::A(a)) if *a == n)
}

fn fx(insn: &Insn) -> Fx {
    let mut f = Fx::default();
    match insn {
        Insn::Label(_)
        | Insn::Branch { .. }
        | Insn::Dbf { .. }
        | Insn::TableIndex { .. }
        | Insn::TableJump { .. }
        | Insn::TableEntry { .. }
        | Insn::Align4
        | Insn::PoolLabel(_)
        | Insn::Words(_)
        | Insn::Encoded { .. }
        | Insn::Rts => f.boundary = true,
        Insn::Link { .. }
        | Insn::Unlk
        | Insn::MovemSave { .. }
        | Insn::MovemRestore { .. }
        | Insn::Jsr(_)
        | Insn::Bsr { .. }
        | Insn::SrImm { .. }
        | Insn::MoveToSr { .. }
        | Insn::MoveFromSr { .. } => f.barrier = true,
        Insn::Pea(ea) => {
            f.address(ea);
            f.address_taken(ea);
            f.use_(Reg::A(SP));
            f.def(Reg::A(SP));
            f.stores.push(Loc::Stack);
        }
        Insn::Move { size, src, dst } => {
            f.src(src, *size);
            f.dst(dst, *size);
            f.cc_set = true;
        }
        Insn::Movea { size, src, dst } => {
            f.src(src, *size);
            f.def(Reg::A(*dst));
        }
        Insn::Lea { src, dst } => {
            f.address(src);
            f.address_taken(src);
            f.def(Reg::A(*dst));
        }
        Insn::Moveq { dst, .. } => {
            f.def(Reg::D(*dst));
            f.cc_set = true;
        }
        Insn::Addq { size, dst, .. } | Insn::Subq { size, dst, .. } => {
            f.rmw(dst, *size);
            // `addq`/`subq` to an address register leave the flags.
            f.cc_set = !matches!(dst, Ea::Reg(Reg::A(_)));
        }
        Insn::Clr { size, dst } => {
            f.dst(dst, *size);
            f.cc_set = true;
        }
        // `btst` sets Z only and keeps N, V and C: it reads the flags it
        // does not set, so the setter before it stays before it (only
        // Z is tested after it today, `sne`/`beq`/`bne`; MC68000 PRM).
        Insn::Btst { bit, dst } => {
            f.use_(Reg::D(*bit));
            f.src(dst, Size::B);
            f.cc_set = true;
            f.cc_use = true;
        }
        Insn::BtstImm { dst, .. } => {
            f.src(dst, Size::B);
            f.cc_set = true;
            f.cc_use = true;
        }
        Insn::Scc { cond, dst } => {
            f.dst(dst, Size::B);
            f.cc_use = *cond != Cond::T;
        }
        Insn::Tst { size, dst } | Insn::Cmpi { size, dst, .. } => {
            f.src(dst, *size);
            f.cc_set = true;
        }
        Insn::CmpaZero(a) => {
            f.use_(Reg::A(*a));
            f.cc_set = true;
        }
        Insn::Cmpm { size, src, dst } => {
            f.src(&Ea::PostInc(*src), *size);
            f.src(&Ea::PostInc(*dst), *size);
            f.cc_set = true;
        }
        Insn::Cmp { size, src, dst } => {
            f.src(src, *size);
            f.use_(Reg::D(*dst));
            f.cc_set = true;
        }
        Insn::Alu { size, src, dst, .. } => {
            f.src(src, *size);
            f.rmw(&Ea::Reg(Reg::D(*dst)), *size);
            f.cc_set = true;
        }
        Insn::Mul { src, dst, .. } | Insn::Div { src, dst, .. } => {
            f.src(src, Size::W);
            f.rmw(&Ea::Reg(Reg::D(*dst)), Size::L);
            f.cc_set = true;
        }
        Insn::AluImm { size, dst, .. } | Insn::Unary { size, dst, .. } => {
            f.rmw(dst, *size);
            f.cc_set = true;
        }
        // `bset`/`bclr` set Z only, like `btst`.
        Insn::BitImm { dst, .. } => {
            f.rmw(dst, Size::B);
            f.cc_set = true;
            f.cc_use = true;
        }
        Insn::Bit { bit, dst, .. } => {
            f.use_(Reg::D(*bit));
            let size = if matches!(dst, Ea::Reg(_)) {
                Size::L
            } else {
                Size::B
            };
            f.rmw(dst, size);
            f.cc_set = true;
            f.cc_use = true;
        }
        Insn::AluToMem { size, src, dst, .. } => {
            f.use_(Reg::D(*src));
            f.rmw(dst, *size);
            f.cc_set = true;
        }
        Insn::Ext { dst, .. } | Insn::Swap(dst) => {
            f.rmw(&Ea::Reg(Reg::D(*dst)), Size::L);
            f.cc_set = true;
        }
        Insn::Shift { count, dst, .. } => {
            if let crate::insn::ShiftCount::Reg(r) = count {
                f.use_(Reg::D(*r));
            }
            f.rmw(&Ea::Reg(Reg::D(*dst)), Size::L);
            f.cc_set = true;
        }
    }
    // Writing the data base or the frame pointer is the prologue's or a
    // routine-level setup: a barrier (`lea <area>,A5` right after the
    // save, codegen-patterns 4).
    if f.defs.contains(Reg::A(5)) || f.defs.contains(Reg::A(6)) {
        f.barrier = true;
    }
    f
}

/// Whether accesses `a` and `b` may touch the same bytes.  `same_base`:
/// the address register both [`Loc::Ptr`] go through holds the same value
/// at both (it is not written between them).
fn alias(a: &Loc, b: &Loc, same_base: bool) -> bool {
    use Loc::*;
    match (a, b) {
        (Const, _) | (_, Const) => false,
        // Pushes are ordered against every other memory access: an
        // argument list is not interleaved with later loads (0xE00D26..
        // 0xE00D2A, a load through A4 after the pushes before it).
        (Stack, _) | (_, Stack) => true,
        (Ptr(x, rx), Ptr(y, ry)) => !(x == y && same_base && !overlap(*rx, *ry)),
        (Ptr(..), _) | (_, Ptr(..)) => true,
        (Frame(x), Frame(y)) => overlap(*x, *y),
        (Frame(_), _) | (_, Frame(_)) => false,
        (Static(x), Static(y)) => overlap(*x, *y),
        // Module data may also be named by an absolute symbol.
        (Static(_), Abs(..)) | (Abs(..), Static(_)) => true,
        (Abs(s, a0, a1), Abs(t, b0, b1)) => s != t || (a0 < b1 && b0 < a1),
    }
}

/// Whether instruction `j` (later) depends on instruction `i` (earlier)
/// through registers or memory.  `between_defs`: the
/// registers written by the instructions strictly between them.
fn depends(i: &Fx, j: &Fx, between_defs: RegSet) -> bool {
    if i.barrier || j.barrier || (i.volatile && j.volatile) {
        return true;
    }
    if !i.defs.intersection(j.uses).is_empty()
        || !i.uses.intersection(j.defs).is_empty()
        || !i.defs.intersection(j.defs).is_empty()
    {
        return true;
    }
    let same = |a: &Loc, b: &Loc| match (a, b) {
        (Loc::Ptr(x, _), Loc::Ptr(y, _)) if x == y => !between_defs.contains(Reg::A(*x)),
        _ => false,
    };
    let mem = i.stores.iter().any(|s| {
        j.loads
            .iter()
            .chain(&j.stores)
            .any(|l| alias(s, l, same(s, l)))
    }) || j
        .stores
        .iter()
        .any(|s| i.loads.iter().any(|l| alias(s, l, same(s, l))));
    if mem {
        return true;
    }
    // A store other than a push is ordered against everything (R10: a
    // call's pad stays below the previous statement's store, 0xE00C12/
    // 0xE00C16; a load through a pointer is not hoisted above the store
    // before it, 0xE3F2DE/0xE3F2E2, 0xE16B20/0xE16B26).
    i.stores.iter().chain(&j.stores).any(|s| *s != Loc::Stack)
}

/// How an instruction treats the condition codes: (reads them, sets
/// them).  A call clobbers them; `andi`/`ori` to SR read and write them;
/// `move.w SR,Dn` reads them.
fn cc_effect(insn: &Insn) -> (bool, bool) {
    match insn {
        Insn::Branch { cond, .. } => (*cond != Cond::T, false),
        Insn::Dbf { until, .. } => (until.is_some(), false),
        Insn::Jsr(_) | Insn::Bsr { .. } | Insn::MoveToSr { .. } => (false, true),
        Insn::SrImm { .. } => (true, true),
        Insn::MoveFromSr { .. } => (true, false),
        Insn::TableIndex { .. } => (false, true),
        x => {
            let f = fx(x);
            (f.cc_use, f.cc_set)
        }
    }
}

/// Whether the condition codes are live after each instruction of `body`
/// (backward over the body's control flow, `regs::successors`; nothing is
/// live at the end of the body).
fn cc_live_out(body: &[Insn]) -> Vec<bool> {
    let succ = crate::regs::successors(body);
    let eff: Vec<(bool, bool)> = body.iter().map(cc_effect).collect();
    let mut live_in = vec![false; body.len()];
    let mut live_out = vec![false; body.len()];
    loop {
        let mut changed = false;
        for i in (0..body.len()).rev() {
            let out = succ[i].iter().any(|s| s.is_some_and(|j| live_in[j]));
            let inn = eff[i].0 || (out && !eff[i].1);
            if out != live_out[i] || inn != live_in[i] {
                live_out[i] = out;
                live_in[i] = inn;
                changed = true;
            }
        }
        if !changed {
            return live_out;
        }
    }
}

/// Schedule `body` (a routine body after register allocation) under
/// `policy`.  `units` are ranges of `body` that move as one node;
/// `volatile` the indices of the instructions that reference volatile
/// objects.
pub fn schedule(body: &mut Vec<Insn>, units: &[Unit], volatile: &[usize], policy: Schedule) {
    if policy == Schedule::Off {
        return;
    }
    let cc_live = cc_live_out(body);
    let mut fxs: Vec<Fx> = body.iter().map(fx).collect();
    for &k in volatile {
        if let Some(f) = fxs.get_mut(k) {
            f.volatile = true;
        }
    }
    // A nested routine's static link, `movea.l An,A1`, stays right before
    // its `bsr` (all 34 calls in the image that pass one in A1, 33 of
    // them `movea.l An,A1`; codegen-patterns 12.4 R10), and a
    // call's pop right after it: nothing is scheduled between the pop and
    // the next call's pad (`addq.w #8,SP; subq.l #2,SP; movea.l
    // (8,A2),A0` at 0xE1AFC8, 0xE1AFDA, 0xE1AFEE, 0xE1BD24, 0xE1BD36).
    for k in 0..body.len().saturating_sub(1) {
        if matches!(body[k + 1], Insn::Bsr { .. })
            && matches!(
                body[k],
                Insn::Movea {
                    src: Ea::Reg(Reg::A(_)),
                    dst: 1,
                    ..
                }
            )
        {
            fxs[k].barrier = true;
        }
        // A routine pointer copied into the register a `jsr (An)` goes
        // through, `movea.l Dn,An`, stays right before the call, after the
        // pushes: every such copy in compiled code (12 sites: DISK_$REVALID
        // 0xE3DB8E `pea (A2); movea.l D2,A1; jsr (A1)`, DISK_$DO_IO
        // 0xE3DAC6, IO_$INIT 0xE329D0; codegen-patterns 1.6).  A load from
        // memory is ordered after the pushes by the memory rule.
        if let (
            Insn::Movea {
                src: Ea::Reg(_),
                dst,
                ..
            },
            Insn::Jsr(Ea::Ind(t)),
        ) = (&body[k], &body[k + 1])
        {
            if dst == t {
                fxs[k].barrier = true;
            }
        }
        let pop = match &body[k + 1] {
            Insn::Addq { dst, .. } => is_reg_a(dst, SP),
            Insn::Lea {
                src: Ea::Disp(_, SP),
                dst,
            } => *dst == SP,
            _ => false,
        };
        if pop && matches!(body[k], Insn::Jsr(_) | Insn::Bsr { .. }) {
            fxs[k + 1].barrier = true;
        }
    }
    // The node each instruction belongs to: its unit's start, or itself.
    let mut head: Vec<usize> = (0..body.len()).collect();
    for &(s, e) in units {
        for h in head.iter_mut().take(e.min(body.len())).skip(s) {
            *h = s;
        }
    }
    let mut order: Vec<usize> = Vec::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        if fxs[i].boundary || fxs[i].barrier {
            order.push(i);
            i += 1;
            continue;
        }
        let start = i;
        while i < body.len() && !fxs[i].boundary && !fxs[i].barrier {
            i += 1;
        }
        order.extend(schedule_run(&fxs, &head, start, i, cc_live[i - 1], policy));
    }
    let old = std::mem::take(body);
    let mut slots: Vec<Option<Insn>> = old.into_iter().map(Some).collect();
    *body = order
        .into_iter()
        .map(|k| slots[k].take().expect("each instruction placed once"))
        .collect();
}

/// One node of a run: a unit's instructions `first..=last` (one
/// instruction when it is not part of a unit).
struct Node {
    first: usize,
    last: usize,
    fx: Fx,
    /// The registers the node's last instruction writes (what the next
    /// instruction would wait for).
    last_defs: RegSet,
}

fn schedule_run(
    fxs: &[Fx],
    head: &[usize],
    start: usize,
    end: usize,
    cc_live: bool,
    policy: Schedule,
) -> Vec<usize> {
    // Nodes in the selector's order.  A unit cut by the run's ends is
    // split at them (it cannot be: units hold no labels or barriers).
    let mut nodes: Vec<Node> = Vec::new();
    let mut k = start;
    while k < end {
        let mut l = k;
        while l + 1 < end && head[l + 1] == head[k] && head[k] != l + 1 {
            l += 1;
        }
        let mut f = Fx::default();
        for x in &fxs[k..=l] {
            f.uses = f.uses.union(x.uses);
            f.defs = f.defs.union(x.defs);
            f.loads.extend(x.loads.iter().cloned());
            f.stores.extend(x.stores.iter().cloned());
            f.cc_set |= x.cc_set;
            f.cc_use |= x.cc_use;
            f.volatile |= x.volatile;
        }
        nodes.push(Node {
            first: k,
            last: l,
            fx: f,
            last_defs: fxs[l].defs,
        });
        k = l + 1;
    }
    let n = nodes.len();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    for j in 0..n {
        let mut between = RegSet::EMPTY;
        for i in (0..j).rev() {
            if depends(&nodes[i].fx, &nodes[j].fx, between) {
                preds[j].push(i);
            }
            between = between.union(nodes[i].fx.defs);
        }
    }
    // Condition codes.
    let setters: Vec<usize> = (0..n).filter(|&i| nodes[i].fx.cc_set).collect();
    for j in 0..n {
        if nodes[j].fx.cc_use {
            if let Some(&s) = setters.iter().rev().find(|&&s| s < j) {
                preds[j].push(s);
                for &t in setters.iter().filter(|&&t| t < s) {
                    preds[s].push(t);
                }
            }
            for &t in setters.iter().filter(|&&t| t > j) {
                preds[t].push(j);
            }
        }
    }
    // When the flags are live at the end of the run (a branch after it,
    // or a join that tests them: PARITY_$CHK_IO 0xE0B1AC `beq` after
    // `moveq`/`clr.w`), the last setter stays the last one.
    if let (true, Some(&last)) = (cc_live, setters.last()) {
        for &t in &setters[..setters.len() - 1] {
            preds[last].push(t);
        }
    }
    let mut placed = vec![false; n];
    let mut out = Vec::with_capacity(end - start);
    let mut prev: Option<usize> = None;
    let window = match policy {
        Schedule::R11 { window: 0 } | Schedule::Off => n,
        Schedule::R11 { window } => window as usize,
    };
    for _ in 0..n {
        let mut ready: Vec<usize> = (0..n)
            .filter(|&j| !placed[j])
            .take(window)
            .filter(|&j| preds[j].iter().all(|&p| placed[p]))
            .collect();
        if ready.is_empty() {
            ready = (0..n)
                .filter(|&j| !placed[j] && preds[j].iter().all(|&p| placed[p]))
                .take(1)
                .collect();
        }
        let pick = match policy {
            Schedule::Off => ready[0],
            Schedule::R11 { .. } => {
                let stalls = |j: usize| match prev {
                    Some(p) => !nodes[p].last_defs.intersection(nodes[j].fx.uses).is_empty(),
                    None => false,
                };
                ready
                    .iter()
                    .copied()
                    .find(|&j| !stalls(j))
                    .unwrap_or(ready[0])
            }
        };
        placed[pick] = true;
        prev = Some(pick);
        out.extend(nodes[pick].first..=nodes[pick].last);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insn::{AluOp, ShiftCount, ShiftOp};

    const R11: Schedule = Schedule::R11 { window: 0 };
    const R11W3: Schedule = Schedule::R11 { window: 3 };

    fn dr(n: u8) -> Ea {
        Ea::Reg(Reg::D(n))
    }
    fn mv(size: Size, src: Ea, dst: Ea) -> Insn {
        Insn::Move { size, src, dst }
    }
    fn movea(src: Ea, dst: u8) -> Insn {
        Insn::Movea {
            size: Size::L,
            src,
            dst,
        }
    }
    fn clr(size: Size, dst: Ea) -> Insn {
        Insn::Clr { size, dst }
    }
    fn alu(op: AluOp, size: Size, src: Ea, dst: u8) -> Insn {
        Insn::Alu { op, size, src, dst }
    }
    fn shift(op: ShiftOp, size: Size, n: u8, dst: u8) -> Insn {
        Insn::Shift {
            op,
            size,
            count: ShiftCount::Imm(n),
            dst,
        }
    }
    fn push_w(src: Ea) -> Insn {
        mv(Size::W, src, Ea::PreDec(7))
    }
    fn pad() -> Insn {
        Insn::Subq {
            size: Size::L,
            value: 2,
            dst: Ea::Reg(Reg::A(7)),
        }
    }

    /// Schedule `g` (the selector's order) and return the order as
    /// indices into `g`.
    fn order(g: &[Insn], units: &[Unit], policy: Schedule) -> Vec<usize> {
        let mut body = g.to_vec();
        schedule(&mut body, units, &[], policy);
        body.iter()
            .map(|x| g.iter().position(|y| y == x).expect("same instructions"))
            .collect()
    }

    fn text(g: &[Insn], ord: &[usize]) -> Vec<String> {
        ord.iter().map(|&k| g[k].to_string()).collect()
    }

    #[test]
    fn off_keeps_the_order() {
        let g = vec![
            mv(Size::L, Ea::Disp(8, 6), dr(0)),
            Insn::Addq {
                size: Size::L,
                value: 1,
                dst: dr(0),
            },
            Insn::Moveq { value: 1, dst: 1 },
        ];
        assert_eq!(order(&g, &[], Schedule::Off), vec![0, 1, 2]);
        assert_eq!(order(&g, &[], R11), vec![0, 2, 1]);
    }

    /// 0xE177A6..0xE177B4 from the selector's order of golden
    /// `regalloc/cse` (`clr.l D1; move.w D0,D1; clr.l D2; move.w (A0),D2;
    /// add.l D1,D2`): both `clr.l` first, then both moves (R11).  With the
    /// image's register-held constant (`move.l #0x400,D3`, 12.3 item 3)
    /// selected after the `add`, it lands between the second move and the
    /// `add`, as at 0xE177AE.
    #[test]
    fn log_worker_zero_extensions() {
        let g = vec![
            clr(Size::L, dr(1)),
            mv(Size::W, dr(0), dr(1)),
            clr(Size::L, dr(2)),
            mv(Size::W, Ea::Ind(0), dr(2)),
            alu(AluOp::Add, Size::L, dr(1), 2),
            mv(Size::L, Ea::Imm(0x400), dr(3)),
            Insn::Cmp {
                size: Size::L,
                src: dr(3),
                dst: 2,
            },
        ];
        for p in [R11, R11W3] {
            assert_eq!(order(&g, &[], p), vec![0, 2, 1, 3, 5, 4, 6], "{p:?}");
        }
    }

    /// 0xE177CC..0xE177DC: `buffer` (A2), the source pointer (D3 -> A1)
    /// and the word count (D0) interleaved, from an order with the
    /// `buffer` load first (codegen-patterns 12.4 R11: assumed).
    #[test]
    fn log_worker_three_chains() {
        let g = vec![
            movea(Ea::Disp(8, 6), 2),
            mv(Size::L, dr(1), dr(3)),
            alu(AluOp::Add, Size::L, Ea::Disp(0x14, 5), 3),
            movea(dr(3), 1),
            mv(Size::W, Ea::Ind(0), dr(0)),
            shift(ShiftOp::Lsr, Size::W, 1, 0),
            Insn::Subq {
                size: Size::W,
                value: 1,
                dst: dr(0),
            },
        ];
        for p in [R11, R11W3] {
            let o = order(&g, &[], p);
            assert_eq!(
                text(&g, &o),
                [
                    "movea.l\t(8,%fp),%a2",
                    "move.l\t%d1,%d3",
                    "move.w\t(%a0),%d0",
                    "add.l\t(0x14,%a5),%d3",
                    "lsr.w\t#1,%d0",
                    "movea.l\t%d3,%a1",
                    "subq.w\t#1,%d0",
                ],
                "{p:?}"
            );
        }
    }

    /// FIM_$ACKNOWLEDGE 0xE0A972..0xE0A994 from an order with each array
    /// reference's index before its base, the source first: the ×12
    /// stays whole and the bases fill the gap after the second index
    /// load.  The unlimited rule pulls the first base into the ×12
    /// instead; a window of 3 does not (12.4 R11).
    #[test]
    fn fim_acknowledge_window() {
        let abs = || Ea::AbsL("PROC1_$AS_ID".into(), 0);
        let g = vec![
            mv(Size::W, abs(), dr(2)),
            shift(ShiftOp::Lsl, Size::W, 2, 2),
            mv(Size::W, dr(2), dr(0)),
            alu(AluOp::Add, Size::W, dr(0), 0),
            alu(AluOp::Add, Size::W, dr(0), 2),
            mv(Size::W, abs(), dr(0)),
            shift(ShiftOp::Lsl, Size::W, 2, 0),
            movea(Ea::ImmSym("QUIT_EC".into()), 0),
            movea(Ea::ImmSym("QUIT_VALUE".into()), 1),
            mv(Size::L, Ea::Idx(0, 0, 2, false), Ea::Idx(0, 1, 0, false)),
        ];
        assert_eq!(order(&g, &[], R11W3), vec![0, 1, 2, 3, 4, 5, 7, 6, 8, 9]);
        assert_eq!(order(&g, &[], R11), vec![0, 7, 1, 8, 2, 3, 4, 5, 6, 9]);
    }

    /// A unit (the selector marks unrolled `(An)+` copies) is not entered;
    /// here a multiplication is used as one, which the selector does not do
    /// (0xE1AA5A).
    #[test]
    fn units_are_not_split() {
        let g = vec![
            shift(ShiftOp::Lsl, Size::W, 2, 2),
            mv(Size::W, dr(2), dr(0)),
            alu(AluOp::Add, Size::W, dr(0), 0),
            alu(AluOp::Add, Size::W, dr(0), 2),
            movea(Ea::ImmSym("T".into()), 0),
        ];
        // Without the unit the base lands inside the expansion.
        assert_eq!(order(&g, &[], R11), vec![0, 4, 1, 2, 3]);
        assert_eq!(order(&g, &[(0, 4)], R11), vec![0, 1, 2, 3, 4]);
    }

    /// A flags consumer keeps its setter: a later setter does not move
    /// between them, an earlier one does not move after the setter, and
    /// the run's last setter stays last (the flags are live out).
    #[test]
    fn flags_are_respected() {
        // load; addq (setter, waits for the load); cmpi (setter S); seq.
        // R11 would like to put `cmpi` before `addq`: then `seq` would
        // read `addq`'s flags.
        let g = vec![
            mv(Size::W, Ea::Disp(8, 6), dr(3)),
            Insn::Addq {
                size: Size::W,
                value: 1,
                dst: dr(3),
            },
            Insn::Cmpi {
                size: Size::W,
                imm: 2,
                dst: dr(0),
            },
            Insn::Scc {
                cond: Cond::Eq,
                dst: dr(1),
            },
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
        // load D1; cmpi (stalls on D1); seq; moveq (a setter after the
        // reader, and the last setter of the run): stays last.
        let g = vec![
            mv(Size::W, Ea::Disp(8, 6), dr(1)),
            Insn::Cmpi {
                size: Size::W,
                imm: 3,
                dst: dr(1),
            },
            Insn::Scc {
                cond: Cond::Eq,
                dst: dr(2),
            },
            Insn::Moveq { value: 0, dst: 3 },
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
        // A flags-setting instruction before a branch stays last: `tst`
        // after its load, `moveq` ready earlier.
        let g = vec![
            mv(Size::L, Ea::Disp(8, 6), dr(0)),
            Insn::Moveq { value: 5, dst: 1 },
            Insn::Tst {
                size: Size::L,
                dst: dr(0),
            },
            Insn::Branch {
                cond: Cond::Ne,
                label: 1,
                short: true,
                back: false,
            },
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
        // The flags are live across a label to a join's test: `moveq`
        // (the run's last setter) does not move above the `addq` whose
        // flags the `beq` after the label reads.
        let g = vec![
            mv(Size::L, Ea::Disp(8, 6), dr(0)),
            Insn::Addq {
                size: Size::L,
                value: 1,
                dst: dr(0),
            },
            Insn::Moveq { value: 2, dst: 1 },
            Insn::Label(1),
            Insn::Branch {
                cond: Cond::Eq,
                label: 2,
                short: true,
                back: false,
            },
            Insn::Label(2),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3, 4, 5]);
        // Not live (the end of the body): the `moveq` fills the stall.
        let g = g[..3].to_vec();
        assert_eq!(order(&g, &[], R11), vec![0, 2, 1]);
    }

    /// Loads are not hoisted above stores (stores are ordered against
    /// everything), nor above pushes; distinct frame slots may pass
    /// loads.
    #[test]
    fn memory_order() {
        // movea; store through A0 (stalls on A0); load through A1.
        let g = vec![
            movea(Ea::Disp(8, 6), 0),
            mv(Size::L, dr(1), Ea::Ind(0)),
            mv(Size::L, Ea::Ind(1), dr(2)),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2]);
        // A register-only instruction does not pass a store either.
        let g = vec![
            movea(Ea::Disp(8, 6), 0),
            mv(Size::L, dr(1), Ea::Disp(4, 0)),
            Insn::Moveq { value: 1, dst: 2 },
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2]);
        // movea; load through A0 (stalls); a frame load passes it.
        let g = vec![
            movea(Ea::Disp(8, 6), 0),
            mv(Size::L, Ea::Disp(4, 0), dr(1)),
            mv(Size::L, Ea::Disp(0xc, 6), dr(2)),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 2, 1]);
        // An argument list: the load for the second push stays after the
        // first push.
        let g = vec![
            movea(Ea::Disp(8, 6), 0),
            push_w(Ea::Ind(0)),
            movea(Ea::Disp(0xc, 6), 1),
            push_w(Ea::Ind(1)),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
        // Overlapping frame bytes: a word store and a long load.
        let g = vec![
            mv(Size::L, Ea::Disp(8, 6), dr(0)),
            mv(Size::W, dr(0), Ea::Disp(-4, 6)),
            mv(Size::L, Ea::Disp(-6, 6), dr(1)),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2]);
    }

    /// References to volatile objects keep their order (ref 3.15.1:
    /// each reference made as written); other loads still pass them.
    #[test]
    fn volatile_references_keep_their_order() {
        // movea; load through A0 (stalls on A0); a second load, which
        // R11 lists first when nothing orders the two.
        let g = vec![
            movea(Ea::Disp(8, 6), 0),
            mv(Size::W, Ea::Disp(6, 0), dr(1)),
            mv(Size::L, Ea::AbsL("CLOCK".into(), 0), dr(2)),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 2, 1]);
        let ordered = |vol: &[usize]| {
            let mut body = g.clone();
            schedule(&mut body, &[], vol, R11);
            body.iter()
                .map(|x| g.iter().position(|y| y == x).unwrap())
                .collect::<Vec<_>>()
        };
        // Both volatile: as written.
        assert_eq!(ordered(&[1, 2]), vec![0, 1, 2]);
        // One volatile: the other load may pass it.
        assert_eq!(ordered(&[2]), vec![0, 2, 1]);
    }

    /// Nothing crosses a call, an SR instruction or the data-base load,
    /// and the static link stays right before its `bsr`.
    #[test]
    fn barriers() {
        let ld = || mv(Size::L, Ea::Disp(8, 6), dr(0));
        let inc = || Insn::Addq {
            size: Size::L,
            value: 1,
            dst: dr(0),
        };
        let free = || Insn::Moveq { value: 1, dst: 1 };
        for b in [
            Insn::Jsr(Ea::AbsL("X".into(), 0)),
            Insn::Bsr {
                target: "Y".into(),
                short: true,
            },
            Insn::SrImm {
                op: AluOp::Or,
                imm: 0x700,
            },
            Insn::SrImm {
                op: AluOp::And,
                imm: 0xf8ff,
            },
            Insn::MoveToSr {
                src: Ea::Imm(0x2700),
            },
            Insn::MoveFromSr { dst: 4 },
            Insn::Lea {
                src: Ea::AbsL("DATA".into(), 0),
                dst: 5,
            },
        ] {
            let g = vec![ld(), inc(), b.clone(), free()];
            assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3], "{b}");
        }
        // pad; push (stalls on SP); static link; bsr.
        let g = vec![
            pad(),
            push_w(Ea::Imm(1)),
            movea(Ea::Reg(Reg::A(6)), 1),
            Insn::Bsr {
                target: "Y".into(),
                short: true,
            },
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
        // A pad stays below a store.
        let g = vec![
            mv(Size::L, Ea::Disp(0x44c, 5), dr(0)),
            Insn::Addq {
                size: Size::L,
                value: 1,
                dst: dr(0),
            },
            mv(Size::L, dr(0), Ea::Disp(-8, 6)),
            pad(),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
    }

    /// Labels and branches end runs: nothing moves across them.
    #[test]
    fn runs_end_at_labels() {
        let g = vec![
            mv(Size::L, Ea::Disp(8, 6), dr(0)),
            Insn::Label(1),
            Insn::Addq {
                size: Size::L,
                value: 1,
                dst: dr(0),
            },
            Insn::Moveq { value: 1, dst: 1 },
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3]);
    }

    /// A small interpreter for the instructions the fuzz test generates:
    /// registers, flags (N, Z, V, C) and byte memory.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Machine {
        d: [u32; 8],
        a: [u32; 8],
        nzvc: [bool; 4],
        mem: std::collections::BTreeMap<u32, u8>,
    }

    impl Machine {
        fn new() -> Machine {
            Machine {
                d: [0x11, 0xfffe, 0x8000_0003, 4, 0, 0, 0, 0],
                // A0/A1 point into the frame and the module data, so
                // that pointer accesses alias named ones.
                a: [0x0ff8, 0x2004, 0x1000, 0, 0, 0x2000, 0x1000, 0x0f00],
                nzvc: [false; 4],
                mem: Default::default(),
            }
        }
        fn rd8(&self, x: u32) -> u32 {
            *self
                .mem
                .get(&x)
                .unwrap_or(&(x.wrapping_mul(31).wrapping_add(7) as u8)) as u32
        }
        fn rd(&self, x: u32, n: i32) -> u32 {
            (0..n as u32).fold(0, |v, k| (v << 8) | self.rd8(x.wrapping_add(k)))
        }
        fn wr(&mut self, x: u32, n: i32, v: u32) {
            for k in 0..n as u32 {
                let sh = 8 * (n as u32 - 1 - k);
                self.mem.insert(x.wrapping_add(k), (v >> sh) as u8);
            }
        }
        fn mask(size: Size) -> u32 {
            match size {
                Size::B => 0xff,
                Size::W => 0xffff,
                Size::L => 0xffff_ffff,
            }
        }
        fn sign(size: Size) -> u32 {
            match size {
                Size::B => 0x80,
                Size::W => 0x8000,
                Size::L => 0x8000_0000,
            }
        }
        fn reg(&self, r: Reg) -> u32 {
            match r {
                Reg::D(n) => self.d[n as usize],
                Reg::A(n) => self.a[n as usize],
            }
        }
        /// The address of a memory operand, stepping `-(An)`/`(An)+`.
        fn addr(&mut self, ea: &Ea, size: Size) -> u32 {
            let step = |a: u8| {
                if a == 7 && size == Size::B {
                    2
                } else {
                    bytes(size) as u32
                }
            };
            match ea {
                Ea::Ind(a) => self.a[*a as usize],
                Ea::Disp(d, a) => self.a[*a as usize].wrapping_add(*d as u32),
                Ea::Idx(d, a, x, long) => {
                    let i = if *long {
                        self.d[*x as usize]
                    } else {
                        self.d[*x as usize] as u16 as i16 as i32 as u32
                    };
                    self.a[*a as usize].wrapping_add(i).wrapping_add(*d as u32)
                }
                Ea::PreDec(a) => {
                    let n = step(*a);
                    self.a[*a as usize] = self.a[*a as usize].wrapping_sub(n);
                    self.a[*a as usize]
                }
                Ea::PostInc(a) => {
                    let v = self.a[*a as usize];
                    let n = step(*a);
                    self.a[*a as usize] = v.wrapping_add(n);
                    v
                }
                Ea::AbsL(_, o) => 0x3000 + *o as u32,
                Ea::AbsW(v) => *v as u32,
                Ea::Pool(_) | Ea::PoolLabel(_) => 0x5000,
                _ => unreachable!("not memory: {ea:?}"),
            }
        }
        /// An operand's place: a register or an address.
        fn place(&mut self, ea: &Ea, size: Size) -> Result<Reg, u32> {
            match ea {
                Ea::Reg(r) => Ok(*r),
                _ => Err(self.addr(ea, size)),
            }
        }
        fn get(&self, p: Result<Reg, u32>, size: Size) -> u32 {
            match p {
                Ok(r) => self.reg(r) & Self::mask(size),
                Err(x) => self.rd(x, bytes(size)),
            }
        }
        fn put(&mut self, p: Result<Reg, u32>, size: Size, v: u32) {
            let m = Self::mask(size);
            match p {
                Ok(Reg::D(n)) => {
                    let d = &mut self.d[n as usize];
                    *d = (*d & !m) | (v & m);
                }
                Ok(Reg::A(n)) => self.a[n as usize] = v,
                Err(x) => self.wr(x, bytes(size), v & m),
            }
        }
        fn read(&mut self, ea: &Ea, size: Size) -> u32 {
            match ea {
                Ea::Imm(v) => *v as u32 & Self::mask(size),
                Ea::ImmSym(_) => 0x4000,
                _ => {
                    let p = self.place(ea, size);
                    self.get(p, size)
                }
            }
        }
        fn nz(&mut self, size: Size, v: u32) {
            let v = v & Self::mask(size);
            self.nzvc = [v & Self::sign(size) != 0, v == 0, false, false];
        }
        fn addsub(&mut self, size: Size, a: u32, b: u32, sub: bool) -> u32 {
            let m = Self::mask(size) as u64;
            let (a, b) = (a as u64 & m, b as u64 & m);
            let r = if sub { a.wrapping_sub(b) } else { a + b } & m;
            let s = Self::sign(size) as u64;
            let c = if sub { b > a } else { a + b > m };
            let v = if sub {
                ((a ^ b) & (a ^ r) & s) != 0
            } else {
                (!(a ^ b) & (a ^ r) & s) != 0
            };
            self.nzvc = [r & s != 0, r == 0, v, c];
            r as u32
        }
        fn cond(&self, c: Cond) -> bool {
            let [n, z, v, cy] = self.nzvc;
            match c {
                Cond::T => true,
                Cond::Hi => !cy && !z,
                Cond::Ls => cy || z,
                Cond::Cc => !cy,
                Cond::Cs => cy,
                Cond::Ne => !z,
                Cond::Eq => z,
                Cond::Pl => !n,
                Cond::Mi => n,
                Cond::Ge => n == v,
                Cond::Lt => n != v,
                Cond::Gt => !z && n == v,
                Cond::Le => z || n != v,
            }
        }
        fn step(&mut self, insn: &Insn) {
            match insn {
                Insn::Label(_) | Insn::Branch { .. } => {}
                Insn::Move { size, src, dst } => {
                    let v = self.read(src, *size);
                    let p = self.place(dst, *size);
                    self.put(p, *size, v);
                    self.nz(*size, v);
                }
                Insn::Movea { size, src, dst } => {
                    let v = self.read(src, *size);
                    self.a[*dst as usize] = if *size == Size::W {
                        v as u16 as i16 as i32 as u32
                    } else {
                        v
                    };
                }
                Insn::Lea { src, dst } => {
                    let x = self.addr(src, Size::L);
                    self.a[*dst as usize] = x;
                }
                Insn::Pea(ea) => {
                    let x = self.addr(ea, Size::L);
                    self.a[7] = self.a[7].wrapping_sub(4);
                    self.wr(self.a[7], 4, x);
                }
                Insn::Moveq { value, dst } => {
                    self.d[*dst as usize] = *value as i32 as u32;
                    self.nz(Size::L, *value as i32 as u32);
                }
                Insn::Addq { size, value, dst } | Insn::Subq { size, value, dst } => {
                    let sub = matches!(insn, Insn::Subq { .. });
                    if let Ea::Reg(Reg::A(a)) = dst {
                        let r = &mut self.a[*a as usize];
                        *r = if sub {
                            r.wrapping_sub(*value as u32)
                        } else {
                            r.wrapping_add(*value as u32)
                        };
                    } else {
                        let p = self.place(dst, *size);
                        let a = self.get(p, *size);
                        let r = self.addsub(*size, a, *value as u32, sub);
                        self.put(p, *size, r);
                    }
                }
                Insn::Clr { size, dst } => {
                    let p = self.place(dst, *size);
                    self.put(p, *size, 0);
                    self.nz(*size, 0);
                }
                Insn::Alu { op, size, src, dst } => {
                    let b = self.read(src, *size);
                    let a = self.d[*dst as usize] & Self::mask(*size);
                    let r = match op {
                        AluOp::Add => self.addsub(*size, a, b, false),
                        AluOp::Sub => self.addsub(*size, a, b, true),
                        AluOp::And => a & b,
                        AluOp::Or => a | b,
                        AluOp::Eor => a ^ b,
                    };
                    if matches!(op, AluOp::And | AluOp::Or | AluOp::Eor) {
                        self.nz(*size, r);
                    }
                    self.put(Ok(Reg::D(*dst)), *size, r);
                }
                Insn::Swap(n) => {
                    let v = self.d[*n as usize].rotate_left(16);
                    self.d[*n as usize] = v;
                    self.nz(Size::L, v);
                }
                Insn::Ext { size, dst } => {
                    let d = self.d[*dst as usize];
                    let v = match size {
                        Size::W => (d & 0xffff_0000) | (d as u8 as i8 as i16 as u16 as u32),
                        _ => d as u16 as i16 as i32 as u32,
                    };
                    self.d[*dst as usize] = v;
                    self.nz(*size, v);
                }
                Insn::Tst { size, dst } => {
                    let v = self.read(dst, *size);
                    self.nz(*size, v);
                }
                Insn::Cmpi { size, imm, dst } => {
                    let v = self.read(dst, *size);
                    self.addsub(*size, v, *imm as u32, true);
                }
                Insn::CmpaZero(a) => {
                    let r = self.a[*a as usize];
                    self.addsub(Size::L, r, 0, true);
                }
                Insn::Scc { cond, dst } => {
                    let v = if self.cond(*cond) { 0xff } else { 0 };
                    let p = self.place(dst, Size::B);
                    self.put(p, Size::B, v);
                }
                Insn::Mul { src, dst, .. } => {
                    let b = self.read(src, Size::W);
                    let v = (self.d[*dst as usize] & 0xffff) * b;
                    self.d[*dst as usize] = v;
                    self.nz(Size::L, v);
                }
                Insn::Unary { size, dst, .. } => {
                    let p = self.place(dst, *size);
                    let a = self.get(p, *size);
                    let r = self.addsub(*size, 0, a, true);
                    self.put(p, *size, r);
                }
                Insn::BtstImm { bit, dst } => {
                    let v = self.read(dst, Size::B);
                    self.nzvc[1] = v & (1 << (bit & 7)) == 0;
                }
                Insn::Shift {
                    size, count, dst, ..
                } => {
                    let n = match count {
                        ShiftCount::Imm(n) => *n as u32,
                        ShiftCount::Reg(r) => self.d[*r as usize] & 7,
                    };
                    let a = self.d[*dst as usize] & Self::mask(*size);
                    let r = a << n;
                    self.put(Ok(Reg::D(*dst)), *size, r);
                    self.nz(*size, r);
                }
                x => unreachable!("not generated: {x}"),
            }
        }
    }

    /// A deterministic pseudo-random generator (no dependencies).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 33) as u32
        }
        fn pick(&mut self, n: usize) -> usize {
            self.next() as usize % n
        }
    }

    fn random_insn(r: &mut Rng) -> Insn {
        let sizes = [Size::B, Size::W, Size::L];
        let dn = |r: &mut Rng| r.pick(4) as u8;
        let an = |r: &mut Rng| r.pick(3) as u8;
        let mem = |r: &mut Rng| -> Ea {
            match r.pick(9) {
                0 => Ea::Disp([-8, -6, -4, -2, 8, 12][r.pick(6)], 6),
                1 => Ea::Disp([0, 2, 4, 6][r.pick(4)], 5),
                2 => Ea::Disp([0, 2, 4][r.pick(3)], r.pick(3) as u8),
                3 => Ea::Ind(r.pick(3) as u8),
                4 => Ea::PostInc(r.pick(3) as u8),
                5 => Ea::PreDec(r.pick(3) as u8),
                6 => Ea::Idx(0, r.pick(3) as u8, r.pick(4) as u8, false),
                7 => Ea::AbsL("X".into(), [0, 2, 4][r.pick(3)]),
                _ => Ea::Ind(7),
            }
        };
        let src = |r: &mut Rng, size: Size| -> Ea {
            match r.pick(4) {
                0 => Ea::Reg(Reg::D(r.pick(4) as u8)),
                1 if size != Size::B => Ea::Reg(Reg::A(r.pick(3) as u8)),
                1 => Ea::Imm(r.pick(256) as i64),
                _ => mem(r),
            }
        };
        let dst = |r: &mut Rng| -> Ea {
            match r.pick(4) {
                0 | 1 => Ea::Reg(Reg::D(r.pick(4) as u8)),
                2 => Ea::PreDec(7),
                _ => mem(r),
            }
        };
        let size = sizes[r.pick(3)];
        match r.pick(24) {
            0..=3 => {
                let s = src(r, size);
                Insn::Move {
                    size,
                    src: s,
                    dst: dst(r),
                }
            }
            4 => Insn::Movea {
                size: [Size::W, Size::L][r.pick(2)],
                src: src(r, Size::L),
                dst: an(r),
            },
            5 => Insn::Lea {
                src: [
                    Ea::Disp(-4, 6),
                    Ea::Disp(4, 0),
                    Ea::Disp(2, 1),
                    Ea::Disp(6, 7),
                ][r.pick(4)]
                .clone(),
                dst: if r.pick(4) == 0 { 7 } else { an(r) },
            },
            6 => Insn::Pea(mem(r)),
            7 => Insn::Moveq {
                value: r.pick(256) as u8 as i8,
                dst: dn(r),
            },
            8 => Insn::Addq {
                size,
                value: 1 + r.pick(8) as u8,
                dst: match r.pick(3) {
                    0 => Ea::Reg(Reg::A([0, 1, 2, 7][r.pick(4)])),
                    1 => mem(r),
                    _ => Ea::Reg(Reg::D(dn(r))),
                },
            },
            9 => Insn::Subq {
                size: Size::L,
                value: 2,
                dst: Ea::Reg(Reg::A(7)),
            },
            10 => Insn::Clr { size, dst: dst(r) },
            11 => Insn::Alu {
                op: [AluOp::Add, AluOp::Sub, AluOp::And, AluOp::Or][r.pick(4)],
                size,
                src: src(r, size),
                dst: dn(r),
            },
            12 => Insn::Swap(dn(r)),
            13 => Insn::Ext {
                size: [Size::W, Size::L][r.pick(2)],
                dst: dn(r),
            },
            14 => Insn::Tst {
                size,
                dst: src(r, size),
            },
            15 => Insn::Cmpi {
                size,
                imm: r.pick(4) as i64,
                dst: src(r, size),
            },
            16 | 17 => Insn::Scc {
                cond: [Cond::Eq, Cond::Ne, Cond::Cs, Cond::Mi, Cond::Lt][r.pick(5)],
                dst: if r.pick(3) == 0 {
                    mem(r)
                } else {
                    Ea::Reg(Reg::D(dn(r)))
                },
            },
            18 => Insn::Mul {
                signed: false,
                src: src(r, Size::W),
                dst: dn(r),
            },
            19 => Insn::Unary {
                op: crate::insn::UnaryOp::Neg,
                size,
                dst: dst(r),
            },
            20 => Insn::CmpaZero(an(r)),
            21 => Insn::Shift {
                op: ShiftOp::Lsl,
                size,
                count: if r.pick(2) == 0 {
                    ShiftCount::Imm(1 + r.pick(8) as u8)
                } else {
                    ShiftCount::Reg(dn(r))
                },
                dst: dn(r),
            },
            22 => Insn::BtstImm {
                bit: r.pick(8) as u8,
                dst: src(r, Size::B),
            },
            _ => Insn::Move {
                size: Size::L,
                src: Ea::Ind(7),
                dst: Ea::PreDec(7),
            },
        }
    }

    /// Random straight-line runs scheduled under each policy compute the
    /// same registers, memory and (when a branch reads them) flags as the
    /// selector's order.
    #[test]
    fn fuzz_schedules_preserve_semantics() {
        let mut r = Rng(0x5eed);
        for case in 0..40000 {
            let n = 2 + r.pick(12);
            let mut g: Vec<Insn> = (0..n).map(|_| random_insn(&mut r)).collect();
            let flags = r.pick(2) == 0;
            if flags {
                g.push(Insn::Branch {
                    cond: Cond::Ne,
                    label: 1,
                    short: true,
                    back: false,
                });
                g.push(Insn::Label(1));
            }
            // Sometimes a unit, as the selector marks block copies.
            let units: Vec<Unit> = if n >= 3 && r.pick(3) == 0 {
                let a = r.pick(n - 2);
                vec![(a, a + 2 + r.pick(n - a - 1))]
            } else {
                vec![]
            };
            let mut want = Machine::new();
            for x in &g {
                want.step(x);
            }
            for p in [
                R11,
                R11W3,
                Schedule::R11 { window: 1 },
                Schedule::R11 { window: 2 },
            ] {
                let mut body = g.clone();
                schedule(&mut body, &units, &[], p);
                let mut got = Machine::new();
                for x in &body {
                    got.step(x);
                }
                if !flags {
                    got.nzvc = want.nzvc;
                }
                assert_eq!(
                    got,
                    want,
                    "case {case} {p:?}\nselector:\n{}\nscheduled:\n{}",
                    g.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    body.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        }
    }

    /// Pseudo-instructions, `dbf` and `case` dispatch end runs: nothing
    /// moves across them (an `Align4` filler before a loop head stays
    /// where the selector put it).
    #[test]
    fn boundaries_stay() {
        let ld = || mv(Size::L, Ea::Disp(8, 6), dr(0));
        let inc = || Insn::Addq {
            size: Size::L,
            value: 1,
            dst: dr(0),
        };
        let free = || Insn::Moveq { value: 1, dst: 1 };
        for b in [
            Insn::Align4,
            Insn::PoolLabel(3),
            Insn::Words(vec![0]),
            Insn::Encoded {
                words: vec![0x6100],
                text: "bsr.s X".into(),
            },
            Insn::Dbf {
                reg: 2,
                label: 1,
                until: None,
            },
            Insn::TableIndex { reg: 3, table: 9 },
            Insn::TableJump {
                reg: 3,
                table: 9,
                targets: vec![],
            },
            Insn::TableEntry {
                table: 9,
                target: 1,
                back: true,
            },
        ] {
            let g = vec![Insn::Label(1), ld(), inc(), b.clone(), free()];
            assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3, 4], "{b}");
        }
    }

    /// The flags are live out of a run when a test after a `dbf` (which
    /// leaves them) or at a loop head reached by a back branch reads
    /// them: the run's last setter stays last.
    #[test]
    fn flags_live_through_dbf_and_back_edges() {
        let ld = || mv(Size::L, Ea::Disp(8, 6), dr(0));
        let inc = || Insn::Addq {
            size: Size::L,
            value: 1,
            dst: dr(0),
        };
        let free = || Insn::Moveq { value: 1, dst: 1 };
        let br = |cond, label, back| Insn::Branch {
            cond,
            label,
            short: true,
            back,
        };
        // 1: load; addq; moveq; dbf D2,1b; beq 2f; 2:
        let g = vec![
            Insn::Label(1),
            ld(),
            inc(),
            free(),
            Insn::Dbf {
                reg: 2,
                label: 1,
                until: None,
            },
            br(Cond::Eq, 2, false),
            Insn::Label(2),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3, 4, 5, 6]);
        // 1: beq 2f; load; addq; moveq; bra 1b; 2:
        let g = vec![
            Insn::Label(1),
            br(Cond::Eq, 2, false),
            ld(),
            inc(),
            free(),
            br(Cond::T, 1, true),
            Insn::Label(2),
        ];
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 3, 4, 5, 6]);
        // The same loop without the test: the `moveq` fills the stall.
        let mut g = g;
        g[1] = Insn::Label(3);
        assert_eq!(order(&g, &[], R11), vec![0, 1, 2, 4, 3, 5, 6]);
    }
}
