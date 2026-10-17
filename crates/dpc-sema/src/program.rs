//! The annotated program handed to IR lowering.
//!
//! Everything a later phase needs is resolved here: names are replaced by
//! [`VarId`]s / [`RoutineId`]s, every expression has a [`TypeId`], every
//! type has its layout ([`crate::types`]), every routine its parameter
//! conventions and frame, every variable its storage.  The tree is still
//! source-shaped (statements and expressions as written); lowering to the
//! explicit IR (CLAUDE.md 4.3) is the next phase's job.
//!
//! Target-neutral (CLAUDE.md 4.5): no register names.  Numbers that come
//! from the ABI (pointer size, argument slots, the first argument's frame
//! offset) were asked of [`dpc_target::Target`].
//!
//! Frame offsets are relative to the routine's frame pointer after its
//! prologue: arguments at positive offsets, locals at negative ones.

use dpc_syntax::ast::{BinOp, ForDir, ParamMode, RoutineKind, UnOp};
use dpc_syntax::{Diagnostic, Span};

use crate::consts::ConstValue;
use crate::conv::{ParamConv, Passing, ResultReg, RoutineOptions};
use crate::types::{BitPos, TypeId, TypeTable};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoutineId(pub u32);

/// A checked compilation unit.
#[derive(Debug, Default)]
pub struct Program {
    /// Unit name, folded (`module proc1_tst;` -> `proc1_tst`).
    pub name: String,
    /// True for `program`, false for `module` (ref 7.1).
    pub is_program: bool,
    /// Default code / data sections of the unit: the module heading's
    /// names, else `.text` / `.data` (ref 5.7.1 p. 5-24, 7.1.1 p. 7-2).
    /// Names written as identifiers are folded to lower case.
    pub code_section: String,
    pub data_section: String,
    pub types: TypeTable,
    pub vars: Vec<Var>,
    pub routines: Vec<Routine>,
    /// Top-level routines in source order of their bodies, which is the
    /// emission order: a routine declared `forward` or `extern` and
    /// defined later in the unit is placed at its body (codegen-patterns
    /// 6, "Order").  A routine with no body here is at its declaration.
    /// Nested ones hang off [`Routine::nested`].
    pub top_routines: Vec<RoutineId>,
    /// Unit-level variables in source order.
    pub unit_vars: Vec<VarId>,
    /// Bytes allocated in each data section by this unit, in order of
    /// first use: (section, size).
    pub data_sections: Vec<(String, u32)>,
    /// The main program's action part (`program` only).
    pub main: Option<Block>,
    /// Warnings (errors make [`crate::analyze`] fail instead).
    pub warnings: Vec<Diagnostic>,
}

impl Program {
    pub fn var(&self, id: VarId) -> &Var {
        &self.vars[id.0 as usize]
    }

    pub fn routine(&self, id: RoutineId) -> &Routine {
        &self.routines[id.0 as usize]
    }

    /// Find a unit-level variable or a routine's local by folded name.
    pub fn find_var(&self, name: &str) -> Option<VarId> {
        self.vars
            .iter()
            .position(|v| v.name == name)
            .map(|i| VarId(i as u32))
    }

    /// Find a routine by folded name.
    pub fn find_routine(&self, name: &str) -> Option<RoutineId> {
        self.routines
            .iter()
            .position(|r| r.name == name)
            .map(|i| RoutineId(i as u32))
    }
}

/// A variable, parameter or function result.
#[derive(Clone, Debug)]
pub struct Var {
    /// Folded name.
    pub name: String,
    pub ty: TypeId,
    pub storage: Storage,
    /// The routine whose frame holds it; `None` for unit-level and static
    /// variables.
    pub owner: Option<RoutineId>,
    /// Initial value of a static variable, as stores into the zeroed
    /// object (ref 3.2.2 p. 3-4, 3.9.2, 3.10.4, 3.11.1).
    pub init: Option<Vec<InitStore>>,
    pub span: Span,
}

/// Where a variable lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Storage {
    /// Allocated elsewhere and reached through its symbol: `extern` (ref
    /// 7.2, p. 7-3).
    Extern { symbol: String },
    /// In a static data section of this unit at `offset` (A5-relative for
    /// the module's own data on m68k, codegen-patterns 4).  `global`: the
    /// symbol is exported (`define`, ref 7.2).  Plain unit-level `var`s and
    /// `static` ones are local (ref 7.2: static "keep[s] its name local").
    Static {
        symbol: String,
        global: bool,
        section: String,
        offset: u32,
    },
    /// In the owner routine's frame (locals, local copies of value
    /// parameters, the function result variable).
    Frame { offset: i32 },
    /// A parameter: see [`Routine::params`]`[index]`.
    Param { index: usize },
    /// The result variable of a function whose result goes through a
    /// hidden address ([`crate::conv::ResultReg::Hidden`]): the object at
    /// the address in the argument slot at frame offset `slot` (the first
    /// argument's, below the parameters).
    HiddenResult { slot: i32 },
}

