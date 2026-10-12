//! m68k instructions as the selector produces them, and their gas text.
//!
//! Syntax: `m68k-elf-as -m68010` (tools/conform.py), register names with
//! `%`, the frame pointer spelled `%fp` (A6), Motorola-style operands
//! `(d,%an)`, as in the hand fixtures under tests/conformance/handasm/.
//! Every size is explicit (`link.w`, `move.l`, absolute operands
//! `(SYM).l`), so gas never chooses an encoding the original did not use
//! (CLAUDE.md 4.4; codegen-patterns 13).

use std::fmt;

/// A register.  `A(6)` is the frame pointer, `A(7)` the stack pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg {
    D(u8),
    A(u8),
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reg::D(n) => write!(f, "%d{n}"),
            Reg::A(6) => f.write_str("%fp"),
            Reg::A(7) => f.write_str("%sp"),
            Reg::A(n) => write!(f, "%a{n}"),
        }
    }
}

/// A set of registers, as `movem` names them.
///
/// Bit `n` is Dn, bit `8 + n` is An: the order of the `movem` mask for
/// the control and postincrement modes (`movem.l (d,A6),<list>`); the
/// predecrement mode (`movem.l <list>,-(SP)`) uses the reverse order
/// (MC68000 PRM, MOVEM).  Image: 0xE1777C `48e7 3020` saves D2-D3/A2,
/// 0xE177F6 `4cee 040c ffe4` restores them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RegSet(u16);

impl RegSet {
    pub const EMPTY: RegSet = RegSet(0);

    fn bit(r: Reg) -> u16 {
        match r {
            Reg::D(n) => {
                debug_assert!(n < 8);
                1 << n
            }
            Reg::A(n) => {
                debug_assert!(n < 8);
                1 << (8 + n)
            }
        }
    }

    pub fn of(regs: &[Reg]) -> RegSet {
        let mut s = RegSet::EMPTY;
        for &r in regs {
            s.insert(r);
        }
        s
    }

    pub fn insert(&mut self, r: Reg) {
        self.0 |= Self::bit(r);
    }

    pub fn remove(&mut self, r: Reg) {
        self.0 &= !Self::bit(r);
    }

    pub fn contains(self, r: Reg) -> bool {
        self.0 & Self::bit(r) != 0
    }

    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn union(self, other: RegSet) -> RegSet {
        RegSet(self.0 | other.0)
    }

    pub fn intersection(self, other: RegSet) -> RegSet {
        RegSet(self.0 & other.0)
    }

    /// The registers in mask order: D0..D7, then A0..A7.
    pub fn iter(self) -> impl Iterator<Item = Reg> {
        (0..16u8).filter(move |b| self.0 & (1 << b) != 0).map(|b| {
            if b < 8 {
                Reg::D(b)
            } else {
                Reg::A(b - 8)
            }
        })
    }

    /// The mask word of `movem` in the control and postincrement modes.
    pub fn mask_control(self) -> u16 {
        self.0
    }

    /// The mask word of `movem` in the predecrement mode.
    pub fn mask_predec(self) -> u16 {
        self.0.reverse_bits()
    }
}

/// The gas register list: runs of consecutive registers of one kind as
/// `%d2-%d3`, joined with `/`, data registers first (`%d2-%d7/%a2-%a4`).
impl fmt::Display for RegSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for kind in 0..2u8 {
            let reg = |n: u8| if kind == 0 { Reg::D(n) } else { Reg::A(n) };
            let mut n = 0u8;
            while n < 8 {
                if !self.contains(reg(n)) {
                    n += 1;
                    continue;
                }
                let start = n;
                while n + 1 < 8 && self.contains(reg(n + 1)) {
                    n += 1;
                }
                if !first {
                    f.write_str("/")?;
                }
                first = false;
                if start == n {
                    write!(f, "{}", reg(start))?;
                } else {
                    write!(f, "{}-{}", reg(start), reg(n))?;
                }
                n += 1;
            }
        }
        Ok(())
    }
}

/// Operation size suffix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    B,
    W,
    L,
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Size::B => "b",
            Size::W => "w",
            Size::L => "l",
        })
    }
}

