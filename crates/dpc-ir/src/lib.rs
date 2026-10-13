//! Low-level, per-routine IR (CLAUDE.md section 4.3).
//!
//! A tree IR, one [`Function`] per routine with a body.  Everything the
//! instruction selector needs to know about storage is explicit:
//!
//! - frame objects are frame-pointer offsets ([`Addr::Frame`]), arguments
//!   at positive offsets, locals at negative ones (the offsets come from
//!   `dpc-sema`, which asked the target where argument 1 lives);
//! - objects of other units are symbols ([`Addr::Symbol`]);
//! - objects in the unit's own static data area are offsets into it
//!   ([`Addr::Static`]; the target reaches the area through a base
//!   register, A5 on m68k, which the routines marked
//!   [`Function::loads_data_base`] load on entry);
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

pub use lower::{lower, lower_with};

use dpc_syntax::Span;

/// One compilation unit's worth of IR.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Module {
    /// Unit name, folded.
    pub name: String,
    /// Routines with bodies, in emission order: the source order of their
    /// *bodies* (`dpc_sema::Program::top_routines`), so a routine declared
    /// `forward`/`extern` earlier (e.g. in an included `.ins.pas`) is
    /// emitted where its body is (codegen-patterns 6, "Order": forward
    /// `bsr`s to module-local routines, e.g. AUDIT_$SHUTDOWN 0xE70D2E).
    /// Routines with no body in the unit are not emitted.
    pub functions: Vec<Function>,
    /// The unit's static data area, if it allocates any static data
    /// (unit-level `var`, `static`, `define`; ref 7.2).
    pub data: Option<DataArea>,
}

/// A unit's static data area: one block, zero-filled (initialised static
/// data is not supported yet), reached as [`Addr::Static`] offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataArea {
    /// Bytes allocated by the unit (`dpc_sema::Program::data_sections`),
    /// raw: rounding the block is the target's decision.
    pub size: u32,
    /// Exported (`define`d, ref 7.2.2) variables: symbol and offset, in
    /// offset order.  Other static variables are local (ref 7.2) and get
    /// no symbol.
    pub globals: Vec<(String, u32)>,
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
    /// The frame offset of the function's result variable
    /// (`dpc_sema::FnResult::var`), if this is a function.  A pointer
    /// result assigned by the last statement goes through it
    /// (codegen-patterns 3, "Pointer results").
    pub result_slot: Option<i32>,
    /// The routine makes the unit's data area addressable on entry (m68k:
    /// saves A5 and loads it with the area's address, codegen-patterns 4).
    /// False for a routine that inherits it from its caller (module-local
    /// worker 0xE17778 reads `(0x14,A5)` without loading A5) or does not
    /// need it.  The rule is in `lower.rs` (`set_loads_data_base`), with
    /// [`DataBaseOptions`].
    pub loads_data_base: bool,
    /// The routine does not load the data base but addresses the unit's
    /// data area from its absolute address (codegen-patterns 4:
    /// AUDIT_$SUSPEND 0xE70DC0, REM_NAME_$REGISTER_SERVER 0xE4A4B2 `lea
    /// <area>.l,A0`); [`DataBaseOptions::absolute`].  Never set together
    /// with [`Self::loads_data_base`].
    pub data_absolute: bool,
    /// The parameters passed by value (ref 5.2 Table 5-1: `val_param` /
    /// `internal`), in parameter order.  The body reads one as
    /// `Expr::Load { addr: Addr::Frame(offset), .. }`; the list lets the
    /// selector recognise those reads (the original loaded parameters
    /// into registers at entry in some routines, codegen-patterns 12.2).
    pub value_params: Vec<ValueParam>,
    /// The frame offsets of the slots of the parameters passed by address
    /// (each slot holds the argument's address), in parameter order.  The
    /// body reaches such a parameter as `Addr::Indirect` on
    /// `Expr::Load { addr: Addr::Frame(offset), .. }`; the list lets the
    /// selector recognise those slots (codegen-patterns 12.2).
    pub address_params: Vec<i32>,
    /// The routine's own local variables of 1, 2 or 4 bytes (not the
    /// function result variable, not parameter copies): frame offset and
    /// IR type, in allocation order.  The selector may hold one in a
    /// register instead of its frame slot (codegen-patterns 12.5); the
    /// slot stays allocated either way.
    pub scalar_locals: Vec<ValueParam>,
    /// For a routine nested in another (ref 2.4): its enclosing routine
    /// and how it reaches the enclosing frame (codegen-patterns 6).
    pub nesting: Option<Nesting>,
    /// The objects of this routine's frame that its nested routines reach
    /// ([`Addr::Enclosing`]): frame offset and size in bytes, in offset
    /// order.  Parameter values and slots among them are read from the
    /// frame at each use, never held in a register (PMAP_$FLUSH 0xE1376C
    /// reloads its `(8,A6)` slot at 0xE1378A, 0xE1379C, 0xE139AE and
    /// 0xE139CA, which 0xE1359C reads as `(8,A2)`); such locals are not in
    /// [`Self::scalar_locals`] (its `(-0x50,A6)`, `(-0x50,A4)` in
    /// 0xE1360C, is only addressed in memory).
    pub uplevel_refs: Vec<(i32, u32)>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// How a nested routine finds its enclosing routine's frame (ref 2.4
