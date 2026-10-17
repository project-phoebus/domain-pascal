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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// `(d,An,Dn.w)` / `(d,An,Dn.l)`: address register indirect with
    /// index and an 8-bit displacement (brief extension word, no scale:
    /// 68010; codegen-patterns 5.2, 13).  Fields: displacement, address
    /// register, index data register, index is a longword.
    Idx(i32, u8, u8, bool),
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
    /// `(c,%pc)`: a constant's cell in the code section, before the
    /// emitter has placed it (`emit.rs`, `Pool::place`); never printed.
    Pool(dpc_ir::PoolConst),
    /// `(.Lc<n>,%pc)`: the placed cell `n` of the constant pool, PC
    /// relative with a 16-bit displacement (`487a dddd` for `pea`,
    /// TERM_$SEND_KBD_STRING 0xE1AB0C; codegen-patterns 1.5).
    PoolLabel(u32),
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
            Ea::Disp(..) | Ea::AbsW(_) | Ea::Idx(..) | Ea::Pool(_) | Ea::PoolLabel(_) => 2,
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
            Ea::Idx(d, a, x, long) => write!(
                f,
                "({},{},{}.{})",
                num(*d as i64),
                Reg::A(*a),
                Reg::D(*x),
                if *long { "l" } else { "w" }
            ),
            Ea::AbsL(s, 0) => write!(f, "({s}).l"),
            Ea::AbsL(s, o) if *o < 0 => write!(f, "({s}-{}).l", num(-(*o as i64))),
            Ea::AbsL(s, o) => write!(f, "({s}+{}).l", num(*o as i64)),
            Ea::AbsW(n) => write!(f, "({}).w", num(*n as i64)),
            Ea::ImmSym(s) => write!(f, "#{s}"),
            Ea::PreDec(a) => write!(f, "-({})", Reg::A(*a)),
            Ea::PostInc(a) => write!(f, "({})+", Reg::A(*a)),
            Ea::Imm(n) => write!(f, "#{}", num(*n)),
            Ea::Pool(c) => write!(f, "(<unplaced constant {:02x?}>,%pc)", c.bytes),
            Ea::PoolLabel(n) => write!(f, "({},%pc)", pool_label(*n)),
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
    /// numbered local label while it is not -1.  With `until`, `db<cc>`:
    /// leave the loop when the condition holds first (`dbne`, the record
    /// compare loop, codegen-patterns 7.1).
    Dbf {
        reg: u8,
        label: u32,
        until: Option<Cond>,
    },
    /// `cmpm.<size> (%ay)+,(%ax)+`: compare the destination's element
    /// with the source's (`(Ax) - (Ay)`), both registers advanced.
    Cmpm {
        size: Size,
        src: u8,
        dst: u8,
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
    /// `cmpa.w #0,%an`: a pointer held in an address register against
    /// `nil` (the 68010 has no `tst An`; codegen-patterns 7.1, 13: 151
    /// sites, `b6fc 0000` at 0xE38EC0 for A3; 0xE3CCE4 `b2fc 0000` for A1,
    /// a `while p <> nil` test).
    CmpaZero(u8),
    /// `suba.l %an,%an` (`91c8` + n << 9): `nil` into an address
    /// register (codegen-patterns 3, "`nil` results"; 24 sites,
    /// `tools/nilsurvey.py areg`: ROUTE_$FIND_PORTP 0xE15B84 `91c8`,
    /// EC2_$REGISTER_EC1 0xE4299C `95ca`).  It writes An only: the old
    /// value is not read, and the condition codes are left alone.
    SubaSelf(u8),
    /// `cmp.<size> src,%dn`.
    Cmp {
        size: Size,
        src: Ea,
        dst: u8,
    },
    /// `<op>.<size> src,%dn` for add, sub, and, or (register destination).
    Alu {
        op: AluOp,
        size: Size,
        src: Ea,
        dst: u8,
    },
    /// `<op>i.<size> #imm,dst` for addi, subi, andi, ori, eori (the
    /// immediate as [`Ea::Imm`] gives it).
    AluImm {
        op: AluOp,
        size: Size,
        imm: i64,
        dst: Ea,
    },
    /// `neg.<size> dst` / `not.<size> dst`.
    Unary {
        op: UnaryOp,
        size: Size,
        dst: Ea,
    },
    /// `ext.<size> %dn`: `ext.w` sign-extends the low byte to a word,
    /// `ext.l` the low word to a longword.
    Ext {
        size: Size,
        dst: u8,
    },
    /// `<op>.<size> #n,%dn` (n = 1..8) or `<op>.<size> %dm,%dn`.
    Shift {
        op: ShiftOp,
        size: Size,
        count: ShiftCount,
        dst: u8,
    },
    /// `mulu.w src,%dn` / `muls.w src,%dn` (16 x 16 -> 32).
    Mul {
        signed: bool,
        src: Ea,
        dst: u8,
    },
    /// `divu.w src,%dn` / `divs.w src,%dn` (32 / 16 -> 16 quotient in the
    /// low word, remainder in the high word).
    Div {
        signed: bool,
        src: Ea,
        dst: u8,
    },
    /// `swap %dn`.
    Swap(u8),
    /// `btst #bit,dst` (immediate bit number; mod 8 on memory, mod 32 on a
    /// data register).
    BtstImm {
        bit: u8,
        dst: Ea,
    },
    /// `bset #bit,<dst>` / `bclr #bit,<dst>` (on memory a byte, bit
    /// 0..7): a boolean field of a packed record set or cleared
    /// (codegen-patterns 5.3: PROC2_$RESUME 0xE41444 `bset #7,(-0x4,A6)`).
    BitImm {
        op: BitOp,
        bit: u8,
        dst: Ea,
    },
    /// `bset %dn,<dst>` / `bclr %dn,<dst>`, the bit number in a data
    /// register (on memory a byte, the number mod 8; on a data register a
    /// longword, mod 32): a run-time member added to a set
    /// (codegen-patterns 5.1: TTY_$K_SET_INPUT_FLAG 0xE67692 `bset
    /// D2,D1`; MSG_$OPENI 0xE59260 `bset D1,(0,A0,D0.w)`).
    Bit {
        op: BitOp,
        bit: u8,
        dst: Ea,
    },
    /// `<op>.<size> Dn,<dst>`, an operation into memory: a bit field
    /// merged into its unit (codegen-patterns 5.3: 0xE008A4 `or.b
    /// D0,(39,A2)`).
    AluToMem {
        op: AluOp,
        size: Size,
        src: u8,
        dst: Ea,
    },
    /// `move.w (T,%pc,%dn.w),%dn`: load the word of `case` jump table
    /// `T` (a numbered local label placed right after the
    /// [`Insn::TableJump`] that follows) at the doubled index in Dn.
    /// Image: OSINFO_$GET_MMAP 0xE5C6CE `303b 0006` (codegen-patterns 7.3).
    TableIndex {
        reg: u8,
        table: u32,
    },
    /// `jmp (T,%pc,%dn.w)`: jump to table `T` plus the offset in Dn.
    /// `targets` are the labels the table's entries name (the successors
    /// of the jump; [`crate::regs::successors`]).  Image: 0xE5C6D2
    /// `4efb 0002`.
    TableJump {
        reg: u8,
        table: u32,
        targets: Vec<u32>,
    },
    /// `.short L-T`: one entry of `case` table `T`, the offset of label
    /// `target` from the table's start (0xE5C6D6: `000a 0026 0032 00a4
    /// 006c`).  `back`: `target` is defined before the entry.
    TableEntry {
        table: u32,
        target: u32,
        back: bool,
    },
    /// `ori.w #imm,%sr` / `andi.w #imm,%sr` (`op` Or or And): the
    /// interrupt built-ins `disable` (`007c 0700`) and `enable` (`027c
    /// f8ff`), TIME_$GET_ADJUST 0xE16AB0 / 0xE16ABA (codegen-patterns 10).
    SrImm {
        op: AluOp,
        imm: i64,
    },
    /// `move.w %sr,%dn` (`40c0+n`): `set_sr` reads the old status word
    /// (PROC1_$SET_VT 0xE149B6 `40c3`).
    MoveFromSr {
        dst: u8,
    },
    /// `move.w src,%sr` (`46c0+ea`): `set_sr` installs the new one
    /// (0xE149B8 `46fc 2700`, 0xE149F4 `46c3`).
    MoveToSr {
        src: Ea,
    },
    /// Pseudo-instruction: pad to a 4-byte boundary with the 2-byte no-op
    /// `movea.l A0,A0` (0x2048), as the compiler does before a loop head
    /// (codegen-patterns 7: 0xE00ED6 before the `dbf` loop at 0xE00ED8;
    /// PKT_$RECENTLY_MISSING 0xE128DA, a routine starting at 2 mod 4).
    /// Emitted as `.balignw 4,0x2048`; the boundary is absolute because
    /// the module's code base is 4-aligned, or, for a fragment, because
    /// the emitter shifts the section (`emit::Options::fragment_origin`).
    Align4,
    /// Pseudo-instruction: the label of constant-pool cell `n`
    /// ([`pool_label`]), at the start of the line.
    PoolLabel(u32),
    /// Pseudo-instruction: words of data, `.short` (a constant-pool
    /// cell, codegen-patterns 1.5: `0000` at 0xE1AB26).
    Words(Vec<u16>),
    /// An instruction written as its encoding, `.short` with the mnemonic
    /// in a comment, where gas cannot produce it (CLAUDE.md 4.4): a
    /// `bsr.s` to a routine outside a fragment, whose 8-bit displacement
    /// gas does not relocate (`emit.rs`, `encode_outside_short_calls`).
    Encoded {
        words: Vec<u16>,
        text: String,
    },
}