/// An effective address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ea {
    /// Data or address register direct.
    Reg(Reg),
    /// `(An)`.
    Ind(u8),
    /// `(d,An)`, d16.
    Disp(i32, u8),
    /// `(sym+off).l`: absolute long.
    AbsL(String, i32),
    /// `(n).w`: absolute short, a 16-bit address the CPU sign-extends.
    /// As the operand of `pea` it pushes the long constant `n`
    /// (codegen-patterns 1.5: 0xE00EB4 `pea (0x16).w`, `4878 0016`).
    AbsW(i32),
    /// `#sym`: the address of a symbol as a 32-bit immediate (0xE00EB8
    /// `move.l #AST_$ZERO_BUFF,-(SP)`, `2f3c 00ff 8c00`).
    ImmSym(String),
    /// `-(An)`.
    PreDec(u8),
    /// `(An)+`.
    PostInc(u8),
    /// `#n`: an immediate.  The selector gives the number as the
    /// instruction size reads it: a byte immediate is 0..=255, so that gas
    /// writes the extension word the way the original did (`move.b
    /// #0xff,(A0)` is `10bc 00ff` at 0xE0A33E; gas encodes `#-1` as
    /// `10bc ffff`).
    Imm(i64),
}

impl Ea {
    /// Bytes of extension words the operand adds to an instruction whose
    /// operation size is `size` (MC68000 PRM, effective addressing).
    pub fn ext_len(&self, size: Size) -> u32 {
        match self {
            Ea::Reg(_) | Ea::Ind(_) | Ea::PreDec(_) | Ea::PostInc(_) => 0,
            // gas assembles `(0,%an)` as `(%an)`, with no extension word.
            // The selector writes `Ea::Ind` for offset 0 anyway; this keeps
            // `Insn::len_at` equal to what gas emits if one slips through.
            Ea::Disp(0, _) => 0,
            Ea::Disp(..) | Ea::AbsW(_) => 2,
            Ea::AbsL(..) | Ea::ImmSym(_) => 4,
            Ea::Imm(_) => match size {
                Size::B | Size::W => 2,
                Size::L => 4,
            },
        }
    }
}

/// A displacement or immediate: small values in decimal, others in hex.
pub(crate) fn num(n: i64) -> String {
    if (-9..=9).contains(&n) {
        n.to_string()
    } else if n < 0 {
        format!("-0x{:x}", -n)
    } else {
        format!("0x{n:x}")
    }
}

impl fmt::Display for Ea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ea::Reg(r) => write!(f, "{r}"),
            Ea::Ind(a) => write!(f, "({})", Reg::A(*a)),
            Ea::Disp(d, a) => write!(f, "({},{})", num(*d as i64), Reg::A(*a)),
            Ea::AbsL(s, 0) => write!(f, "({s}).l"),
            Ea::AbsL(s, o) if *o < 0 => write!(f, "({s}-{}).l", num(-(*o as i64))),
            Ea::AbsL(s, o) => write!(f, "({s}+{}).l", num(*o as i64)),
            Ea::AbsW(n) => write!(f, "({}).w", num(*n as i64)),
            Ea::ImmSym(s) => write!(f, "#{s}"),
            Ea::PreDec(a) => write!(f, "-({})", Reg::A(*a)),
            Ea::PostInc(a) => write!(f, "({})+", Reg::A(*a)),
            Ea::Imm(n) => write!(f, "#{}", num(*n)),
        }
    }
}

/// A condition code, for `Scc` and `Bcc` (MC68000 PRM, condition tests).
/// [`Cond::T`] is "always": `st`, and `bra` as a branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    /// Always (`st`, `bra`).
    T,
    /// Unsigned higher.
    Hi,
    /// Unsigned lower or same.
    Ls,
    /// Carry clear: unsigned higher or same (gas also spells it `hs`).
    Cc,
    /// Carry set: unsigned lower (`lo`).
    Cs,
    Ne,
    Eq,
    Pl,
    Mi,
    Ge,
    Lt,
    Gt,
    Le,
}