/// "Nested Routines"; codegen-patterns 6, `tools/nestsurvey.py`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nesting {
    /// The enclosing routine's symbol.
    pub parent: String,
    pub link: StaticLink,
}

/// Where a nested routine's static link (the enclosing routine's frame
/// pointer) comes from.  The rule (codegen-patterns 6, Verified by survey:
/// 11 of 11 and 75 of 75 nested routines of the image): passed by the
/// caller exactly when the routine needs it (reaches the enclosing frame or
/// passes the link on; ROUTE_$SERVICE's nested 0xE69E40, which does not,
/// is called by its sibling with no A1 load, 0xE69F3C) and some call of
/// the routine comes from a routine other than its enclosing one (a
/// sibling, the routine itself, a deeper one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticLink {
    /// The caller passes it (m68k: in A1, `movea.l A6,A1` / `movea.l
    /// An,A1` before the `bsr`; the callee copies it, 0xE135A4 `movea.l
    /// A1,A2`).
    Passed,
    /// Only the enclosing routine calls it, so the enclosing frame is the
    /// caller's: the dynamic link the routine's own frame saved (m68k:
    /// `movea.l (A6),An`, 0xE13616).
    CallerFrame,
}

impl Function {
    /// A procedure with an empty body (codegen-patterns 2: compiled to a
    /// bare `rts`, TIME_$Q_INIT 0xE16C5C, MT_$RELEASE 0xE70A4C).  It
    /// never loads the data base.
    pub fn is_empty_procedure(&self) -> bool {
        self.result.is_none() && self.body.is_empty()
    }
}

/// Which routines make the unit's data area addressable on entry, beyond
/// the default rule (codegen-patterns 4, "Two modes"; `dpc --a5-*`).
///
/// The default ("used mode"): an exported routine loads the data base iff
/// it addresses the unit's data, directly or through `internal` routines
/// of the unit it calls that do not load it themselves; an `internal`
/// routine never loads it (it inherits it).  A routine compiled to a bare
/// `rts` ([`Function::is_empty_procedure`]) never loads it.  Names are
/// matched case-insensitively against routine symbols.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataBaseOptions {
    /// "Always mode" from this routine on, in emission order: it and every
    /// later exported routine load the data base whatever their body
    /// (SMD 0xE1D750: SMD_$COPY_FONT_TO_MD_HDM does not, SMD_$BUSY_WAIT
    /// 0xE1D8A4 and SMD_$LITES do).
    pub always_from: Option<String>,
    /// These routines load the data base whatever their body, `internal`
    /// ones included (the switch-backs of PKT 0xE1202C, XPD 0xE5AF38,
    /// FILE_ 0xE5D050; the unnamed routines that load it, e.g. 0xE14B12,
    /// RING 0xE76048).
    pub always: Vec<String>,
    /// These exported routines do not load the data base and address the
    /// unit's data absolutely (AUDIT_$SUSPEND 0xE70DB6, AUDIT_$RESUME
    /// 0xE70DD6, REM_NAME_$REGISTER_SERVER 0xE4A4AE).
    pub absolute: Vec<String>,
}

