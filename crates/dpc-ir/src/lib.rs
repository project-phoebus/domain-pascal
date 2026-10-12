//! Low-level, per-routine IR (CLAUDE.md section 4.3).
//!
//! A tree IR, one [`Function`] per routine with a body.  Everything the
//! instruction selector needs to know about storage is explicit:
//!
//! - frame objects are frame-pointer offsets ([`Addr::Frame`]), arguments
//!   at positive offsets, locals at negative ones (the offsets come from
//!   `dpc-sema`, which asked the target where argument 1 lives);
//! - objects of other units are symbols ([`Addr::Symbol`]);
//! - data reached through a pointer is [`Addr::Indirect`];
//! - every value has an explicit [`Scalar`] type: a [`Width`] and a
//!   [`Sign`], so the selector can choose sign- or zero-extension, signed
//!   or unsigned compares and branches, `divs`/`divu`, `muls`/`mulu`.
//!
//! The tree keeps the source's operand structure, so the selector can
//! reproduce the original compiler's evaluation order pattern by pattern
//! (codegen-patterns 12: the original did very little optimisation).
//!
//! Target seam (CLAUDE.md 4.5): this crate names no machine registers and
//! does not assume m68k.  The frame size is the raw byte count from
//! semantic analysis; rounding it (the `link` displacement) is the
//! target's decision.
//!
//! [`lower`] builds the IR from the checked program.  Constructs it cannot
//! express yet are reported as "not supported yet" at their source
//! position; nothing is silently dropped.

mod lower;

#[cfg(test)]
mod tests;

pub use lower::lower;

use dpc_syntax::Span;

/// One compilation unit's worth of IR.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Module {
    /// Unit name, folded.
    pub name: String,
    /// Routines with bodies, in emission order: for now the order of each
    /// routine's *first declaration* (`dpc_sema::Program::top_routines`).
    /// TODO(codegen): a routine declared `forward`/`extern` earlier (e.g.
    /// in an included `.ins.pas`) is emitted at its declaration, not at
    /// its body; the original presumably emitted in body order.
    pub functions: Vec<Function>,
}

/// One routine with a body.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    /// Assembler symbol (upper-cased, `$` kept; codegen-patterns 11).
    pub symbol: String,
    /// Exported symbol.
    pub global: bool,
    /// The code section the routine goes to (ref 5.7.1; `.text` by
    /// default).
    pub code_section: String,
    /// Bytes of frame used below the frame pointer, raw (not rounded):
    /// `dpc_sema::Routine::frame_size`.
    pub frame_size: u32,
    /// The function result, if this is a function.
    pub result: Option<ResultSpec>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// Where a function leaves its result: a register class and the type of
/// the value.  The target maps the class to a register (m68k: Data = D0,
/// Address = A0; codegen-patterns 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultSpec {
    pub class: ResultClass,
    pub ty: Scalar,
}

/// The result register class (mirrors `dpc_sema::conv::ResultReg`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultClass {
    Data,
    Address,
    AddressAndData,
}

/// Width of a scalar value or memory access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    /// 1 byte.
    Byte,
    /// 2 bytes.
    Word,
    /// 4 bytes.
    Long,
}

impl Width {
    /// The width of an object of `size` bytes, if it is a scalar width.
    pub fn of_size(size: u32) -> Option<Width> {
        match size {
            1 => Some(Width::Byte),
            2 => Some(Width::Word),
            4 => Some(Width::Long),
            _ => None,
        }
    }

    pub fn bytes(self) -> u32 {
        match self {
            Width::Byte => 1,
            Width::Word => 2,
            Width::Long => 4,
        }
    }
}

/// Whether a scalar's bits are read as two's complement or as an unsigned
/// number.  The rule from source types is in `lower.rs` (`sign_of`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Signed,
    Unsigned,
}

/// The IR type of a scalar value: how many bytes, and how to read them.
///
/// The width alone decides memory accesses and register moves; the sign
/// matters only to operations that widen, compare, divide, multiply or
/// shift right (codegen-patterns 8; e.g. 0xE177A6..0xE177B8 `clr.l D1;
/// move.w D0,D1` zero-extends a `0..65535` word, `cmpi.w #0x400,D1; bcc`
/// compares it unsigned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scalar {
    pub width: Width,
    pub sign: Sign,
}

impl Scalar {
    pub const fn signed(width: Width) -> Scalar {
        Scalar {
            width,
            sign: Sign::Signed,
        }
    }

    pub const fn unsigned(width: Width) -> Scalar {
        Scalar {
            width,
            sign: Sign::Unsigned,
        }
    }

    /// A pointer value (4 bytes, unsigned).
    pub const POINTER: Scalar = Scalar::unsigned(Width::Long);

    /// A Pascal boolean (1 byte, 0xFF true, 0 false; ref 3.5.3 p. 3-11).
    /// Signed: the image tests booleans with `tst.b` and `bmi`/`bpl`
    /// (codegen-patterns 5, 7).
    pub const BOOLEAN: Scalar = Scalar::signed(Width::Byte);

    pub fn is_signed(self) -> bool {
        self.sign == Sign::Signed
    }

    pub fn bytes(self) -> u32 {
        self.width.bytes()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// Evaluate the value into the function's result register.  Produced
    /// for `f := e` when it is the last statement of `f` (see
    /// `lower.rs`, `result_assignment`).
    SetResult(Expr),
}

/// A value.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A constant of the given type.
    Const { value: i64, ty: Scalar },
    /// The scalar object at `addr`, read as `ty`.
    Load { addr: Addr, ty: Scalar },
    /// Membership of element `elem` in the set value `set` (ref "In",
    /// p. 4-102; layout ref 3.9.3): true when bit `elem` of `set`, read as
    /// one big-endian integer, is 1.  `set` is a register-sized set value
    /// of `set_width`.  The result is a boolean.
    SetMember {
        elem: Box<Expr>,
        set: Box<Expr>,
        set_width: Width,
    },
}

impl Expr {
    /// The IR type of the value.
    pub fn ty(&self) -> Scalar {
        match self {
            Expr::Const { ty, .. } | Expr::Load { ty, .. } => *ty,
            Expr::SetMember { .. } => Scalar::BOOLEAN,
        }
    }
}

/// An address.
#[derive(Clone, Debug, PartialEq)]
pub enum Addr {
    /// Frame pointer + offset.
    Frame(i32),
    /// An absolute symbol + offset (another unit's object).
    Symbol { name: String, offset: i32 },
    /// The pointer value `base` + offset.
    Indirect { base: Box<Expr>, offset: i32 },
}

impl Addr {
    /// This address plus `delta` bytes.
    pub fn offset_by(self, delta: i32) -> Addr {
        match self {
            Addr::Frame(o) => Addr::Frame(o + delta),
            Addr::Symbol { name, offset } => Addr::Symbol {
                name,
                offset: offset + delta,
            },
            Addr::Indirect { base, offset } => Addr::Indirect {
                base,
                offset: offset + delta,
            },
        }
    }
}
