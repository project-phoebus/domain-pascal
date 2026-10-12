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
    /// `-(An)`.
    PreDec(u8),
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
            Ea::PreDec(a) => write!(f, "-({})", Reg::A(*a)),
        }
    }
}

/// A condition for `Scc` (and, later, `Bcc`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    Ne,
    Eq,
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cond::Ne => "ne",
            Cond::Eq => "eq",
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
            Insn::Btst { bit, dst } => write!(f, "btst\t{},{dst}", Reg::D(*bit)),
            Insn::Scc { cond, dst } => write!(f, "s{cond}\t{dst}"),
            Insn::Align4 => f.write_str(".balignw\t4,0x2048"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                Insn::Move {
                    size: Size::L,
                    src: Ea::Reg(Reg::D(2)),
                    dst: Ea::PreDec(7),
                },
                "move.l\t%d2,-(%sp)",
            ),
            (Insn::Unlk, "unlk\t%fp"),
            (Insn::Rts, "rts"),
            (Insn::Align4, ".balignw\t4,0x2048"),
        ];
        for (i, want) in cases {
            assert_eq!(i.to_string(), want);
        }
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