/// A parameter passed by value: where its value is in the frame and its
/// IR type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueParam {
    /// Frame-pointer offset of the value (`dpc_sema::Param::value_offset`).
    pub offset: i32,
    pub ty: Scalar,
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
    /// Store `value` into the scalar object at `addr`: `d := e` for a
    /// designator `d` other than a function result.  `value` has the
    /// object's width (a constant is given the object's type).
    Store { addr: Addr, value: Expr },
    /// Copy `bytes` bytes from the object at `src` to the object at `dst`:
    /// `d := s` for a record or array that is not scalar-sized (ref
    /// "Assignment", p. 4-25).  The objects do not overlap partially (two
    /// designators of the same type are the same object or disjoint).
    Copy { dst: Addr, src: Addr, bytes: u32 },
    /// A procedure call (ref 5.2).
    Call(Call),
    /// Store `value` (a constant of the element type) into `count`
    /// consecutive objects of `value`'s type, the first at `start`: a
    /// `for` loop over an array whose index is used for nothing else
    /// (`lower.rs`, `fill`).  `count` >= 1.
    Fill {
        start: Addr,
        count: u32,
        value: Expr,
    },
    /// `if cond then then_ else else_` (ref "If", p. 4-99).  An empty
    /// `else_` is an `if` without `else`; `then_` is never empty.
    If {
        cond: Cond,
        then_: Vec<Stmt>,
        else_: Vec<Stmt>,
    },
    /// `repeat body until until` (ref "Repeat/Until", p. 4-161): the body runs
    /// at least once, and again while `until` is false.
    Repeat { body: Vec<Stmt>, until: Cond },
    /// `while cond do body` (ref "While", p. 4-208): the body runs while
    /// `cond` is true, tested before each iteration.  `body` is never
    /// empty.
    While { cond: Cond, body: Vec<Stmt> },
    /// A `for` loop (ref "For", pp. 4-88..4-90) that is not a
    /// [`StmtKind::Fill`]; see [`ForLoop`].
    For(ForLoop),
    /// `exit` (ref "Exit", p. 4-77): leave the innermost enclosing loop
    /// (`for`, `while`, `repeat`) for the statement after it.
    Exit,
    /// `next` (ref "Next", p. 4-123): skip the rest of the innermost
    /// enclosing loop's body and go on with its next iteration (the test
    /// of a `while` or `repeat`, the step of a `for`).
    Next,
    /// `case selector of ... otherwise ... end` (ref "Case", pp.
    /// 4-36..4-37).  `selector` is a word or a longword (a byte selector
    /// is widened by lowering); every label fits its type and no label
    /// occurs twice.  `arms` are in source order, an arm's labels in source
    /// order; an arm's `body` may be empty (an empty statement).  When no
    /// label matches and there is no `otherwise`, execution continues
    /// after the statement: Domain Pascal "does not detect the lack of a
    /// case statement constant corresponding to a run-time case value"
    /// (ref Appendix D, ISO 6.8.3.5, p. D-4).  `otherwise`, when present,
    /// is not empty.
    Case {
        selector: Expr,
        arms: Vec<CaseArm>,
        otherwise: Option<Vec<Stmt>>,
    },
    /// `return` (ref "Return", p. 4-166): leave the routine.
    Return,
    /// `disable` (ref Appendix E, p. E-3): turn the interrupt enable off
    /// (m68k: raise the priority mask to 7, codegen-patterns 10).
    Disable,
    /// `enable` (ref Appendix E, p. E-4): turn the interrupt enable on
    /// (m68k: priority mask 0, codegen-patterns 10).
    Enable,
    /// `discard(e)` (ref "Discard", p. 4-62) of a value that is not a
    /// call: evaluate `e` for its effect, drop the value.  Only
    /// [`Expr::SetSr`] is produced (`lower.rs`); a discarded function call
    /// is a [`StmtKind::Call`].
    Discard(Expr),
}