impl Insn {
    /// The effective-address operands of the instruction, in operand
    /// order; register-number fields (`moveq`'s, `dbf`'s, a data register
    /// destination of `cmp`/`add`/`mul`, ...) are not [`Ea`]s and are not
    /// listed.  Exhaustive over the variants, so a new variant must say
    /// what its operands are.
    pub fn eas(&self) -> Vec<&Ea> {
        match self {
            Insn::Link { .. }
            | Insn::Unlk
            | Insn::Rts
            | Insn::MovemSave { .. }
            | Insn::MovemRestore { .. }
            | Insn::Moveq { .. }
            | Insn::Dbf { .. }
            | Insn::Cmpm { .. }
            | Insn::Label(_)
            | Insn::Bsr { .. }
            | Insn::Branch { .. }
            | Insn::CmpaZero(_)
            | Insn::SubaSelf(_)
            | Insn::Ext { .. }
            | Insn::Shift { .. }
            | Insn::Swap(_)
            | Insn::TableIndex { .. }
            | Insn::TableJump { .. }
            | Insn::TableEntry { .. }
            | Insn::SrImm { .. }
            | Insn::MoveFromSr { .. }
            | Insn::Align4
            | Insn::PoolLabel(_)
            | Insn::Words(_)
            | Insn::Encoded { .. } => vec![],
            Insn::Pea(ea) | Insn::Jsr(ea) | Insn::MoveToSr { src: ea } => vec![ea],
            Insn::Move { src, dst, .. } => vec![src, dst],
            Insn::Movea { src, .. }
            | Insn::Lea { src, .. }
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
        }
    }
}