/// One store of an initialiser: `value` at byte `offset` of the variable,
/// with the size of type `ty`, or in bits `bits` from there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitStore {
    pub offset: u32,
    pub ty: TypeId,
    pub value: InitDatum,
    /// A bit field of a packed record (ref 3.10.7): `width` bits from bit
    /// `bit` (0..7) of the byte at `offset`, counted from its most
    /// significant bit ([`BitPos`]), holding the ordinal of `value` (a
    /// [`InitDatum::Const`]), two's complement, most significant bit
    /// first.  `None`: the whole object of type `ty`.
    pub bits: Option<BitPos>,
}

/// The value of one [`InitStore`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitDatum {
    Const(ConstValue),
    /// `addr(...)` of an object with a static address (owner's ruling on
    /// 3.13.4, p. 3-51; docs/reference-clarifications.md "3.13.4
    /// addr(...) as an initial value"): a pointer-sized address, fixed
    /// at link time.
    Addr(StaticAddr),
}

impl From<ConstValue> for InitDatum {
    fn from(c: ConstValue) -> Self {
        InitDatum::Const(c)
    }
}

impl PartialEq<ConstValue> for InitDatum {
    fn eq(&self, other: &ConstValue) -> bool {
        matches!(self, InitDatum::Const(c) if c == other)
    }
}

/// An address known at link time: `offset` bytes into `base`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticAddr {
    pub base: StaticBase,
    pub offset: i64,
}

/// What a [`StaticAddr`] is relative to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticBase {
    /// A routine's entry (a top-level routine that is not `internal`,
    /// ref 3.13.3 p. 3-50).
    Routine(RoutineId),
    /// A variable with [`Storage::Static`] or [`Storage::Extern`].
    Var(VarId),
}

/// A procedure or function.
#[derive(Clone, Debug)]
pub struct Routine {
    /// Folded name.
    pub name: String,
    /// Assembler symbol: the name upper-cased, `$` kept (codegen-patterns
    /// 11).  For local routines this is only a label; the IR may rename.
    pub symbol: String,
    pub kind: RoutineKind,
    /// Exported (global symbol) or local to the unit.  Top-level routines
    /// are global unless `internal` (ref 5.5.4, p. 5-17); nested routines
    /// are local.
    pub global: bool,
    /// Declared `extern` and never given a body here: calls go to the
    /// symbol.
    pub external: bool,
    /// Enclosing routine of a nested routine (ref 2.4).
    pub parent: Option<RoutineId>,
    /// 0 for a top-level routine, parent's level + 1 for a nested one.
    pub level: u32,
    /// Effective options (after `default_routine_options` and user option
    /// names are expanded, ref 5.6).
    pub options: RoutineOptions,
    pub params: Vec<Param>,
    /// Bytes of arguments the caller pushes (sum of the slots).
    pub arg_bytes: u32,
    /// Pad the caller pushes before the arguments so the total is a
    /// multiple of the target's argument-area alignment
    /// (`subq.l #2,SP` on m68k, codegen-patterns 1.4).
    pub arg_pad: u32,
    pub result: Option<FnResult>,
    pub code_section: String,
    pub data_section: String,
    /// Locals, parameter copies and the result variable, in allocation
    /// order.
    pub frame_vars: Vec<VarId>,
    /// Bytes of frame below the frame pointer used by `frame_vars`, raw
    /// (not rounded).  The `link` displacement is the backend's to derive:
    /// every `link.w A6,#d` at a mapped routine in the image has `d` a
    /// multiple of 4 (codegen-patterns 2; PROC1_$TST_LOCK 0xE148CA has
    /// `link #-4` where this is 1).  TODO(codegen): whether that is a
    /// rounding of the total or 4-byte slots per object is not settled.
    pub frame_size: u32,
    /// Nested routines in source order.
    pub nested: Vec<RoutineId>,
    /// The unit-level and `static` variables (no owner routine) that the
    /// body names, first reference each, in source order.  IR lowering
    /// checks them against the routine's data section (codegen-patterns
    /// 4, 11: the data base reaches only the block of the routine's own
    /// data section).
    pub static_refs: Vec<(VarId, Span)>,
    /// The action part; `None` for an `extern` / `forward`-only routine.
    pub body: Option<Block>,
    pub span: Span,
}

/// One parameter of a routine.
#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    /// The variable that names it in the body.
    pub var: VarId,
    pub mode: ParamMode,
    pub univ: bool,
    pub ty: TypeId,
    pub conv: ParamConv,
    /// Frame offset of the argument slot in the callee.
    pub slot_offset: i32,
    /// Frame offset of the value itself when passed by value (differs from
    /// `slot_offset` when the value is smaller than its slot; on m68k a
    /// byte is in the slot's first byte, so they are equal).
    pub value_offset: i32,
    /// Frame offset of the callee's local copy, when
    /// [`ParamConv::local_copy`] is set.  Body references to the parameter
    /// mean the copy.
    pub copy_offset: Option<i32>,
}