impl Cond {
    /// The condition that holds exactly when `self` does not.  [`Cond::T`]
    /// has none (no "never" branch is emitted).
    pub fn negate(self) -> Cond {
        match self {
            Cond::T => panic!("no negation of `always`"),
            Cond::Hi => Cond::Ls,
            Cond::Ls => Cond::Hi,
            Cond::Cc => Cond::Cs,
            Cond::Cs => Cond::Cc,
            Cond::Ne => Cond::Eq,
            Cond::Eq => Cond::Ne,
            Cond::Pl => Cond::Mi,
            Cond::Mi => Cond::Pl,
            Cond::Ge => Cond::Lt,
            Cond::Lt => Cond::Ge,
            Cond::Gt => Cond::Le,
            Cond::Le => Cond::Gt,
        }
    }
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cond::T => "t",
            Cond::Hi => "hi",
            Cond::Ls => "ls",
            Cond::Cc => "cc",
            Cond::Cs => "cs",
            Cond::Ne => "ne",
            Cond::Eq => "eq",
            Cond::Pl => "pl",
            Cond::Mi => "mi",
            Cond::Ge => "ge",
            Cond::Lt => "lt",
            Cond::Gt => "gt",
            Cond::Le => "le",
        })
    }
}

/// One instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Insn {
    /// `link.w %fp,#disp`.
    Link {
        disp: i32,
    },
    Unlk,
    Rts,
    /// `movem.l <regs>,-(%sp)`: the prologue's register save.
    MovemSave {
        regs: RegSet,
    },
    /// `movem.l (disp,%fp),<regs>`: the epilogue's restore from the frame.
    MovemRestore {
        regs: RegSet,
        disp: i32,
    },
    /// `pea <ea>`.
    Pea(Ea),
    /// `move.<size> src,dst`.
    Move {
        size: Size,
        src: Ea,
        dst: Ea,
    },
    /// `movea.<size> src,%an`.
    Movea {
        size: Size,
        src: Ea,
        dst: u8,
    },
    /// `lea <src>,%an`.
    Lea {
        src: Ea,
        dst: u8,
    },
    /// `moveq #value,%dn` (-128..=127).
    Moveq {
        value: i8,
        dst: u8,
    },
    /// `dbf %dn,<label>b`: decrement the word in Dn, branch back to the
    /// numbered local label while it is not -1.
    Dbf {
        reg: u8,
        label: u32,
    },
    /// A numbered local label (`N:`, gas local label; branches name it
    /// `Nb`).  The emitter writes it at the start of the line.
    Label(u32),
    /// `addq.<size> #value,dst` (1..=8).
    Addq {
        size: Size,
        value: u8,
        dst: Ea,
    },
    /// `subq.<size> #value,dst` (1..=8).
    Subq {
        size: Size,
        value: u8,
        dst: Ea,
    },
    /// `jsr <ea>`.
    Jsr(Ea),
    /// `bsr.s` (`short`) or `bsr.w` to a routine of the same section.
    /// The size is decided by the emitter's layout (`emit.rs`): `.s`
    /// whenever the target is in reach, as the original (1800 `bsr.w`,
    /// all out of `.s` reach; 121 `bsr.s`, all in reach; compiled code
    /// 0xE00800..0xE88834).
    Bsr {
        target: String,
        short: bool,
    },
    /// `clr.<size> dst`.
    Clr {
        size: Size,
        dst: Ea,
    },
    /// `btst %dn,<dst>` (register form: long, bit mod 32).
    Btst {
        bit: u8,
        dst: Ea,
    },
    /// `s<cc> <dst>`.
    Scc {
        cond: Cond,
        dst: Ea,
    },
    /// `b<cc>.s` / `b<cc>.w` (`bra` for [`Cond::T`]) to the numbered local
    /// label `label` of the same routine.  `back`: the label is defined
    /// before the branch (gas `Nb`, else `Nf`).  The size is decided by
    /// the emitter's layout (`emit.rs`, `size_branches`): `.s` whenever
    /// the target is in reach, as the original (compiled code
    /// 0xE00800..0xE88834: 11515 `.s` branches, all in reach; 2793 `.w`,
    /// all but 2 out of reach, 0xE7065C and 0xE706DE).
    Branch {
        cond: Cond,
        label: u32,
        short: bool,
        back: bool,
    },
    /// `tst.<size> dst`.
    Tst {
        size: Size,
        dst: Ea,
    },
    /// `cmpi.<size> #imm,dst` (the immediate as [`Ea::Imm`] gives it).
    Cmpi {
        size: Size,
        imm: i64,
        dst: Ea,
    },
    /// `cmp.<size> src,%dn`.
    Cmp {
        size: Size,
        src: Ea,
        dst: u8,
    },
    /// Pseudo-instruction: pad to a 4-byte boundary with the 2-byte no-op
    /// `movea.l A0,A0` (0x2048), as the compiler does before a loop head
    /// (codegen-patterns 7: 0xE00ED6 before the `dbf` loop at 0xE00ED8;
    /// PKT_$RECENTLY_MISSING 0xE128DA, a routine starting at 2 mod 4).
    /// Emitted as `.balignw 4,0x2048`; the boundary is absolute because
    /// the module's code base is 4-aligned, or, for a fragment, because
    /// the emitter shifts the section (`emit::Options::fragment_origin`).
    Align4,
}