impl StmtKind {
    /// The statement sequences nested in this statement: an `if`'s two
    /// parts, a loop's body, a `case`'s arms and `otherwise` part.
    pub fn bodies(&self) -> Vec<&[Stmt]> {
        match self {
            StmtKind::If { then_, else_, .. } => vec![then_, else_],
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => vec![body],
            StmtKind::For(l) => vec![&l.body],
            StmtKind::Case {
                arms, otherwise, ..
            } => arms
                .iter()
                .map(|a| a.body.as_slice())
                .chain(otherwise.as_deref())
                .collect(),
            _ => vec![],
        }
    }
}

/// Visit every statement of `body` and of the sequences nested in it, each
/// statement before the ones inside it.
pub fn visit_stmts<'a>(body: &'a [Stmt], f: &mut dyn FnMut(&'a Stmt)) {
    for s in body {
        f(s);
        for b in s.kind.bodies() {
            visit_stmts(b, f);
        }
    }
}

/// `for index := from to|downto to do body` (ref "For", pp. 4-88..4-90).
///
/// The index variable is a local of the routine (a word in its frame,
/// [`Self::index`]) that nothing outside the bodies of `for` loops over it
/// reads, that the body does not assign, and whose address is never taken
/// (`lower.rs`, `for_loop`): the target may keep it in a register only
/// (codegen-patterns 7.4: the frame slot stays allocated and is never
/// written).  The body reads it as `Expr::Load { addr: Addr::Frame(..) }`.
/// `from` and `to` have the index's type.  Ref p. 4-90: "If you set up a
/// meaningless relationship between the start_exp and the stop_exp ...
/// Pascal does not execute the loop even once"; the value of the index
/// after a loop that ends normally is not defined (warning "Potential
/// illegal use of FOR index variable", ref p. 9-31).
#[derive(Clone, Debug, PartialEq)]
pub struct ForLoop {
    /// The index variable: its frame offset and IR type (a word).
    pub index: ValueParam,
    pub from: Expr,
    pub to: Expr,
    /// `downto`: the index decreases.
    pub down: bool,
    /// Never empty.
    pub body: Vec<Stmt>,
}

/// One arm of a [`StmtKind::Case`]: its labels (ordinal values of the
/// selector's type) and its statements.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseArm {
    pub labels: Vec<i64>,
    pub body: Vec<Stmt>,
}

/// A condition that decides a branch (the test of an `if` or a loop).
/// Conditions are compiled to compares and conditional branches, never
/// materialised as a boolean value (codegen-patterns 7).
#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    /// `lhs op rhs`.  Both operands have the same width and sign; the
    /// sign decides a signed or unsigned compare.
    Compare { op: Rel, lhs: Expr, rhs: Expr },
    /// A boolean value is true (its byte is 0xFF, ref 3.5.3 p. 3-11).
    Test(Expr),
    /// `not c` (ref "Not", p. 4-126).
    Not(Box<Cond>),
    /// `a and then b`: `b` is evaluated only when `a` is true (ref "And
    /// Then", pp. 4-18..4-19).
    AndThen(Box<Cond>, Box<Cond>),
    /// `a or else b`: `b` is evaluated only when `a` is false (ref "Or
    /// Else", p. 4-134).
    OrElse(Box<Cond>, Box<Cond>),
}

/// A relational operator (ref Table 4-1, p. 4-3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rel {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Rel {
    /// The relation that holds exactly when `self` does not.
    pub fn negate(self) -> Rel {
        match self {
            Rel::Eq => Rel::Ne,
            Rel::Ne => Rel::Eq,
            Rel::Lt => Rel::Ge,
            Rel::Ge => Rel::Lt,
            Rel::Le => Rel::Gt,
            Rel::Gt => Rel::Le,
        }
    }
}

/// A call: the callee, the arguments in parameter order (argument 1
/// first), and the alignment pad the caller pushes before them
/// (`dpc_sema::Routine::arg_pad`; codegen-patterns 1.4).
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub callee: Callee,
    pub args: Vec<Arg>,
    pub pad: u32,
    /// The static link the caller passes, for a callee whose link is
    /// [`StaticLink::Passed`]; `None` for any other callee.
    pub link: Option<LinkArg>,
}

