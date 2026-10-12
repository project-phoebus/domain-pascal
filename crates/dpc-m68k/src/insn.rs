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
}

impl fmt::Display for Insn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Insn::Link { disp } => write!(f, "link.w\t%fp,#{}", num(*disp as i64)),
            Insn::Unlk => f.write_str("unlk\t%fp"),
            Insn::Rts => f.write_str("rts"),
            Insn::Move { size, src, dst } => write!(f, "move.{size}\t{src},{dst}"),
            Insn::Movea { size, src, dst } => {
                write!(f, "movea.{size}\t{src},{}", Reg::A(*dst))
            }
            Insn::Btst { bit, dst } => write!(f, "btst\t{},{dst}", Reg::D(*bit)),
            Insn::Scc { cond, dst } => write!(f, "s{cond}\t{dst}"),
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
            (Insn::Unlk, "unlk\t%fp"),
            (Insn::Rts, "rts"),
        ];
        for (i, want) in cases {
            assert_eq!(i.to_string(), want);
        }
    }
}