impl fmt::Display for Insn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Insn::Link { disp } => write!(f, "link.w\t%fp,#{}", num(*disp as i64)),
            Insn::Unlk => f.write_str("unlk\t%fp"),
            Insn::Rts => f.write_str("rts"),
            Insn::MovemSave { regs } => write!(f, "movem.l\t{regs},-(%sp)"),
            Insn::MovemRestore { regs, disp } => {
                write!(f, "movem.l\t({},%fp),{regs}", num(*disp as i64))
            }
            Insn::Pea(ea) => write!(f, "pea\t{ea}"),
            Insn::Move { size, src, dst } => write!(f, "move.{size}\t{src},{dst}"),
            Insn::Movea { size, src, dst } => {
                write!(f, "movea.{size}\t{src},{}", Reg::A(*dst))
            }
            Insn::Lea { src, dst } => write!(f, "lea\t{src},{}", Reg::A(*dst)),
            Insn::Moveq { value, dst } => {
                write!(f, "moveq\t#{},{}", num(*value as i64), Reg::D(*dst))
            }
            Insn::Dbf { reg, label } => write!(f, "dbf\t{},{label}b", Reg::D(*reg)),
            Insn::Label(n) => write!(f, "{n}:"),
            Insn::Addq { size, value, dst } => write!(f, "addq.{size}\t#{value},{dst}"),
            Insn::Subq { size, value, dst } => write!(f, "subq.{size}\t#{value},{dst}"),
            Insn::Jsr(ea) => write!(f, "jsr\t{ea}"),
            Insn::Bsr { target, short } => {
                write!(f, "bsr.{}\t{target}", if *short { "s" } else { "w" })
            }
            Insn::Clr { size, dst } => write!(f, "clr.{size}\t{dst}"),
            Insn::Btst { bit, dst } => write!(f, "btst\t{},{dst}", Reg::D(*bit)),
            Insn::Scc { cond, dst } => write!(f, "s{cond}\t{dst}"),
            Insn::Align4 => f.write_str(".balignw\t4,0x2048"),
            Insn::Branch {
                cond,
                label,
                short,
                back,
            } => {
                let mn = match cond {
                    Cond::T => "bra".to_string(),
                    c => format!("b{c}"),
                };
                let size = if *short { "s" } else { "w" };
                let dir = if *back { "b" } else { "f" };
                write!(f, "{mn}.{size}\t{label}{dir}")
            }
            Insn::Tst { size, dst } => write!(f, "tst.{size}\t{dst}"),
            Insn::Cmpi { size, imm, dst } => {
                write!(f, "cmpi.{size}\t#{},{dst}", num(*imm))
            }
            Insn::Cmp { size, src, dst } => write!(f, "cmp.{size}\t{src},{}", Reg::D(*dst)),
        }
    }
}