/// The static link passed to a nested routine ([`StaticLink::Passed`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkArg {
    /// The caller's own frame: the callee is nested in the caller
    /// (PMAP_$FLUSH 0xE13928 `movea.l A6,A1; bsr.w 0xE1359C`).
    OwnFrame,
    /// The caller's own static link: the callee is nested in the same
    /// routine as the caller (a sibling: 0xE13718 `movea.l A4,A1; bsr.w
    /// 0xE1359C` in 0xE1360C, A4 from `(A6)`), or is the caller itself
    /// (0xE397D0, recursive).
    OwnLink,
}

impl Call {
    /// Bytes the caller pops after the call: the arguments and the pad.
    pub fn bytes(&self) -> u32 {
        self.args.iter().map(Arg::slot).sum::<u32>() + self.pad
    }
}

/// Where a call goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Callee {
    /// A routine with a body in this unit, in the same code section,
    /// named by its symbol: a PC-relative call (m68k `bsr`,
    /// codegen-patterns 1.6).
    Local(String),
    /// A routine of another unit, reached through its symbol (m68k `jsr
    /// abs.l`).
    External(String),
}

/// One argument as the caller pushes it.  `slot` is the bytes it
/// occupies in the push (`dpc_sema::conv::ParamConv::slot_size`: on m68k
/// a byte value takes a 2-byte slot, an address 4).
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// The value itself (ref 5.2 Table 5-1: `val_param` / `internal`
    /// value and `in` parameters of 4 bytes or less).
    Value { value: Expr, slot: u32 },
    /// The address of the object at `addr` (the default convention, and
    /// the `var`-like modes).
    ///
    /// `writable`: the parameter is `var`, `out` or `in out`, so the callee
    /// may change the object behind the caller's back (ref 5.3.1, p. 5-6:
    /// "whatever happens to y in addc happens also to x"; 5.3.2, p. 5-7).
    /// False for a value or `in` parameter passed by address: the callee
    /// copies a value parameter (ref 5.2, p. 5-4, "by making a local copy
    /// of the argument") and may not assign an `in` one (5.3.2, p. 5-7,
    /// "Assignment to IN argument").  The selector never holds an object
    /// passed this way in a register across the call (bead
    /// domain-pascal-pe9; codegen-patterns 12.2).
    Address {
        addr: Addr,
        slot: u32,
        writable: bool,
    },
}

impl Arg {
    /// Bytes the argument occupies in the caller's push.
    pub fn slot(&self) -> u32 {
        match self {
            Arg::Value { slot, .. } | Arg::Address { slot, .. } => *slot,
        }
    }
}

/// A value.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A constant of the given type.  `value` is the number the bits
    /// stand for under `ty` (a boolean is -1 for true, its 0xFF byte, and
    /// 0 for false: ref 3.5.3, p. 3-11).
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
    /// The address of the object at `addr`, as a pointer value (`addr(x)`,
    /// ref p. 4-13).
    AddrOf(Addr),
    /// `lhs op rhs` on integers of type `ty` (ref 4.3.1, p. 4-5, and the
    /// bit operators, pp. 4-33, 4-109, 4-171, 4-29, 4-221).  For the
    /// arithmetic and bit operators both operands have type `ty` (lowering
    /// inserts [`Expr::Convert`] where the source mixed widths); for the
    /// shifts `lhs` has type `ty` and `rhs` is the count, of any integer
    /// type.
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        ty: Scalar,
    },
    /// `op operand`, the operand of type `ty`.
    Unary {
        op: UnOp,
        operand: Box<Expr>,
        ty: Scalar,
    },
    /// `value` converted to `ty`: widened (sign- or zero-extended by
    /// `value`'s sign, ref 4.3.1 p. 4-5) or narrowed to its low bytes.
    Convert { value: Box<Expr>, ty: Scalar },
    /// The result of a function call (ref 5.2; the result register,
    /// codegen-patterns 3), of type `ty`.
    Call { call: Call, ty: Scalar },
    /// A condition as a boolean value (0xFF true, 0 false; ref 3.5.3).
    Test(Box<Cond>),
    /// `set_sr(newsr)` (ref Appendix E, p. E-5): the processor status
    /// word before `newsr` (a word) is installed; a signed word.
    SetSr(Box<Expr>),
}

