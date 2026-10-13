//! Abstract syntax tree (CLAUDE.md section 4.1).
//!
//! Plain enums and structs.  Every node carries a [`Span`].  Identifiers
//! are folded to lower case ([`Ident::name`]); the original spelling is the
//! span's text ([`crate::SourceMap::span_text`]).  The tree knows nothing
//! about the target: no registers, slots or sections other than the names
//! the source writes (CLAUDE.md 4.5).
//!
//! The parser does no name resolution.  Things it cannot tell apart are
//! left to semantic analysis:
//!
//! - `f(x)` is an [`ExprKind::Call`] whether `f` is a function, a
//!   predeclared routine (`addr`, `sizeof`, `lshft`, ...) or a type used as
//!   a type transfer function (ref 4-197).
//! - a bare name in a statement (`disable;`) is a [`StmtKind::Call`] of an
//!   [`ExprKind::Name`].
//! - a bracketed initialiser (`:= [1, 2]`) is an [`Init::List`] whether the
//!   variable is an array, a record or a set (ref 3.9.2, 3.10.4, 3.11.1).
//!
//! Compiler directives that survive preprocessing (alignment and inlining,
//! ref 4.7 pp. 4-52..4-58) appear as [`Decl::Pragma`] items at declaration
//! boundaries; see [`PragmaMark`].

use crate::lexer::Pragma;
use crate::source::Span;

/// The smallest span covering `a` and `b`.  When they lie in different
/// files (a construct that starts in an `%include` file and ends outside
/// it) the result is `a`.
pub fn join(a: Span, b: Span) -> Span {
    if a.file != b.file {
        return a;
    }
    Span::new(a.file, a.lo.min(b.lo), a.hi.max(b.hi))
}

/// An identifier, folded to lower case (ref 2.1.6, p. 2-6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A label: an identifier or an unsigned integer (ref 2.2.2.1, p. 2-11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    Int(u64, Span),
    Ident(Ident),
}

impl Label {
    pub fn span(&self) -> Span {
        match self {
            Label::Int(_, s) => *s,
            Label::Ident(i) => i.span,
        }
    }
}

/// A directive marker kept for later phases (ref 4.7, pp. 4-52..4-58).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PragmaMark {
    pub pragma: Pragma,
    pub span: Span,
}

// ---------------------------------------------------------------------
// Compilation units
// ---------------------------------------------------------------------

/// `program` or `module` (ref 2.2.1 p. 2-10; 7.1.1 p. 7-2).
#[derive(Clone, Debug, PartialEq)]
pub enum UnitKind {
    /// `program name [(file_list)] ...`.  The file list is ignored by the
    /// compiler (p. 2-10) but kept here.
    Program {
        file_list: Vec<Ident>,
    },
    Module,
}

/// A compilation unit (ref 2.2 Figure 2-1, p. 2-8; 7.1 Figure 7-1, p. 7-2).
#[derive(Clone, Debug, PartialEq)]
pub struct CompilationUnit {
    pub kind: UnitKind,
    pub name: Ident,
    /// `, code_section_name` (p. 2-10, p. 7-2).
    pub code_section: Option<Ident>,
    /// `, data_section_name` (p. 2-10, p. 7-2).
    pub data_section: Option<Ident>,
    /// Directives before the heading (e.g. `%natural_alignment;` at the
    /// top of the file, p. 4-57).
    pub leading_pragmas: Vec<PragmaMark>,
    /// Declaration parts and routines, in source order.
    pub decls: Vec<Decl>,
    /// The main program's action part; `None` for a module.
    pub body: Option<Block>,
    /// Directives after the declarations (for a program: found in or after
    /// the action part).
    pub trailing_pragmas: Vec<PragmaMark>,
    pub span: Span,
}

// ---------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------