impl Param {
    pub fn passing(&self) -> Passing {
        self.conv.passing
    }
}

/// A function's result.
#[derive(Clone, Debug)]
pub struct FnResult {
    pub ty: TypeId,
    pub reg: ResultReg,
    /// The result variable that `f := e` assigns, in the frame; `None`
    /// for a routine without a body here.  Whether Apollo's compiler
    /// allocated such a slot is an open question (codegen-patterns 2,
    /// Uncertain): IR lowering may keep the value in a register instead.
    pub var: Option<VarId>,
}

// ---------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Empty,
    /// `label: stmt`; labels are kept as written (folded identifier or
    /// decimal digits).
    Labeled {
        label: String,
        stmt: Box<Stmt>,
    },
    /// `target := value`.  `target` is a designator (see
    /// [`Expr::is_designator`]) or [`ExprKind::Result`].  The types are
    /// assignment compatible; integer widening is implicit (compare the
    /// two expressions' types).
    Assign {
        target: Expr,
        value: Expr,
    },
    /// A procedure call (or a function called for effect is rejected).
    Call(Call),
    /// A predeclared procedure (`disable`, `enable`, `discard`).
    Builtin {
        builtin: Builtin,
        args: Vec<Expr>,
    },
    Compound(Block),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    /// Arm labels are ordinals (ref p. 4-36).
    Case {
        expr: Expr,
        arms: Vec<CaseArm>,
        otherwise: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    Repeat {
        body: Vec<Stmt>,
        cond: Expr,
    },
    For {
        var: Expr,
        from: Expr,
        dir: ForDir,
        to: Expr,
        body: Box<Stmt>,
    },
    /// `with r1, r2 do body`: each record designator is evaluated once on
    /// entry and named by its [`WithId`]; field references inside the body
    /// are [`ExprKind::Field`] on an [`ExprKind::WithRecord`].
    With {
        items: Vec<(WithId, Expr)>,
        body: Box<Stmt>,
    },
    Goto(String),
    /// ref p. 4-77: leave the innermost loop.
    Exit,
    /// ref p. 4-123: next iteration of the innermost loop.
    Next,
    /// ref p. 4-166: return from the routine.
    Return,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WithId(pub u32);

#[derive(Clone, Debug)]
pub struct CaseArm {
    pub labels: Vec<i64>,
    pub stmt: Stmt,
}

/// A call of a routine or through a routine pointer.
#[derive(Clone, Debug)]
pub struct Call {
    pub callee: Callee,
    /// One per parameter, in parameter order (argument 1 first).
    pub args: Vec<Arg>,
}

#[derive(Clone, Debug)]
pub enum Callee {
    Routine(RoutineId),
    /// Through a routine pointer value (ref 5.3.4 p. 5-12).
    Pointer(Box<Expr>),
}

/// One actual argument with the convention of its parameter.
#[derive(Clone, Debug)]
pub struct Arg {
    pub expr: Expr,
    pub conv: ParamConv,
    /// The argument is passed by address but is not a designator (a
    /// constant or expression), or is a variable of another size than the
    /// value or `in` parameter: the caller must materialise it (pooled
    /// constant in the code section, or a frame temporary; codegen-patterns
    /// 1.5).
    pub needs_temp: bool,
}

// ---------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: TypeId,
    /// The value, when the expression is a compile-time constant.
    pub value: Option<ConstValue>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    /// A literal, named constant or enumeration constant; see
    /// [`Expr::value`].
    Const,
    /// A variable or parameter.  `uplevel` is how many static levels out
    /// the owner is (0 = the current routine or a static variable).
    Var {
        var: VarId,
        uplevel: u32,
    },
    /// The result variable of function `routine` (`f := e` inside `f`).
    Result {
        routine: RoutineId,
        uplevel: u32,
    },
    /// The record designator bound by an enclosing `with`.
    WithRecord(WithId),
    /// `base.field`; `offset` is the field's byte offset (of a packed
    /// record's field, the byte its first bit is in), `bits` the bits of
    /// a packed record's field (ref 3.10.7).
    Field {
        base: Box<Expr>,
        name: String,
        offset: u32,
        bits: Option<crate::types::BitPos>,
    },
    /// `base[index]` (one dimension; `a[i, j]` is `a[i][j]`).  The element
    /// is at `(index - lo) * elem_size` bytes into the array, unless
    /// `elem_bits` is given: the array is a packed array of 1-, 2- or 4-bit
    /// elements ([`crate::types::TypeTable::packed_elem_bits`]), the element
    /// is bits `(index - lo) * n ..` of it from the most significant bit of
    /// its first byte and has no address of its own (`elem_size` is then
    /// the element type's unpacked size, not a stride).
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        lo: i64,
        elem_size: u32,
        elem_bits: Option<u32>,
    },
    /// `base^`.
    Deref(Box<Expr>),
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    /// Binary operator.  `In`'s right operand is a set; relational
    /// operators yield boolean.
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `[a, b..c]`: a set constructor with run-time elements (a constant
    /// one is [`ExprKind::Const`]).
    SetCons(Vec<(Expr, Option<Expr>)>),
    /// A function call.
    Call(Call),
    /// A predeclared function.
    Builtin {
        builtin: Builtin,
        args: Vec<Expr>,
    },
    /// A type transfer function `T(x)` (ref p. 4-197): the bits of `x`
    /// seen as type `T` (the expression's type).
    Transfer(Box<Expr>),
    /// `addr(r)` of a top-level, non-`internal` routine (ref "Addressing
    /// Procedure and Function Pointers", p. 4-145; 3.13.3 p. 3-50; C.3.5):
    /// the routine's address, a `univ_ptr`.
    RoutineAddr(RoutineId),
}