impl Insn {
    /// The encoded length in bytes of the instruction at byte position
    /// `pos` (only [`Insn::Align4`] depends on it: 2 at 2 mod 4, else 0;
    /// positions are always even).  MC68000 PRM encodings.
    pub fn len_at(&self, pos: u32) -> u32 {
        match self {
            Insn::Label(_) => 0,
            Insn::Align4 => (4 - pos % 4) % 4,
            Insn::Unlk | Insn::Rts | Insn::Moveq { .. } => 2,
            Insn::Link { .. } | Insn::Dbf { .. } | Insn::MovemSave { .. } => 4,
            Insn::MovemRestore { .. } => 6,
            Insn::Bsr { short, .. } => {
                if *short {
                    2
                } else {
                    4
                }
            }
            Insn::Pea(ea) | Insn::Jsr(ea) | Insn::Lea { src: ea, .. } => 2 + ea.ext_len(Size::L),
            Insn::Move { size, src, dst } => 2 + src.ext_len(*size) + dst.ext_len(*size),
            Insn::Movea { size, src, .. } => 2 + src.ext_len(*size),
            Insn::Addq { size, dst, .. }
            | Insn::Subq { size, dst, .. }
            | Insn::Clr { size, dst } => 2 + dst.ext_len(*size),
            Insn::Btst { dst, .. } | Insn::Scc { dst, .. } => 2 + dst.ext_len(Size::B),
            Insn::Branch { short, .. } => {
                if *short {
                    2
                } else {
                    4
                }
            }
            Insn::Tst { size, dst } => 2 + dst.ext_len(*size),
            Insn::Cmpi { size, dst, .. } => 2 + Ea::Imm(0).ext_len(*size) + dst.ext_len(*size),
            Insn::Cmp { size, src, .. } => 2 + src.ext_len(*size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FP_FOR_TEST: u8 = 6;

    #[test]
    fn renders_gas_syntax() {
        let cases: Vec<(Insn, &str)> = vec![
            (Insn::Link { disp: -4 }, "link.w\t%fp,#-4"),
            (Insn::Link { disp: -0x20 }, "link.w\t%fp,#-0x20"),
            (
                Insn::Movea {
                    size: Size::L,
                    src: Ea::AbsL("PROC1_$CURRENT_PCB".into(), 0),
                    dst: 0,
                },
                "movea.l\t(PROC1_$CURRENT_PCB).l,%a0",
            ),
            (
                Insn::Move {
                    size: Size::W,
                    src: Ea::Disp(8, 6),
                    dst: Ea::Reg(Reg::D(0)),
                },
                "move.w\t(8,%fp),%d0",
            ),
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::Disp(0x40, 0),
                    dst: Ea::Reg(Reg::D(1)),
                },
                "move.l\t(0x40,%a0),%d1",
            ),
            (
                Insn::Move {
                    size: Size::B,
                    src: Ea::Ind(1),
                    dst: Ea::AbsL("X".into(), -12),
                },
                "move.b\t(%a1),(X-0xc).l",
            ),
            (
                Insn::Btst {
                    bit: 0,
                    dst: Ea::Reg(Reg::D(1)),
                },
                "btst\t%d0,%d1",
            ),
            (
                Insn::Scc {
                    cond: Cond::Ne,
                    dst: Ea::Reg(Reg::D(1)),
                },
                "sne\t%d1",
            ),
            (
                Insn::MovemSave {
                    regs: RegSet::of(&[Reg::D(2), Reg::D(3), Reg::A(2)]),
                },
                "movem.l\t%d2-%d3/%a2,-(%sp)",
            ),
            (
                Insn::MovemRestore {
                    regs: RegSet::of(&[Reg::D(2), Reg::D(3), Reg::A(2)]),
                    disp: -28,
                },
                "movem.l\t(-0x1c,%fp),%d2-%d3/%a2",
            ),
            (Insn::Pea(Ea::Ind(5)), "pea\t(%a5)"),
            (
                Insn::Clr {
                    size: Size::L,
                    dst: Ea::Disp(4, 0),
                },
                "clr.l\t(4,%a0)",
            ),
            (
                Insn::Move {
                    size: Size::B,
                    src: Ea::Imm(0xff),
                    dst: Ea::Ind(0),
                },
                "move.b\t#0xff,(%a0)",
            ),
            (
                Insn::Move {
                    size: Size::W,
                    src: Ea::Imm(-1),
                    dst: Ea::Disp(0x14, 2),
                },
                "move.w\t#-1,(0x14,%a2)",
            ),
            (
                Insn::Scc {
                    cond: Cond::T,
                    dst: Ea::Disp(0xe0, 5),
                },
                "st\t(0xe0,%a5)",
            ),
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::Reg(Reg::D(2)),
                    dst: Ea::PreDec(7),
                },
                "move.l\t%d2,-(%sp)",
            ),
            (
                Insn::Lea {
                    src: Ea::Ind(5),
                    dst: 0,
                },
                "lea\t(%a5),%a0",
            ),
            (
                Insn::Lea {
                    src: Ea::Disp(-0x98, FP_FOR_TEST),
                    dst: 1,
                },
                "lea\t(-0x98,%fp),%a1",
            ),
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::PostInc(0),
                    dst: Ea::PostInc(1),
                },
                "move.l\t(%a0)+,(%a1)+",
            ),
            (Insn::Moveq { value: 35, dst: 0 }, "moveq\t#0x23,%d0"),
            (Insn::Dbf { reg: 0, label: 1 }, "dbf\t%d0,1b"),
            (Insn::Label(1), "1:"),
            (Insn::Unlk, "unlk\t%fp"),
            (Insn::Rts, "rts"),
            (Insn::Align4, ".balignw\t4,0x2048"),
            (Insn::Pea(Ea::AbsW(0x16)), "pea\t(0x16).w"),
            (Insn::Pea(Ea::AbsW(-1)), "pea\t(-1).w"),
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::ImmSym("AST_$ZERO_BUFF".into()),
                    dst: Ea::PreDec(7),
                },
                "move.l\t#AST_$ZERO_BUFF,-(%sp)",
            ),
            (
                Insn::Subq {
                    size: Size::L,
                    value: 2,
                    dst: Ea::Reg(Reg::A(7)),
                },
                "subq.l\t#2,%sp",
            ),
            (
                Insn::Addq {
                    size: Size::W,
                    value: 4,
                    dst: Ea::Reg(Reg::A(7)),
                },
                "addq.w\t#4,%sp",
            ),
            (
                Insn::Jsr(Ea::AbsL("ML_$LOCK".into(), 0)),
                "jsr\t(ML_$LOCK).l",
            ),
            (
                Insn::Bsr {
                    target: "ZERO_PAGE".into(),
                    short: true,
                },
                "bsr.s\tZERO_PAGE",
            ),
            (
                Insn::Bsr {
                    target: "W".into(),
                    short: false,
                },
                "bsr.w\tW",
            ),
            (
                Insn::Branch {
                    cond: Cond::Cc,
                    label: 3,
                    short: true,
                    back: false,
                },
                "bcc.s\t3f",
            ),
            (
                Insn::Branch {
                    cond: Cond::T,
                    label: 12,
                    short: false,
                    back: true,
                },
                "bra.w\t12b",
            ),
            (
                Insn::Tst {
                    size: Size::B,
                    dst: Ea::AbsL("B".into(), 0),
                },
                "tst.b\t(B).l",
            ),
            (
                Insn::Cmpi {
                    size: Size::W,
                    imm: 0x400,
                    dst: Ea::Reg(Reg::D(1)),
                },
                "cmpi.w\t#0x400,%d1",
            ),
            (
                Insn::Cmp {
                    size: Size::L,
                    src: Ea::Ind(0),
                    dst: 2,
                },
                "cmp.l\t(%a0),%d2",
            ),
        ];
        for (i, want) in cases {
            assert_eq!(i.to_string(), want);
        }
    }
}