/// One declaration part, routine or directive.  Parts may come in any
/// order and repeat (ref 2.2.2, p. 2-11).
///
/// A directive between two items of one part splits the part in two: the
/// second half has `continues_previous` set, so semantic analysis can treat
/// the halves as one part (e.g. for forward pointer types).
#[derive(Clone, Debug, PartialEq)]
pub enum Decl {
    Label(LabelPart),
    Const(ConstPart),
    Type(TypePart),
    Var(VarPart),
    Define(DefinePart),
    Attribute(AttributePart),
    RoutineOption(RoutineOptionPart),
    Routine(Box<Routine>),
    Pragma(PragmaMark),
}

/// `label l1, ..., lN;` (ref 2.2.2.1, p. 2-11).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelPart {
    pub labels: Vec<Label>,
    pub span: Span,
}

/// `const id = value; ...` (ref 2.2.2.2, p. 2-12).
#[derive(Clone, Debug, PartialEq)]
pub struct ConstPart {
    pub defs: Vec<ConstDef>,
    pub continues_previous: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstDef {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

/// `type id = type; ...` (ref 2.2.2.3, p. 2-13).
#[derive(Clone, Debug, PartialEq)]
pub struct TypePart {
    pub defs: Vec<TypeDef>,
    pub continues_previous: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDef {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

/// The name of an overlay section in `var (name)`: an identifier or a
/// quoted string (ref 3.14, p. 3-52).
#[derive(Clone, Debug, PartialEq)]
pub enum SectionName {
    Ident(Ident),
    Str(String, Span),
}

/// `var [(section)] decl; ...` (ref 2.2.2.4 p. 2-14; 3.14 p. 3-52; 7.2
/// p. 7-3).
#[derive(Clone, Debug, PartialEq)]
pub struct VarPart {
    pub section: Option<SectionName>,
    pub decls: Vec<VarDecl>,
    pub continues_previous: bool,
    pub span: Span,
}

/// Variable allocation clause (ref 7.2, p. 7-3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocKind {
    Define,
    Extern,
    Static,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alloc {
    pub kind: AllocKind,
    pub span: Span,
}

/// `a, b : [alloc] type [:= init]` (ref 2.2.2.4 p. 2-14; 7.2 p. 7-3;
/// initialisers 3.2.2 p. 3-4).
#[derive(Clone, Debug, PartialEq)]
pub struct VarDecl {
    pub names: Vec<Ident>,
    pub alloc: Option<Alloc>,
    pub ty: TypeExpr,
    pub init: Option<Init>,
    pub span: Span,
}

/// `define v1 [:= init1]; ...` (ref 7.2.2, p. 7-5).  Also names routines
/// (7.3.4, p. 7-11).
#[derive(Clone, Debug, PartialEq)]
pub struct DefinePart {
    pub items: Vec<DefineItem>,
    pub continues_previous: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefineItem {
    pub name: Ident,
    pub init: Option<Init>,
    pub span: Span,
}

/// `attribute id = attr, ...; ...` (ref 3.16, p. 3-78).
#[derive(Clone, Debug, PartialEq)]
pub struct AttributePart {
    pub defs: Vec<AttributeDef>,
    pub continues_previous: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeDef {
    pub name: Ident,
    pub attrs: Vec<Attribute>,
    pub span: Span,
}

/// `routine_option id = opt, ...; ...` (ref 5.6.1, p. 5-21).  The name
/// `default_routine_options` is special (5.6.4, p. 5-22); that is for
/// semantic analysis.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutineOptionPart {
    pub defs: Vec<RoutineOptionDef>,
    pub continues_previous: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutineOptionDef {
    pub name: Ident,
    pub options: Vec<Ident>,
    pub span: Span,
}

// ---------------------------------------------------------------------
// Initialisers
// ---------------------------------------------------------------------

/// The value after `:=` in a var or define declaration.
#[derive(Clone, Debug, PartialEq)]
pub enum Init {
    /// A constant expression (`0`, `'text'`, `nil`, `msg1`, `chr(10)`).
    Expr(Expr),
    /// `[item, ...]`, a trailing comma allowed (ref 3.11.1.2 p. 3-40;
    /// 3.11.1.3 p. 3-41).  Array, record or set: see the module docs.
    List { items: Vec<InitItem>, span: Span },
}

impl Init {
    pub fn span(&self) -> Span {
        match self {
            Init::Expr(e) => e.span,
            Init::List { span, .. } => *span,
        }
    }
}

/// The upper bound of an index range in an array type or an initialiser:
/// an expression or `*` (ref 3.11.1.5 p. 3-42; 3.11.1.6 p. 3-43).
#[derive(Clone, Debug, PartialEq)]
pub enum Bound {
    Expr(Expr),
    Star(Span),
}

impl Bound {
    pub fn span(&self) -> Span {
        match self {
            Bound::Expr(e) => e.span,
            Bound::Star(s) => *s,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InitItem {
    /// A value for the next component or field (3.10.4 p. 3-21; 3.11.1.2
    /// p. 3-39).
    Value(Init),
    /// `lo..hi` without `:=`: a set element range (3.9.2, p. 3-15).
    Range { lo: Expr, hi: Expr, span: Span },
    /// `n of value` or `* of value` (`count` is `None`) (3.11.1.3, p. 3-40).
    Repeat {
        count: Option<Expr>,
        value: Init,
        span: Span,
    },
    /// `key := value`, where the key is a field name (3.10.4 p. 3-21), an
    /// index or an index range `lo..hi` / `lo..*` (3.11.1.1 p. 3-39;
    /// 3.11.1.4 p. 3-41; 3.11.1.6 p. 3-43).
    Keyed {
        key: Expr,
        key_hi: Option<Bound>,
        value: Init,
        span: Span,
    },
}

// ---------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeKind,
    pub span: Span,
}

/// `record`, `packed record`, `aligned record`, `unaligned record`
/// (ref 3.10.3 p. 3-21; 3.10.6 pp. 3-33..3-34).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordForm {
    Plain,
    Packed,
    Aligned,
    Unaligned,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    /// A type identifier (`integer`, `integer32`, `univ_ptr`, `string`,
    /// a user type, ...).
    Named(Ident),
    /// `lo..hi` (ref 3.8, p. 3-14).  `hi` is `Bound::Star` only in an array
    /// index (ref 3.11.1.5, p. 3-42).
    Subrange { lo: Expr, hi: Bound },
    /// `(a, b, c)` (ref 3.7, p. 3-13).
    Enum(Vec<Ident>),
    /// `[packed] array[i1, ..., iN] of elem` (ref 3.11 p. 3-37; 3.11.3
    /// p. 3-45).
    Array {
        packed: bool,
        indices: Vec<TypeExpr>,
        elem: Box<TypeExpr>,
    },
    /// Records (ref 3.10, pp. 3-17..3-21).
    Record {
        form: RecordForm,
        fields: Vec<FieldDecl>,
        variant: Option<Box<VariantPart>>,
    },
    /// `[packed] set of elem` (ref 3.9.1, p. 3-15).
    Set { packed: bool, elem: Box<TypeExpr> },
    /// `[packed] file of elem` (ref 3.12, p. 3-48).
    File { packed: bool, elem: Box<TypeExpr> },
    /// `varying[max] of elem` (ref 3.11.2, p. 3-44).
    Varying { max: Expr, elem: Box<TypeExpr> },
    /// `^T` (ref 3.13.1, p. 3-49).
    Pointer(Box<TypeExpr>),
    /// `^procedure (...)` / `^function (...): T` (ref 3.13.3, p. 3-50).
    RoutinePointer(Box<RoutineSig>),
    /// `[attr, ...] T` (ref 3.15, p. 3-53).
    Attributed {
        attrs: Vec<Attribute>,
        ty: Box<TypeExpr>,
    },
}

/// A variable/type attribute: `volatile`, `device(read)`, `address(n)`,
/// `aligned(1)`, `long`, a user attribute name, ... (ref 3.15
/// pp. 3-53..3-55; 3.16 p. 3-78).
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub name: Ident,
    /// `None` when written without parentheses.
    pub args: Option<Vec<Expr>>,
    pub span: Span,
}

/// `name1, ..., nameN : type` (ref 3.10.1, p. 3-17).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDecl {
    pub names: Vec<Ident>,
    pub ty: TypeExpr,
    pub span: Span,
}

/// `case [tag :] typename of arms` (ref 3.10.2, p. 3-19).
#[derive(Clone, Debug, PartialEq)]
pub struct VariantPart {
    pub tag: Option<Ident>,
    pub tag_type: Ident,
    pub arms: Vec<VariantArm>,
    pub span: Span,
}

/// `c1, ..., cN : (fields [; variant_part])` (ref 3.10.2, p. 3-19).
#[derive(Clone, Debug, PartialEq)]
pub struct VariantArm {
    pub labels: Vec<Expr>,
    pub fields: Vec<FieldDecl>,
    pub variant: Option<Box<VariantPart>>,
    pub span: Span,
}

// ---------------------------------------------------------------------
// Routines
// ---------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutineKind {
    Procedure,
    Function,
}

/// Parameter mode (ref 5.3, p. 5-5; 5.3.2, p. 5-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamMode {
    /// No mode: a value parameter.
    Value,
    Var,
    In,
    Out,
    InOut,
}

/// One group of a parameter list (ref 5.1, p. 5-1).
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    /// `[mode] a, b : [univ] type` (ref 5.1 p. 5-1; 5.3.3 univ).
    Data {
        mode: ParamMode,
        names: Vec<Ident>,
        univ: bool,
        ty: TypeExpr,
        span: Span,
    },
    /// `procedure p (...)` / `function f (...): T` as a parameter (ref 5.4,
    /// p. 5-13).
    Routine { sig: RoutineSig, span: Span },
}

/// A routine heading without options: kind, optional name, parameters and
/// result type.  Used for parameters and for routine pointer types (whose
/// `name` is `None`).
#[derive(Clone, Debug, PartialEq)]
pub struct RoutineSig {
    pub kind: RoutineKind,
    pub name: Option<Ident>,
    /// `None` when no parenthesised list was written.
    pub params: Option<Vec<Param>>,
    pub result: Option<TypeExpr>,
    /// Routine options written after a routine pointer type, in source
    /// order (`^procedure (x: integer); val_param;`).  The reference shows
    /// the short form after the type, `pptr : ^Procedure(...); EXTERN;`
    /// ("Addressing Procedure and Function Pointers", p. 4-146, checked in
    /// the PDF), and Appendix B messages 181 ("INTERNAL option is illegal
    /// for PROCEDURE^ or FUNCTION^ types") and 182 ("Incompatible VAL_PARAM
    /// options": "The type declaration did not specify val_param")
    /// presuppose options on such types.  Always empty for a routine
    /// parameter (5.4).
    pub options: Vec<Ident>,
    pub span: Span,
}

/// A routine attribute (ref 5.7, p. 5-24).
#[derive(Clone, Debug, PartialEq)]
pub enum RoutineAttr {
    /// `section(code, data)`; either name may be omitted (5.7.1, p. 5-25).
    Section {
        code: Option<Ident>,
        data: Option<Ident>,
        span: Span,
    },
}

/// A procedure or function declaration (ref 2.2.3.1 p. 2-16; chapter 5).
#[derive(Clone, Debug, PartialEq)]
pub struct Routine {
    /// `[section(...)]` before the heading (5.7.1, p. 5-25).
    pub attrs: Vec<RoutineAttr>,
    pub kind: RoutineKind,
    pub name: Ident,
    /// `None` when no parameter list was written.  A body for a routine
    /// declared earlier `extern`/`forward` may repeat just the name (ref
    /// 7.3.4, p. 7-11).
    pub params: Option<Vec<Param>>,
    pub result: Option<TypeExpr>,
    /// Routine options in source order, from both `options(...)` and the
    /// short form (5.5.1, p. 5-15).  Includes user-defined option names
    /// (5.6).
    pub options: Vec<Ident>,
    /// `None` for a `forward` or `extern` declaration.
    pub body: Option<RoutineBody>,
    pub span: Span,
}

/// Declarations (including nested routines) and the action part (ref
/// 2.2.3.2..2.2.3.4, pp. 2-16..2-17).
#[derive(Clone, Debug, PartialEq)]
pub struct RoutineBody {
    pub decls: Vec<Decl>,
    pub block: Block,
}

// ---------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------

/// `begin stmt; ...; stmt end` (ref "Statements", p. 4-188).
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForDir {
    To,
    Downto,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// The empty statement (p. 4-188).
    Empty,
    /// `label: stmt` (p. 4-188).
    Labeled {
        label: Label,
        stmt: Box<Stmt>,
    },
    /// `target := value`.
    Assign {
        target: Expr,
        value: Expr,
    },
    /// A procedure call: a name, a call expression, or a call through a
    /// routine pointer (`p^(x)`, ref 5.3.4 p. 5-12).
    Call(Expr),
    Compound(Block),
    /// ref p. 4-99.
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    /// ref pp. 4-36..4-38.  `otherwise` holds a statement list (p. 4-36).
    Case {
        expr: Expr,
        arms: Vec<CaseArm>,
        otherwise: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    /// ref p. 4-161.
    Repeat {
        body: Vec<Stmt>,
        cond: Expr,
    },
    For {
        var: Ident,
        from: Expr,
        dir: ForDir,
        to: Expr,
        body: Box<Stmt>,
    },
    /// ref pp. 4-211..4-214, incl. the `v : alias` extension (p. 4-212).
    With {
        items: Vec<WithItem>,
        body: Box<Stmt>,
    },
    /// ref p. 4-95.
    Goto(Label),
    /// ref p. 4-77.
    Exit,
    /// ref p. 4-123.
    Next,
    /// ref p. 4-166.
    Return,
}

/// `c1, ..., cN : stmt` (ref p. 4-36).
#[derive(Clone, Debug, PartialEq)]
pub struct CaseArm {
    pub labels: Vec<Expr>,
    pub stmt: Stmt,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WithItem {
    pub record: Expr,
    /// `v : identifier` (p. 4-212).
    pub alias: Option<Ident>,
}

// ---------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// Unary operators.  `not` and `~` are prefix operators on a factor;
/// `-`/`+` are signs of a simple expression (standard Pascal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
    /// `~` (p. 4-33).
    BitNot,
    Neg,
    Plus,
}

/// Binary operators (Table 4-1, p. 4-3; precedence Table 4-3, p. 4-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    // Table 4-3 level 2.
    Pow,
    // Level 3.
    BitAnd,
    Mul,
    RealDiv,
    Div,
    Mod,
    And,
    // Level 4.
    BitOr,
    Add,
    Sub,
    Or,
    // Level 5.
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    // Level 6.
    AndThen,
    // Level 7.
    OrElse,
}

/// One piece of a string literal with embedded character codes:
/// `'newline'(CR, LF)` (ref 2.1.5.1, p. 2-5).
#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Text(String),
    Code(Expr),
}

/// `lo` or `lo..hi` in a set constructor.
#[derive(Clone, Debug, PartialEq)]
pub struct SetElem {
    pub lo: Expr,
    pub hi: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int(u64),
    /// As written; reals are out of scope for code generation.
    Real(String),
    /// A string literal.  Usually one `Text` part.
    Str(Vec<StrPart>),
    /// A name (variable, constant, function, `nil`, `true`, ...).
    Name(Ident),
    /// `[e1, lo..hi, ...]`
    Set(Vec<SetElem>),
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `callee(args)`: function call, type transfer, predeclared routine.
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// `base[i, j]`
    Index {
        base: Box<Expr>,
        indices: Vec<Expr>,
    },
    /// `base.field`
    Field {
        base: Box<Expr>,
        field: Ident,
    },
    /// `base^`
    Deref(Box<Expr>),
    /// `(e)`: kept so diagnostics and later phases can see the grouping.
    Paren(Box<Expr>),
}