/// The assembler label of constant-pool cell `n`: a `.L` local label,
/// not kept in the object's symbol table.
pub fn pool_label(n: u32) -> String {
    format!(".Lc{n}")
}

/// The two-operand arithmetic and logic operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AluOp {
    Add,
    Sub,
    And,
    Or,
    Eor,
}

impl AluOp {
    fn name(self) -> &'static str {
        match self {
            AluOp::Add => "add",
            AluOp::Sub => "sub",
            AluOp::And => "and",
            AluOp::Or => "or",
            AluOp::Eor => "eor",
        }
    }
}

/// `neg` / `not`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitOp {
    Set,
    Clr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Logical left, logical right, arithmetic right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftOp {
    Lsl,
    Lsr,
    Asr,
}

/// A shift count: an immediate 1..=8 or a data register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftCount {
    Imm(u8),
    Reg(u8),
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
            Insn::Dbf {
                reg,
                label,
                until: None,
            } => write!(f, "dbf\t{},{label}b", Reg::D(*reg)),
            Insn::Dbf {
                reg,
                label,
                until: Some(c),
            } => write!(f, "db{c}\t{},{label}b", Reg::D(*reg)),
            Insn::Cmpm { size, src, dst } => write!(
                f,
                "cmpm.{size}\t{},{}",
                Ea::PostInc(*src),
                Ea::PostInc(*dst)
            ),
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
            Insn::PoolLabel(n) => write!(f, "{}:", pool_label(*n)),
            Insn::Encoded { words, text } => {
                f.write_str(".short\t")?;
                for (i, x) in words.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "0x{x:04x}")?;
                }
                write!(f, "\t| {text}")
            }
            Insn::Words(w) => {
                f.write_str(".short\t")?;
                for (i, x) in w.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "0x{x:04x}")?;
                }
                Ok(())
            }
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
            Insn::CmpaZero(a) => write!(f, "cmpa.w\t#0,{}", Reg::A(*a)),
            Insn::SubaSelf(a) => write!(f, "suba.l\t{},{}", Reg::A(*a), Reg::A(*a)),
            Insn::Cmp { size, src, dst } => write!(f, "cmp.{size}\t{src},{}", Reg::D(*dst)),
            // A symbol's address added or subtracted: the immediate-EA
            // encodings `d0bc`/`90bc` (+ the register's number << 9),
            // which gas does not choose (it writes `addi`/`subi`):
            // NET_$OPEN 0xE5A1C0 `90bc 00e2 44f4` `sub.l
            // #NET_IO_$NIL_DRIVER,D0`; 0xE0F14E `d2bc 00ff 8800` `add.l
            // #AST_$COPY_BUFF,D1` (codegen-patterns 5.2, 13).
            Insn::Alu {
                op: op @ (AluOp::Add | AluOp::Sub),
                size: Size::L,
                src: Ea::ImmSym(s),
                dst,
            } => {
                let base: u16 = if *op == AluOp::Add { 0xd0bc } else { 0x90bc };
                write!(
                    f,
                    ".short\t0x{:04x}\t| {}.l #{s},{}\n\t.long\t{s}",
                    base | (u16::from(*dst) << 9),
                    op.name(),
                    Reg::D(*dst)
                )
            }
            Insn::Alu { op, size, src, dst } => {
                write!(f, "{}.{size}\t{src},{}", op.name(), Reg::D(*dst))
            }
            Insn::AluImm { op, size, imm, dst } => {
                write!(f, "{}i.{size}\t#{},{dst}", op.name(), num(*imm))
            }
            Insn::Unary { op, size, dst } => {
                let mn = match op {
                    UnaryOp::Neg => "neg",
                    UnaryOp::Not => "not",
                };
                write!(f, "{mn}.{size}\t{dst}")
            }
            Insn::Ext { size, dst } => write!(f, "ext.{size}\t{}", Reg::D(*dst)),
            Insn::Shift {
                op,
                size,
                count,
                dst,
            } => {
                let mn = match op {
                    ShiftOp::Lsl => "lsl",
                    ShiftOp::Lsr => "lsr",
                    ShiftOp::Asr => "asr",
                };
                match count {
                    ShiftCount::Imm(n) => write!(f, "{mn}.{size}\t#{n},{}", Reg::D(*dst)),
                    ShiftCount::Reg(r) => {
                        write!(f, "{mn}.{size}\t{},{}", Reg::D(*r), Reg::D(*dst))
                    }
                }
            }
            Insn::Mul { signed, src, dst } => write!(
                f,
                "{}.w\t{src},{}",
                if *signed { "muls" } else { "mulu" },
                Reg::D(*dst)
            ),
            Insn::Div { signed, src, dst } => write!(
                f,
                "{}.w\t{src},{}",
                if *signed { "divs" } else { "divu" },
                Reg::D(*dst)
            ),
            Insn::Swap(d) => write!(f, "swap\t{}", Reg::D(*d)),
            Insn::SrImm { op, imm } => write!(f, "{}i.w\t#{},%sr", op.name(), num(*imm)),
            Insn::MoveFromSr { dst } => write!(f, "move.w\t%sr,{}", Reg::D(*dst)),
            Insn::MoveToSr { src } => write!(f, "move.w\t{src},%sr"),
            Insn::BtstImm { bit, dst } => write!(f, "btst\t#{bit},{dst}"),
            Insn::BitImm { op, bit, dst } => {
                let mn = match op {
                    BitOp::Set => "bset",
                    BitOp::Clr => "bclr",
                };
                write!(f, "{mn}\t#{bit},{dst}")
            }
            Insn::Bit { op, bit, dst } => {
                let mn = match op {
                    BitOp::Set => "bset",
                    BitOp::Clr => "bclr",
                };
                write!(f, "{mn}\t{},{dst}", Reg::D(*bit))
            }
            Insn::AluToMem { op, size, src, dst } => {
                write!(f, "{}.{size}\t{},{dst}", op.name(), Reg::D(*src))
            }
            Insn::TableIndex { reg, table } => {
                write!(f, "move.w\t({table}f,%pc,{r}.w),{r}", r = Reg::D(*reg))
            }
            Insn::TableJump { reg, table, .. } => {
                write!(f, "jmp\t({table}f,%pc,{}.w)", Reg::D(*reg))
            }
            Insn::TableEntry {
                table,
                target,
                back,
            } => write!(
                f,
                ".short\t{target}{}-{table}b",
                if *back { "b" } else { "f" }
            ),
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
            Insn::PoolLabel(_) => 0,
            Insn::Words(w) | Insn::Encoded { words: w, .. } => 2 * w.len() as u32,
            Insn::Unlk | Insn::Rts | Insn::Moveq { .. } | Insn::Cmpm { .. } => 2,
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
            Insn::Btst { dst, .. } | Insn::Scc { dst, .. } | Insn::Bit { dst, .. } => {
                2 + dst.ext_len(Size::B)
            }
            Insn::Branch { short, .. } => {
                if *short {
                    2
                } else {
                    4
                }
            }
            Insn::Tst { size, dst } => 2 + dst.ext_len(*size),
            Insn::Cmpi { size, dst, .. } => 2 + Ea::Imm(0).ext_len(*size) + dst.ext_len(*size),
            Insn::CmpaZero(_) => 4,
            Insn::SubaSelf(_) => 2,
            Insn::Cmp { size, src, .. } | Insn::Alu { size, src, .. } => 2 + src.ext_len(*size),
            Insn::AluImm { size, dst, .. } => 2 + Ea::Imm(0).ext_len(*size) + dst.ext_len(*size),
            Insn::Unary { size, dst, .. } => 2 + dst.ext_len(*size),
            Insn::Ext { .. } | Insn::Shift { .. } | Insn::Swap(_) | Insn::MoveFromSr { .. } => 2,
            Insn::SrImm { .. } => 4,
            Insn::MoveToSr { src } => 2 + src.ext_len(Size::W),
            Insn::Mul { src, .. } | Insn::Div { src, .. } => 2 + src.ext_len(Size::W),
            Insn::BtstImm { dst, .. } | Insn::BitImm { dst, .. } => 4 + dst.ext_len(Size::B),
            Insn::AluToMem { size, dst, .. } => 2 + dst.ext_len(*size),
            // One opcode word and the brief extension word.
            Insn::TableIndex { .. } | Insn::TableJump { .. } => 4,
            Insn::TableEntry { .. } => 2,
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
            (
                Insn::Dbf {
                    reg: 0,
                    label: 1,
                    until: None,
                },
                "dbf\t%d0,1b",
            ),
            (
                Insn::Dbf {
                    reg: 1,
                    label: 2,
                    until: Some(Cond::Ne),
                },
                "dbne\t%d1,2b",
            ),
            (
                Insn::Cmpm {
                    size: Size::L,
                    src: 1,
                    dst: 0,
                },
                "cmpm.l\t(%a1)+,(%a0)+",
            ),
            (Insn::Label(1), "1:"),
            (Insn::Unlk, "unlk\t%fp"),
            (Insn::Rts, "rts"),
            (Insn::Align4, ".balignw\t4,0x2048"),
            // A `case` dispatch (OSINFO_$GET_MMAP 0xE5C6CE..0xE5C6D8).
            (
                Insn::TableIndex { reg: 0, table: 7 },
                "move.w\t(7f,%pc,%d0.w),%d0",
            ),
            (
                Insn::TableJump {
                    reg: 1,
                    table: 7,
                    targets: vec![2],
                },
                "jmp\t(7f,%pc,%d1.w)",
            ),
            (
                Insn::TableEntry {
                    table: 7,
                    target: 2,
                    back: false,
                },
                ".short\t2f-7b",
            ),
            (Insn::Pea(Ea::AbsW(0x16)), "pea\t(0x16).w"),
            // A symbol's address added or subtracted: the immediate-EA
            // encodings gas does not choose (NET_$OPEN 0xE5A1C0 `90bc`;
            // 0xE0F14E `d2bc`).
            (
                Insn::Alu {
                    op: AluOp::Sub,
                    size: Size::L,
                    src: Ea::ImmSym("NET_IO_$NIL_DRIVER".into()),
                    dst: 0,
                },
                ".short\t0x90bc\t| sub.l #NET_IO_$NIL_DRIVER,%d0\n\t.long\tNET_IO_$NIL_DRIVER",
            ),
            (
                Insn::Alu {
                    op: AluOp::Add,
                    size: Size::L,
                    src: Ea::ImmSym("AST_$COPY_BUFF".into()),
                    dst: 1,
                },
                ".short\t0xd2bc\t| add.l #AST_$COPY_BUFF,%d1\n\t.long\tAST_$COPY_BUFF",
            ),
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
            (
                Insn::Dbf {
                    reg: 0,
                    label: 1,
                    until: None,
                },
                4,
            ), // 0xE00EDA 51c8 fffc
            (
                Insn::Dbf {
                    reg: 1,
                    label: 1,
                    until: Some(Cond::Ne),
                },
                4,
            ), // 0xE3D852 56c9 fffc
            (
                Insn::Cmpm {
                    size: Size::L,
                    src: 1,
                    dst: 0,
                },
                2,
            ), // 0xE1C1B6 b189
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
            (Insn::TableIndex { reg: 0, table: 1 }, 4), // 0xE5C6CE 303b 0006
            (
                Insn::TableJump {
                    reg: 0,
                    table: 1,
                    targets: vec![],
                },
                4,
            ), // 0xE5C6D2 4efb 0002
            (
                Insn::TableEntry {
                    table: 1,
                    target: 2,
                    back: false,
                },
                2,
            ), // 0xE5C6D6 000a
            (
                Insn::SrImm {
                    op: AluOp::Or,
                    imm: 0x700,
                },
                4,
            ), // 0xE16AB0 007c 0700
            (Insn::MoveFromSr { dst: 3 }, 2), // 0xE149B6 40c3
            (
                Insn::MoveToSr {
                    src: Ea::Imm(0x2700),
                },
                4,
            ), // 0xE149B8 46fc 2700
            (
                Insn::MoveToSr {
                    src: Ea::Reg(Reg::D(3)),
                },
                2,
            ), // 0xE149F4 46c3
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