#[cfg(test)]
mod len_tests {
    use super::*;

    /// Lengths of instructions as encoded in the image.
    #[test]
    fn lengths_match_the_image() {
        let sp = 7;
        let cases: Vec<(Insn, u32)> = vec![
            (Insn::Link { disp: -12 }, 4),  // 0xE00EB0 4e56 fff4
            (Insn::Pea(Ea::AbsW(0x16)), 4), // 0xE00EB4 4878 0016
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::ImmSym("S".into()),
                    dst: Ea::PreDec(sp),
                },
                6,
            ), // 0xE00EB8 2f3c 00ff 8c00
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::Disp(8, 6),
                    dst: Ea::PreDec(sp),
                },
                4,
            ), // 0xE00EBE 2f2e 0008
            (Insn::Jsr(Ea::AbsL("S".into(), 0)), 6), // 0xE00EC2 4eb9 ...
            (
                Insn::Lea {
                    src: Ea::Disp(12, sp),
                    dst: sp,
                },
                4,
            ), // 0xE00EC8 4fef 000c
            (
                Insn::Movea {
                    size: Size::L,
                    src: Ea::ImmSym("S".into()),
                    dst: 0,
                },
                6,
            ), // 0xE00ECC 207c 00ff 8c00
            (
                Insn::Move {
                    size: Size::W,
                    src: Ea::Imm(0xff),
                    dst: Ea::Reg(Reg::D(0)),
                },
                4,
            ), // 0xE00ED2 303c 00ff
            (
                Insn::Clr {
                    size: Size::L,
                    dst: Ea::PostInc(0),
                },
                2,
            ), // 0xE00ED8 4298
            (Insn::Dbf { reg: 0, label: 1 }, 4), // 0xE00EDA 51c8 fffc
            (Insn::Unlk, 2),
            (Insn::Rts, 2),
            (
                Insn::Subq {
                    size: Size::L,
                    value: 2,
                    dst: Ea::Reg(Reg::A(sp)),
                },
                2,
            ), // 0xE00EF0 558f
            (
                Insn::Move {
                    size: Size::W,
                    src: Ea::Imm(0x14),
                    dst: Ea::PreDec(sp),
                },
                4,
            ), // 0xE00EF2 3f3c 0014
            (
                Insn::Addq {
                    size: Size::W,
                    value: 4,
                    dst: Ea::Reg(Reg::A(sp)),
                },
                2,
            ), // 0xE00EFC 584f
            (
                Insn::Bsr {
                    target: "S".into(),
                    short: true,
                },
                2,
            ), // 0xE00F02 61ac
            (
                Insn::Bsr {
                    target: "S".into(),
                    short: false,
                },
                4,
            ), // 0xE03636 6100 d878
            (
                Insn::Move {
                    size: Size::L,
                    src: Ea::Imm(0x3a0001),
                    dst: Ea::Ind(0),
                },
                6,
            ), // 0xE0B28C 20bc 003a 0001
            (
                Insn::Move {
                    size: Size::W,
                    src: Ea::Disp(0x1e4, 5),
                    dst: Ea::Ind(0),
                },
                4,
            ), // 0xE41B66 30ad 01e4
            (
                Insn::MovemSave {
                    regs: RegSet::of(&[Reg::D(2), Reg::D(3), Reg::A(2)]),
                },
                4,
            ), // 0xE1777C 48e7 3020
            (
                Insn::MovemRestore {
                    regs: RegSet::of(&[Reg::D(2), Reg::D(3), Reg::A(2)]),
                    disp: -0x1c,
                },
                6,
            ), // 0xE177F6 4cee 040c ffe4
            (
                Insn::Btst {
                    bit: 0,
                    dst: Ea::Reg(Reg::D(1)),
                },
                2,
            ), // 0xE148DC 0101
            (
                Insn::Scc {
                    cond: Cond::T,
                    dst: Ea::Disp(0xe0, 5),
                },
                4,
            ), // 0xE6E468 50ed 00e0
            (Insn::Label(1), 0),
            (Insn::Pea(Ea::Disp(0, 6)), 2), // gas: `pea (0,%fp)` assembles as 4856, `pea (%fp)`
            (
                Insn::Tst {
                    size: Size::L,
                    dst: Ea::Disp(0x14, 5),
                },
                4,
            ), // 0xE1778C 4aad 0014
            (
                Insn::Branch {
                    cond: Cond::Ne,
                    label: 1,
                    short: true,
                    back: false,
                },
                2,
            ), // 0xE17790 6604
            (
                Insn::Branch {
                    cond: Cond::T,
                    label: 1,
                    short: false,
                    back: false,
                },
                4,
            ), // 0xE00E68 6000 ff02
            (
                Insn::Cmpi {
                    size: Size::W,
                    imm: 0x400,
                    dst: Ea::Reg(Reg::D(1)),
                },
                4,
            ), // 0xE1779A 0c41 0400
            (
                Insn::Cmpi {
                    size: Size::L,
                    imm: 0x230001,
                    dst: Ea::Ind(0),
                },
                6,
            ), // 0xE58624 0c90 0023 0001
            (
                Insn::Cmpi {
                    size: Size::L,
                    imm: 0x400,
                    dst: Ea::Disp(0x10, 3),
                },
                8,
            ), // 0xE4BAE0 0cab 0000 0400 0010
            (
                Insn::Cmp {
                    size: Size::W,
                    src: Ea::Disp(0x14, 6),
                    dst: 5,
                },
                4,
            ), // 0xE6A9B6 ba6e 0014
        ];
        for (i, n) in cases {
            assert_eq!(i.len_at(0), n, "{i}");
            assert_eq!(i.len_at(2), n, "{i}");
        }
        // The filler: 2 bytes at 2 mod 4 (0xE00ED6), none at 0 mod 4.
        assert_eq!(Insn::Align4.len_at(0xE00ED6), 2);
        assert_eq!(Insn::Align4.len_at(0xE00ED8), 0);
    }
}