impl Expr {
    /// A designator: something with an address (variable, parameter,
    /// field, element, dereference, with-record).
    pub fn is_designator(&self) -> bool {
        match &self.kind {
            ExprKind::Var { .. } | ExprKind::WithRecord(_) | ExprKind::Deref(_) => true,
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => base.is_designator(),
            _ => false,
        }
    }

    /// A type transfer (`T(v)`) or a field or element selected from one
    /// (`T(v).f`, `T(v)[i].g`).  Ref "Type Transfer Functions", p. 4-197:
    /// "Domain Pascal just 'sees' the argument as a value of the new type";
    /// the reference does not say whether such a selection may be
    /// assigned to, passed to a `var` parameter or have its address taken,
    /// so it is no designator.  A field of one is read in place
    /// (`dpc-ir` `transfer_view`).
    pub fn selects_transfer(&self) -> bool {
        match &self.kind {
            ExprKind::Transfer(_) => true,
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => base.selects_transfer(),
            _ => false,
        }
    }

    /// A type transfer of a variable (`T(v)`, `T(v.g)`, `T(U(v).f)`) or a
    /// field or element selected from one: whether it is itself a variable
    /// (an assignment target, a `var` argument, `addr`'s operand, a `with`
    /// record) is open question 46 (docs/reference-clarifications.md;
    /// ref p. 4-197 does not say).
    pub fn transfers_variable(&self) -> bool {
        match &self.kind {
            ExprKind::Transfer(x) => x.is_designator() || x.transfers_variable(),
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
                base.transfers_variable()
            }
            _ => false,
        }
    }
}

/// Predeclared routines implemented so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    /// `ord(x)` (p. 4-137).
    Ord,
    /// `chr(n)` (p. 4-39).
    Chr,
    /// `odd(x)` (Table 4-4, p. 4-6).
    Odd,
    /// `abs(x)` (Table 4-4).
    Abs,
    /// `sqr(n)` ("Sqr", p. 4-184): `n * n`.
    Sqr,
    /// `lshft(x, n)` (p. 4-109).
    Lshft,
    /// `rshft(x, n)` (Table 4-4).
    Rshft,
    /// `arshft(x, n)` (Table 4-4).
    Arshft,
    /// `xor(a, b)` (p. 4-221).
    Xor,
    /// `disable` (Appendix E, p. E-3): interrupts off.
    Disable,
    /// `enable` (Appendix E, p. E-4): interrupts on.
    Enable,
    /// `addr(x)` ("Addr", p. 4-13): the address of variable `x`, a
    /// `univ_ptr` value.
    Addr,
    /// `set_sr(newsr)` (Appendix E, p. E-5): read the status register,
    /// install `newsr`, return the old value (a word).
    SetSr,
    /// `discard(exp)` ("Discard", p. 4-62): evaluate `exp`, drop its value.
    /// A procedure.
    Discard,
    /// `succ(x)` ("Succ", p. 4-191).
    Succ,
    /// `pred(x)` ("Pred", p. 4-147).
    Pred,
    /// `max(a, b)` ("Max", p. 4-111).
    Max,
    /// `min(a, b)` ("Min", p. 4-113).
    Min,
    /// `in_range(x)` ("In_range", p. 4-104): `x` lies in its type's range.
    InRange,
    /// `sizeof(x)` ("Sizeof", p. 4-181); always folded to a constant.
    Sizeof,
    /// `firstof(x)` ("Firstof", p. 4-87); always folded to a constant.
    Firstof,
    /// `lastof(x)` ("Lastof", p. 4-106); always folded to a constant.
    Lastof,
}