/// A binary integer operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// `div`: truncating division (ref "Div", p. 4-66).
    Div,
    /// `mod` (ref "Mod", pp. 4-115..4-116): without -iso the remainder has
    /// the dividend's sign (`-10 mod 3` is -1), as MC68000 `divs` gives.
    Mod,
    /// `&` (ref "Bit operators", p. 4-33).
    And,
    /// `!` (p. 4-33).
    Or,
    /// `xor(a, b)` (p. 4-221).
    Xor,
    /// `lshft(x, n)` (p. 4-109): zeros shifted in on the right.
    Shl,
    /// `rshft(x, n)` (p. 4-171): zeros shifted in on the left.
    Shr,
    /// `arshft(x, n)` (p. 4-29): the sign bit kept.
    Ashr,
}

impl BinOp {
    /// Whether `a op b` = `b op a`.
    pub fn commutative(self) -> bool {
        matches!(
            self,
            BinOp::Add | BinOp::Mul | BinOp::And | BinOp::Or | BinOp::Xor
        )
    }
}

/// A unary integer operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// `-x`.
    Neg,
    /// `~x` (ref p. 4-33): the bits complemented.
    Not,
}

impl Expr {
    /// The IR type of the value.
    pub fn ty(&self) -> Scalar {
        match self {
            Expr::Const { ty, .. }
            | Expr::Load { ty, .. }
            | Expr::Binary { ty, .. }
            | Expr::Unary { ty, .. }
            | Expr::Convert { ty, .. }
            | Expr::Call { ty, .. } => *ty,
            Expr::SetMember { .. } | Expr::Test(_) => Scalar::BOOLEAN,
            Expr::SetSr(_) => Scalar::signed(Width::Word),
            Expr::AddrOf(_) => Scalar::POINTER,
        }
    }
}

/// An address.
#[derive(Clone, Debug, PartialEq)]
pub enum Addr {
    /// Frame pointer + offset.
    Frame(i32),
    /// The enclosing routine's frame + offset, for a routine nested one
    /// level in it (ref 2.4): a variable or parameter of the enclosing
    /// routine, reached through the static link (m68k: `(d,An)` with An
    /// the link, codegen-patterns 6: 0xE135AA `btst #0,(0x15,A2)`).
    Enclosing(i32),
    /// An absolute symbol + offset (another unit's object).
    Symbol { name: String, offset: i32 },
    /// The unit's own static data area ([`Module::data`]) + offset.
    Static(i32),
    /// The pointer value `base` + offset.
    Indirect { base: Box<Expr>, offset: i32 },
    /// An array element: the address `base` + `index` × `scale` +
    /// `offset`.  `offset` holds the lower bound folded in (`-lo ×
    /// scale`, codegen-patterns 5.2: `(-2,A0,Dn.w)` for a 1-based array of
    /// words, AUDIT_$IS_PROCESS_AUDITED 0xE70DA8) and the offsets of
    /// fields selected after the indexing; `base`'s own offset is the
    /// array's.  `index` is an integer or ordinal value of 2 or 4 bytes.
    Index {
        base: Box<Addr>,
        index: Box<Expr>,
        scale: u32,
        offset: i32,
    },
}

impl Addr {
    /// This address plus `delta` bytes.
    pub fn offset_by(self, delta: i32) -> Addr {
        match self {
            Addr::Frame(o) => Addr::Frame(o + delta),
            Addr::Enclosing(o) => Addr::Enclosing(o + delta),
            Addr::Static(o) => Addr::Static(o + delta),
            Addr::Symbol { name, offset } => Addr::Symbol {
                name,
                offset: offset + delta,
            },
            Addr::Indirect { base, offset } => Addr::Indirect {
                base,
                offset: offset + delta,
            },
            Addr::Index {
                base,
                index,
                scale,
                offset,
            } => Addr::Index {
                base,
                index,
                scale,
                offset: offset + delta,
            },
        }
    }
}
