//! Register allocation (codegen-patterns 2, 12.4 plan step 1).
//!
//! The selector (`isel.rs`) writes a routine body over **virtual
//! registers**; [`allocate`] then gives each one a physical register and
//! rewrites the body.  There is no other register state.
//!
//! **Virtual registers** are spelled with the instruction types' own
//! register numbers: `Reg::D(n)` / `Reg::A(n)` with `n >= VBASE`, and the
//! same numbers in the `u8` fields of [`Ea`] and [`Insn`] (`Ea::Ind(n)`,
//! `Insn::Movea { dst: n, .. }`, ...).  Numbers below `VBASE` are physical
//! registers: A5 (the module data base, codegen-patterns 4), A6 and A7,
//! which are never allocated, and D0 as the function result register
//! (codegen-patterns 3).  Only [`allocate`]'s output reaches the emitter.
//!
//! **The model** (codegen-patterns 12.4, R1-R3; all three Verified as
//! properties of the image's output):
//!
//! 1. *Live ranges.*  Liveness is computed over the body's instructions
//!    and its branches (labels, `b<cc>`, `bra`, `dbf`), backward to a fixed
//!    point.  Two values interfere when one is defined where the other is
//!    live (Chaitin).  A parameter loaded at entry is an ordinary value: its
//!    register is free again after its last use (0xE17778: D1, last read
//!    0xE177A2, gets another value at 0xE177A6).
//! 2. *R1.*  Each value gets the lowest-numbered register of its class,
//!    D0..D7 or A0..A4, that no interfering value already holds: free over
//!    the value's whole live range, not only at its definition.  There is
//!    no separate "scratch first" pool: D2 is simply the register after D1
//!    (0xE177A0 `moveq #-2,D2` with D0 and D1 holding parameters, no call).
//! 3. *R2.*  A value does not take a register that its defining
//!    instruction reads as a register operand (a copy, `lea` from a base, a
//!    two-address operator's other operand): no copy coalescing
//!    (0xE177D0 `move.l D1,D3`; 0xE177DA `movea.l D3,A1` then `lea
//!    (2,A1),A0`).  Loads through a dying base register are *not* covered:
//!    `movea.l (d,A0),A0` may reuse the base (12.4 R2, Uncertain; what
//!    `dpc` did before this allocator, and no golden or conformance unit
//!    contradicts it).  A two-address result (`sne` into the tested
//!    value's register, 0xE148DE) is the same virtual register as its
//!    operand, so it stays in place.
//! 4. *R3.*  A call (`jsr`, `bsr`) writes D0, D1, A0 and A1, so a value
//!    live across one gets D2..D7 / A2..A4 (1834 values in the survey, no
//!    D1/A1 read reached by a call but at 0xE47442/0xE47624).  An M$ARITH
//!    call writes only D0 and A0 (0xE3CB8E..0xE3CB9E keeps A1 across `jsr
//!    M$OIU$WLW`; `../source/domain_os/math/sau2/arith.s`).
//!
//! **Allocation order** (Uncertain, 12.4 R1 "The order is not settled"):
//! values are coloured in the address order of their first definition,
//! each against the values coloured before it (rule `colQU` of
//! `tools/regalloc.py`: 72.8% of data and 73.1% of address values in the
//! survey).  Known misses of this order: parameters held across calls
//! take D3/A3 in the image while later temporaries take D2/A2 (module-local
//! 0xE01C52; AST_$ASSOC 0xE0445C); R4 (registers held longer than
//! liveness: to the end of the statement) is not modelled, except R4(c)
//! for a value that a call's arguments read last, which the selector
//! passes in as `holds` (PROC1_$SET_ASID 0xE148FE D2; `crate::isel`), and
//! R4(b) for by-address slots read once, which the selector keeps apart
//! from the values of earlier statements ([`allocate_apart`]; IIC_$RECEIVE
//! 0xE70AC0..0xE70AD0).  R5: a function's result variable and the
//! parameters loaded at entry are coloured first ([`allocate_apart`]'s
//! `first`; PARITY_$CHK_IO 0xE0B180..0xE0B18A).  R1b: a pointer local
//! assigned a call's result is coloured before a parameter live across
//! that call that dies in the assignment's block ([`call_results_first`];
//! KBD_$PUT 0xE1CEB4/0xE1CEC6).
//!
//! The callee-saved registers the assignment uses are what the prologue
//! saves ([`crate::frame::Frame`]), with A5 when the body loads it.

use std::collections::{BTreeSet, HashMap};

use crate::insn::{Ea, Insn, Reg, RegSet, Size};

/// Data or address register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegClass {
    Data,
    Addr,
}

/// The first virtual register number of each class.
pub const VBASE: u8 = 16;

/// Allocatable registers in allocation order (R1): D0..D7, A0..A4.  A5 is
/// the module data base and A6 the frame pointer; they are callee-saved but
/// not allocatable.
pub const ALLOC_D: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
pub const ALLOC_A: [u8; 5] = [0, 1, 2, 3, 4];

/// The data base register (codegen-patterns 4).
pub const DATA_BASE: Reg = Reg::A(5);

/// Every register a routine must preserve for its caller (ref 5.5.8
/// `nosave`; codegen-patterns 2): D2-D7, A2-A5 (A6 is restored by `unlk`,
/// A7 by the frame discipline).
pub fn callee_saved() -> RegSet {
    let mut s = RegSet::EMPTY;
    for n in 2..8 {
        s.insert(Reg::D(n));
    }
    for n in 2..6 {
        s.insert(Reg::A(n));
    }
    s
}

/// Whether `r` is a virtual register.
pub fn is_virtual(r: Reg) -> bool {
    match r {
        Reg::D(n) | Reg::A(n) => n >= VBASE,
    }
}

/// Hands out virtual registers, numbered per class in creation order.
#[derive(Clone, Debug, Default)]
pub struct VRegs {
    next_d: u8,
    next_a: u8,
}

impl VRegs {
    pub fn new() -> VRegs {
        VRegs::default()
    }

    /// A new virtual register of `class`, or `None` when the routine needs
    /// more than the numbering holds.
    pub fn new_reg(&mut self, class: RegClass) -> Option<Reg> {
        let next = match class {
            RegClass::Data => &mut self.next_d,
            RegClass::Addr => &mut self.next_a,
        };
        let n = VBASE.checked_add(*next)?;
        *next = next.checked_add(1)?;
        Some(match class {
            RegClass::Data => Reg::D(n),
            RegClass::Addr => Reg::A(n),
        })
    }
}

/// The registers an instruction reads and writes, as the allocator sees
/// them.  Physical A5..A7 are left out: they are never allocated.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Access {
    pub uses: Vec<Reg>,
    pub defs: Vec<Reg>,
    /// Registers read as whole register operands (not as an address base)
    /// by an instruction that defines a register: R2's "no coalescing".
    pub reg_operands: Vec<Reg>,
}