#[cfg(test)]
mod cond_tests {
    use super::*;

    /// Every condition but `always` has its opposite, and negating twice
    /// gives the condition back.
    #[test]
    fn negation_pairs() {
        let all = [
            Cond::Hi,
            Cond::Ls,
            Cond::Cc,
            Cond::Cs,
            Cond::Ne,
            Cond::Eq,
            Cond::Pl,
            Cond::Mi,
            Cond::Ge,
            Cond::Lt,
            Cond::Gt,
            Cond::Le,
        ];
        for c in all {
            assert_ne!(c.negate(), c);
            assert_eq!(c.negate().negate(), c);
        }
        assert_eq!(Cond::Cc.negate(), Cond::Cs);
        assert_eq!(Cond::Hi.negate(), Cond::Ls);
        assert_eq!(Cond::Pl.negate(), Cond::Mi);
    }
}

#[cfg(test)]
mod regset_tests {
    use super::*;

    fn d(n: u8) -> Reg {
        Reg::D(n)
    }
    fn a(n: u8) -> Reg {
        Reg::A(n)
    }

    /// Mask words of saves and restores in the image (the prologue
    /// `48e7 <predec>`, the epilogue `4cee <control> <disp>`).
    #[test]
    fn movem_masks_match_the_image() {
        let cases: &[(&[Reg], u16, u16, &str)] = &[
            // 0xE1777C / 0xE177F6 (LOG_$READ's worker)
            (&[d(2), d(3), a(2)], 0x3020, 0x040c, "%d2-%d3/%a2"),
            // 0xE135A0 / 0xE13602 (PMAP_$FLUSH's nested routine)
            (&[d(2), a(2), a(3)], 0x2030, 0x0c04, "%d2/%a2-%a3"),
            // OSINFO_$GET_MMAP 0xE5C698 / 0xE5C9F6
            (
                &[d(2), d(3), d(4), d(5), d(6), d(7), a(2), a(3), a(4)],
                0x3f38,
                0x1cfc,
                "%d2-%d7/%a2-%a4",
            ),
            // MAC_OS_$OPEN 0xE0B24A / 0xE0B452 (A5 in the movem)
            (
                &[d(2), d(3), d(4), d(5), d(6), d(7), a(2), a(3), a(4), a(5)],
                0x3f3c,
                0x3cfc,
                "%d2-%d7/%a2-%a5",
            ),
        ];
        for (regs, predec, control, text) in cases {
            let s = RegSet::of(regs);
            assert_eq!(s.mask_predec(), *predec, "{text}");
            assert_eq!(s.mask_control(), *control, "{text}");
            assert_eq!(s.to_string(), *text);
            assert_eq!(s.len() as usize, regs.len());
            assert_eq!(s.iter().collect::<Vec<_>>(), regs.to_vec());
        }
    }

    #[test]
    fn register_list_text() {
        assert_eq!(RegSet::of(&[d(2)]).to_string(), "%d2");
        assert_eq!(RegSet::of(&[d(2), d(4)]).to_string(), "%d2/%d4");
        assert_eq!(RegSet::of(&[d(7), a(0)]).to_string(), "%d7/%a0");
        assert_eq!(RegSet::of(&[a(4), a(5)]).to_string(), "%a4-%a5");
        assert_eq!(RegSet::EMPTY.to_string(), "");
        let mut s = RegSet::of(&[d(2), a(2)]);
        s.remove(d(2));
        assert_eq!(s, RegSet::of(&[a(2)]));
    }
}