fn tracked(r: Reg) -> bool {
    !matches!(r, Reg::A(5..=7))
}

impl Access {
    fn use_(&mut self, r: Reg) {
        if tracked(r) && !self.uses.contains(&r) {
            self.uses.push(r);
        }
    }

    fn def(&mut self, r: Reg) {
        if tracked(r) && !self.defs.contains(&r) {
            self.defs.push(r);
        }
    }

    /// The registers an operand reads to form its address (`(An)`,
    /// `(d,An)`, `-(An)`, `(An)+`); the last two also write An.
    fn ea_address(&mut self, ea: &Ea) {
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

    /// A source operand.
    fn src(&mut self, ea: &Ea) {
        if let Ea::Reg(r) = ea {
            self.use_(*r);
            if tracked(*r) {
                self.reg_operands.push(*r);
            }
        } else {
            self.ea_address(ea);
        }
    }

    /// A destination operand that is only written.
    fn dst(&mut self, ea: &Ea) {
        match ea {
            Ea::Reg(r) => self.def(*r),
            _ => self.ea_address(ea),
        }
    }

    /// A destination operand that is read and written (`addq`, `dbf`).
    fn dst_rmw(&mut self, ea: &Ea) {
        match ea {
            Ea::Reg(r) => {
                self.use_(*r);
                self.def(*r);
            }
            _ => self.ea_address(ea),
        }
    }

    /// A destination operand that is only read (`tst`, `cmpi`, `btst`).
    fn read(&mut self, ea: &Ea) {
        match ea {
            Ea::Reg(r) => self.use_(*r),
            _ => self.ea_address(ea),
        }
    }
}

/// Whether a call to `ea` is an M$ARITH entry, which writes only D0 and A0
/// (12.4 R3; `../source/domain_os/math/sau2/arith.s`).
fn is_marith(ea: &Ea) -> bool {
    matches!(ea, Ea::AbsL(s, _) if s.starts_with("M$"))
}

/// The registers `insn` reads and writes.  A call writes the scratch
/// registers it does not preserve (R3).
pub fn access(insn: &Insn) -> Access {
    let mut a = Access::default();
    match insn {
        Insn::Link { .. }
        | Insn::Unlk
        | Insn::Rts
        | Insn::MovemSave { .. }
        | Insn::MovemRestore { .. }
        | Insn::Label(_)
        | Insn::Align4
        | Insn::PoolLabel(_)
        | Insn::Words(_)
        | Insn::Encoded { .. }
        | Insn::Branch { .. } => {}
        Insn::Pea(ea) => a.ea_address(ea),
        Insn::Move { src, dst, .. } => {
            a.src(src);
            a.dst(dst);
        }
        Insn::Movea { src, dst, .. } => {
            a.src(src);
            a.def(Reg::A(*dst));
        }
        Insn::Lea { src, dst } => {
            // `lea (d,An),Am` computes from An like an operator (R2:
            // 0xE177E0 `lea (2,A1),A0` while A0 is free otherwise).  An
            // indexed `lea (d,An,Dx),Am` may take its dying base's
            // register: an element address read twice or more does where
            // that is the lowest free (XPD_$FIND_INDEX 0xE5AF60 `lea
            // (0,A0,D1.l),A0`; 129 sites against 6 that R2 fits,
            // codegen-patterns 12.4 R2, 5.2; bead domain-pascal-o1kl).
            if let Ea::Ind(b) | Ea::Disp(_, b) = src {
                if tracked(Reg::A(*b)) {
                    a.reg_operands.push(Reg::A(*b));
                }
            }
            a.ea_address(src);
            a.def(Reg::A(*dst));
        }
        Insn::Moveq { dst, .. } => a.def(Reg::D(*dst)),
        Insn::Dbf { reg, .. } => {
            a.use_(Reg::D(*reg));
            a.def(Reg::D(*reg));
        }
        Insn::Addq { dst, .. } | Insn::Subq { dst, .. } => a.dst_rmw(dst),
        Insn::Jsr(ea) => {
            a.ea_address(ea);
            let clobbered: &[Reg] = if is_marith(ea) {
                &[Reg::D(0), Reg::A(0)]
            } else {
                &[Reg::D(0), Reg::D(1), Reg::A(0), Reg::A(1)]
            };
            for &r in clobbered {
                a.def(r);
            }
        }
        Insn::Bsr { .. } => {
            for r in [Reg::D(0), Reg::D(1), Reg::A(0), Reg::A(1)] {
                a.def(r);
            }
        }
        Insn::Clr { dst, .. } => a.dst(dst),
        Insn::Btst { bit, dst } => {
            a.use_(Reg::D(*bit));
            a.read(dst);
        }
        // `s<cc> Dn` writes the low byte; the selector gives it the
        // register of the value it replaces (in place, 0xE148DE).
        Insn::Scc { dst, .. } => a.dst(dst),
        Insn::Tst { dst, .. } | Insn::Cmpi { dst, .. } | Insn::BtstImm { dst, .. } => a.read(dst),
        Insn::Cmp { src, dst, .. } => {
            a.read(src);
            a.use_(Reg::D(*dst));
        }
        Insn::CmpaZero(r) => a.use_(Reg::A(*r)),
        Insn::Cmpm { src, dst, .. } => {
            a.read(&Ea::PostInc(*src));
            a.read(&Ea::PostInc(*dst));
        }
        // Two-address operations update their destination in place: the
        // same value before and after (the selector writes them on a
        // virtual register it may modify).
        Insn::Alu { src, dst, .. } | Insn::Mul { src, dst, .. } | Insn::Div { src, dst, .. } => {
            a.src(src);
            a.dst_rmw(&Ea::Reg(Reg::D(*dst)));
        }
        Insn::AluImm { dst, .. } | Insn::Unary { dst, .. } | Insn::BitImm { dst, .. } => {
            a.dst_rmw(dst)
        }
        Insn::Bit { bit, dst, .. } => {
            a.use_(Reg::D(*bit));
            a.dst_rmw(dst);
        }
        Insn::AluToMem { src, dst, .. } => {
            a.use_(Reg::D(*src));
            a.dst_rmw(dst);
        }
        Insn::Ext { dst, .. } | Insn::Swap(dst) => a.dst_rmw(&Ea::Reg(Reg::D(*dst))),
        Insn::Shift { count, dst, .. } => {
            if let crate::insn::ShiftCount::Reg(r) = count {
                a.use_(Reg::D(*r));
            }
            a.dst_rmw(&Ea::Reg(Reg::D(*dst)));
        }
        Insn::TableIndex { reg, .. } => a.dst_rmw(&Ea::Reg(Reg::D(*reg))),
        Insn::TableJump { reg, .. } => a.use_(Reg::D(*reg)),
        Insn::TableEntry { .. } | Insn::SrImm { .. } => {}
        Insn::MoveFromSr { dst } => a.def(Reg::D(*dst)),
        Insn::MoveToSr { src } => a.read(src),
    }
    a
}

/// Apply `f` to every register `insn` names.
pub fn rename(insn: &mut Insn, f: &dyn Fn(Reg) -> Reg) {
    let fa = |n: &mut u8| {
        if let Reg::A(m) = f(Reg::A(*n)) {
            *n = m;
        }
    };
    let fd = |n: &mut u8| {
        if let Reg::D(m) = f(Reg::D(*n)) {
            *n = m;
        }
    };
    let fe = |ea: &mut Ea| match ea {
        Ea::Reg(r) => *r = f(*r),
        Ea::Ind(a) | Ea::Disp(_, a) | Ea::PreDec(a) | Ea::PostInc(a) => fa(a),
        Ea::Idx(_, a, x, _) => {
            fa(a);
            fd(x);
        }
        _ => {}
    };
    match insn {
        Insn::Link { .. }
        | Insn::Unlk
        | Insn::Rts
        | Insn::MovemSave { .. }
        | Insn::MovemRestore { .. }
        | Insn::Label(_)
        | Insn::Align4
        | Insn::PoolLabel(_)
        | Insn::Words(_)
        | Insn::Encoded { .. }
        | Insn::Branch { .. }
        | Insn::Bsr { .. }
        | Insn::TableEntry { .. }
        | Insn::SrImm { .. } => {}
        Insn::MoveFromSr { dst } => fd(dst),
        Insn::MoveToSr { src } => fe(src),
        Insn::TableIndex { reg, .. } | Insn::TableJump { reg, .. } => fd(reg),
        Insn::Pea(ea) | Insn::Jsr(ea) => fe(ea),
        Insn::Move { src, dst, .. } => {
            fe(src);
            fe(dst);
        }
        Insn::Movea { src, dst, .. } => {
            fe(src);
            fa(dst);
        }
        Insn::Lea { src, dst } => {
            fe(src);
            fa(dst);
        }
        Insn::Moveq { dst, .. } => fd(dst),
        Insn::CmpaZero(a) => fa(a),
        Insn::Cmpm { src, dst, .. } => {
            fa(src);
            fa(dst);
        }
        Insn::Dbf { reg, .. } => fd(reg),
        Insn::Addq { dst, .. }
        | Insn::Subq { dst, .. }
        | Insn::Clr { dst, .. }
        | Insn::Scc { dst, .. }
        | Insn::Tst { dst, .. }
        | Insn::Cmpi { dst, .. } => fe(dst),
        Insn::Btst { bit, dst } | Insn::Bit { bit, dst, .. } => {
            fd(bit);
            fe(dst);
        }
        Insn::Cmp { src, dst, .. }
        | Insn::Alu { src, dst, .. }
        | Insn::Mul { src, dst, .. }
        | Insn::Div { src, dst, .. } => {
            fe(src);
            fd(dst);
        }
        Insn::AluImm { dst, .. }
        | Insn::Unary { dst, .. }
        | Insn::BtstImm { dst, .. }
        | Insn::BitImm { dst, .. } => fe(dst),
        Insn::AluToMem { src, dst, .. } => {
            fd(src);
            fe(dst);
        }
        Insn::Ext { dst, .. } | Insn::Swap(dst) => fd(dst),
        Insn::Shift { count, dst, .. } => {
            if let crate::insn::ShiftCount::Reg(r) = count {
                fd(r);
            }
            fd(dst);
        }
    }
}

/// The successors of each instruction: the next one, and a branch's or
/// `dbf`'s label; `bra` only its label.  `None` stands for the end of the
/// body (the epilogue).
pub fn successors(insns: &[Insn]) -> Vec<Vec<Option<usize>>> {
    let labels: HashMap<u32, usize> = insns
        .iter()
        .enumerate()
        .filter_map(|(i, x)| match x {
            Insn::Label(l) => Some((*l, i)),
            _ => None,
        })
        .collect();
    let next = |i: usize| (i + 1 < insns.len()).then_some(i + 1);
    insns
        .iter()
        .enumerate()
        .map(|(i, x)| match x {
            Insn::Branch { cond, label, .. } => {
                let t = labels.get(label).copied();
                if *cond == crate::insn::Cond::T {
                    vec![t]
                } else {
                    vec![next(i), t]
                }
            }
            Insn::Dbf { label, .. } => vec![next(i), labels.get(label).copied()],
            // A `case` dispatch goes to its table's targets only; the
            // table's words are not executed.
            Insn::TableJump { targets, .. } => {
                targets.iter().map(|l| labels.get(l).copied()).collect()
            }
            Insn::Rts | Insn::TableEntry { .. } => vec![],
            _ => vec![next(i)],
        })
        .collect()
}

/// Live-out sets per instruction (registers, physical and virtual), with
/// `exit` live at the end of the body and each `(i, r)` of `holds` live
/// out of instruction `i` (a value held through a call, [`allocate`]).
pub fn liveness(insns: &[Insn], exit: &[Reg], holds: &[(usize, Reg)]) -> Vec<BTreeSet<RegKey>> {
    let succ = successors(insns);
    let acc: Vec<Access> = insns.iter().map(access).collect();
    let exit: BTreeSet<RegKey> = exit.iter().map(|&r| RegKey(r)).collect();
    let mut live_in: Vec<BTreeSet<RegKey>> = vec![BTreeSet::new(); insns.len()];
    let mut live_out: Vec<BTreeSet<RegKey>> = vec![BTreeSet::new(); insns.len()];
    loop {
        let mut changed = false;
        for i in (0..insns.len()).rev() {
            let mut out = BTreeSet::new();
            for s in &succ[i] {
                match s {
                    Some(j) => out.extend(live_in[*j].iter().copied()),
                    None => out.extend(exit.iter().copied()),
                }
            }
            out.extend(holds.iter().filter(|h| h.0 == i).map(|h| RegKey(h.1)));
            let mut inn = out.clone();
            for d in &acc[i].defs {
                inn.remove(&RegKey(*d));
            }
            for u in &acc[i].uses {
                inn.insert(RegKey(*u));
            }
            if out != live_out[i] || inn != live_in[i] {
                changed = true;
                live_out[i] = out;
                live_in[i] = inn;
            }
        }
        if !changed {
            return live_out;
        }
    }
}

/// Pairs `(l, p)` for [`allocate_apart`]'s `late` (codegen-patterns 12.4
/// R1b; bead domain-pascal-ue7; Uncertain, a shape fitted to the image):
/// a pointer local `l` assigned a pointer function's result (`movea.l
/// A0,l` at index `k` of `results`, right after the call and its pop)
/// gets its register before the address register `p` of a parameter
/// loaded at entry (`entry`) when `p` is live across that call and its
/// last use follows in the block of the assignment (the status test after
/// `p := f(..., status)`).  The parameter then takes the next register:
/// KBD_$PUT 0xE1CEB4/0xE1CEC6 A3/A2, KBD_$SET_KBD_TYPE 0xE7252C/0xE7253E,
/// TTY_$I_SET_RAW 0xE673B8/0xE673CC, TTY_$K_SET_FLAG 0xE67430/0xE67446,
/// KBD_$GET_CHAR_AND_MODE 0xE724CC/0xE724E0, and the AUDIT_ routine at
/// 0xE712BA (0xE712C6 A4 / 0xE712D8 A2, a result in D0, `movea.l D0,A2`,
/// which this function does not look for): every such pair in the image
/// (`tools/regalloc.py paramlocal`).  Six sites of one idiom, the condition
/// chosen after seeing them; no counterexample (codegen-patterns 12.4
/// R1b, "How much the six show").  A parameter used again after the
/// block keeps the address order: there the image goes either way (13
/// and 13 pairs).
pub fn call_results_first(
    insns: &[Insn],
    exit: &[Reg],
    holds: &[(usize, Reg)],
    results: &[usize],
    entry: &[Reg],
) -> Vec<(Reg, Reg)> {
    let live_out = liveness(insns, exit, holds);
    let mut out = Vec::new();
    for &k in results {
        let Some(Insn::Movea {
            size: Size::L,
            src: Ea::Reg(Reg::A(0)),
            dst,
        }) = insns.get(k)
        else {
            continue;
        };
        let l = Reg::A(*dst);
        if !is_virtual(l) {
            continue;
        }
        // The call: back over the caller's pop.
        let mut c = k;
        while c > 0 {
            c -= 1;
            let pops = match &insns[c] {
                Insn::Addq { dst, .. } => *dst == Ea::Reg(Reg::A(7)),
                Insn::Lea { dst, .. } => *dst == 7,
                _ => false,
            };
            if !pops {
                break;
            }
        }
        if !matches!(insns[c], Insn::Jsr(_) | Insn::Bsr { .. }) {
            continue;
        }
        // The end of the assignment's block: the last instruction before a
        // label, or the first branch.
        let mut end = k;
        while end + 1 < insns.len() {
            if matches!(insns[end + 1], Insn::Label(_)) {
                break;
            }
            end += 1;
            if matches!(
                insns[end],
                Insn::Branch { .. } | Insn::Dbf { .. } | Insn::TableJump { .. } | Insn::Rts
            ) {
                break;
            }
        }
        for &p in entry {
            if !matches!(p, Reg::A(_)) || !live_out[c].contains(&RegKey(p)) {
                continue;
            }
            let used = (k + 1..=end).any(|i| access(&insns[i]).uses.contains(&p));
            if used && !live_out[end].contains(&RegKey(p)) {
                out.push((l, p));
            }
        }
    }
    out
}

/// [`Reg`] with an order, for the sets of [`liveness`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegKey(pub Reg);

impl RegKey {
    fn key(self) -> (u8, u8) {
        match self.0 {
            Reg::D(n) => (0, n),
            Reg::A(n) => (1, n),
        }
    }
}

impl PartialOrd for RegKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RegKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// Assign a physical register to every virtual register of `insns` (see
/// the module comment), rewrite them, and return the callee-saved
/// registers the assignment uses.  `exit` is live at the end of the body
/// (the result register); each `(i, r)` of `holds` keeps `r` live out of
/// instruction `i` (12.4 R4(c): a value an argument of a call statement
/// reads is held through the call, [`crate::isel`]).  `Err` when a value
/// finds no register.
pub fn allocate(
    insns: &mut [Insn],
    exit: &[Reg],
    holds: &[(usize, Reg)],
) -> Result<RegSet, String> {
    allocate_apart(insns, exit, holds, &[], None, &[], &[], &[])
}

/// [`allocate`], with each pair of `apart` kept in different registers
/// although their live ranges need not overlap (12.4 R4(b): a by-address
/// slot loaded for one use does not take the register of a value an
/// earlier statement left in it, IIC_$RECEIVE 0xE70AC0..0xE70AD0;
/// [`crate::isel`]).
///
/// `first`: a function's result variable (a data value every assignment to
/// the result writes, [`crate::isel`]) and the values of the parameters
/// loaded at entry.  The result variable is coloured before every other
/// value, so it gets D0 when no call follows an assignment to it, and
/// then the entry loads of data parameters that do not interfere with it,
/// the one used last first, so that the parameter whose last use comes
/// latest before the result is assigned shares D0 with it (codegen-patterns
/// 12.4 R5; Uncertain, fitted to the eight sites there: PARITY_$CHK_IO
/// 0xE0B180..0xE0B18A `(8)`->D1, `(0xc)`->D0; 0xE65904, 0xE76352 and
/// 0xE76422 likewise; 0xE0B202, 0xE7630C, 0xE54B58 in parameter order;
/// PROC1_$SUSPENDP 0xE1487E `(8)`->D1 beside the result in D0).  The other
/// values follow in the usual order.
///
/// `late`: pairs `(l, p)` where `p` is coloured right after `l` instead of
/// in its own place ([`call_results_first`]).
///
/// `coalesce`: copies `(d, s)` exempt from R2: `d` may take the register
/// of `s` where `s` dies at the copy (a `for` loop's count copied from the
/// bound it tests, `move.w D2,D2` at the module-local 0xE17EC2 and
/// `move.w D0,D0` at XNS_IDP_$REGISTER_ADDR 0xE19028; codegen-patterns
/// 7.4, [`crate::isel`]).
pub fn allocate_apart(
    insns: &mut [Insn],
    exit: &[Reg],
    holds: &[(usize, Reg)],
    apart: &[(Reg, Reg)],
    first: Option<(Reg, Vec<Reg>)>,
    late: &[(Reg, Reg)],
    coalesce: &[(Reg, Reg)],
    recycle: &[(Reg, Reg)],
) -> Result<RegSet, String> {
    let web = first.as_ref().map(|(w, _)| *w);
    let live_out = liveness(insns, exit, holds);
    // Interference edges: a definition against what is live after it
    // (Chaitin), and against the register operands of its own instruction
    // (R2).
    let mut adj: HashMap<RegKey, BTreeSet<RegKey>> = HashMap::new();
    let mut edge = |a: Reg, b: Reg| {
        if a != b {
            adj.entry(RegKey(a)).or_default().insert(RegKey(b));
            adj.entry(RegKey(b)).or_default().insert(RegKey(a));
        }
    };
    let recycle: BTreeSet<(RegKey, RegKey)> = recycle
        .iter()
        .map(|&(a, b)| (RegKey(a), RegKey(b)))
        .collect();
    for &(a, b) in apart {
        if !recycle.contains(&(RegKey(a), RegKey(b))) {
            edge(a, b);
        }
    }
    // Virtual registers in the order of their first definition.
    let mut order: Vec<Reg> = Vec::new();
    for (i, insn) in insns.iter().enumerate() {
        let acc = access(insn);
        for &d in &acc.defs {
            for l in &live_out[i] {
                edge(d, l.0);
            }
            // Not for a move into a physical register (the function
            // result into D0): a value already in D0 is not copied
            // (PROC1_$GET_LOCKS 0xE148F0 loads the result into D0 itself).
            // Nor for a function's result variable (`first`): a value it
            // is assigned from may share its register when it dies there
            // (0xE15222 returns its parameter from the D0 it was loaded
            // into, 0xE15226; the tail wrapper's call result stays in D0,
            // section 3).
            if is_virtual(d) && Some(d) != web {
                for &u in &acc.reg_operands {
                    if !coalesce.contains(&(d, u)) {
                        edge(d, u);
                    }
                }
            }
            if is_virtual(d) && !order.contains(&d) {
                order.push(d);
            }
        }
        for &u in &acc.uses {
            if is_virtual(u) && !order.contains(&u) {
                return Err(format!("internal: {u} used before it is defined"));
            }
        }
    }
    if let Some((web, entry)) = first {
        if order.contains(&web) {
            let near = adj.get(&RegKey(web)).cloned().unwrap_or_default();
            let last_use = |r: Reg| {
                insns
                    .iter()
                    .rposition(|i| access(i).uses.contains(&r))
                    .unwrap_or(0)
            };
            let mut loads: Vec<(usize, Reg)> = entry
                .iter()
                .filter(|&&r| {
                    matches!(r, Reg::D(_)) && order.contains(&r) && !near.contains(&RegKey(r))
                })
                .map(|&r| (last_use(r), r))
                .collect();
            loads.sort_by(|a, b| b.0.cmp(&a.0));
            let mut lead = vec![web];
            lead.extend(loads.into_iter().map(|(_, r)| r));
            order.retain(|r| !lead.contains(r));
            lead.extend(order);
            order = lead;
        }
    }
    for &(l, p) in late {
        let (Some(il), Some(ip)) = (
            order.iter().position(|&r| r == l),
            order.iter().position(|&r| r == p),
        ) else {
            continue;
        };
        if ip < il {
            order.remove(ip);
            order.insert(il, p);
        }
    }
    // `recycle`'s pairs that are not interference edges too: kept apart
    // as long as a register is otherwise free (below).
    let mut soft: HashMap<RegKey, BTreeSet<RegKey>> = HashMap::new();
    for &(a, b) in &recycle {
        if a != b && !adj.get(&a).is_some_and(|n| n.contains(&b)) {
            soft.entry(a).or_default().insert(b);
            soft.entry(b).or_default().insert(a);
        }
    }
    let def_at = |r: Reg| insns.iter().position(|i| access(i).defs.contains(&r));
    let mut assigned: HashMap<RegKey, Reg> = HashMap::new();
    for v in order {
        let held = |n: Option<&BTreeSet<RegKey>>| -> Vec<Reg> {
            n.into_iter()
                .flatten()
                .filter_map(|u| {
                    if is_virtual(u.0) {
                        assigned.get(u).copied()
                    } else {
                        Some(u.0)
                    }
                })
                .collect()
        };
        let hard = held(adj.get(&RegKey(v)));
        let mut taken = hard.clone();
        taken.extend(held(soft.get(&RegKey(v))));
        let class: Vec<Reg> = match v {
            Reg::D(_) => ALLOC_D.iter().map(|&n| Reg::D(n)).collect(),
            Reg::A(_) => ALLOC_A.iter().map(|&n| Reg::A(n)).collect(),
        };
        let pick = class
            .iter()
            .copied()
            .find(|r| !taken.contains(r))
            .or_else(|| {
                recycled(
                    v,
                    &class,
                    &hard,
                    soft.get(&RegKey(v)),
                    &assigned,
                    insns,
                    &def_at,
                )
            });
        let Some(p) = pick else {
            return Err("more values live at once than there are registers".into());
        };
        assigned.insert(RegKey(v), p);
    }
    let map = |r: Reg| {
        if is_virtual(r) {
            assigned[&RegKey(r)]
        } else {
            r
        }
    };
    let mut saves = RegSet::EMPTY;
    for insn in insns.iter_mut() {
        rename(insn, &map);
    }
    for &p in assigned.values() {
        if callee_saved().contains(p) {
            saves.insert(p);
        }
    }
    Ok(saves)
}

/// [`allocate_apart`]'s choice for a value `v` that finds no register
/// free of its interfering values (`hard`) and of the values `recycle`
/// keeps it apart from (`soft`: for a slot load of the straight-line
/// prefix, the address values of earlier statements, 12.4 R4(b); for one
/// of an argument list longer than A0-A4, the list's other loads, R4(a);
/// [`crate::isel`]).  When the last value written to every register of
/// the class that `hard` leaves, before `v`, is such an earlier value and
/// the dead load of a parameter slot (`movea.l (d,A6),An`, d >= 8), the
/// original takes the one written least recently, which is also the one
/// whose value died first (Reported; bead domain-pascal-ei42, survey on
/// `tools/regalloc.py`'s model: 19 of 19 such loads before the first
/// branch, call or label fit both rules, and round-robin in register
/// order too; e.g. MSG_$SEND 0xE59A30 A0 and 0xE59A38 A1 after A0-A4,
/// DISK_$PV_ASSIGN_N 0xE6C87A..0xE6C894 A0, A1, A0, A1 with A2-A4 held;
/// argument lists MST_$MAP_AT 0xE42F9C A0, MSG_$SENDI 0xE59AE0..0xE59AF2
/// A0, A1, A2, A3; after a call, MSG_$SEND_HW 0xE59B6E..0xE59B8C fits
/// least recently written and died first, not round-robin).  Nothing
/// separates least recently written from died first, so a choice where
/// they differ is refused (`None`), as is one where a register's last
/// value was another kind (the original's choice not settled:
/// SIO6509_$INIT 0xE33552 takes A3, a `lea`'s, where A0 was written least
/// recently; 0xE59E4E) or is not known yet (a value defined before `v`
/// and not coloured).
fn recycled(
    v: Reg,
    class: &[Reg],
    hard: &[Reg],
    soft: Option<&BTreeSet<RegKey>>,
    assigned: &HashMap<RegKey, Reg>,
    insns: &[Insn],
    def_at: &dyn Fn(Reg) -> Option<usize>,
) -> Option<Reg> {
    let soft = soft?;
    let at_v = def_at(v)?;
    // The register each instruction before `v` writes in `v`'s class,
    // physical or as coloured (`None`: a virtual one not coloured yet).
    let mut writes: Vec<(usize, Option<Reg>, Reg)> = Vec::new();
    for (k, i) in insns[..at_v].iter().enumerate() {
        for d in access(i).defs {
            if std::mem::discriminant(&d) != std::mem::discriminant(&v) {
                continue;
            }
            let r = if is_virtual(d) {
                assigned.get(&RegKey(d)).copied()
            } else {
                Some(d)
            };
            writes.push((k, r, d));
        }
    }
    if writes.iter().any(|w| w.1.is_none()) {
        return None;
    }
    let last_use = |u: Reg| insns.iter().rposition(|i| access(i).uses.contains(&u));
    // (written at, died at, register)
    let mut cands: Vec<(usize, usize, Reg)> = Vec::new();
    for &r in class.iter().filter(|r| !hard.contains(r)) {
        // The last value written to `r` before `v`.
        let &(k, _, u) = writes.iter().rev().find(|w| w.1 == Some(r))?;
        if !is_virtual(u) || !soft.contains(&RegKey(u)) {
            return None;
        }
        let Reg::A(n) = u else {
            return None;
        };
        let slot = matches!(
            &insns[k],
            Insn::Movea { src: Ea::Disp(d, 6), dst, .. } if *dst == n && *d >= 8
        );
        if !slot {
            return None;
        }
        cands.push((k, last_use(u).unwrap_or(k), r));
    }
    let lru = cands.iter().min_by_key(|c| c.0)?;
    let died = cands.iter().min_by_key(|c| c.1)?;
    if lru.2 != died.2 || cands.iter().filter(|c| c.1 == died.1).count() > 1 {
        return None;
    }
    Some(lru.2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insn::Cond;

    const FP: u8 = 6;
    const SP: u8 = 7;

    fn d(n: u8) -> Reg {
        Reg::D(n)
    }
    fn a(n: u8) -> Reg {
        Reg::A(n)
    }
    fn vd(k: u8) -> Reg {
        Reg::D(VBASE + k)
    }
    fn va(k: u8) -> Reg {
        Reg::A(VBASE + k)
    }
    fn an(r: Reg) -> u8 {
        match r {
            Reg::A(n) => n,
            Reg::D(_) => unreachable!(),
        }
    }
    fn dn(r: Reg) -> u8 {
        match r {
            Reg::D(n) => n,
            Reg::A(_) => unreachable!(),
        }
    }

    fn load(off: i32, r: Reg) -> Insn {
        match r {
            Reg::D(_) => Insn::Move {
                size: Size::W,
                src: Ea::Disp(off, FP),
                dst: Ea::Reg(r),
            },
            Reg::A(n) => Insn::Movea {
                size: Size::L,
                src: Ea::Disp(off, FP),
                dst: n,
            },
        }
    }

    fn store(r: Reg, base: Reg, off: i32) -> Insn {
        Insn::Move {
            size: Size::W,
            src: Ea::Reg(r),
            dst: Ea::Disp(off, an(base)),
        }
    }

    fn jsr() -> Insn {
        Insn::Jsr(Ea::AbsL("X".into(), 0))
    }

    fn text(insns: &[Insn]) -> Vec<String> {
        insns.iter().map(|i| i.to_string()).collect()
    }

    #[test]
    fn virtual_numbering() {
        let mut v = VRegs::new();
        assert_eq!(v.new_reg(RegClass::Data), Some(vd(0)));
        assert_eq!(v.new_reg(RegClass::Addr), Some(va(0)));
        assert_eq!(v.new_reg(RegClass::Data), Some(vd(1)));
        assert!(is_virtual(vd(1)) && !is_virtual(d(7)) && !is_virtual(a(5)));
    }

    #[test]
    fn access_of_common_instructions() {
        // move.w D1',(4,A0'): reads both, defines nothing.
        let x = access(&store(vd(1), va(0), 4));
        assert_eq!(x.uses, [vd(1), va(0)]);
        assert!(x.defs.is_empty());
        // (A0)+ reads and writes A0; A5..A7 are not tracked.
        let x = access(&Insn::Move {
            size: Size::L,
            src: Ea::PostInc(0),
            dst: Ea::Disp(4, 5),
        });
        assert_eq!((x.uses, x.defs), (vec![a(0)], vec![a(0)]));
        // A call writes D0, D1, A0, A1; an M$ARITH call only D0, A0.
        assert_eq!(access(&jsr()).defs, [d(0), d(1), a(0), a(1)]);
        let m = access(&Insn::Jsr(Ea::AbsL("M$MIS$LLL".into(), 0)));
        assert_eq!(m.defs, [d(0), a(0)]);
        // A load through a base is not a register operand (R2); a copy is.
        assert!(access(&load(8, vd(0))).reg_operands.is_empty());
        let c = access(&Insn::Movea {
            size: Size::L,
            src: Ea::Reg(vd(0)),
            dst: VBASE,
        });
        assert_eq!(c.reg_operands, [vd(0)]);
    }

    #[test]
    fn liveness_follows_branches() {
        // v0 := (8); if ... goto L; v0 is read after the label.
        let insns = vec![
            load(8, vd(0)),
            Insn::Tst {
                size: Size::W,
                dst: Ea::Disp(10, FP),
            },
            Insn::Branch {
                cond: Cond::Eq,
                label: 1,
                short: true,
                back: false,
            },
            Insn::Clr {
                size: Size::W,
                dst: Ea::Disp(12, FP),
            },
            Insn::Label(1),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(0)),
                dst: Ea::PreDec(SP),
            },
        ];
        let live = liveness(&insns, &[], &[]);
        for (i, out) in live.iter().enumerate().take(5) {
            assert!(out.contains(&RegKey(vd(0))), "live out of {i}");
        }
        assert!(live[5].is_empty());
        // A loop: a value read in the body is live around the back edge.
        let lp = vec![
            load(8, vd(0)),
            Insn::Label(1),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(0)),
                dst: Ea::Disp(-2, FP),
            },
            Insn::Branch {
                cond: Cond::Ne,
                label: 1,
                short: true,
                back: true,
            },
        ];
        let live = liveness(&lp, &[], &[]);
        assert!(live[3].contains(&RegKey(vd(0))));
        // A `case` dispatch: live into every table target, not into the
        // table's words.
        let tbl = vec![
            load(8, vd(0)),
            load(10, vd(1)),
            Insn::TableIndex {
                reg: dn(vd(1)),
                table: 9,
            },
            Insn::TableJump {
                reg: dn(vd(1)),
                table: 9,
                targets: vec![1, 2],
            },
            Insn::Label(9),
            Insn::TableEntry {
                table: 9,
                target: 1,
                back: false,
            },
            Insn::Label(1),
            Insn::Clr {
                size: Size::W,
                dst: Ea::Disp(12, FP),
            },
            Insn::Label(2),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(0)),
                dst: Ea::Disp(-2, FP),
            },
        ];
        let live = liveness(&tbl, &[], &[]);
        assert!(live[2].contains(&RegKey(vd(1))));
        assert!(live[3].contains(&RegKey(vd(0))));
        assert!(!live[3].contains(&RegKey(vd(1))));
        assert!(live[4].is_empty() && live[5].is_empty());
        assert_eq!(successors(&tbl)[3], vec![Some(6), Some(8)]);
        // The result register is live at the end.
        let live = liveness(&[load(8, d(0))], &[d(0)], &[]);
        assert!(live[0].contains(&RegKey(d(0))));
    }

    /// TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A: D0, D1, A0 in order.
    #[test]
    fn lowest_free_in_definition_order() {
        let mut insns = vec![
            load(8, vd(0)),
            load(10, vd(1)),
            load(12, va(0)),
            store(vd(0), va(0), 8),
            store(vd(1), va(0), 10),
        ];
        let saves = allocate(&mut insns, &[], &[]).unwrap();
        assert!(saves.is_empty());
        assert_eq!(
            text(&insns[..3]),
            [
                "move.w\t(8,%fp),%d0",
                "move.w\t(0xa,%fp),%d1",
                "movea.l\t(0xc,%fp),%a0"
            ]
        );
    }

    /// KBD_$PUT 0xE1CEB4..0xE1CEC8 (12.4 R1b): the status address, live
    /// across the call and last used by the test in the block of the
    /// pointer local's assignment, is coloured after the local: the local
    /// in A2, the status in A3.  Used again after the block, it keeps the
    /// address order.
    #[test]
    fn call_result_before_dying_parameter() {
        let body = |reuse: bool| {
            let mut b = vec![
                load(0x18, va(0)),
                Insn::Pea(Ea::Ind(an(va(0)))),
                jsr(),
                Insn::Addq {
                    size: Size::W,
                    value: 4,
                    dst: Ea::Reg(a(SP)),
                },
                Insn::Movea {
                    size: Size::L,
                    src: Ea::Reg(a(0)),
                    dst: an(va(1)),
                },
                Insn::Tst {
                    size: Size::L,
                    dst: Ea::Ind(an(va(0))),
                },
                Insn::Branch {
                    cond: Cond::Ne,
                    label: 1,
                    short: true,
                    back: false,
                },
                Insn::Pea(Ea::Ind(an(va(1)))),
            ];
            if reuse {
                b.push(Insn::Pea(Ea::Ind(an(va(0)))));
            }
            b.push(jsr());
            // The local live across the call (the original holds it
            // through its last call, 12.4 R4(c)).
            b.push(Insn::Tst {
                size: Size::L,
                dst: Ea::Ind(an(va(1))),
            });
            b.push(Insn::Label(1));
            b
        };
        let mut insns = body(false);
        let after = call_results_first(&insns, &[], &[], &[4], &[va(0)]);
        assert_eq!(after, [(va(1), va(0))]);
        allocate_apart(&mut insns, &[], &[], &[], None, &after, &[], &[]).unwrap();
        let t = text(&insns);
        assert_eq!(t[0], "movea.l\t(0x18,%fp),%a3");
        assert_eq!(t[4], "movea.l\t%a0,%a2");
        // Without the pairs: address order.
        let mut plain = body(false);
        allocate(&mut plain, &[], &[]).unwrap();
        assert_eq!(text(&plain)[0], "movea.l\t(0x18,%fp),%a2");
        // The status used again after the block: no pair.
        let insns = body(true);
        assert!(call_results_first(&insns, &[], &[], &[4], &[va(0)]).is_empty());
    }

    /// IIC_$RECEIVE 0xE70AC0..0xE70AD0: values kept apart (12.4 R4(b)) take
    /// new registers although the earlier ones are dead, A2 included.
    #[test]
    fn values_kept_apart() {
        let clr = |a: Reg| Insn::Clr {
            size: Size::W,
            dst: Ea::Ind(an(a)),
        };
        let body = vec![
            load(0x1c, va(0)),
            clr(va(0)),
            load(0x18, va(1)),
            clr(va(1)),
            load(0x10, va(2)),
            clr(va(2)),
        ];
        let mut plain = body.clone();
        allocate(&mut plain, &[], &[]).unwrap();
        assert!(text(&plain).iter().all(|l| !l.contains("%a1")));
        let mut insns = body;
        let apart = [(va(1), va(0)), (va(2), va(0)), (va(2), va(1))];
        let saves = allocate_apart(&mut insns, &[], &[], &apart, None, &[], &[], &[]).unwrap();
        assert_eq!(
            text(&insns),
            [
                "movea.l\t(0x1c,%fp),%a0",
                "clr.w\t(%a0)",
                "movea.l\t(0x18,%fp),%a1",
                "clr.w\t(%a1)",
                "movea.l\t(0x10,%fp),%a2",
                "clr.w\t(%a2)",
            ]
        );
        assert!(saves.contains(a(2)));
    }

    /// 0xE17778: a parameter's register is reused after its last use, and
    /// a third simultaneous value takes D2 without any call (0xE177A0).
    #[test]
    fn registers_free_after_last_use() {
        let mut insns = vec![
            load(8, vd(0)),
            load(10, vd(1)),
            load(12, vd(2)), // D0, D1 live: D2
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(0)),
                dst: Ea::Disp(-2, FP),
            },
            load(14, vd(3)), // D0 dead now: D0
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(1)),
                dst: Ea::Reg(vd(3)),
            },
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(2)),
                dst: Ea::Disp(-4, FP),
            },
        ];
        let saves = allocate(&mut insns, &[], &[]).unwrap();
        let regs: Vec<String> = insns
            .iter()
            .map(|i| i.to_string().rsplit(',').next().unwrap().to_string())
            .collect();
        assert_eq!(&regs[..3], ["%d0", "%d1", "%d2"]);
        assert_eq!(insns[4].to_string(), "move.w\t(0xe,%fp),%d0");
        // D2 is callee-saved: saved although no call is made.
        assert_eq!(saves, RegSet::of(&[d(2)]));
    }

    /// R1 looks at the whole live range, not only the definition point: a
    /// register that is free where the value is defined but taken later
    /// in its range is not chosen.
    #[test]
    fn free_over_the_whole_range() {
        // v0 is defined first and gets D0; v1 is defined while v0 is live
        // (D1); v2 is defined after v0's last use but is live while a
        // physical D0 (the result) is defined: not D0.
        let mut insns = vec![
            load(8, vd(0)),
            load(10, vd(1)),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(0)),
                dst: Ea::Disp(-2, FP),
            },
            load(12, vd(2)),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(1)),
                dst: Ea::Reg(d(0)),
            },
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(2)),
                dst: Ea::Disp(-4, FP),
            },
        ];
        allocate(&mut insns, &[d(0)], &[]).unwrap();
        assert_eq!(insns[3].to_string(), "move.w\t(0xc,%fp),%d2");
    }

    /// R3: a value live across a call takes a callee-saved register
    /// (module-local 0xE32B76, 0xE32B7E/0xE32B82 A2, D2); one that dies before the
    /// call takes a scratch register.
    #[test]
    fn values_across_calls_are_callee_saved() {
        let mut insns = vec![
            load(8, va(0)),
            load(12, vd(0)),
            load(14, vd(1)),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(1)),
                dst: Ea::PreDec(SP),
            },
            jsr(),
            store(vd(0), va(0), 0x34),
        ];
        let saves = allocate(&mut insns, &[], &[]).unwrap();
        assert_eq!(
            text(&insns[..3]),
            [
                "movea.l\t(8,%fp),%a2",
                "move.w\t(0xc,%fp),%d2",
                "move.w\t(0xe,%fp),%d0"
            ]
        );
        assert_eq!(saves, RegSet::of(&[d(2), a(2)]));
        // Across an M$ARITH call D1 and A1 survive.
        let mut insns = vec![
            load(8, va(0)),
            load(10, va(1)),
            Insn::Jsr(Ea::AbsL("M$OIU$WLW".into(), 0)),
            store(vd(0), va(1), 0),
        ];
        insns.insert(2, load(12, vd(0)));
        insns.insert(3, store(vd(0), va(0), 0));
        let saves = allocate(&mut insns, &[], &[]).unwrap();
        assert_eq!(insns[1].to_string(), "movea.l\t(0xa,%fp),%a1");
        assert!(saves.is_empty());
    }

    /// R4(c): a value held through a call (`holds`) takes a callee-saved
    /// register although its last use is before the call (PROC1_$SET_ASID
    /// 0xE148FE D2, pushed at 0xE1490E).
    #[test]
    fn held_values_are_callee_saved() {
        let body = || {
            vec![
                load(8, vd(0)),
                Insn::Move {
                    size: Size::W,
                    src: Ea::Reg(vd(0)),
                    dst: Ea::PreDec(SP),
                },
                jsr(),
            ]
        };
        let mut insns = body();
        assert!(allocate(&mut insns, &[], &[]).unwrap().is_empty());
        assert_eq!(insns[0].to_string(), "move.w\t(8,%fp),%d0");
        let mut insns = body();
        let saves = allocate(&mut insns, &[], &[(2, vd(0))]).unwrap();
        assert_eq!(insns[0].to_string(), "move.w\t(8,%fp),%d2");
        assert_eq!(saves, RegSet::of(&[d(2)]));
    }

    /// R2: no coalescing of a copy with its dying source; a load through a
    /// dying base may reuse it.
    #[test]
    fn copies_are_not_coalesced() {
        // 0xE177DA..0xE177E0: movea.l D3,A1 ... lea (2,A1),A0: the `lea`
        // result does not take its dying base's register.
        let mut insns = vec![
            load(8, vd(0)),
            Insn::Movea {
                size: Size::L,
                src: Ea::Reg(vd(0)),
                dst: an(va(0)),
            },
            Insn::Lea {
                src: Ea::Disp(2, an(va(0))),
                dst: an(va(1)),
            },
            Insn::Clr {
                size: Size::W,
                dst: Ea::Ind(an(va(1))),
            },
        ];
        allocate(&mut insns, &[], &[]).unwrap();
        assert_eq!(insns[1].to_string(), "movea.l\t%d0,%a0");
        assert_eq!(insns[2].to_string(), "lea\t(2,%a0),%a1");
        // 0xE177D0 `move.l D1,D3`: a data copy at its source's last use.
        let mut insns = vec![
            load(8, vd(0)),
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(0)),
                dst: Ea::Reg(vd(1)),
            },
            Insn::Move {
                size: Size::W,
                src: Ea::Reg(vd(1)),
                dst: Ea::Disp(-2, FP),
            },
        ];
        allocate(&mut insns, &[], &[]).unwrap();
        assert_eq!(insns[1].to_string(), "move.w\t%d0,%d1");

        let mut chain = vec![
            load(8, va(0)),
            Insn::Movea {
                size: Size::L,
                src: Ea::Disp(4, an(va(0))),
                dst: an(va(1)),
            },
            Insn::Clr {
                size: Size::W,
                dst: Ea::Ind(an(va(1))),
            },
        ];
        allocate(&mut chain, &[], &[]).unwrap();
        assert_eq!(chain[1].to_string(), "movea.l\t(4,%a0),%a0");
    }

    /// A copy exempt from R2 (`coalesce`) may take its dying source's
    /// register: a `for` loop's count copied from its tested bound,
    /// XNS_IDP_$REGISTER_ADDR 0xE19020..0xE19028 `move.w (1336,A5),D0;
    /// tst.w D0; bmi.s; move.w D0,D0`.
    #[test]
    fn coalesced_copy() {
        let copy = || {
            vec![
                load(8, vd(0)),
                Insn::Move {
                    size: Size::W,
                    src: Ea::Reg(vd(0)),
                    dst: Ea::Reg(vd(1)),
                },
                Insn::Move {
                    size: Size::W,
                    src: Ea::Reg(vd(1)),
                    dst: Ea::Disp(-2, FP),
                },
            ]
        };
        let mut insns = copy();
        allocate_apart(&mut insns, &[], &[], &[], None, &[], &[(vd(1), vd(0))], &[]).unwrap();
        assert_eq!(insns[1].to_string(), "move.w\t%d0,%d0");
        // Without the exemption, R2 keeps them apart.
        let mut insns = copy();
        allocate_apart(&mut insns, &[], &[], &[], None, &[], &[], &[]).unwrap();
        assert_eq!(insns[1].to_string(), "move.w\t%d0,%d1");
    }

    /// A two-address result stays in its operand's register (0xE148DE
    /// `sne D1`).
    #[test]
    fn in_place_results() {
        let mut insns = vec![
            load(8, vd(0)),
            Insn::Move {
                size: Size::L,
                src: Ea::Disp(0x40, FP),
                dst: Ea::Reg(vd(1)),
            },
            Insn::Btst {
                bit: dn(vd(0)),
                dst: Ea::Reg(vd(1)),
            },
            Insn::Scc {
                cond: Cond::Ne,
                dst: Ea::Reg(vd(1)),
            },
            Insn::Move {
                size: Size::B,
                src: Ea::Reg(vd(1)),
                dst: Ea::Reg(d(0)),
            },
        ];
        allocate(&mut insns, &[d(0)], &[]).unwrap();
        assert_eq!(
            text(&insns[2..]),
            ["btst\t%d0,%d1", "sne\t%d1", "move.b\t%d1,%d0"]
        );
    }

    #[test]
    fn running_out_of_registers_is_an_error() {
        let mut insns: Vec<Insn> = (0..6).map(|k| load(8 + 4 * k as i32, va(k))).collect();
        for k in 0..6 {
            insns.push(Insn::Clr {
                size: Size::W,
                dst: Ea::Ind(an(va(k))),
            });
        }
        assert!(allocate(&mut insns, &[], &[]).is_err());
    }
}
