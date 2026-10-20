//! The checker: walks the AST, builds scopes, types, layouts, routines and
//! variables, and produces the annotated [`Program`].
//!
//! Declarations are processed in source order (ref 2.2.2 p. 2-11: parts in
//! any order; a name must be declared before use, except pointer targets
//! within one type part, ref 3.13.1).

use std::collections::{HashMap, HashSet};

use dpc_syntax::ast::{self, BinOp, Decl, ParamMode, RoutineKind, UnOp};
use dpc_syntax::lexer::Pragma;
use dpc_syntax::{Diagnostic, Span};
use dpc_target::Target;

use crate::consts::ConstValue;
use crate::conv::{self, Passing, RoutineOptions};
use crate::program::*;
use crate::types::{
    round_up, BitPos, DeviceAccess, Field, PackedElem, RecordType, SigParam, Signature, TypeAttrs,
    TypeId, TypeKind, TypeTable, VariantArm, VariantPart, MAX_OBJECT_SIZE,
};

/// Smallest alignment of an object in a routine's frame, in bytes: the
/// original compiler word-aligns byte locals (codegen-patterns 2, "Frame
/// layout"; Verified by survey, bead domain-pascal-ek7).
const FRAME_MIN_ALIGN: u32 = 2;

/// Smallest alignment of a static object (a unit-level, `define` or
/// `static` variable) in its data section, in bytes: the original
/// compiler word-aligns byte variables there too, one to a word
/// (codegen-patterns 11, "Byte-sized module variables"; Verified by
/// survey, bead domain-pascal-yvko: the 26 map-named byte variables of
/// compiled modules' data blocks are all at even addresses, and the 10
/// followed by another map name have it 2 bytes on, e.g.
/// NETWORK_$ACTIVITY_FLAG 0xE24C42 .. NETWORK_$REALLY_DISKLESS 0xE24C4A,
/// `tools/datasurvey.py names`; SIO_IO's char commands
/// at +0x48, +0x4A, +0x54..+0x5C of 0xE2DEB8, `70 00 60 00`).  Ref 3.5.3
/// p. 3-11 ("stores freestanding Boolean objects on byte boundaries") and
/// 3.6.4 p. 3-13 ("freestanding char variables are byte aligned") give
/// them byte alignment, which 3.10.5.1 p. 3-23 defines as a permission
/// ("can allocate byte aligned objects on any byte boundary"); the
/// padding is the compiler's.
const STATIC_MIN_ALIGN: u32 = 2;

/// What a name means.
#[derive(Clone, Debug)]
enum Sym {
    Const {
        value: ConstValue,
        ty: TypeId,
    },
    Type(TypeId),
    Var(VarId),
    Routine(RoutineId),
    Builtin(Builtin),
    /// A `routine_option` name (ref 5.6.1, p. 5-21).
    Options(RoutineOptions),
    /// A name from an attribute declaration part (ref 3.16, p. 3-78): the
    /// attributes it stands for, already resolved ("the compiler evaluates
    /// attributes when they are declared").
    Attributes(Attrs),
    /// A field visible through `with` (ref p. 4-211).
    WithField {
        with: WithId,
        rec_ty: TypeId,
    },
    /// `with r : alias` (ref p. 4-212).
    WithAlias {
        with: WithId,
        rec_ty: TypeId,
    },
    /// A label (ref 2.2.2.1, p. 2-11), with the routine that declares it
    /// (`None`: the main program).
    Label(Option<RoutineId>),
}

#[derive(Debug, Default)]
struct Scope {
    names: HashMap<String, Sym>,
}

/// Attributes written in a `[...]` list or named in an attribute
/// declaration part, resolved (ref 3.15, 3.16).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Attrs {
    volatile: bool,
    atomic: bool,
    device: Option<DeviceAccess>,
    /// `aligned(n)`: 2**n bytes.
    aligned: Option<u32>,
    natural: bool,
    /// A size attribute: bits (ref 3.15.5).
    size: Option<SizeAttr>,
}

/// A size attribute, `[name [(n)]]` (ref 3.15.5, p. 3-60): n units of
/// `bit`, `byte`, `word`, `long` or `quad`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SizeAttr {
    bits: u32,
    span: Span,
}

/// How a record's fields are laid out.
#[derive(Clone, Copy, Debug)]
struct Layout {
    packed: bool,
    /// The natural alignment rules (not for a packed record).
    natural: bool,
}

/// The predeclared attribute names (ref 3.15 p. 3-53; Table 3-6 p. 3-55).
const PREDECLARED_ATTRIBUTES: &[&str] = &[
    "volatile", "atomic", "device", "address", "bit", "byte", "word", "long", "quad", "aligned",
    "natural",
];

/// The routine whose body is being checked.
struct RoutineCtx {
    id: RoutineId,
    level: u32,
    /// Bytes allocated below the frame pointer so far.
    frame: u32,
}

/// A pointer type whose target was not yet declared (ref 3.13.1).
struct PendingPointer {
    ptr: TypeId,
    name: String,
    span: Span,
}

pub(crate) struct Checker<'t> {
    target: &'t dyn Target,
    types: TypeTable,
    vars: Vec<Var>,
    routines: Vec<Routine>,
    scopes: Vec<Scope>,
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
    ctx: Vec<RoutineCtx>,
    /// Bytes allocated so far in each static data section.
    sections: Vec<(String, u32)>,
    code_section: String,
    data_section: String,
    pending_pointers: Vec<PendingPointer>,
    /// Names given in a `define` part before their `var` declaration (ref
    /// 7.2.3 Method 3, p. 7-6).
    pre_defined: HashSet<String>,
    /// Every unit-level `define` item, (name, span, named before its
    /// declaration), checked once the unit is complete.
    defines: Vec<(String, Span, bool)>,
    /// The section named by the `var (section_name)` part an `extern`
    /// variable is declared in (ref 3.14, p. 3-52: "All the variables
    /// named in all the identifier_lists will be stored in
    /// section_name"), by [`VarId`]: where a later `define` of it
    /// allocates it (ref 7.2.2, Method 2).
    extern_sections: HashMap<u32, String>,
    /// The members of `var (section_name)` parts, (section, variable,
    /// span), in declaration order, and the order they were allocated
    /// in (ref 3.14, p. 3-52; [`Checker::check_overlay_order`]).
    overlay_decls: Vec<(String, VarId, Span)>,
    overlay_allocs: Vec<(String, VarId)>,
    next_with: u32,
    /// The `with` records ([`WithId`]) that are, or are part of, an `in`
    /// parameter ([`Checker::in_param_part`]).
    in_withs: HashSet<u32>,
    loop_depth: u32,
    /// `%natural_alignment` in effect (ref 4.7 p. 4-57); push stack.
    natural: bool,
    align_stack: Vec<bool>,
    /// String-literal types, by length.
    string_types: HashMap<u32, TypeId>,
    /// The unit is a module (no main program).
    is_module: bool,
    /// Labels prefixing a statement: (declaring routine, name) and the
    /// first definition's position.
    label_defs: HashMap<(Option<RoutineId>, String), Span>,
    /// `goto` statements: the target's declaring routine and name, and
    /// the statement's position (checked against `label_defs` at the end).
    label_gotos: Vec<(Option<RoutineId>, String, Span)>,
    /// The type of an unsigned integer32 run-time value, the result of a
    /// shift of an unsigned operand ([`Checker::shift_result`]): an
    /// `integer32` subrange `0..2147483647`, made when first needed.
    unsigned32: Option<TypeId>,
    /// The type of `abs` of a 16-bit integer ([`Checker::abs_result`]):
    /// an `integer32` subrange `0..32767` stored in a word, made when
    /// first needed.
    abs16: Option<TypeId>,
    /// One-byte views of the types of packed fields that fill a byte
    /// ([`Checker::byte_field_type`]), by field type.
    byte_fields: HashMap<TypeId, TypeId>,
    /// The operand of the `addr` being checked ([`Checker::builtin_fn`]),
    /// and whether the next [`Checker::index`] is its last index: a
    /// constant index out of range there is only a best-effort warning.
    addr_operand: Option<Span>,
    lenient_index: bool,
}

fn upper(name: &str) -> String {
    name.to_ascii_uppercase()
}

impl<'t> Checker<'t> {
    pub(crate) fn new(target: &'t dyn Target) -> Self {
        let types = TypeTable::new(target.pointer_size());
        let mut c = Checker {
            target,
            types,
            vars: Vec::new(),
            routines: Vec::new(),
            scopes: vec![Scope::default()],
            errors: Vec::new(),
            warnings: Vec::new(),
            ctx: Vec::new(),
            sections: Vec::new(),
            code_section: ".text".into(),
            data_section: ".data".into(),
            pending_pointers: Vec::new(),
            pre_defined: HashSet::new(),
            defines: Vec::new(),
            extern_sections: HashMap::new(),
            overlay_decls: Vec::new(),
            overlay_allocs: Vec::new(),
            next_with: 0,
            in_withs: HashSet::new(),
            loop_depth: 0,
            natural: false,
            align_stack: Vec::new(),
            string_types: HashMap::new(),
            is_module: false,
            label_defs: HashMap::new(),
            label_gotos: Vec::new(),
            unsigned32: None,
            abs16: None,
            byte_fields: HashMap::new(),
            addr_operand: None,
            lenient_index: false,
        };
        c.predeclare();
        c
    }

    /// The predeclared identifiers used so far (ref Appendix A, Table A-2).
    fn predeclare(&mut self) {
        let b = self.types.builtins;
        let put = |c: &mut Self, n: &str, s: Sym| {
            c.scopes[0].names.insert(n.to_string(), s);
        };
        // ref 3.2.1 p. 3-4: integer = integer16.
        put(self, "integer", Sym::Type(b.integer16));
        put(self, "integer16", Sym::Type(b.integer16));
        put(self, "integer32", Sym::Type(b.integer32));
        put(self, "boolean", Sym::Type(b.boolean));
        put(self, "char", Sym::Type(b.char));
        // ref 3.3.1 p. 3-6: real = single.
        put(self, "real", Sym::Type(b.real));
        put(self, "single", Sym::Type(b.real));
        put(self, "double", Sym::Type(b.double));
        put(self, "univ_ptr", Sym::Type(b.univ_ptr));
        // ref 3.11, p. 3-38: "String is a predefined Domain Pascal array
        // type. ... string = array[1..80] of CHAR;".
        let index = self.new_type(TypeKind::Subrange {
            base: b.integer16,
            lo: 1,
            hi: 80,
        });
        let string = self.new_type(TypeKind::Array {
            index,
            elem: b.char,
            packed: false,
        });
        self.types.set_name_if_none(string, "string");
        put(self, "string", Sym::Type(string));
        put(
            self,
            "true",
            Sym::Const {
                value: ConstValue::Bool(true),
                ty: b.boolean,
            },
        );
        put(
            self,
            "false",
            Sym::Const {
                value: ConstValue::Bool(false),
                ty: b.boolean,
            },
        );
        put(
            self,
            "nil",
            Sym::Const {
                value: ConstValue::Nil,
                ty: b.nil,
            },
        );
        // ref 3.2.3 p. 3-5: "The predeclared integer constant maxint has
        // the value +32767."
        put(
            self,
            "maxint",
            Sym::Const {
                value: ConstValue::Int(32767),
                ty: b.integer16,
            },
        );
        for (n, bi) in [
            ("ord", Builtin::Ord),
            ("chr", Builtin::Chr),
            ("odd", Builtin::Odd),
            ("abs", Builtin::Abs),
            ("sqr", Builtin::Sqr),
            ("lshft", Builtin::Lshft),
            ("rshft", Builtin::Rshft),
            ("arshft", Builtin::Arshft),
            ("xor", Builtin::Xor),
            ("disable", Builtin::Disable),
            ("enable", Builtin::Enable),
            ("addr", Builtin::Addr),
            // Appendix E, Table E-1 (p. E-1); Table 4-7 (p. 4-10).
            ("set_sr", Builtin::SetSr),
            ("discard", Builtin::Discard),
            ("succ", Builtin::Succ),
            ("pred", Builtin::Pred),
            ("max", Builtin::Max),
            ("min", Builtin::Min),
            ("in_range", Builtin::InRange),
            ("sizeof", Builtin::Sizeof),
            ("firstof", Builtin::Firstof),
            ("lastof", Builtin::Lastof),
        ] {
            put(self, n, Sym::Builtin(bi));
        }
    }

    // -----------------------------------------------------------------
    // Diagnostics, scopes
    // -----------------------------------------------------------------

    fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(Diagnostic::error(span, msg));
    }

    fn unsupported(&mut self, span: Span, what: &str) {
        self.error(span, format!("{what} is not supported yet"));
    }

    fn lookup(&self, name: &str) -> Option<&Sym> {
        self.scopes.iter().rev().find_map(|s| s.names.get(name))
    }

    fn declare(&mut self, name: &ast::Ident, sym: Sym) {
        let scope = self.scopes.last_mut().unwrap();
        if scope.names.contains_key(&name.name) {
            self.errors.push(Diagnostic::error(
                name.span,
                format!("`{}` is already declared in this scope", name.name),
            ));
            return;
        }
        scope.names.insert(name.name.clone(), sym);
    }

    fn cur_level(&self) -> u32 {
        self.ctx.last().map_or(0, |c| c.level)
    }

    fn new_type(&mut self, kind: TypeKind) -> TypeId {
        self.types.add(kind, self.target.pointer_size())
    }

    // -----------------------------------------------------------------
    // Storage allocation
    // -----------------------------------------------------------------

    /// Allocate `ty` in the current routine's frame, below the frame
    /// pointer, at the type's freestanding alignment.  Returns the frame
    /// offset of the object's first byte.
    ///
    /// Objects are allocated in declaration order, downward (the order
    /// is not verified, codegen-patterns 2, "Frame layout").  Every frame
    /// object is at least word aligned: a byte variable takes the high
    /// byte of a word of its own, and two of them are never packed into
    /// one word (codegen-patterns 2, "Frame layout": 121 lone byte
    /// objects in the image's frames, all at even offsets, e.g.
    /// FILE_$REMOVE_WHEN_UNLOCKED 0xE5FC8E `pea (-2,A6)` in `link #-4`;
    /// 4-byte scalars keep their longword alignment, 659 lone longwords
    /// all at 0 mod 4; `tools/framesurvey.py align`).
    fn alloc_frame(&mut self, ty: TypeId) -> i32 {
        let (size, align) = {
            let i = self.types.get(ty);
            (i.size.max(1), i.align.max(FRAME_MIN_ALIGN))
        };
        let ctx = self
            .ctx
            .last_mut()
            .expect("frame allocation outside a routine");
        // Saturates; an oversized frame is reported by `routine_body`.
        ctx.frame = round_up(ctx.frame.saturating_add(size), align);
        -(ctx.frame as i32)
    }

    /// Allocate `ty` in static data section `section`, in declaration
    /// order ("Domain Pascal allocates variables defined in a var
    /// declaration part sequentially within the specified section", ref
    /// 3.14 p. 3-52), at the type's freestanding alignment but at least
    /// word aligned ([`STATIC_MIN_ALIGN`]): a byte variable takes the high
    /// byte of a word of its own, as in a frame.
    fn alloc_static(&mut self, section: &str, ty: TypeId) -> u32 {
        let (size, align) = {
            let i = self.types.get(ty);
            (i.size, i.align.max(STATIC_MIN_ALIGN))
        };
        let idx = match self.sections.iter().position(|(n, _)| n == section) {
            Some(i) => i,
            None => {
                self.sections.push((section.to_string(), 0));
                self.sections.len() - 1
            }
        };
        let off = round_up(self.sections[idx].1, align);
        self.sections[idx].1 = off.saturating_add(size);
        off
    }

    fn add_var(&mut self, var: Var) -> VarId {
        let id = VarId(self.vars.len() as u32);
        self.vars.push(var);
        id
    }

    // -----------------------------------------------------------------
    // Compilation unit
    // -----------------------------------------------------------------

    pub(crate) fn unit(mut self, unit: &ast::CompilationUnit) -> Result<Program, Vec<Diagnostic>> {
        self.scopes.push(Scope::default());
        if let Some(c) = &unit.code_section {
            self.code_section = c.name.clone();
        }
        if let Some(d) = &unit.data_section {
            self.data_section = d.name.clone();
        }
        for p in &unit.leading_pragmas {
            self.pragma(p);
        }
        self.is_module = matches!(unit.kind, ast::UnitKind::Module);
        let mut unit_vars = Vec::new();
        let top = self.decls(&unit.decls, None, &mut unit_vars);
        let main = unit.body.as_ref().map(|b| self.block(b));
        for p in &unit.trailing_pragmas {
            self.pragma(p);
        }
        // ref 9 Diagnostic Messages, p. 9-45: "ERROR Unmatched
        // %PUSH_ALIGNMENT directive.  This error indicates that you have
        // more %push_alignment directives than %pop_alignment directives."
        if !self.align_stack.is_empty() {
            self.errors.push(Diagnostic::error(
                unit.name.span,
                "Unmatched %push_alignment directive: more %push_alignment than \
                 %pop_alignment directives (ref 4.7 p. 4-58; 9, p. 9-45)",
            ));
        }
        // Every `goto` names a label that prefixes a statement of the
        // routine declaring it (ref "Goto", p. 4-95: "transfers control to
        // the statement immediately following lbl").
        for (owner, name, span) in &self.label_gotos {
            if !self.label_defs.contains_key(&(*owner, name.clone())) {
                self.errors.push(Diagnostic::error(
                    *span,
                    format!("label `{name}` prefixes no statement"),
                ));
            }
        }
        self.check_defines();
        self.check_overlay_order();
        for r in &self.routines {
            if r.options.forward && r.body.is_none() {
                self.errors.push(Diagnostic::error(
                    r.span,
                    format!("routine `{}` is declared forward but never defined", r.name),
                ));
            }
        }
        for (name, size) in &self.sections {
            if *size > MAX_OBJECT_SIZE {
                self.errors.push(Diagnostic::error(
                    unit.name.span,
                    format!("data section `{name}` is too large"),
                ));
            }
        }
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        let types = std::mem::take(&mut self.types);
        Ok(Program {
            name: unit.name.name.clone(),
            is_program: matches!(unit.kind, ast::UnitKind::Program { .. }),
            code_section: self.code_section,
            data_section: self.data_section,
            types,
            vars: self.vars,
            routines: self.routines,
            top_routines: top,
            unit_vars,
            data_sections: self.sections,
            main,
            warnings: self.warnings,
        })
    }

    /// The alignment environment at the start of the unit: natural with
    /// `-natural` (ref 6.4.22, p. 6-26), as if `%natural_alignment`
    /// came first.
    pub(crate) fn set_natural(&mut self, natural: bool) {
        self.natural = natural;
    }

    fn pragma(&mut self, p: &ast::PragmaMark) {
        match p.pragma {
            Pragma::NaturalAlignment => self.natural = true,
            Pragma::WordAlignment => self.natural = false,
            Pragma::PushAlignment => self.align_stack.push(self.natural),
            Pragma::PopAlignment => match self.align_stack.pop() {
                Some(n) => self.natural = n,
                None => self.error(p.span, "%pop_alignment without a matching %push_alignment"),
            },
            // Inlining is an optimisation hint; nothing to check.
            Pragma::BeginInline
            | Pragma::EndInline
            | Pragma::BeginNoinline
            | Pragma::EndNoinline => {}
        }
    }

    /// Process one declaration list.  Returns the routines declared in it
    /// (in order); variables declared are appended to `vars_out`.
    fn decls(
        &mut self,
        decls: &[Decl],
        owner: Option<RoutineId>,
        vars_out: &mut Vec<VarId>,
    ) -> Vec<RoutineId> {
        let mut routines = Vec::new();
        for (i, d) in decls.iter().enumerate() {
            match d {
                Decl::Label(l) => {
                    for lab in &l.labels {
                        let name = label_name(lab);
                        let id = ast::Ident {
                            name,
                            span: lab.span(),
                        };
                        self.declare_label(&id);
                    }
                }
                Decl::Const(c) => {
                    for def in &c.defs {
                        self.const_def(def);
                    }
                }
                Decl::Type(t) => {
                    for def in &t.defs {
                        let ty = self.type_expr(&def.ty);
                        self.types.set_name_if_none(ty, &def.name.name);
                        self.declare(&def.name, Sym::Type(ty));
                    }
                    // A type part ends unless the next item continues it
                    // (a directive split it, see ast::Decl).
                    let continues = matches!(
                        decls[i + 1..].iter().find(|d| !matches!(d, Decl::Pragma(_))),
                        Some(Decl::Type(next)) if next.continues_previous
                    );
                    if !continues {
                        self.resolve_pending_pointers();
                    }
                }
                Decl::Var(v) => {
                    for vd in &v.decls {
                        self.var_decl(v.section.as_ref(), vd, owner, vars_out);
                    }
                }
                Decl::Define(dp) => {
                    for item in &dp.items {
                        self.define_item(item);
                    }
                }
                Decl::Attribute(a) => {
                    for def in &a.defs {
                        self.attribute_def(def);
                    }
                }
                Decl::RoutineOption(ro) => {
                    for def in &ro.defs {
                        self.routine_option_def(def);
                    }
                }
                Decl::Routine(r) => {
                    // Body order: a routine whose body follows an earlier
                    // `forward`/`extern` heading (ref 5.5.2 p. 5-16; 7.3.4
                    // p. 7-11) moves to its body's position, as the
                    // original emitted it (codegen-patterns 6, "Order":
                    // forward `bsr`s, e.g. AUDIT_$SHUTDOWN 0xE70D2E).
                    if let Some(id) = self.routine(r, owner) {
                        if let Some(i) = routines.iter().position(|&x| x == id) {
                            if r.body.is_some() {
                                routines.remove(i);
                                routines.push(id);
                            }
                        } else {
                            routines.push(id);
                        }
                    }
                }
                Decl::Pragma(p) => self.pragma(p),
            }
        }
        self.resolve_pending_pointers();
        routines
    }

    fn declare_label(&mut self, id: &ast::Ident) {
        let owner = self.ctx.last().map(|c| c.id);
        if owner.is_none() && self.is_module {
            // ref 9, p. 9-37: "ERROR Labels not permitted at MODULE level."
            self.error(
                id.span,
                "labels are not permitted at module level (ref p. 9-37)",
            );
        }
        self.declare(id, Sym::Label(owner));
    }

    /// The routine declaring label `name`, when it is a label.
    fn label_owner(&self, name: &str) -> Option<Option<RoutineId>> {
        match self.lookup(name) {
            Some(Sym::Label(owner)) => Some(*owner),
            _ => None,
        }
    }

    fn const_def(&mut self, def: &ast::ConstDef) {
        let e = self.expr(&def.value);
        let pointer_transfer =
            matches!(e.kind, ExprKind::Transfer(_)) && self.types.is_pointer(e.ty);
        match e.value {
            Some(value) if !pointer_transfer => {
                self.declare(&def.name, Sym::Const { value, ty: e.ty })
            }
            _ => {
                if pointer_transfer {
                    // Ref 2.2.2.2, p. 2-12: "Value can also be the pointer
                    // expression nil"; it lists "A type transfer function"
                    // among the terms of a const value but describes no
                    // other pointer constant, so a type transfer to a
                    // pointer type (bead domain-pascal-gqp) is not taken
                    // as a const value (kept as before the ruling; not
                    // settled by the reference).
                    self.error(
                        def.value.span,
                        "a pointer constant other than nil (ref 2.2.2.2, p. 2-12: a const value \
                         may be the pointer expression nil)",
                    );
                } else if !self.types.is_error(e.ty) {
                    self.error(def.value.span, "constant expression expected");
                }
                let ty = self.types.builtins.error;
                self.declare(
                    &def.name,
                    Sym::Const {
                        value: ConstValue::Int(0),
                        ty,
                    },
                );
            }
        }
    }

    fn routine_option_def(&mut self, def: &ast::RoutineOptionDef) {
        let mut opts = RoutineOptions::default();
        for o in &def.options {
            // ref 5.6.1 p. 5-22: extern, forward, internal "cannot appear in
            // a routine_option declaration part".
            if matches!(o.name.as_str(), "extern" | "forward" | "internal") {
                self.error(
                    o.span,
                    format!(
                        "`{}` cannot appear in a routine_option declaration (ref 5.6.1)",
                        o.name
                    ),
                );
                continue;
            }
            self.apply_option(&mut opts, o);
        }
        self.declare(&def.name, Sym::Options(opts));
    }

    /// Add option `o` (predeclared or user-defined) to `opts`.
    fn apply_option(&mut self, opts: &mut RoutineOptions, o: &ast::Ident) -> bool {
        if opts.set(&o.name) {
            return true;
        }
        match self.lookup(&o.name) {
            Some(Sym::Options(user)) => {
                let u = *user;
                merge_options(opts, &u);
                true
            }
            _ => {
                self.error(o.span, format!("unknown routine option `{}`", o.name));
                false
            }
        }
    }

    // -----------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------

    fn resolve_pending_pointers(&mut self) {
        let pending = std::mem::take(&mut self.pending_pointers);
        for p in pending {
            match self.lookup(&p.name) {
                Some(Sym::Type(t)) => {
                    let t = *t;
                    self.types.set_kind(p.ptr, TypeKind::Pointer { target: t });
                }
                _ => {
                    self.error(p.span, format!("undefined type `{}`", p.name));
                    let e = self.types.builtins.error;
                    self.types.set_kind(p.ptr, TypeKind::Pointer { target: e });
                }
            }
        }
    }

    fn named_type(&mut self, id: &ast::Ident) -> TypeId {
        match self.lookup(&id.name) {
            Some(Sym::Type(t)) => *t,
            Some(_) => {
                self.error(id.span, format!("`{}` is not a type", id.name));
                self.types.builtins.error
            }
            None => {
                self.error(id.span, format!("undefined type `{}`", id.name));
                self.types.builtins.error
            }
        }
    }

    fn type_expr(&mut self, t: &ast::TypeExpr) -> TypeId {
        use ast::TypeKind as T;
        let err = self.types.builtins.error;
        match &t.kind {
            T::Named(id) => self.named_type(id),
            T::Subrange { lo, hi } => {
                let hi = match hi {
                    ast::Bound::Expr(e) => e,
                    ast::Bound::Star(s) => {
                        self.unsupported(*s, "an open array bound `*`");
                        return err;
                    }
                };
                self.subrange(lo, hi, t.span)
            }
            T::Enum(ids) => {
                let consts: Vec<String> = ids.iter().map(|i| i.name.clone()).collect();
                let ty = self.new_type(TypeKind::Enum { consts });
                for (ord, id) in ids.iter().enumerate() {
                    self.declare(
                        id,
                        Sym::Const {
                            value: ConstValue::Enum {
                                ty,
                                ord: ord as i64,
                            },
                            ty,
                        },
                    );
                }
                ty
            }
            T::Array {
                packed,
                indices,
                elem,
            } => {
                let mut elem_ty = self.type_expr(elem);
                if let Some(a) = self.types.attrs(elem_ty).align {
                    if self.types.size(elem_ty) % a != 0 {
                        // Elements are contiguous (ref 3.15.6.2 p. 3-66);
                        // whether an over-aligned element is padded is not
                        // stated.
                        self.unsupported(
                            elem.span,
                            "an array of elements aligned on more than their size (ref 3.15.6.2)",
                        );
                        return err;
                    }
                }
                let mut index_tys = Vec::new();
                for ix in indices {
                    let it = self.type_expr(ix);
                    if !self.types.is_ordinal(it) && !self.types.is_error(it) {
                        self.error(ix.span, "array index type must be an ordinal type");
                    }
                    index_tys.push(it);
                }
                for it in index_tys.into_iter().rev() {
                    if *packed {
                        // ref 3.11.4.2 Table 3-5 p. 3-47
                        // ([`TypeTable::packed_array_elem`]); each dimension
                        // of `packed array[i, j]` is a packed array.
                        match self.types.packed_array_elem(elem_ty) {
                            Ok(PackedElem::Unpacked | PackedElem::Bits(_)) => {}
                            Ok(PackedElem::Byte) => elem_ty = self.types.sized(elem_ty, 8),
                            Err(what) => {
                                self.unsupported(t.span, &what);
                                return err;
                            }
                        }
                    }
                    elem_ty = self.new_type(TypeKind::Array {
                        index: it,
                        elem: elem_ty,
                        packed: *packed,
                    });
                }
                self.check_size(elem_ty, t.span)
            }
            T::Record {
                form,
                fields,
                variant,
            } => {
                let natural = match form {
                    // ref 3.10.7: the packing rules do not depend on the
                    // alignment environment (Table 3-3 states no other).
                    ast::RecordForm::Packed => {
                        let r = self.record(fields, variant.as_deref(), true, false);
                        return self.check_size(r, t.span);
                    }
                    // ref 3.10.6 p. 3-33: an aligned record is laid out by
                    // the natural rules "regardless of the alignment
                    // environment".
                    ast::RecordForm::Aligned => true,
                    // ref 3.10.6 p. 3-34: unaligned records always use the
                    // word alignment rules, which are also the default.
                    ast::RecordForm::Unaligned => false,
                    // ref 4.7 p. 4-57: the environment's rules.
                    ast::RecordForm::Plain => self.natural,
                };
                let r = self.record(fields, variant.as_deref(), false, natural);
                self.check_size(r, t.span)
            }
            T::Set { elem, .. } => {
                // ref 3.9.1 NOTE p. 3-15: `packed` does not change a set.
                let e = self.type_expr(elem);
                if self.types.is_error(e) {
                    return err;
                }
                match self.types.ordinal_range(e) {
                    // ref 3.9.1 p. 3-15: an integer subrange base must not
                    // start below 0 nor end above 1023; 3.9.3 p. 3-16: at
                    // most 1024 elements, ordinals 0..1023.
                    Some((lo, hi)) if lo >= 0 && hi <= 1023 => {
                        self.new_type(TypeKind::Set { elem: e })
                    }
                    Some(_) => {
                        self.error(
                            elem.span,
                            "set base type must have ordinals in 0..1023 (ref 3.9.1, 3.9.3)",
                        );
                        err
                    }
                    None => {
                        self.error(elem.span, "set base type must be an ordinal type");
                        err
                    }
                }
            }
            T::File { .. } => {
                self.unsupported(t.span, "a file type");
                err
            }
            T::Varying { max, elem } => {
                let e = self.type_expr(elem);
                if !self.types.is_char(e) && !self.types.is_error(e) {
                    self.error(elem.span, "varying arrays must be of char (ref 3.11.2)");
                }
                let m = self.const_int(max).unwrap_or(1);
                if !(1..=65535).contains(&m) {
                    self.error(max.span, "varying length out of range");
                    return err;
                }
                self.new_type(TypeKind::Varying {
                    max: m as u32,
                    elem: e,
                })
            }
            T::Pointer(inner) => {
                if let T::Named(id) = &inner.kind {
                    match self.lookup(&id.name) {
                        Some(Sym::Type(target)) => {
                            let target = *target;
                            return self.new_type(TypeKind::Pointer { target });
                        }
                        Some(_) => {
                            self.error(id.span, format!("`{}` is not a type", id.name));
                            return err;
                        }
                        None => {
                            // ref 3.13.1: the target may be declared later in
                            // the same type part.
                            let ptr = self.new_type(TypeKind::Pointer { target: err });
                            self.pending_pointers.push(PendingPointer {
                                ptr,
                                name: id.name.clone(),
                                span: id.span,
                            });
                            return ptr;
                        }
                    }
                }
                let target = self.type_expr(inner);
                self.new_type(TypeKind::Pointer { target })
            }
            T::RoutinePointer(sig) => {
                let s = self.signature(sig);
                self.new_type(TypeKind::RoutinePointer(Box::new(s)))
            }
            T::Attributed { attrs, ty } => {
                let a = self.attribute_list(attrs);
                let base = self.type_expr(ty);
                self.apply_attrs(base, a, t.span)
            }
        }
    }

    /// Resolve the attributes of a `[...]` list (ref 3.15, 3.16).
    fn attribute_list(&mut self, attrs: &[ast::Attribute]) -> Attrs {
        let mut out = Attrs::default();
        for a in attrs {
            self.attribute(a, &mut out);
        }
        out
    }

    /// Add attribute `a` to `out`.  Implemented: `volatile`, `atomic`,
    /// `device` with `read`/`write` (ref 3.15.1-3.15.3), `aligned(n)` and
    /// `natural` (3.15.6), and names from an attribute declaration part
    /// (3.16).  `address` (3.15.4) and the size attributes (3.15.5) are
    /// not supported yet.
    fn attribute(&mut self, a: &ast::Attribute, out: &mut Attrs) {
        let name = a.name.name.as_str();
        let no_args = |c: &mut Self| {
            if a.args.is_some() {
                c.error(a.span, format!("`{name}` takes no arguments (ref 3.15)"));
            }
        };
        match name {
            "volatile" => {
                no_args(self);
                out.volatile = true;
            }
            "atomic" => {
                no_args(self);
                out.atomic = true;
            }
            // ref 3.15.3 p. 3-59: `device(read)`, `device(write)`,
            // `device(read, write)` = `device(write, read)` = `device`.
            "device" => {
                let mut d = DeviceAccess {
                    read: true,
                    write: true,
                };
                if let Some(args) = &a.args {
                    let mut read = false;
                    let mut write = false;
                    for x in args {
                        match &x.kind {
                            ast::ExprKind::Name(i) if i.name == "read" => read = true,
                            ast::ExprKind::Name(i) if i.name == "write" => write = true,
                            _ => self.error(
                                x.span,
                                "a device attribute takes `read` and/or `write` (ref 3.15.3, p. 3-59)",
                            ),
                        }
                    }
                    d = DeviceAccess { read, write };
                }
                out.device = Some(d);
            }
            // ref 3.15.6.1 p. 3-63: "[aligned [(n)]] ... where n is the
            // number of low order zeros in the address ... If you omit a
            // value for n, then the compiler will default to a value of 0
            // (byte aligned)"; aligned(n) is 2**n-byte alignment, n up to
            // 4 (octaword).
            "aligned" => {
                let n = match &a.args {
                    None => Some(0),
                    Some(args) if args.len() == 1 => self.const_int(&args[0]),
                    Some(_) => {
                        self.error(
                            a.span,
                            "`aligned` takes one argument (ref 3.15.6.1, p. 3-63)",
                        );
                        None
                    }
                };
                match n {
                    Some(n @ 0..=2) => out.aligned = Some(1 << n),
                    Some(3..=4) => self.unsupported(
                        a.span,
                        "quadword and octaword alignment (`aligned(3)`, `aligned(4)`, ref 3.15.6.1)",
                    ),
                    Some(_) => self.error(
                        a.span,
                        "`aligned(n)` takes n from 0 to 4 (ref 3.15.6.1, p. 3-63)",
                    ),
                    None => {}
                }
                out.natural = false;
            }
            "natural" => {
                no_args(self);
                out.natural = true;
                out.aligned = None;
            }
            "address" => self.unsupported(a.span, "the `address` attribute (ref 3.15.4)"),
            // ref 3.15.5 p. 3-60: "[attribute-name [(n)]] typename ... If
            // (n) is missing, then the compiler uses a default value of 1";
            // bit 1 bit, byte 8, word 16, long 32, quad 64.
            "bit" | "quad" => {
                let what = match name {
                    "bit" => "the `bit` size attribute (bit arrays, ref 3.15.5 p. 3-62)",
                    _ => "the `quad` size attribute (ref 3.15.5)",
                };
                self.unsupported(a.span, what);
            }
            "byte" | "word" | "long" => {
                let unit: i64 = match name {
                    "byte" => 8,
                    "word" => 16,
                    _ => 32,
                };
                let n = match &a.args {
                    None => Some(1),
                    Some(args) if args.len() == 1 => self.const_int(&args[0]),
                    Some(_) => {
                        self.error(
                            a.span,
                            format!("`{name}` takes one argument (ref 3.15.5, p. 3-60)"),
                        );
                        None
                    }
                };
                let Some(n) = n else { return };
                let bits = n
                    .checked_mul(unit)
                    .filter(|b| (1..=i64::from(u32::MAX)).contains(b));
                let Some(bits) = bits.filter(|_| n >= 1) else {
                    self.error(
                        a.span,
                        format!("Size of {} bits is invalid for specified type (ref 3.15.5, p. 3-60; message 300, p. 9-40)", n.saturating_mul(unit)),
                    );
                    return;
                };
                let bits = bits as u32;
                // ref p. 9-40, message 298: "Argument to token attribute
                // conflicts with value already specified for this type"
                // (`int = [word, long] -32768..32767`).
                if out.size.is_some_and(|s| s.bits != bits) {
                    self.error(
                        a.span,
                        format!(
                            "Argument to `{name}` attribute conflicts with value already \
                             specified for this type (ref 3.15.5; message 298, p. 9-40)"
                        ),
                    );
                    return;
                }
                out.size = Some(SizeAttr { bits, span: a.span });
            }
            _ => match self.lookup(name) {
                Some(Sym::Attributes(x)) => {
                    let x = *x;
                    out.volatile |= x.volatile;
                    out.atomic |= x.atomic;
                    if x.device.is_some() {
                        out.device = x.device;
                    }
                    if x.aligned.is_some() || x.natural {
                        out.aligned = x.aligned;
                        out.natural = x.natural;
                    }
                    if let Some(sz) = x.size {
                        if out.size.is_some_and(|s| s.bits != sz.bits) {
                            self.error(
                                a.span,
                                format!(
                                    "Argument to `{name}` attribute conflicts with value \
                                     already specified for this type (ref 3.15.5; message 298, \
                                     p. 9-40)"
                                ),
                            );
                        } else {
                            out.size = Some(SizeAttr {
                                bits: sz.bits,
                                span: a.span,
                            });
                        }
                    }
                }
                _ => self.error(
                    a.span,
                    format!("`{name}` is not an attribute (ref 3.15, 3.16)"),
                ),
            },
        }
    }

    /// `name = attr, ...;` in an attribute declaration part (ref 3.16, p.
    /// 3-78).  The name may not be a predeclared attribute (ref Appendix
    /// B: "Illegal to redefine ... The name of a user-defined attribute
    /// list may not conflict with a predeclared attribute or option").
    fn attribute_def(&mut self, def: &ast::AttributeDef) {
        if PREDECLARED_ATTRIBUTES.contains(&def.name.name.as_str()) {
            self.error(
                def.name.span,
                format!(
                    "Illegal to redefine `{}`: a predeclared attribute (ref 3.16)",
                    def.name.name
                ),
            );
            return;
        }
        let a = self.attribute_list(&def.attrs);
        self.declare(&def.name, Sym::Attributes(a));
    }

    /// The type `base` with attributes `a` (ref 3.15).
    fn apply_attrs(&mut self, base: TypeId, a: Attrs, span: Span) -> TypeId {
        if self.types.is_error(base) {
            return base;
        }
        let mut base = base;
        if let Some(sz) = a.size {
            match self.sized(base, sz) {
                Some(t) => base = t,
                None => return self.types.builtins.error,
            }
        }
        let a = Attrs { size: None, ..a };
        if a == Attrs::default() {
            return base;
        }
        // ref 3.15.2 p. 3-57: "Atomic attributes can only be used with
        // scalar variables.  The scalar data types are integer, Boolean,
        // character, and enumerated."
        if a.atomic && !self.types.is_ordinal(base) {
            self.error(
                span,
                "`atomic` can only be used with scalar variables: integer, boolean, char, \
                 enumerated (ref 3.15.2, p. 3-57)",
            );
            return self.types.builtins.error;
        }
        let align = if a.natural {
            // ref 3.15.6.1 p. 3-63: `[NATURAL] integer32` "at least
            // longword aligned".
            Some(self.types.get(base).natural_align.max(1))
        } else {
            a.aligned
        };
        if let Some(al) = align {
            match self.types.kind(base) {
                TypeKind::Set { .. } | TypeKind::Varying { .. } => {
                    self.unsupported(
                        span,
                        "an alignment attribute on a set or varying string type (alignment of                          these not stated, ref 3.15.6)",
                    );
                    return self.types.builtins.error;
                }
                TypeKind::Array { .. } if al > 2 => {
                    self.unsupported(
                        span,
                        "an array type aligned on more than a word (ref 3.15.6: its element                          layout is not stated)",
                    );
                    return self.types.builtins.error;
                }
                _ => {}
            }
        }
        self.types.attributed(
            base,
            TypeAttrs {
                volatile: a.volatile,
                atomic: a.atomic,
                device: a.device,
                align,
                size_bits: None,
            },
        )
    }

    /// `base` with size attribute `sz` (ref 3.15.5, pp. 3-60..3-62), or
    /// `None` after a diagnostic.  The rules (p. 3-61): "Every data type
    /// has a minimum size.  A size attribute must specify at least the
    /// number of bits required for the type to which it is applied.  A
    /// size attribute can specify a number of bits greater than the
    /// minimum for integer, boolean, and set data types."  The minimum is
    /// Table 3-7 (p. 3-61): integer16 16 bits, integer32 32, single/real
    /// 32, double 64, boolean 8, char 8, enumerated 16, a subrange the
    /// "Number of bits needed to store subrange".  Fewer bits, or more for
    /// another type, is message 300 (p. 9-40): "Size of token1 bits is
    /// invalid for specified type" (`bad : [byte] number`, number =
    /// integer32).
    ///
    /// Implemented: a size equal to the type's own storage (no change, as
    /// `[long] integer32`), and `[byte]` on an integer subrange that needs
    /// at most 8 bits (`[byte] 0..255`, `[byte] -128..127`: one byte,
    /// [`crate::types::TypeTable::sized`]; p. 3-62: "[byte] 0..255
    /// corresponds to the unsigned char data type in Domain/C"), and
    /// `[byte]` on a set of at most 8 elements (`[byte] set of 0..7`, a
    /// one-byte set, member k at bit k: the owner's ruling,
    /// reference-clarifications "A size attribute on a set type"; a
    /// set's minimum is the bits its range needs, ordinals 0..hi).  Not
    /// supported yet: a size larger than the type's storage (`[long]
    /// integer16`, `[long] boolean`, `[long] set of 0..7`, p. 3-60 and
    /// 7.5.1 p. 7-15, whose values the compiler then represents "using
    /// all of the bits"), a size between a byte and a set's storage
    /// (`[byte(3)] set of 0..17`), a size attribute on a type without a
    /// Table 3-7 minimum (pointers, structured types), on a subrange of an enumeration,
    /// boolean or char (an enumeration's minimum is 16 bits, a subrange's
    /// its bits: which applies is not stated), and a second, different
    /// size on a type that has one (p. 9-40, message 299: a warning, the
    /// size that results not stated).
    fn sized(&mut self, base: TypeId, sz: SizeAttr) -> Option<TypeId> {
        let span = sz.span;
        let bits = sz.bits;
        let info = self.types.get(base);
        if let Some(old) = info.attrs.size_bits {
            if old == bits {
                return Some(base);
            }
            self.unsupported(
                span,
                "a size attribute on a type that has another (message 299, p. 9-40: a warning; \
                 the size that results is not stated)",
            );
            return None;
        }
        let default = info.size;
        let (min, may_grow) = match self.types.kind(base).clone() {
            TypeKind::Integer16 => (16, true),
            TypeKind::Integer32 => (32, true),
            TypeKind::Real { size } => (8 * size, false),
            TypeKind::Boolean => (8, true),
            TypeKind::Char => (8, false),
            TypeKind::Enum { .. } => (16, false),
            TypeKind::Subrange { base: host, lo, hi }
                if matches!(
                    self.types.kind(host),
                    TypeKind::Integer16 | TypeKind::Integer32
                ) =>
            {
                (crate::types::TypeTable::subrange_bits(lo, hi), true)
            }
            TypeKind::Subrange { .. } => {
                self.unsupported(
                    span,
                    "a size attribute on a subrange of char, boolean or an enumeration (ref \
                     3.15.5 Table 3-7: its minimum is not settled)",
                );
                return None;
            }
            // Table 3-7 lists no minimum for sets; the owner's ruling
            // (reference-clarifications, "A size attribute on a set type",
            // 2026-10-16): the minimum is the bits the set's range needs,
            // one per ordinal 0..hi (3.9.3: the set is a bit string
            // indexed by ordinal), and `[byte] set of 0..7` is a 1-byte
            // set, element k at bit k.  "The compiler uses all of the
            // bits" of a larger size (p. 3-61), so a set of fewer than 8
            // elements may be `[byte]` too.  Sizes other than one byte or
            // the set's own storage are refused below.
            TypeKind::Set { elem } => {
                let hi = self
                    .types
                    .ordinal_range(elem)
                    .map_or(0, |(_, hi)| hi.max(0));
                ((hi + 1).min(u32::MAX as i64) as u32, true)
            }
            _ => {
                self.unsupported(
                    span,
                    "a size attribute on a type that Table 3-7 (ref 3.15.5, p. 3-61) gives no \
                     minimum size",
                );
                return None;
            }
        };
        if bits < min || (bits > min && !may_grow) {
            self.error(
                span,
                format!(
                    "Size of {bits} bits is invalid for specified type (ref 3.15.5 Table 3-7, \
                     p. 3-61; message 300, p. 9-40)"
                ),
            );
            return None;
        }
        let bytes = bits / 8;
        if bits % 8 != 0 || bytes > default {
            self.unsupported(
                span,
                "a size attribute larger than the type's own storage (ref 3.15.5: the compiler \
                 \"uses all of the bits in the larger object to represent the value\")",
            );
            return None;
        }
        if bytes < default && bytes != 1 {
            // Every integer subrange that needs 9..16 bits is a word
            // already (`TypeTable::layout_of`).
            self.unsupported(span, "this size attribute");
            return None;
        }
        Some(self.types.sized(base, bits))
    }

    /// Reject a type larger than [`MAX_OBJECT_SIZE`].
    fn check_size(&mut self, ty: TypeId, span: Span) -> TypeId {
        if self.types.size(ty) > MAX_OBJECT_SIZE {
            self.error(span, "type is too large");
            return self.types.builtins.error;
        }
        ty
    }

    fn subrange(&mut self, lo: &ast::Expr, hi: &ast::Expr, span: Span) -> TypeId {
        let err = self.types.builtins.error;
        let l = self.expr(lo);
        let h = self.expr(hi);
        let (Some(lv), Some(hv)) = (l.value.clone(), h.value.clone()) else {
            if !self.types.is_error(l.ty) && !self.types.is_error(h.ty) {
                self.error(span, "subrange bounds must be constants");
            }
            return err;
        };
        let (Some(lb), Some(hb)) = (self.types.ordinal_base(l.ty), self.types.ordinal_base(h.ty))
        else {
            self.error(span, "subrange bounds must be ordinal");
            return err;
        };
        let (lo_o, hi_o) = (lv.ordinal().unwrap_or(0), hv.ordinal().unwrap_or(0));
        let int = self.types.is_integer(lb) && self.types.is_integer(hb);
        if !int && lb != hb {
            self.error(span, "subrange bounds have different types");
            return err;
        }
        if lo_o > hi_o {
            self.error(span, "subrange lower bound exceeds upper bound");
            return err;
        }
        let base = if int {
            // ref 3.8 p. 3-14: `13..19` is a "Subrange of INTEGER" (16
            // bits); 3.4 p. 3-10: "When you have an unsigned subrange that is
            // large enough to require 31 bits, the compiler allocates 32
            // bits."  An unsigned subrange with an upper bound in
            // 32768..65535 (e.g. 0..65535) gets an integer32 host here,
            // because its values do not fit integer16, but is *stored* in
            // 16 bits (5.3.4.1 p. 5-13; see `TypeTable::layout_of`).
            if lo_o >= -32768 && hi_o <= 32767 {
                self.types.builtins.integer16
            } else {
                self.types.builtins.integer32
            }
        } else {
            lb
        };
        self.new_type(TypeKind::Subrange {
            base,
            lo: lo_o,
            hi: hi_o,
        })
    }

    /// Lay out a record in the default word-alignment environment (ref
    /// 3.10.5.5 p. 3-26), or a packed record (ref 3.10.7; offsets in bits,
    /// see [`TypeTable::layout_packed_fields`]).
    fn record(
        &mut self,
        fields: &[ast::FieldDecl],
        variant: Option<&ast::VariantPart>,
        packed: bool,
        natural: bool,
    ) -> TypeId {
        let mut seen = HashSet::new();
        let mut fixed = self.field_list(fields, &mut seen);
        let mode = Layout { packed, natural };
        let end = self.lay_out(&mut fixed, 0, mode);
        let variant = variant.map(|v| Box::new(self.variant_part(v, end, &mut seen, mode)));
        self.new_type(TypeKind::Record(RecordType {
            fields: fixed,
            variant,
            packed,
            natural,
        }))
    }

    /// Lay out `fields` from `start`: a byte offset, or for a packed
    /// record a bit offset.  Returns the end in the same unit.
    fn lay_out(&mut self, fields: &mut [Field], start: u32, mode: Layout) -> u32 {
        if !mode.packed {
            if mode.natural {
                // Under the natural rules, an array, set or varying string
                // field's alignment is not stated (ref 3.10.5.2 gives the
                // natural alignment of simple types and records only).
                if let Some(f) = fields.iter().find(|f| {
                    self.types.attrs(f.ty).align.is_none()
                        && matches!(
                            self.types.kind(f.ty),
                            TypeKind::Array { .. }
                                | TypeKind::Set { .. }
                                | TypeKind::Varying { .. }
                        )
                }) {
                    let span = f.span;
                    self.unsupported(
                        span,
                        "an array, set or varying field laid out by the natural alignment rules                          (its natural alignment is not stated, ref 3.10.5.2)",
                    );
                    for f in fields.iter_mut() {
                        f.ty = self.types.builtins.error;
                    }
                    return start;
                }
            }
            return self.types.layout_fields(fields, start, mode.natural);
        }
        // An alignment attribute on a packed record's field: accepted
        // where it agrees with the field's alignment by Table 3-3 (ref
        // 3.10.7 p. 3-36): word for a word-aligned field
        // (`cal_$timezone_rec_t`'s `drift: [aligned(1)] time_$clock_t`,
        // cal.ins.pas), byte for a byte-aligned one.  What it does
        // otherwise is not stated.
        for (i, f) in fields.iter().enumerate() {
            let Some(a) = self.types.attrs(f.ty).align else {
                continue;
            };
            let agrees = match self.types.packed_field_bits(f.ty) {
                Ok((_, crate::types::PackAlign::Word)) => a <= 2,
                Ok((_, crate::types::PackAlign::Byte)) => a == 1,
                _ => false,
            };
            if !agrees {
                let span = fields[i].span;
                self.unsupported(
                    span,
                    "an alignment attribute on a packed record's field that differs from its                      packed alignment (ref 3.10.7 Table 3-3)",
                );
                for f in fields.iter_mut() {
                    f.ty = self.types.builtins.error;
                }
                return start;
            }
        }
        match self.types.layout_packed_fields(fields, start) {
            Ok(end) => end,
            Err((i, what)) => {
                let span = fields[i].span;
                self.unsupported(span, &what);
                for f in fields.iter_mut() {
                    f.ty = self.types.builtins.error;
                }
                start
            }
        }
    }

    fn field_list(&mut self, fields: &[ast::FieldDecl], seen: &mut HashSet<String>) -> Vec<Field> {
        let mut out = Vec::new();
        for fd in fields {
            let ty = self.type_expr(&fd.ty);
            for n in &fd.names {
                if !seen.insert(n.name.clone()) {
                    self.error(n.span, format!("duplicate field `{}`", n.name));
                }
                out.push(Field {
                    name: n.name.clone(),
                    ty,
                    offset: 0,
                    span: n.span,
                    bits: None,
                });
            }
        }
        out
    }

    /// A variant part starting at offset `start` (ref 3.10.2 p. 3-19).
    /// Every arm overlays the others, starting where the tag ends.
    fn variant_part(
        &mut self,
        v: &ast::VariantPart,
        start: u32,
        seen: &mut HashSet<String>,
        mode: Layout,
    ) -> VariantPart {
        let tag_type = self.named_type(&v.tag_type);
        if !self.types.is_ordinal(tag_type) && !self.types.is_error(tag_type) {
            self.error(v.tag_type.span, "variant tag type must be ordinal");
        }
        let mut arm_start = start;
        let tag = v.tag.as_ref().map(|t| {
            if !seen.insert(t.name.clone()) {
                self.errors.push(Diagnostic::error(
                    t.span,
                    format!("duplicate field `{}`", t.name),
                ));
            }
            let mut f = vec![Field {
                name: t.name.clone(),
                ty: tag_type,
                offset: 0,
                span: t.span,
                bits: None,
            }];
            arm_start = self.lay_out(&mut f, start, mode);
            f.pop().unwrap()
        });
        let mut arms = Vec::new();
        for arm in &v.arms {
            let mut labels = Vec::new();
            for l in &arm.labels {
                let e = self.expr(l);
                match e.value.as_ref().and_then(|v| v.ordinal()) {
                    Some(o) => {
                        if !self.ordinal_compatible(e.ty, tag_type) {
                            self.error(l.span, "variant label does not match the tag type");
                        }
                        labels.push(o);
                    }
                    None => {
                        if !self.types.is_error(e.ty) {
                            self.error(l.span, "variant label must be an ordinal constant");
                        }
                    }
                }
            }
            let mut fields = self.field_list(&arm.fields, seen);
            let end = self.lay_out(&mut fields, arm_start, mode);
            let variant = arm
                .variant
                .as_deref()
                .map(|sub| Box::new(self.variant_part(sub, end, seen, mode)));
            arms.push(VariantArm {
                labels,
                fields,
                variant,
            });
        }
        VariantPart {
            tag,
            tag_type,
            arms,
        }
    }

    fn signature(&mut self, sig: &ast::RoutineSig) -> Signature {
        let options = self.signature_options(sig);
        let mut params = Vec::new();
        let mut arg_bytes = 0u32;
        for p in sig.params.iter().flatten() {
            match p {
                ast::Param::Data {
                    mode,
                    names,
                    univ,
                    ty,
                    ..
                } => {
                    let t = self.param_type(ty);
                    for _ in names {
                        // ref 5.2 Table 5-1, with the type's options, as for
                        // a routine heading.
                        let conv = conv::param_conv(
                            *mode,
                            *univ,
                            t,
                            options.small_by_value(),
                            options.c_param,
                            &self.types,
                            self.target,
                        );
                        arg_bytes = arg_bytes.saturating_add(conv.slot_size);
                        params.push(SigParam {
                            mode: *mode,
                            univ: *univ,
                            ty: t,
                            conv,
                        });
                    }
                }
                ast::Param::Routine { span, .. } => {
                    self.unsupported(*span, "a routine parameter (ref 5.4)");
                }
            }
        }
        let result = sig.result.as_ref().map(|r| self.type_expr(r));
        let arg_pad = round_up(arg_bytes, self.target.arg_area_align()) - arg_bytes;
        Signature {
            params,
            result,
            options,
            arg_pad,
        }
    }

    /// The options of a routine pointer type (ref p. 4-146, checked in the
    /// PDF: `pptr : ^Procedure(...); EXTERN;`).  The reference documents
    /// two rules for them, both in Appendix B: message 181, "INTERNAL
    /// option is illegal for PROCEDURE^ or FUNCTION^ types", and message
    /// 182, "Incompatible VAL_PARAM options ... The type declaration did
    /// not specify val_param, but the function heading did".  So
    /// `val_param` on the type decides the convention of calls through it
    /// (Table 5-1), which the image needs: its driver dispatches push
    /// small values by value (MAC_OS_$OPEN 0xE0B3E4 `move.w (0x30,A3),-(SP)`
    /// before `jsr (A1)`; KBD_$RCV 0xE1CE56 `move.b D0,-(SP)` before `jsr
    /// (A0)`; codegen-patterns 1.6).  The other options are refused: what
    /// `extern` (the p. 4-146 example) or `forward` mean on a type is not
    /// stated, and no call through a pointer shows the effect of
    /// `d0_return`, `a0_return`, `c_param`, `variable`, `abnormal`,
    /// `nosave` or `noreturn`.
    fn signature_options(&mut self, sig: &ast::RoutineSig) -> RoutineOptions {
        let mut opts = RoutineOptions::default();
        for o in &sig.options {
            let mut one = RoutineOptions::default();
            if !self.apply_option(&mut one, o) {
                continue;
            }
            if one.internal {
                self.error(
                    o.span,
                    "INTERNAL option is illegal for PROCEDURE^ or FUNCTION^ types (ref \
                     Appendix B, message 181)",
                );
                continue;
            }
            let only_val_param = RoutineOptions {
                val_param: true,
                ..RoutineOptions::default()
            };
            if one != only_val_param {
                // TODO(ref-question): what other options mean on a type
                // (docs/reference-clarifications.md, open question 19).
                self.unsupported(
                    o.span,
                    &format!(
                        "the routine option `{}` on a routine pointer type (only `val_param` \
                         is)",
                        o.name
                    ),
                );
                continue;
            }
            opts.val_param = true;
        }
        // TODO(ref-question) (open question 20): 5.6.4 (p. 5-22) applies
        // default_routine_options to "every subsequent routine in that
        // module that does not specify a routine option"; whether a
        // routine pointer type counts as a routine is not stated.  The
        // convention of every call through such a type would depend on
        // it, so it is refused rather than guessed.
        if sig.options.is_empty() {
            if let Some(Sym::Options(d)) = self.lookup("default_routine_options") {
                if *d != RoutineOptions::default() {
                    self.unsupported(
                        sig.span,
                        "a routine pointer type without options under default_routine_options \
                         (whether the defaults apply to it, ref 5.6.4 p. 5-22, is not stated)",
                    );
                }
            }
        }
        opts
    }

    /// A parameter's type must be a type name (ref 5.1 p. 5-2: "you cannot
    /// specify an anonymous data type"), optionally preceded by attribute
    /// lists, `[aligned(1)] status_$t` (owner ruling, docs/reference-
    /// clarifications.md, "5.1 Attribute lists on parameter types"; Apollo's
    /// rws.ins.pas and error.ins.pas).  `[attrs] T` is the type a named
    /// `= [attrs] T` declaration would give (ref 3.15, p. 3-53: the list
    /// goes "immediately before the type"), so every attribute 3.15 allows
    /// on a type is accepted.  For an argument passed by reference the
    /// attributes describe the argument's object, which the callee may
    /// assume no more aligned than that (ref 3.15.6.8, p. 3-76).
    fn param_type(&mut self, ty: &ast::TypeExpr) -> TypeId {
        match &ty.kind {
            ast::TypeKind::Named(id) => self.named_type(id),
            ast::TypeKind::Attributed { ty: inner, .. } if Self::attributed_name(inner) => {
                self.type_expr(ty)
            }
            _ => {
                self.error(
                    ty.span,
                    "a parameter's type must be a type name (ref 5.1, p. 5-2)",
                );
                self.types.builtins.error
            }
        }
    }

    /// Whether `ty` is a type name, possibly after attribute lists.
    fn attributed_name(ty: &ast::TypeExpr) -> bool {
        match &ty.kind {
            ast::TypeKind::Named(_) => true,
            ast::TypeKind::Attributed { ty, .. } => Self::attributed_name(ty),
            _ => false,
        }
    }

    // -----------------------------------------------------------------
    // Variables
    // -----------------------------------------------------------------

    fn var_decl(
        &mut self,
        section: Option<&ast::SectionName>,
        vd: &ast::VarDecl,
        owner: Option<RoutineId>,
        vars_out: &mut Vec<VarId>,
    ) {
        let ty = self.type_expr(&vd.ty);
        let kind = vd.alloc.map(|a| a.kind);
        let overlay = section.map(|s| match s {
            ast::SectionName::Ident(i) => i.name.clone(),
            ast::SectionName::Str(s, _) => s.clone(),
        });
        for name in &vd.names {
            let symbol = upper(&name.name);
            let data_section = match owner {
                Some(r) => self.routines[r.0 as usize].data_section.clone(),
                None => self.data_section.clone(),
            };
            let section = overlay.clone().unwrap_or(data_section);
            let mut kind = kind;
            if kind == Some(ast::AllocKind::Extern)
                && owner.is_none()
                && self.pre_defined.contains(&name.name)
            {
                // ref 7.2.3 p. 7-6, Method 3: `define x;` then
                // `x : extern T := v;` defines x here.
                kind = Some(ast::AllocKind::Define);
            }
            let storage = match (kind, owner) {
                // ref 7.2 p. 7-3: extern "tells Pascal to not allocate the
                // variable".
                (Some(ast::AllocKind::Extern), _) => Storage::Extern { symbol },
                // define: "allocate the variable in a static data area and
                // make its name externally accessible".
                (Some(ast::AllocKind::Define), _) => Storage::Static {
                    offset: self.alloc_static(&section, ty),
                    symbol,
                    global: true,
                    section,
                },
                // static: "allocate the variable in a static data area and
                // keep its name local".  A unit-level var without a clause
                // is in the unit's data area too (ref 5.7.1 p. 5-24); its
                // name is treated as local (TODO(ref-question): 7.2 does not
                // say whether it is exported).
                (Some(ast::AllocKind::Static), _) | (None, None) => Storage::Static {
                    offset: self.alloc_static(&section, ty),
                    symbol,
                    global: false,
                    section,
                },
                (None, Some(_)) => {
                    if overlay.is_some() {
                        Storage::Static {
                            offset: self.alloc_static(&section, ty),
                            symbol,
                            global: false,
                            section,
                        }
                    } else {
                        Storage::Frame {
                            offset: self.alloc_frame(ty),
                        }
                    }
                }
            };
            let frame_var = matches!(storage, Storage::Frame { .. });
            let id = self.add_var(Var {
                name: name.name.clone(),
                ty,
                storage,
                owner: if frame_var { owner } else { None },
                init: None,
                span: name.span,
            });
            if let (true, Some(r)) = (frame_var, owner) {
                self.routines[r.0 as usize].frame_vars.push(id);
            }
            if let (Storage::Extern { .. }, Some(sec)) =
                (&self.vars[id.0 as usize].storage, &overlay)
            {
                self.extern_sections.insert(id.0, sec.clone());
            }
            if let Some(sec) = &overlay {
                self.overlay_decls.push((sec.clone(), id, name.span));
                if matches!(self.vars[id.0 as usize].storage, Storage::Static { .. }) {
                    self.overlay_allocs.push((sec.clone(), id));
                }
            }
            vars_out.push(id);
            // A variable is in scope in its own initial value, so
            // `addr(itself)` is its own address; a variable declared
            // later in the var part is not (owner's ruling,
            // docs/reference-clarifications.md "A variable's own address
            // in its initializer"; ../source network/network.pas
            // network_$rqst_done_ec).  Its type and storage are settled
            // above.
            self.declare(name, Sym::Var(id));
            let init = match &vd.init {
                None => None,
                Some(init) => {
                    if frame_var {
                        // ref 3.2.2 p. 3-4: a routine's variable can be
                        // initialised only if it is static.
                        self.error(
                            init.span(),
                            "a routine's variable can be initialised only if it is static (ref 3.2.2, p. 3-4)",
                        );
                        None
                    } else {
                        let mut stores = Vec::new();
                        // A variable an initial value names (`addr(v)`,
                        // ref 3.13.4) is a relocation in the data, not an
                        // access by the routine's body: keep it out of
                        // the routine's `static_refs`.
                        let refs = self
                            .ctx
                            .last()
                            .map(|c| (c.id, self.routines[c.id.0 as usize].static_refs.len()));
                        self.init(ty, 0, init, &mut stores);
                        if let Some((r, n)) = refs {
                            self.routines[r.0 as usize].static_refs.truncate(n);
                        }
                        Some(stores)
                    }
                }
            };
            self.vars[id.0 as usize].init = init;
        }
    }

    /// Once the unit is complete: every overlay section's members must
    /// have been allocated in declaration order, those not allocated (an
    /// `extern` never `define`d here) after all the others.  ref 3.14
    /// (p. 3-52) allocates the variables of a `var (section_name)` part
    /// "sequentially within the specified section", and another unit
    /// shares the section by position (7.2.4, Method 4, p. 7-7); but an
    /// `extern` member takes its slot only when a `define` allocates it
    /// (`define_item`, ref 7.2.2), so an `extern` before other members, or
    /// `define`s out of declaration order, would place the members
    /// elsewhere than another unit's view of the same declarations does.
    /// Whether an `extern` member reserves its slot at its declaration is
    /// an open question (docs/reference-clarifications.md, question 48);
    /// until it is ruled, such a unit is refused (bead domain-pascal-m15q).
    /// A `var (name)` naming one of the unit's own data sections is the
    /// unit's block, which no other unit shares: not checked.
    fn check_overlay_order(&mut self) {
        let decls = std::mem::take(&mut self.overlay_decls);
        let allocs = std::mem::take(&mut self.overlay_allocs);
        let own = |c: &Self, n: &str| {
            n == c.data_section || c.routines.iter().any(|r| r.data_section == n)
        };
        let mut done: Vec<&str> = Vec::new();
        for (sec, _, _) in &decls {
            if done.contains(&sec.as_str()) || own(self, sec) {
                continue;
            }
            done.push(sec);
            let decl: Vec<(VarId, Span)> = decls
                .iter()
                .filter(|(s, _, _)| s == sec)
                .map(|&(_, v, sp)| (v, sp))
                .collect();
            let alloc: Vec<VarId> = allocs
                .iter()
                .filter(|(s, _)| s == sec)
                .map(|&(_, v)| v)
                .collect();
            // The first member out of place; none, or only past the
            // allocated ones (an unallocated `extern` tail), is fine.
            let first = (0..decl.len()).find(|&i| alloc.get(i) != Some(&decl[i].0));
            if let Some(i) = first.filter(|&i| i < alloc.len()) {
                let (v, span) = decl[i];
                let name = self.vars[v.0 as usize].name.clone();
                self.unsupported(
                    span,
                    &format!(
                        "overlay section `{sec}` member `{name}` out of its slot in declaration \
                         order (an `extern` member takes no slot until a `define` allocates it, \
                         so another unit's view of the section would disagree; ref 3.14 p. 3-52, \
                         docs/reference-clarifications.md question 48)"
                    ),
                );
            }
        }
    }

    /// Once the unit is complete, every name in a define part must have
    /// been declared as a variable or a routine, and a defined routine
    /// must have its body in this unit.
    fn check_defines(&mut self) {
        let mut seen = HashSet::new();
        for (name, span, before) in std::mem::take(&mut self.defines) {
            if !seen.insert(name.clone()) {
                continue;
            }
            match self.lookup(&name).cloned() {
                None => self.error(
                    span,
                    format!("`{name}` is named in a define statement but never declared"),
                ),
                Some(Sym::Var(_)) => {}
                // ref 7.3.4 p. 7-11: "define exponent; -- Use a define
                // statement to tell the compiler that exponent is a global
                // symbol defined in module math2"; a routine with only an
                // `extern` heading here is not defined here.
                Some(Sym::Routine(r)) => {
                    if self.routines[r.0 as usize].body.is_none() {
                        self.error(
                            span,
                            format!(
                                "routine `{name}` is named in a define statement but has no body in this unit (ref 7.3.4, p. 7-11)"
                            ),
                        );
                    }
                }
                // Defined after the define part: `define_item` reported it.
                Some(_) if !before => {}
                Some(_) => self.error(span, format!("`{name}` cannot be defined")),
            }
        }
    }

    /// `define x [:= v];` (ref 7.2.2, p. 7-5).
    fn define_item(&mut self, item: &ast::DefineItem) {
        let sym = self.lookup(&item.name.name).cloned();
        self.defines
            .push((item.name.name.clone(), item.name.span, sym.is_none()));
        let Some(sym) = sym else {
            // Method 3 (p. 7-6): the define comes first.
            self.pre_defined.insert(item.name.name.clone());
            if item.init.is_some() {
                self.error(
                    item.span,
                    "an initial value in `define` needs the variable declared first (ref 7.2.2)",
                );
            }
            return;
        };
        match sym {
            Sym::Var(v) => {
                let (ty, storage) = {
                    let var = &self.vars[v.0 as usize];
                    (var.ty, var.storage.clone())
                };
                // Allocated in the section of its `var (section_name)`
                // part, if it had one (ref 3.14, p. 3-52).
                let section = self
                    .extern_sections
                    .get(&v.0)
                    .cloned()
                    .unwrap_or_else(|| self.data_section.clone());
                let new = match storage {
                    Storage::Extern { symbol } => {
                        if self.extern_sections.contains_key(&v.0) {
                            self.overlay_allocs.push((section.clone(), v));
                        }
                        Storage::Static {
                            offset: self.alloc_static(&section, ty),
                            symbol,
                            global: true,
                            section,
                        }
                    }
                    Storage::Static {
                        symbol,
                        section,
                        offset,
                        ..
                    } => Storage::Static {
                        symbol,
                        global: true,
                        section,
                        offset,
                    },
                    _ => {
                        self.error(item.span, "only unit-level variables can be defined");
                        return;
                    }
                };
                self.vars[v.0 as usize].storage = new;
                if let Some(init) = &item.init {
                    let mut stores = Vec::new();
                    self.init(ty, 0, init, &mut stores);
                    self.vars[v.0 as usize].init = Some(stores);
                }
            }
            // ref 7.3.4 p. 7-11: a define part may also name routines,
            // but "you cannot assign an initial value to a procedure or
            // function symbol".
            Sym::Routine(_) => {
                if item.init.is_some() {
                    self.error(
                        item.span,
                        format!(
                            "`{}` is a routine: a define item for a routine cannot have an initial value (ref 7.3.4, p. 7-11)",
                            item.name.name
                        ),
                    );
                }
            }
            _ => self.error(
                item.name.span,
                format!("`{}` cannot be defined", item.name.name),
            ),
        }
    }

    /// The link-time address an initial value `x` denotes, if it is
    /// `addr(...)` of an object with a static address (owner's ruling on
    /// 3.13.4, p. 3-51; docs/reference-clarifications.md "3.13.4
    /// addr(...) as an initial value"): a routine (`addr(r)`, already
    /// limited to top-level routines that are not `internal`, ref 3.13.3
    /// p. 3-50), or a unit-level, `static`, `define`d or `extern`
    /// variable, or a field or element of one at a constant offset.
    /// `Err(Some(msg))`: an `addr` with no static address; `Err(None)`:
    /// not an `addr` at all.
    fn static_addr(&self, x: &Expr) -> Result<StaticAddr, Option<String>> {
        match &x.kind {
            ExprKind::RoutineAddr(r) => Ok(StaticAddr {
                base: StaticBase::Routine(*r),
                offset: 0,
            }),
            ExprKind::Builtin {
                builtin: Builtin::Addr,
                args,
            } if args.len() == 1 => self.static_designator(&args[0]).map_err(Some),
            _ => Err(None),
        }
    }

    /// The static address of designator `d` (see [`Checker::static_addr`]).
    fn static_designator(&self, d: &Expr) -> Result<StaticAddr, String> {
        match &d.kind {
            ExprKind::Var { var, .. } => match &self.vars[var.0 as usize].storage {
                Storage::Static { .. } | Storage::Extern { .. } => Ok(StaticAddr {
                    base: StaticBase::Var(*var),
                    offset: 0,
                }),
                Storage::Frame { .. } | Storage::Param { .. } | Storage::HiddenResult { .. } => {
                    Err(format!(
                        "`addr` of `{}` is not a valid initial value: a routine's variable or \
                         parameter has no static address (ref 3.13.4, p. 3-51)",
                        self.vars[var.0 as usize].name
                    ))
                }
            },
            // A packed record's field that is a whole byte, word or
            // longword has an address, `offset` (ref 3.10.7; as
            // `addr` of one in a statement); a bit field has none and
            // `addr` has already refused it.
            ExprKind::Field { base, offset, .. } if !self.is_bit_field(d) => {
                let mut a = self.static_designator(base)?;
                a.offset += i64::from(*offset);
                Ok(a)
            }
            ExprKind::Index {
                base,
                index,
                lo,
                elem_size,
                elem_bits,
            } => {
                if elem_bits.is_some() {
                    return Err(
                        "`addr` of an element of a packed array of bits (ref 3.11.4.2) is not a \
                         valid initial value"
                            .into(),
                    );
                }
                let mut a = self.static_designator(base)?;
                let Some(k) = index.value.as_ref().and_then(|v| v.ordinal()) else {
                    return Err(
                        "`addr` of an element with a variable index is not a valid initial value"
                            .into(),
                    );
                };
                a.offset += (k - lo) * i64::from(*elem_size);
                Ok(a)
            }
            _ => Err(
                "`addr` of an object with no static address is not a valid initial value \
                 (ref 3.13.4, p. 3-51)"
                    .into(),
            ),
        }
    }

    /// Flatten an initialiser for an object of type `ty` at byte `base`.
    fn init(&mut self, ty: TypeId, base: u32, init: &ast::Init, out: &mut Vec<InitStore>) {
        match init {
            ast::Init::Expr(e) => {
                let x = self.expr(e);
                // ref 3.11.1.1 p. 3-39: one character initialises element
                // 1 of a char array (`blank_line : STRING := chr(10)').
                let char_into_array = self.types.char_array_len(ty).is_some_and(|n| n > 0)
                    && self.types.is_char(x.ty);
                if !char_into_array && !self.check_assignable(ty, &x, e.span) {
                    return;
                }
                if self.types.is_error(x.ty) {
                    return;
                }
                let value = match x.value {
                    Some(value) => InitDatum::Const(value),
                    None => match self.static_addr(&x) {
                        Ok(a) => InitDatum::Addr(a),
                        Err(Some(msg)) => return self.error(e.span, msg),
                        Err(None) => return self.error(e.span, "initial value must be a constant"),
                    },
                };
                out.push(InitStore {
                    offset: base,
                    ty,
                    value,
                    bits: None,
                });
            }
            ast::Init::List { items, span } => match self.types.kind(ty).clone() {
                TypeKind::Set { elem } => {
                    let mut members = Vec::new();
                    for it in items {
                        let (lo, hi) = match it {
                            ast::InitItem::Value(ast::Init::Expr(e)) => (e, None),
                            ast::InitItem::Range { lo, hi, .. } => (lo, Some(hi)),
                            _ => {
                                self.error(*span, "malformed set initialiser");
                                continue;
                            }
                        };
                        let l = self.set_member(elem, lo);
                        let h = hi.map(|h| self.set_member(elem, h)).unwrap_or(l);
                        let Some((l, h)) = l.zip(h) else {
                            continue;
                        };
                        // A member outside the set's base type has no bit
                        // in the set (ref 3.9.3 p. 3-16); refused, not
                        // enumerated (bead domain-pascal-1dn: `[0..
                        // 2147483647]` for a `set of 0..31` ran for ever).
                        let (elo, ehi) = self.types.ordinal_range(elem).unwrap_or((0, 1023));
                        if l <= h && (l < elo || h > ehi) {
                            self.error(
                                lo.span,
                                format!("set element out of range {elo}..{ehi} of the set's type"),
                            );
                            continue;
                        }
                        members.extend(l..=h);
                    }
                    members.sort_unstable();
                    members.dedup();
                    out.push(InitStore {
                        offset: base,
                        ty,
                        value: InitDatum::Const(ConstValue::Set(members)),
                        bits: None,
                    });
                }
                TypeKind::Array { index, elem, .. } => {
                    let (lo, hi) = self.types.ordinal_range(index).unwrap_or((0, -1));
                    let esize = self.types.size(elem);
                    // A packed array of 1-, 2- or 4-bit elements (ref
                    // 3.11.4.2, 3.15.5 p. 3-62): element k is the bits
                    // (k - lo) * n .. of the array from the most
                    // significant bit of its first byte, initialised like a
                    // packed record's bit field.
                    let ebits = self.types.packed_elem_bits(ty);
                    let mut next = lo;
                    let at = |k: i64| base + ((k - lo) as u32) * esize;
                    let one =
                        |c: &mut Self, k: i64, v: &ast::Init, out: &mut Vec<InitStore>| match ebits
                        {
                            Some(n) => {
                                let bit = (k - lo) as u32 * n;
                                let pos = BitPos { bit, width: n };
                                c.init_bits(elem, base + bit / 8, pos, v, out)
                            }
                            None => c.init(elem, at(k), v, out),
                        };
                    for it in items {
                        match it {
                            ast::InitItem::Value(v) => {
                                if next > hi {
                                    self.error(*span, "too many initial values for the array");
                                    return;
                                }
                                one(self, next, v, out);
                                next += 1;
                            }
                            ast::InitItem::Repeat { count, value, .. } => {
                                let n = match count {
                                    Some(c) => self.const_int(c).unwrap_or(0),
                                    None => hi - next + 1,
                                };
                                for _ in 0..n {
                                    if next > hi {
                                        self.error(*span, "too many initial values for the array");
                                        return;
                                    }
                                    one(self, next, value, out);
                                    next += 1;
                                }
                            }
                            ast::InitItem::Keyed {
                                key, key_hi, value, ..
                            } => {
                                let k = self.const_index(index, key);
                                let kh = match key_hi {
                                    None => k,
                                    Some(ast::Bound::Expr(e)) => self.const_index(index, e),
                                    Some(ast::Bound::Star(_)) => Some(hi),
                                };
                                let (Some(k), Some(kh)) = (k, kh) else {
                                    continue;
                                };
                                for j in k..=kh {
                                    one(self, j, value, out);
                                }
                                next = kh + 1;
                            }
                            ast::InitItem::Range { lo: l, .. } => {
                                self.error(l.span, "a range is only valid in a set initialiser");
                            }
                        }
                    }
                }
                TypeKind::Record(rec) => {
                    // ref 3.10.4 pp. 3-21..3-22: `field := value' or a
                    // bare value for the field after the previous one;
                    // a variant's field by name ("the third field in the
                    // variant portion ... lb := 'a'").  A bare value
                    // after a variant's field (the "next" field across
                    // arms) is not covered by the text.
                    let mut next = Some(0usize);
                    for it in items {
                        let f = match it {
                            ast::InitItem::Value(v) => {
                                let Some(i) = next else {
                                    self.unsupported(
                                        *span,
                                        "a positional initial value after a variant's field",
                                    );
                                    return;
                                };
                                let Some(f) = rec.fields.get(i) else {
                                    if rec.variant.is_some() {
                                        self.unsupported(
                                            *span,
                                            "a positional initial value for a variant's field",
                                        );
                                    } else {
                                        self.error(*span, "too many initial values for the record");
                                    }
                                    return;
                                };
                                next = Some(i + 1);
                                Some((f.clone(), v))
                            }
                            ast::InitItem::Keyed {
                                key,
                                key_hi: None,
                                value,
                                ..
                            } => {
                                let ast::ExprKind::Name(id) = &key.kind else {
                                    self.error(key.span, "field name expected");
                                    continue;
                                };
                                if let Some(i) = rec.fields.iter().position(|f| f.name == id.name) {
                                    next = Some(i + 1);
                                    Some((rec.fields[i].clone(), value))
                                } else if let Some(f) = rec.field(&id.name) {
                                    next = None;
                                    Some((f.clone(), value))
                                } else {
                                    self.error(id.span, format!("no field `{}`", id.name));
                                    None
                                }
                            }
                            _ => {
                                self.error(*span, "malformed record initialiser");
                                None
                            }
                        };
                        let Some((f, v)) = f else { continue };
                        // A packed record's field that is whole bytes
                        // holding the whole object of its type
                        // (status_$t's `all', ref 3.10.7 Table 3-3) is
                        // initialised like an unpacked one; any other is
                        // a bit field.
                        match f.bits {
                            Some(bits) if bits.whole_bytes() != Some(self.types.size(f.ty)) => {
                                self.init_bits(f.ty, base + f.offset, bits, v, out)
                            }
                            _ => self.init(f.ty, base + f.offset, v, out),
                        }
                    }
                }
                _ => self.error(
                    *span,
                    "a bracketed initialiser needs a set, array or record",
                ),
            },
        }
    }

    /// Initialise bit field `bits` of a packed record whose first bit is
    /// in byte `byte` (ref 3.10.7, p. 3-35: "Pascal allocates fields left
    /// to right within bytes and then by increasing byte address"): the
    /// value's ordinal in the field's bits, most significant bit first
    /// ([`BitPos`]).  A boolean is one bit, set for true; an enumeration
    /// its ordinal; a subrange its value, two's complement for negative
    /// ones (p. 3-37: `-100..40` takes 8 bits "because it can take a
    /// range of 140 values").  Image: KBD's kbd_$state_table at WIRED_DATA
    /// 0xE2DDE4+0xC0, `FB 20 E8 31 ...`: a char, then two 4-bit
    /// subranges in one byte, the first in its high bits.  The two's
    /// complement of a negative value in the data is not seen in the
    /// image; stores of such values are masked to the field
    /// (0xE31DF2..0xE31E00, codegen-patterns 5.3).
    ///
    /// A set field that is not whole bytes is refused: where its
    /// elements lie within the field is not stated (Table 3-3, p. 3-36,
    /// gives only its size) and no read of one is catalogued (bead
    /// domain-pascal-bk8k).
    fn init_bits(
        &mut self,
        ty: TypeId,
        byte: u32,
        bits: BitPos,
        init: &ast::Init,
        out: &mut Vec<InitStore>,
    ) {
        let span = match init {
            ast::Init::Expr(e) => e.span,
            ast::Init::List { span, .. } => *span,
        };
        let set = matches!(self.types.kind(ty), TypeKind::Set { .. });
        if set && bits.whole_bytes() == Some(1) {
            // A set of 0..7 in one byte: the byte as a `[byte] set of
            // 0..7`, member k at bit k, as its reads and writes
            // ([`Self::byte_field_type`]; XPD_$INHERIT_PTRACE_OPTIONS
            // 0xE5B17C `btst #3,(13,A0)` for `3 in opts.flags2`).
            let t = self.byte_field_type(ty);
            return self.init(t, byte, init, out);
        }
        if set {
            // Best effort (not byte-identical; bead domain-pascal-eoc6):
            // member k at bit k of the field's value, as the code reads
            // and writes it (codegen-patterns 5.3).
            if let Err(d) = dpc_syntax::diag::best_effort(Diagnostic::error(
                span,
                "initialising a set field of a packed record that is not whole bytes (where its \
                 elements lie is not stated, ref 3.10.7 Table 3-3) is not supported yet",
            )) {
                self.errors.push(d);
                return;
            }
        }
        if !set && !matches!(init, ast::Init::Expr(_)) {
            return self.error(span, "a packed record's bit field needs a single value");
        }
        let mut one = Vec::new();
        self.init(ty, byte, init, &mut one);
        for mut st in one {
            if let InitDatum::Const(ConstValue::Set(members)) = &st.value {
                let mask = members
                    .iter()
                    .filter(|&&k| (0..i64::from(bits.width)).contains(&k))
                    .fold(0i64, |m, k| m | (1 << k));
                st.value = InitDatum::Const(ConstValue::Int(mask));
            }
            if !matches!(&st.value, InitDatum::Const(c) if c.ordinal().is_some()) {
                return self.unsupported(span, "this initial value of a packed bit field");
            }
            st.bits = Some(BitPos {
                bit: bits.bit % 8,
                width: bits.width,
            });
            out.push(st);
        }
    }

    fn set_member(&mut self, elem: TypeId, e: &ast::Expr) -> Option<i64> {
        let x = self.expr(e);
        if !self.ordinal_compatible(x.ty, elem) {
            self.error(e.span, "set element has the wrong type");
            return None;
        }
        let v = x.value.and_then(|v| v.ordinal());
        if v.is_none() {
            self.error(e.span, "set element must be a constant");
        }
        v
    }

    fn const_index(&mut self, index: TypeId, e: &ast::Expr) -> Option<i64> {
        let x = self.expr(e);
        if !self.ordinal_compatible(x.ty, index) {
            self.error(e.span, "index has the wrong type");
            return None;
        }
        match x.value.and_then(|v| v.ordinal()) {
            Some(k) => {
                let (lo, hi) = self.types.ordinal_range(index).unwrap_or((k, k));
                if k < lo || k > hi {
                    self.error(e.span, "index out of range");
                    return None;
                }
                Some(k)
            }
            None => {
                self.error(e.span, "index must be a constant");
                None
            }
        }
    }

    fn const_int(&mut self, e: &ast::Expr) -> Option<i64> {
        let x = self.expr(e);
        match x.value {
            Some(ConstValue::Int(v)) => Some(v),
            _ => {
                if !self.types.is_error(x.ty) {
                    self.error(e.span, "integer constant expected");
                }
                None
            }
        }
    }

    // -----------------------------------------------------------------
    // Routines
    // -----------------------------------------------------------------

    /// Process a routine declaration (heading, and body if any).
    fn routine(&mut self, r: &ast::Routine, parent: Option<RoutineId>) -> Option<RoutineId> {
        // For the driver's internal-error backstop (`diag::set_context`).
        let outer = dpc_syntax::diag::set_context(Some(r.span));
        let id = self.routine_in_context(r, parent);
        dpc_syntax::diag::set_context(outer);
        id
    }

    fn routine_in_context(
        &mut self,
        r: &ast::Routine,
        parent: Option<RoutineId>,
    ) -> Option<RoutineId> {
        // A body for a routine declared `forward` or `extern` earlier in
        // this scope (ref 5.5.2 p. 5-16; 7.3.4 p. 7-11).
        let earlier = self.scopes.last().unwrap().names.get(&r.name.name).cloned();
        let id = match earlier {
            Some(Sym::Routine(id))
                if r.body.is_some() && self.routines[id.0 as usize].body.is_none() =>
            {
                self.complete_heading(id, r);
                id
            }
            _ => {
                let id = self.heading(r, parent);
                self.declare(&r.name, Sym::Routine(id));
                id
            }
        };
        if let Some(body) = &r.body {
            self.routine_body(id, body);
        }
        Some(id)
    }

    /// Check a body's heading against the earlier declaration.
    fn complete_heading(&mut self, id: RoutineId, r: &ast::Routine) {
        let rt = &self.routines[id.0 as usize];
        if rt.kind != r.kind {
            self.error(
                r.name.span,
                "routine kind differs from its earlier declaration",
            );
        }
        if let Some(params) = &r.params {
            let n: usize = params
                .iter()
                .map(|p| match p {
                    ast::Param::Data { names, .. } => names.len(),
                    ast::Param::Routine { .. } => 1,
                })
                .sum();
            if n != self.routines[id.0 as usize].params.len() {
                self.error(
                    r.name.span,
                    "parameter list differs from the earlier declaration",
                );
            }
        }
        let rt = &mut self.routines[id.0 as usize];
        rt.external = false;
        rt.span = r.span;
        for a in &r.attrs {
            let ast::RoutineAttr::Section { code, data, .. } = a;
            if let Some(c) = code {
                rt.code_section = c.name.clone();
            }
            if let Some(d) = data {
                rt.data_section = d.name.clone();
            }
        }
    }

    fn heading(&mut self, r: &ast::Routine, parent: Option<RoutineId>) -> RoutineId {
        // Options (ref 5.5, 5.6).
        let mut opts = RoutineOptions::default();
        let mut interface_given = false;
        for o in &r.options {
            if self.apply_option(&mut opts, o)
                && !matches!(o.name.as_str(), "forward" | "extern" | "internal")
            {
                interface_given = true;
            }
        }
        // ref 5.6.4 p. 5-22: default_routine_options apply to "every
        // subsequent routine in that module that does not specify a routine
        // option".  TODO(ref-question): whether `forward`, `extern` or
        // `internal` alone count as "specifying a routine option" is not
        // stated; they are linkage, not interface, options (5.6.1 forbids
        // them in a routine_option part), so here they do not cancel the
        // defaults.
        if !interface_given {
            if let Some(Sym::Options(d)) = self.lookup("default_routine_options") {
                let d = *d;
                merge_options(&mut opts, &d);
            }
        }
        let level = parent.map_or(0, |p| self.routines[p.0 as usize].level + 1);
        let nested = parent.is_some();
        // Nested routines take small value parameters by value, like
        // `internal` routines: PMAP_$FLUSH calls its nested routine at
        // 0xE1359C with `move.w D6,-(SP); move.l D5,-(SP); pea (A2)`
        // (0xE13920..0xE1392A; codegen-patterns 6).  The reference does not
        // state this rule.
        let small_by_value = opts.small_by_value() || nested;
        let (code_section, data_section) = match parent {
            Some(p) => {
                let pr = &self.routines[p.0 as usize];
                (pr.code_section.clone(), pr.data_section.clone())
            }
            None => (self.code_section.clone(), self.data_section.clone()),
        };
        let (mut code_section, mut data_section) = (code_section, data_section);
        for a in &r.attrs {
            // ref 5.7.1 p. 5-24: "If you omit either the codesect or the
            // datasect, the present default continues to take effect."
            let ast::RoutineAttr::Section { code, data, .. } = a;
            if let Some(c) = code {
                code_section = c.name.clone();
            }
            if let Some(d) = data {
                data_section = d.name.clone();
            }
        }

        let id = RoutineId(self.routines.len() as u32);
        self.routines.push(Routine {
            name: r.name.name.clone(),
            symbol: upper(&r.name.name),
            kind: r.kind,
            // ref 5.5.4 p. 5-17: top-level routines are global symbols
            // unless internal.
            global: !nested && !opts.internal,
            external: opts.extern_,
            parent,
            level,
            options: opts,
            params: Vec::new(),
            arg_bytes: 0,
            arg_pad: 0,
            result: None,
            code_section,
            data_section,
            frame_vars: Vec::new(),
            frame_size: 0,
            nested: Vec::new(),
            static_refs: Vec::new(),
            body: None,
            span: r.span,
        });

        // Parameters and their frame offsets (codegen-patterns 1.1:
        // argument 1 at the first argument offset, pushed right to left).
        let mut params = Vec::new();
        let mut offset = self.target.first_arg_offset();
        for p in r.params.iter().flatten() {
            match p {
                ast::Param::Data {
                    mode,
                    names,
                    univ,
                    ty,
                    ..
                } => {
                    let t = self.param_type(ty);
                    for n in names {
                        let c = conv::param_conv(
                            *mode,
                            *univ,
                            t,
                            small_by_value,
                            opts.c_param,
                            &self.types,
                            self.target,
                        );
                        let value_offset = if c.passing == Passing::ByValue {
                            offset + self.target.small_arg_offset_in_slot(self.types.size(t)) as i32
                        } else {
                            offset
                        };
                        let var = self.add_var(Var {
                            name: n.name.clone(),
                            ty: t,
                            storage: Storage::Param {
                                index: params.len(),
                            },
                            owner: Some(id),
                            init: None,
                            span: n.span,
                        });
                        params.push(Param {
                            name: n.name.clone(),
                            var,
                            mode: *mode,
                            univ: *univ,
                            ty: t,
                            conv: c,
                            slot_offset: offset,
                            value_offset,
                            copy_offset: None,
                        });
                        offset = offset.saturating_add(c.slot_size.min(MAX_OBJECT_SIZE) as i32);
                    }
                }
                ast::Param::Routine { span, .. } => {
                    self.unsupported(*span, "a routine parameter (ref 5.4)");
                }
            }
        }
        if offset == i32::MAX {
            self.error(r.span, "the routine's arguments are too large");
        }
        let arg_bytes = (offset - self.target.first_arg_offset()) as u32;
        let arg_pad = round_up(arg_bytes, self.target.arg_area_align()) - arg_bytes;

        let result = r.result.as_ref().map(|rt| {
            let t = self.type_expr(rt);
            let reg = match conv::result_reg(t, &opts, &self.types, self.target) {
                Ok(reg) => reg,
                Err(msg) => {
                    self.error(rt.span, msg);
                    conv::ResultReg::Data
                }
            };
            (t, reg)
        });
        // A result through a hidden address (codegen-patterns 3, "Record
        // results over 4 bytes"): the address is the first argument, at the
        // first argument offset (ACL_$DEFAULT_ACL 0xE4788C reads its
        // parameter at (0xc,A6), 0xE478C8 the result's address at
        // (0x8,A6)), pushed last; the parameters follow it.  No pad,
        // whatever the total: PROC2_$UPGID_TO_UID 0xE41030..0xE4103C
        // `move.w (-0xa,A6),-(SP); pea (-0x14,A6); bsr.w; addq.w #6,SP`, 6
        // bytes (one site, 0xE41038: the only pop of 2 mod 4 bytes in
        // compiled code; every other such call has 4 or 8 bytes;
        // Uncertain as a rule).  TODO(target): ABI decisions, to move
        // into the target description (bead domain-pascal-wiue).
        let (arg_bytes, arg_pad) = if matches!(result, Some((_, conv::ResultReg::Hidden))) {
            let shift = self.target.pointer_size() as i32;
            for prm in &mut params {
                prm.slot_offset += shift;
                prm.value_offset += shift;
            }
            (arg_bytes + shift as u32, 0)
        } else {
            (arg_bytes, arg_pad)
        };
        let rt = &mut self.routines[id.0 as usize];
        rt.params = params;
        rt.arg_bytes = arg_bytes;
        rt.arg_pad = arg_pad;
        if let Some((t, reg)) = result {
            // The result variable is created when the body is checked;
            // until then `var` is a placeholder.
            rt.result = Some(FnResult {
                ty: t,
                reg,
                var: None,
            });
        }
        if let Some(p) = parent {
            // Heading order: the emission order of nested routines
            // (codegen-patterns 6, "Order").  A nested routine declared
            // `forward` is listed at its heading; top-level routines are
            // placed at their bodies (`decls`).  Whether nested ones follow
            // body order too is not seen in the image (`dpc-ir` refuses a
            // `forward` nested routine).
            self.routines[p.0 as usize].nested.push(id);
        }
        id
    }

    fn routine_body(&mut self, id: RoutineId, body: &ast::RoutineBody) {
        let level = self.routines[id.0 as usize].level;
        // A routine that is not a global symbol (a nested routine, or an
        // `internal` one, ref 5.5.4 p. 5-17) starts its frame with a
        // pointer-sized slot; locals follow below it (codegen-patterns 2,
        // "Frame size", and 6, "Frame"; `tools/nestsurvey.py slots`).
        // Nested: none of the image's 86 has `link #0`, none addresses
        // `(-4..-1,A6)` except three that store a static link there
        // (0xE4F3BA `move.l A1,(-4,A6)`, read by its own nested routines
        // as `(-4,A0)`; 0xE4EB40, 0xE4DFFE); 0xE1359C has `link #-4` and
        // no locals.  Unnamed, not nested: 353 of 355 have a frame and
        // leave `(-4..-1,A6)` alone (exceptions 0xE6B074 `link #0`,
        // 0xE70396), against 190 `link #0` and 320 that address it among
        // the 1194 map-named routines.
        let frame = if self.routines[id.0 as usize].global {
            0
        } else {
            self.target.pointer_size()
        };
        self.ctx.push(RoutineCtx { id, level, frame });
        self.scopes.push(Scope::default());

        // Frame order (codegen-patterns 2, "Frame size"): the slot above,
        // then local copies of value parameters (their place is
        // Uncertain), then locals, and the function result variable last,
        // below the locals (Uncertain, bead domain-pascal-67c: six sites
        // whose link size forces it below a local, e.g. PKT_$NEXT_ID
        // 0xE1248E, a word local at (-2,A6) in `link #-4', the result in
        // D2, and PROC2_$GET_PID 0xE40CBA; against it NETBUF_$RTNVA
        // 0xE0ED26, a token at (-6,A6) under a 4-byte result in `link
        // #-8', and SIO_$I_ERR 0xE67D9C, the result in the middle).  The
        // variable is created here, so that nested routines can name it,
        // and placed after the declarations.
        // A result through a hidden address is that address's object, at
        // the first argument offset, and takes no frame space
        // (ACL_$DEFAULT_ACL 0xE4787E `link #0`, its three assignments
        // `movea.l (0x8,A6),A2; move.l (A1)+,(A2); move.l (A1)+,(0x4,A2)`;
        // codegen-patterns 3, "Record results over 4 bytes").
        let result_var = if let Some(res) = self.routines[id.0 as usize].result.clone() {
            let name = self.routines[id.0 as usize].name.clone();
            let span = self.routines[id.0 as usize].span;
            let hidden = res.reg == conv::ResultReg::Hidden;
            let storage = if hidden {
                Storage::HiddenResult {
                    slot: self.target.first_arg_offset(),
                }
            } else {
                Storage::Frame { offset: 0 }
            };
            let var = self.add_var(Var {
                name,
                ty: res.ty,
                storage,
                owner: Some(id),
                init: None,
                span,
            });
            let rt = &mut self.routines[id.0 as usize];
            rt.result.as_mut().unwrap().var = Some(var);
            if hidden {
                None
            } else {
                rt.frame_vars.push(var);
                Some((var, res.ty))
            }
        } else {
            None
        };
        let nparams = self.routines[id.0 as usize].params.len();
        for i in 0..nparams {
            let (var, ty, copy) = {
                let p = &self.routines[id.0 as usize].params[i];
                (p.var, p.ty, p.conv.local_copy)
            };
            if copy {
                let off = self.alloc_frame(ty);
                self.routines[id.0 as usize].params[i].copy_offset = Some(off);
            }
            let (name, span) = {
                let v = &self.vars[var.0 as usize];
                (v.name.clone(), v.span)
            };
            self.declare(&ast::Ident { name, span }, Sym::Var(var));
        }

        let mut locals = Vec::new();
        // Nested routines are recorded in Routine::nested by heading().
        self.decls(&body.decls, Some(id), &mut locals);
        if let Some((var, ty)) = result_var {
            let offset = self.alloc_frame(ty);
            self.vars[var.0 as usize].storage = Storage::Frame { offset };
        }
        let block = self.block(&body.block);

        self.scopes.pop();
        let ctx = self.ctx.pop().unwrap();
        if ctx.frame > MAX_OBJECT_SIZE {
            let span = self.routines[id.0 as usize].span;
            self.error(span, "the routine's local variables are too large");
        }
        let rt = &mut self.routines[id.0 as usize];
        rt.frame_size = ctx.frame;
        rt.body = Some(block);
    }

    // -----------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------

    fn block(&mut self, b: &ast::Block) -> Block {
        Block {
            stmts: b.stmts.iter().map(|s| self.stmt(s)).collect(),
        }
    }

    fn stmt(&mut self, s: &ast::Stmt) -> Stmt {
        use ast::StmtKind as S;
        let kind = match &s.kind {
            S::Empty => StmtKind::Empty,
            S::Labeled { label, stmt } => {
                let name = label_name(label);
                let here = self.ctx.last().map(|c| c.id);
                match self.label_owner(&name) {
                    None => self.error(label.span(), format!("label `{name}` is not declared")),
                    // ref "Goto", p. 4-95: "(Name_of_Label) has not been
                    // declared in routine (name_of_routine)": a label is
                    // declared in the routine whose statement it prefixes.
                    Some(owner) if owner != here => self.error(
                        label.span(),
                        format!("label `{name}` is not declared in this routine (ref p. 4-95)"),
                    ),
                    Some(owner) => {
                        if self
                            .label_defs
                            .insert((owner, name.clone()), label.span())
                            .is_some()
                        {
                            self.error(
                                label.span(),
                                format!("label `{name}` prefixes more than one statement"),
                            );
                        }
                    }
                }
                StmtKind::Labeled {
                    label: name,
                    stmt: Box::new(self.stmt(stmt)),
                }
            }
            S::Assign { target, value } => self.assign(target, value),
            S::Call(e) => self.call_stmt(e),
            S::Compound(b) => StmtKind::Compound(self.block(b)),
            S::If {
                cond,
                then_branch,
                else_branch,
            } => StmtKind::If {
                cond: self.condition(cond),
                then_branch: Box::new(self.stmt(then_branch)),
                else_branch: else_branch.as_ref().map(|e| Box::new(self.stmt(e))),
            },
            S::Case {
                expr,
                arms,
                otherwise,
            } => self.case(expr, arms, otherwise.as_deref()),
            S::While { cond, body } => {
                let cond = self.condition(cond);
                self.loop_depth += 1;
                let body = Box::new(self.stmt(body));
                self.loop_depth -= 1;
                StmtKind::While { cond, body }
            }
            S::Repeat { body, cond } => {
                self.loop_depth += 1;
                let body = body.iter().map(|s| self.stmt(s)).collect();
                self.loop_depth -= 1;
                StmtKind::Repeat {
                    body,
                    cond: self.condition(cond),
                }
            }
            S::For {
                var,
                from,
                dir,
                to,
                body,
            } => {
                let v = self.name_expr(&ast::Expr {
                    kind: ast::ExprKind::Name(var.clone()),
                    span: var.span,
                });
                if !matches!(v.kind, ExprKind::Var { .. }) && !self.types.is_error(v.ty) {
                    self.error(var.span, "a for loop needs a variable");
                } else if !self.types.is_ordinal(v.ty) && !self.types.is_error(v.ty) {
                    self.error(var.span, "a for loop variable must be ordinal");
                }
                let f = self.expr(from);
                let t = self.expr(to);
                // Errors 75 and 77, p. 9-32.
                if !self.ordinal_compatible(f.ty, v.ty) {
                    self.error(
                        from.span,
                        "for statement initial value is not compatible with the index variable",
                    );
                }
                if !self.ordinal_compatible(t.ty, v.ty) {
                    self.error(
                        to.span,
                        "for statement limit value is not compatible with the index variable",
                    );
                }
                self.loop_depth += 1;
                let body = Box::new(self.stmt(body));
                self.loop_depth -= 1;
                StmtKind::For {
                    var: v,
                    from: f,
                    dir: *dir,
                    to: t,
                    body,
                }
            }
            S::With { items, body } => self.with(items, body),
            S::Goto(l) => {
                let name = label_name(l);
                match self.label_owner(&name) {
                    None => self.error(l.span(), format!("label `{name}` is not declared")),
                    Some(owner) => self.label_gotos.push((owner, name.clone(), l.span())),
                }
                StmtKind::Goto(name)
            }
            S::Exit | S::Next => {
                if self.loop_depth == 0 {
                    let w = if matches!(s.kind, S::Exit) {
                        "exit"
                    } else {
                        "next"
                    };
                    self.error(s.span, format!("`{w}` outside a loop (ref p. 4-77, 4-123)"));
                }
                if matches!(s.kind, S::Exit) {
                    StmtKind::Exit
                } else {
                    StmtKind::Next
                }
            }
            S::Return => StmtKind::Return,
        };
        Stmt { kind, span: s.span }
    }

    fn condition(&mut self, e: &ast::Expr) -> Expr {
        let x = self.expr(e);
        if !self.types.is_boolean(x.ty) && !self.types.is_error(x.ty) {
            self.error(e.span, "condition must be boolean");
        }
        x
    }

    fn assign(&mut self, target: &ast::Expr, value: &ast::Expr) -> StmtKind {
        let t = self.assign_target(target);
        let v = self.expr(value);
        if !self.types.is_error(t.ty) {
            // Error 79, p. 9-32.
            self.check_assignable(t.ty, &v, value.span);
        }
        StmtKind::Assign {
            target: t,
            value: v,
        }
    }

    fn assign_target(&mut self, target: &ast::Expr) -> Expr {
        let err = self.types.builtins.error;
        if let ast::ExprKind::Name(id) = &target.kind {
            if let Some(Sym::Routine(r)) = self.lookup(&id.name) {
                let r = *r;
                // `f := e` inside function f (or a routine nested in it).
                let inside = self.ctx.iter().any(|c| c.id == r);
                let rt = &self.routines[r.0 as usize];
                if inside && rt.kind == RoutineKind::Function {
                    let ty = rt.result.as_ref().map_or(err, |x| x.ty);
                    let uplevel = self.cur_level() - rt.level;
                    return Expr {
                        kind: ExprKind::Result {
                            routine: r,
                            uplevel,
                        },
                        ty,
                        value: None,
                        span: target.span,
                    };
                }
                self.error(target.span, format!("cannot assign to `{}` here", id.name));
                return self.error_expr(target.span);
            }
        }
        let t = self.expr(target);
        if self.types.is_error(t.ty) {
            return t;
        }
        if !t.is_designator() && t.transfers_variable() {
            // Ref p. 4-197 says nothing of a transfer as a variable (open
            // question 46); no ../source site assigns to one (bead
            // domain-pascal-w0rj).
            self.unsupported(
                target.span,
                "assigning to a type transfer of a variable or a field or element of one \
                 (whether it is a variable: open question 46, ref p. 4-197)",
            );
            return self.error_expr(target.span);
        }
        if !t.is_designator() {
            self.error(
                target.span,
                "the left side of an assignment must be a variable",
            );
            return self.error_expr(target.span);
        }
        if let ExprKind::Var { var, .. } = &t.kind {
            if let Storage::Param { index } = self.vars[var.0 as usize].storage {
                let owner = self.vars[var.0 as usize].owner.unwrap();
                if self.routines[owner.0 as usize].params[index].mode == ParamMode::In {
                    // ref 5.3.2 p. 5-7.
                    self.error(target.span, "Assignment to IN argument (ref 5.3.2)");
                }
            }
        } else if self.in_param_part(&t) {
            self.unsupported(
                target.span,
                "assigning to a field or element of an `in` parameter (ref 5.3.2 p. 5-7: the \
                 routine \"is not allowed to alter its value\")",
            );
        }
        t
    }

    /// Whether designator `e` is an `in` parameter or a field or element of
    /// one (not an object it points to).  ref 5.3.2 p. 5-7: the routine
    /// "is not allowed to alter its value".  Callers rely on that: the
    /// original holds a parameter in a register across a call that takes
    /// its address for an `in` or value parameter (codegen-patterns 12.2,
    /// MST_$TOUCH 0xE0DEFC), and so does `dpc` (bead domain-pascal-pe9);
    /// such a callee must not write the caller's object through any path.
    fn in_param_part(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Var { var, .. } => match self.vars[var.0 as usize].storage {
                Storage::Param { index } => {
                    let owner = self.vars[var.0 as usize].owner.unwrap();
                    self.routines[owner.0 as usize].params[index].mode == ParamMode::In
                }
                _ => false,
            },
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => self.in_param_part(base),
            ExprKind::WithRecord(w) => self.in_withs.contains(&w.0),
            _ => false,
        }
    }

    fn call_stmt(&mut self, e: &ast::Expr) -> StmtKind {
        let (callee, args): (&ast::Expr, &[ast::Expr]) = match &e.kind {
            ast::ExprKind::Call { callee, args } => (callee, args),
            _ => (e, &[]),
        };
        if let ast::ExprKind::Name(id) = &callee.kind {
            match self.lookup(&id.name).cloned() {
                // Appendix E (pp. E-3, E-4): procedures without arguments.
                // E.2 (p. E-2): they "generate privileged instructions and
                // may only be executed from supervisor mode"; that is a
                // run-time matter, not a compile-time restriction.
                Some(Sym::Builtin(b @ (Builtin::Disable | Builtin::Enable))) => {
                    if !args.is_empty() {
                        self.error(e.span, format!("`{}` takes no arguments", id.name));
                    }
                    return StmtKind::Builtin {
                        builtin: b,
                        args: Vec::new(),
                    };
                }
                // "Discard", p. 4-62: "discard (exp) {discard is a
                // procedure.}"; exp is "Any expression, including a function
                // call".
                Some(Sym::Builtin(Builtin::Discard)) => {
                    if args.len() != 1 {
                        self.error(e.span, "`discard` takes 1 argument");
                        return StmtKind::Empty;
                    }
                    let x = self.expr(&args[0]);
                    return StmtKind::Builtin {
                        builtin: Builtin::Discard,
                        args: vec![x],
                    };
                }
                Some(Sym::Routine(r)) => {
                    if self.routines[r.0 as usize].kind == RoutineKind::Function {
                        self.error(
                            callee.span,
                            format!("function `{}` called as a procedure", id.name),
                        );
                    }
                    let call = self.call_args(
                        Callee::Routine(r),
                        r_sig(&self.routines[r.0 as usize]),
                        args,
                        e.span,
                    );
                    return StmtKind::Call(call);
                }
                Some(Sym::Builtin(_)) => {
                    self.error(callee.span, format!("`{}` is a function", id.name));
                    return StmtKind::Empty;
                }
                None => {
                    self.error(callee.span, format!("undefined identifier `{}`", id.name));
                    return StmtKind::Empty;
                }
                _ => {}
            }
        }
        // A call through a procedure pointer: `p^(args)` or `p^`.
        if let ast::ExprKind::Deref(inner) = &callee.kind {
            let p = self.expr(inner);
            if let TypeKind::RoutinePointer(sig) = self.types.kind(p.ty).clone() {
                if self.call_result_deref(&p, callee.span) {
                    return StmtKind::Empty;
                }
                if sig.result.is_some() {
                    self.error(callee.span, "function pointer called as a procedure");
                }
                let params = self.sig_params(&sig);
                let call = self.call_args(Callee::Pointer(Box::new(p)), params, args, e.span);
                return StmtKind::Call(call);
            }
        }
        self.error(e.span, "procedure call expected");
        StmtKind::Empty
    }

    fn case(
        &mut self,
        expr: &ast::Expr,
        arms: &[ast::CaseArm],
        otherwise: Option<&[ast::Stmt]>,
    ) -> StmtKind {
        let x = self.expr(expr);
        if !self.types.is_ordinal(x.ty) && !self.types.is_error(x.ty) {
            self.error(expr.span, "case selector must be ordinal");
        }
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for arm in arms {
            let mut labels = Vec::new();
            for l in &arm.labels {
                let e = self.expr(l);
                if !self.ordinal_compatible(e.ty, x.ty) {
                    self.error(l.span, "case label does not match the selector type");
                }
                match e.value.as_ref().and_then(|v| v.ordinal()) {
                    Some(o) => {
                        if !seen.insert(o) {
                            self.error(l.span, "duplicate case label");
                        }
                        labels.push(o);
                    }
                    None => {
                        if !self.types.is_error(e.ty) {
                            self.error(l.span, "case label must be a constant")
                        }
                    }
                }
            }
            out.push(CaseArm {
                labels,
                stmt: self.stmt(&arm.stmt),
            });
        }
        StmtKind::Case {
            expr: x,
            arms: out,
            otherwise: otherwise.map(|ss| ss.iter().map(|s| self.stmt(s)).collect()),
        }
    }

    /// `with` (ref pp. 4-211..4-214): each item opens a scope with the
    /// record's fields, or binds the alias (p. 4-212).
    fn with(&mut self, items: &[ast::WithItem], body: &ast::Stmt) -> StmtKind {
        let mut bound = Vec::new();
        let mut pushed = 0;
        for it in items {
            let r = self.expr(&it.record);
            let wid = WithId(self.next_with);
            self.next_with += 1;
            if self.in_param_part(&r) {
                self.in_withs.insert(wid.0);
            }
            let rec = match self.types.kind(r.ty) {
                TypeKind::Record(rec) => Some(rec.clone()),
                TypeKind::Error => None,
                _ => {
                    self.error(it.record.span, "with needs a record");
                    None
                }
            };
            if !r.is_designator() && r.transfers_variable() {
                // Open question 46 ([`Self::assign_target`]).
                self.unsupported(
                    it.record.span,
                    "`with` over a type transfer of a variable or a field or element of one \
                     (whether it is a variable: open question 46, ref p. 4-197)",
                );
            } else if !r.is_designator() && !self.types.is_error(r.ty) {
                self.error(it.record.span, "with needs a record variable");
            }
            let mut scope = Scope::default();
            match (&it.alias, rec) {
                (Some(alias), _) => {
                    scope.names.insert(
                        alias.name.clone(),
                        Sym::WithAlias {
                            with: wid,
                            rec_ty: r.ty,
                        },
                    );
                }
                (None, Some(rec)) => {
                    for f in rec.all_fields() {
                        scope.names.insert(
                            f.name.clone(),
                            Sym::WithField {
                                with: wid,
                                rec_ty: r.ty,
                            },
                        );
                    }
                }
                (None, None) => {}
            }
            self.scopes.push(scope);
            pushed += 1;
            bound.push((wid, r));
        }
        let body = Box::new(self.stmt(body));
        for _ in 0..pushed {
            self.scopes.pop();
        }
        StmtKind::With { items: bound, body }
    }

    // -----------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------

    fn error_expr(&self, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Const,
            ty: self.types.builtins.error,
            value: None,
            span,
        }
    }

    fn konst(&mut self, value: ConstValue, ty: TypeId, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Const,
            ty,
            value: Some(value),
            span,
        }
    }

    /// The type of an integer constant: integer16 if it fits, else
    /// integer32 (ref 3.2.3 p. 3-5: constants are held as 32 bits; their
    /// range is -2147483648..+2147483647).
    fn int_const(&mut self, v: i64, span: Span) -> Expr {
        let b = self.types.builtins;
        let ty = if (-32768..=32767).contains(&v) {
            b.integer16
        } else if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
            b.integer32
        } else {
            self.error(span, "integer constant out of range (ref 3.2.3)");
            b.error
        };
        self.konst(ConstValue::Int(v), ty, span)
    }

    fn string_type(&mut self, len: u32) -> TypeId {
        if let Some(t) = self.string_types.get(&len) {
            return *t;
        }
        let t = self.new_type(TypeKind::StringLit { len });
        self.string_types.insert(len, t);
        t
    }

    pub(crate) fn expr(&mut self, e: &ast::Expr) -> Expr {
        use ast::ExprKind as E;
        let span = e.span;
        match &e.kind {
            E::Int(v) => {
                // ref 3.4 NOTE p. 3-10: a based constant with the sign bit
                // set (16#FFFFFFFF) is negative.
                let v = if *v > i32::MAX as u64 && *v <= u32::MAX as u64 {
                    *v as u32 as i32 as i64
                } else if *v > u32::MAX as u64 {
                    i64::MAX
                } else {
                    *v as i64
                };
                self.int_const(v, span)
            }
            E::Real(_) => {
                self.unsupported(span, "a real constant");
                self.error_expr(span)
            }
            E::Str(parts) => self.string(parts, span),
            E::Name(_) => self.name_expr(e),
            E::Set(elems) => self.set_cons(elems, span),
            E::Unary { op, operand } => self.unary(*op, operand, span),
            E::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs, span),
            E::Call { callee, args } => self.call_expr(callee, args, span),
            E::Index { base, indices } => {
                let at_addr = self.addr_operand == Some(span);
                if at_addr {
                    self.addr_operand = None;
                }
                let mut b = self.expr(base);
                for (k, ix) in indices.iter().enumerate() {
                    self.lenient_index = at_addr && k + 1 == indices.len();
                    b = self.index(b, ix, span);
                }
                b
            }
            E::Field { base, field } => {
                let b = self.expr(base);
                self.field(b, field, span)
            }
            E::Deref(inner) => {
                let p = self.expr(inner);
                // `xp^` of a function pointer with no argument list calls
                // it (ref "Invoking Procedure and Function Pointers",
                // p. 4-145: `flag := xp^`).
                if let TypeKind::RoutinePointer(sig) = self.types.kind(p.ty).clone() {
                    if self.call_result_deref(&p, span) {
                        return self.error_expr(span);
                    }
                    let Some(res) = sig.result else {
                        self.error(span, "procedure pointer used as a value");
                        return self.error_expr(span);
                    };
                    let params = self.sig_params(&sig);
                    let call = self.call_args(Callee::Pointer(Box::new(p)), params, &[], span);
                    return Expr {
                        kind: ExprKind::Call(call),
                        ty: res,
                        value: None,
                        span,
                    };
                }
                let ty = match self.types.kind(p.ty) {
                    TypeKind::Pointer { target } => *target,
                    TypeKind::Error => p.ty,
                    TypeKind::UnivPtr => {
                        self.error(span, "a univ_ptr cannot be dereferenced");
                        self.types.builtins.error
                    }
                    _ => {
                        self.error(span, "only a pointer can be dereferenced");
                        self.types.builtins.error
                    }
                };
                Expr {
                    kind: ExprKind::Deref(Box::new(p)),
                    ty,
                    value: None,
                    span,
                }
            }
            E::Paren(inner) => {
                let mut x = self.expr(inner);
                x.span = span;
                x
            }
        }
    }

    fn string(&mut self, parts: &[ast::StrPart], span: Span) -> Expr {
        let mut bytes = Vec::new();
        for p in parts {
            match p {
                ast::StrPart::Text(t) => {
                    for ch in t.chars() {
                        let c = ch as u32;
                        if c > 255 {
                            self.error(span, "character outside ISO Latin-1");
                        }
                        bytes.push(c as u8);
                    }
                }
                ast::StrPart::Code(e) => {
                    // ref 2.1.5.1 p. 2-5.
                    match self.const_int(e) {
                        Some(v) if (0..=255).contains(&v) => bytes.push(v as u8),
                        Some(_) => self.error(e.span, "character code out of range"),
                        None => {}
                    }
                }
            }
        }
        if bytes.len() == 1 {
            let c = self.types.builtins.char;
            return self.konst(ConstValue::Char(bytes[0]), c, span);
        }
        let ty = self.string_type(bytes.len() as u32);
        self.konst(ConstValue::Str(bytes), ty, span)
    }

    fn name_expr(&mut self, e: &ast::Expr) -> Expr {
        let span = e.span;
        let ast::ExprKind::Name(id) = &e.kind else {
            return self.expr(e);
        };
        let Some(sym) = self.lookup(&id.name).cloned() else {
            self.error(span, format!("undefined identifier `{}`", id.name));
            return self.error_expr(span);
        };
        match sym {
            Sym::Const { value, ty } => self.konst(value, ty, span),
            Sym::Var(v) => self.var_ref(v, span),
            Sym::WithField { with, rec_ty } => {
                let base = Expr {
                    kind: ExprKind::WithRecord(with),
                    ty: rec_ty,
                    value: None,
                    span,
                };
                self.field(base, id, span)
            }
            Sym::WithAlias { with, rec_ty } => Expr {
                kind: ExprKind::WithRecord(with),
                ty: rec_ty,
                value: None,
                span,
            },
            Sym::Routine(r) => {
                // A parameterless function call (standard Pascal).
                if self.routines[r.0 as usize].kind == RoutineKind::Procedure {
                    self.error(span, format!("procedure `{}` used as a value", id.name));
                    return self.error_expr(span);
                }
                self.call_expr_routine(r, &[], span)
            }
            Sym::Type(_) => {
                self.error(span, format!("type `{}` used as a value", id.name));
                self.error_expr(span)
            }
            Sym::Builtin(_) => {
                self.error(span, format!("`{}` needs arguments", id.name));
                self.error_expr(span)
            }
            Sym::Options(_) | Sym::Attributes(_) | Sym::Label(_) => {
                self.error(span, format!("`{}` is not a value", id.name));
                self.error_expr(span)
            }
        }
    }

    fn var_ref(&mut self, v: VarId, span: Span) -> Expr {
        let var = &self.vars[v.0 as usize];
        let uplevel = match var.owner {
            Some(o) => self.cur_level() - self.routines[o.0 as usize].level,
            None => 0,
        };
        if var.owner.is_none() {
            if let Some(c) = self.ctx.last() {
                let refs = &mut self.routines[c.id.0 as usize].static_refs;
                if !refs.iter().any(|&(x, _)| x == v) {
                    refs.push((v, span));
                }
            }
        }
        Expr {
            kind: ExprKind::Var { var: v, uplevel },
            ty: var.ty,
            value: None,
            span,
        }
    }

    /// A field of a packed record that is not a whole object of its type
    /// at an address the type can be accessed at (ref 3.10.7): its bits
    /// are part of a byte, word or longword and it has no address of its
    /// own.  See [`crate::types::BitPos::is_whole_object`].
    pub(crate) fn is_bit_field(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Field { bits: Some(b), .. } => {
                !self.types.is_error(e.ty) && !b.is_whole_object(self.types.size(e.ty))
            }
            _ => false,
        }
    }

    fn field(&mut self, base: Expr, field: &ast::Ident, span: Span) -> Expr {
        let rec = match self.types.kind(base.ty) {
            TypeKind::Record(r) => r,
            TypeKind::Error => return self.error_expr(span),
            _ => {
                self.error(span, "only a record has fields");
                return self.error_expr(span);
            }
        };
        match rec.field(&field.name) {
            Some(f) => {
                let (ty, offset, bits) = (f.ty, f.offset, f.bits);
                let ty = match bits {
                    Some(b) if b.whole_bytes() == Some(1) => self.byte_field_type(ty),
                    _ => ty,
                };
                Expr {
                    kind: ExprKind::Field {
                        base: Box::new(base),
                        name: field.name.clone(),
                        offset,
                        bits,
                    },
                    ty,
                    value: None,
                    span,
                }
            }
            None => {
                self.error(field.span, format!("no field `{}`", field.name));
                self.error_expr(span)
            }
        }
    }

    /// The type of a packed record field of type `ty` that takes the 8
    /// bits of one byte (ref 3.10.7 Table 3-3, p. 3-36): a set of 0..7
    /// (`set of 0..7` alone is a word, 3.9.3), an integer subrange of 8
    /// bits (`0..255`, `-128..127`, stored in a word unpacked, 3.8.1).
    /// It is the byte as an object of `ty` with a size attribute of a byte
    /// (`[byte] set of 0..7`, `[byte] 0..255`; [`TypeTable::sized`],
    /// compatible with `ty`), read and written as one, member k of a set
    /// at bit k.  Image: a set, XPD_$RESET_PTRACE_OPTS 0xE5B15E `clr.b
    /// (12,A0)`, XPD_$INHERIT_PTRACE_OPTIONS 0xE5B17C `btst #3,(13,A0)`;
    /// a subrange, `status_$t.modc` read as `clr.w D0; move.b (1,A0),D0`
    /// (0xE1AAB8), DBUF's `e_flags.block_type := 0` as `clr.b (13,A2)`
    /// (0xE3AC6A).  Any other type keeps its own (a char is a byte; an
    /// enumeration of 129..256 values is not seen).
    fn byte_field_type(&mut self, ty: TypeId) -> TypeId {
        if self.types.size(ty) <= 1
            || !(matches!(self.types.kind(ty), TypeKind::Set { .. }) || self.types.is_integer(ty))
        {
            return ty;
        }
        if let Some(&t) = self.byte_fields.get(&ty) {
            return t;
        }
        let t = self.types.sized(ty, 8);
        self.byte_fields.insert(ty, t);
        t
    }

    fn index(&mut self, base: Expr, ix: &ast::Expr, span: Span) -> Expr {
        let lenient = std::mem::take(&mut self.lenient_index);
        let i = self.expr(ix);
        let (index, elem) = match self.types.kind(base.ty) {
            TypeKind::Array { index, elem, .. } => (*index, *elem),
            TypeKind::Error => return self.error_expr(span),
            TypeKind::Varying { .. } => {
                self.unsupported(span, "indexing a varying string");
                return self.error_expr(span);
            }
            _ => {
                self.error(span, "only an array can be indexed");
                return self.error_expr(span);
            }
        };
        if !self.ordinal_compatible(i.ty, index) {
            self.error(ix.span, "index has the wrong type");
        }
        let lo = self.types.ordinal_range(index).map_or(0, |r| r.0);
        if let Some(k) = i.value.as_ref().and_then(|v| v.ordinal()) {
            if let Some((lo, hi)) = self.types.ordinal_range(index) {
                if k < lo || k > hi {
                    // Best effort: the last index of `addr`'s operand (a
                    // C-style end pointer, `addr(a[hi + 1])`,
                    // rem_file/server.pas): the address computed from the
                    // index as for any other; no element is accessed.
                    // Any access stays an error, as does `--strict`.
                    let warned = lenient
                        && dpc_syntax::diag::best_effort(Diagnostic::error(
                            ix.span,
                            "a constant index out of range in the operand of `addr` (an \
                             address past the array, computed from the index) is not \
                             supported yet",
                        ))
                        .is_ok();
                    if !warned {
                        self.error(ix.span, "index out of range");
                    }
                }
            }
        }
        let elem_size = self.types.size(elem);
        let elem_bits = self.types.packed_elem_bits(base.ty);
        Expr {
            kind: ExprKind::Index {
                base: Box::new(base),
                index: Box::new(i),
                lo,
                elem_size,
                elem_bits,
            },
            ty: elem,
            value: None,
            span,
        }
    }

    /// The type of a set constructor: a set of the elements' host type;
    /// integer elements give `set of 0..hi`, `hi` the largest constant
    /// member, or 1023 (the largest set, ref 3.9.3) if any element is not
    /// constant.  `[]` is a set of the error type, compatible with every
    /// set.
    fn set_cons(&mut self, elems: &[ast::SetElem], span: Span) -> Expr {
        let mut out = Vec::new();
        let mut host: Option<TypeId> = None;
        let mut all_const = true;
        let mut members = Vec::new();
        let mut max = 0i64;
        for el in elems {
            let lo = self.expr(&el.lo);
            let hi = el.hi.as_ref().map(|h| self.expr(h));
            for x in std::iter::once(&lo).chain(hi.as_ref()) {
                match self.types.ordinal_base(x.ty) {
                    Some(b) => {
                        let b = if self.types.is_integer(b) {
                            self.types.builtins.integer16
                        } else {
                            b
                        };
                        match host {
                            None => host = Some(b),
                            Some(h) if h != b => {
                                self.error(x.span, "set elements have different types")
                            }
                            _ => {}
                        }
                    }
                    None => {
                        if !self.types.is_error(x.ty) {
                            self.error(x.span, "set elements must be ordinal");
                        }
                    }
                }
            }
            let l = lo.value.as_ref().and_then(|v| v.ordinal());
            let h = match &hi {
                Some(h) => h.value.as_ref().and_then(|v| v.ordinal()),
                None => l,
            };
            match (l, h) {
                (Some(l), Some(h)) => {
                    if l < 0 || h > 1023 {
                        self.error(el.lo.span, "set element out of range 0..1023 (ref 3.9.3)");
                    } else {
                        members.extend(l..=h);
                        max = max.max(h);
                    }
                }
                _ => all_const = false,
            }
            out.push((lo, hi));
        }
        let elem = match host {
            None => self.types.builtins.error,
            Some(h) if self.types.is_integer(h) => {
                let hi = if all_const { max } else { 1023 };
                self.new_type(TypeKind::Subrange {
                    base: self.types.builtins.integer16,
                    lo: 0,
                    hi,
                })
            }
            Some(h) => h,
        };
        let ty = self.new_type(TypeKind::Set { elem });
        if all_const {
            members.sort_unstable();
            members.dedup();
            return self.konst(ConstValue::Set(members), ty, span);
        }
        Expr {
            kind: ExprKind::SetCons(out),
            ty,
            value: None,
            span,
        }
    }

    fn unary(&mut self, op: UnOp, operand: &ast::Expr, span: Span) -> Expr {
        // `-2147483648` (ref 3.4 p. 3-9: `signed_32 = -2147483648..
        // 2147483647`): the literal alone does not fit 32 bits.
        if let (UnOp::Neg, ast::ExprKind::Int(v)) = (op, &operand.kind) {
            if *v == 1 << 31 {
                return self.int_const(i32::MIN as i64, span);
            }
        }
        let x = self.expr(operand);
        if self.types.is_error(x.ty) {
            return self.error_expr(span);
        }
        let (ty, value) = match op {
            UnOp::Not => {
                if !self.types.is_boolean(x.ty) {
                    self.error(span, "`not` needs a boolean operand");
                    return self.error_expr(span);
                }
                let v = match &x.value {
                    Some(ConstValue::Bool(b)) => Some(ConstValue::Bool(!b)),
                    _ => None,
                };
                (self.types.builtins.boolean, v)
            }
            UnOp::BitNot | UnOp::Neg | UnOp::Plus => {
                if !self.types.is_integer(x.ty) {
                    self.error(span, "operator needs an integer operand");
                    return self.error_expr(span);
                }
                let base = self.types.ordinal_base(x.ty).unwrap();
                match &x.value {
                    Some(ConstValue::Int(v)) => {
                        let r = match op {
                            UnOp::BitNot => {
                                // ~ on a constant: complement within its
                                // 32-bit representation (ref 3.2.3).
                                !(*v as i32) as i64
                            }
                            UnOp::Neg => -v,
                            _ => *v,
                        };
                        return self.int_const(r, span);
                    }
                    _ => (base, None),
                }
            }
        };
        Expr {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(x),
            },
            ty,
            value,
            span,
        }
    }

    /// The type of `lshft(x, n)` and `rshft(x, n)` (owner's ruling,
    /// reference-clarifications "Lshft of a 16-bit operand; the sign of a
    /// shift of an unsigned word"; "Rshft", p. 4-171): an operand smaller
    /// than 32 bits is expanded to 32 bits by its sign (a `0..65535` or
    /// `[byte] 0..255` value with zeros) and shifted, the result an
    /// integer32, narrowed only where it is stored into a smaller
    /// target; the shift of an unsigned run-time value is unsigned (ref
    /// 4.3.3, pp. 4-6..4-7: compares, `div` and `mod` in their unsigned
    /// forms), the type an `integer32` subrange `0..2147483647` as for
    /// any unsigned longword (bead domain-pascal-4rfc).  `arshft` keeps
    /// its operand's width ([`Checker::int_result`]): p. 4-29's
    /// expansion gives the same value as the 16-bit shift widened.
    fn shift_result(&mut self, x: &Expr) -> TypeId {
        let bi = self.types.builtins;
        if x.value.is_some() || !self.types.unsigned_integer(x.ty) {
            return bi.integer32;
        }
        self.shift_result_unsigned()
    }

    /// The `integer32` subrange `0..2147483647` of [`Self::unsigned32`].
    fn shift_result_unsigned(&mut self) -> TypeId {
        let bi = self.types.builtins;
        match self.unsigned32 {
            Some(t) => t,
            None => {
                let t = self.new_type(TypeKind::Subrange {
                    base: bi.integer32,
                    lo: 0,
                    hi: 0x7fff_ffff,
                });
                self.unsigned32 = Some(t);
                t
            }
        }
    }

    /// The type of `abs(x)` of a run-time integer `x` ("Abs", p. 4-12:
    /// "an integer value"): a non-negative subrange, which the image
    /// treats as unsigned (codegen-patterns 8, "abs"; the "Max"/"Min"
    /// entries, pp. 4-111, 4-113: "The unsigned scalar data types are
    /// non-negative subranges of integers").  Of a 32-bit `x`,
    /// `0..2147483647` (p. 4-12: "number cannot be -2147483648"), compared
    /// unsigned with constants and unsigned longwords (TIME_$ADJUST_TIME_
    /// OF_DAY 0xE168FA `cmpi.l #8000,D0; bls`, LOCATE_SERVER 0xE4A44E
    /// `cmp.l (0x20,A5),D0; bls`).  Of a 16-bit `x`, `0..32767` stored in
    /// a word with an `integer32` host, unsigned as `0..65535` is (its
    /// sums in 32 bits, zero-extended: TPAD_$DATA 0xE69284 `clr.l D0;
    /// ...; move.w D5w,D0w; addi.l #0xa,D0`, 0xE6976A..0xE69778; compared
    /// unsigned with a constant, 0xE69674 `cmpi.w #0x5,D0w; bls`); its
    /// compare with a signed word is `dpc-ir`'s (`widen_operands`).
    fn abs_result(&mut self, x: TypeId) -> TypeId {
        let bi = self.types.builtins;
        if self.int_result(x, bi.integer16) == bi.integer32 {
            return self.shift_result_unsigned();
        }
        match self.abs16 {
            Some(t) => t,
            None => {
                let t = self.new_type(TypeKind::Subrange {
                    base: bi.integer32,
                    lo: 0,
                    hi: 32767,
                });
                self.abs16 = Some(t);
                t
            }
        }
    }

    /// The wider of two integer types (ref 4.3.1 p. 4-5: 16-bit operands
    /// are expanded to 32 bits when necessary).
    fn int_result(&self, a: TypeId, b: TypeId) -> TypeId {
        let bi = self.types.builtins;
        let wide = |t: TypeId| {
            matches!(
                self.types.ordinal_base(t).map(|x| self.types.kind(x)),
                Some(TypeKind::Integer32)
            )
        };
        if wide(a) || wide(b) {
            bi.integer32
        } else {
            bi.integer16
        }
    }

    fn binary(&mut self, op: BinOp, lhs: &ast::Expr, rhs: &ast::Expr, span: Span) -> Expr {
        let l = self.expr(lhs);
        let r = self.expr(rhs);
        let bi = self.types.builtins;
        if self.types.is_error(l.ty) || self.types.is_error(r.ty) {
            return self.error_expr(span);
        }
        let lset = matches!(self.types.kind(l.ty), TypeKind::Set { .. });
        let rset = matches!(self.types.kind(r.ty), TypeKind::Set { .. });
        let ints = self.types.is_integer(l.ty) && self.types.is_integer(r.ty);
        let bools = self.types.is_boolean(l.ty) && self.types.is_boolean(r.ty);
        let li = l.value.as_ref().and_then(|v| match v {
            ConstValue::Int(i) => Some(*i),
            _ => None,
        });
        let ri = r.value.as_ref().and_then(|v| match v {
            ConstValue::Int(i) => Some(*i),
            _ => None,
        });
        let (ty, value): (TypeId, Option<ConstValue>) = match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul if lset || rset => {
                if !(lset && rset && self.sets_compatible(l.ty, r.ty)) {
                    self.error(span, "incompatible set operands");
                    return self.error_expr(span);
                }
                let v = match (&l.value, &r.value) {
                    (Some(ConstValue::Set(a)), Some(ConstValue::Set(b))) => {
                        let s: Vec<i64> = match op {
                            BinOp::Add => {
                                let mut s: Vec<i64> = a.iter().chain(b).copied().collect();
                                s.sort_unstable();
                                s.dedup();
                                s
                            }
                            BinOp::Sub => a.iter().filter(|x| !b.contains(x)).copied().collect(),
                            _ => a.iter().filter(|x| b.contains(x)).copied().collect(),
                        };
                        Some(ConstValue::Set(s))
                    }
                    _ => None,
                };
                // The non-constructor operand's type, if any.
                let ty = if matches!(r.kind, ExprKind::Const | ExprKind::SetCons(_)) {
                    l.ty
                } else {
                    r.ty
                };
                (ty, v)
            }
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::BitAnd
            | BinOp::BitOr => {
                if !ints {
                    self.error(span, "operator needs integer operands");
                    return self.error_expr(span);
                }
                if let (Some(a), Some(b)) = (li, ri) {
                    let v = match op {
                        BinOp::Add => Some(a + b),
                        BinOp::Sub => Some(a - b),
                        BinOp::Mul => Some(a * b),
                        // Constant folding of div/mod with negative
                        // operands is left to run time until the M$ARITH
                        // sign rules are modelled (CLAUDE.md 6.5).
                        BinOp::Div if b > 0 && a >= 0 => Some(a / b),
                        BinOp::Mod if b > 0 && a >= 0 => Some(a % b),
                        BinOp::Div | BinOp::Mod if b == 0 => {
                            self.error(span, "division by zero");
                            None
                        }
                        BinOp::BitAnd => Some(((a as i32) & (b as i32)) as i64),
                        BinOp::BitOr => Some(((a as i32) | (b as i32)) as i64),
                        _ => None,
                    };
                    if let Some(v) = v {
                        return self.int_const(v, span);
                    }
                }
                (self.int_result(l.ty, r.ty), None)
            }
            BinOp::RealDiv | BinOp::Pow => {
                self.unsupported(span, "real arithmetic");
                return self.error_expr(span);
            }
            BinOp::And | BinOp::Or | BinOp::AndThen | BinOp::OrElse => {
                if !bools {
                    self.error(span, "operator needs boolean operands");
                    return self.error_expr(span);
                }
                let v = match (&l.value, &r.value) {
                    (Some(ConstValue::Bool(a)), Some(ConstValue::Bool(b))) => {
                        Some(ConstValue::Bool(match op {
                            BinOp::And | BinOp::AndThen => *a && *b,
                            _ => *a || *b,
                        }))
                    }
                    _ => None,
                };
                (bi.boolean, v)
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let ordered = !matches!(op, BinOp::Eq | BinOp::Ne);
                let ok = if self.types.is_ordinal(l.ty) && self.types.is_ordinal(r.ty) {
                    self.ordinal_compatible(l.ty, r.ty)
                } else if lset && rset {
                    // Sets: =, <>, <= (subset), >= (superset) (Table 4-1).
                    self.sets_compatible(l.ty, r.ty) && !matches!(op, BinOp::Lt | BinOp::Gt)
                } else if self.types.is_pointer(l.ty) && self.types.is_pointer(r.ty) {
                    // Table 4-1: < > etc. are for "Non-pointer types".
                    !ordered && self.pointers_compatible(l.ty, r.ty)
                } else if self.strings_compatible(l.ty, r.ty) {
                    true
                } else if self.is_structured(l.ty) && self.same_type(l.ty, r.ty) {
                    // Table 4-1 p. 4-2: `=` and `<>` apply to "All types",
                    // records and arrays among them.  Ordering them (the
                    // table's "Non-pointer types" row) has no meaning the
                    // reference gives: not supported.
                    if ordered {
                        self.unsupported(span, "ordering records or arrays (`<`, `>`, ...)");
                        return self.error_expr(span);
                    }
                    true
                } else {
                    false
                };
                if !ok {
                    self.error(span, "operands of the comparison are not compatible");
                    return self.error_expr(span);
                }
                let v = match (
                    l.value.as_ref().and_then(|v| v.ordinal()),
                    r.value.as_ref().and_then(|v| v.ordinal()),
                ) {
                    (Some(a), Some(b)) => Some(ConstValue::Bool(match op {
                        BinOp::Eq => a == b,
                        BinOp::Ne => a != b,
                        BinOp::Lt => a < b,
                        BinOp::Le => a <= b,
                        BinOp::Gt => a > b,
                        _ => a >= b,
                    })),
                    _ => None,
                };
                (bi.boolean, v)
            }
            BinOp::In => {
                let elem = match self.types.kind(r.ty) {
                    TypeKind::Set { elem } => *elem,
                    _ => {
                        self.error(rhs.span, "`in` needs a set on the right");
                        return self.error_expr(span);
                    }
                };
                if !self.types.is_ordinal(l.ty) {
                    self.error(lhs.span, "`in` needs an ordinal on the left");
                    return self.error_expr(span);
                }
                if !self.types.is_error(elem) && !self.ordinal_compatible(l.ty, elem) {
                    self.error(span, "element type does not match the set");
                    return self.error_expr(span);
                }
                let v = match (l.value.as_ref().and_then(|v| v.ordinal()), &r.value) {
                    (Some(k), Some(ConstValue::Set(m))) => Some(ConstValue::Bool(m.contains(&k))),
                    _ => None,
                };
                (bi.boolean, v)
            }
        };
        Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(r),
            },
            ty,
            value,
            span,
        }
    }

    fn call_expr(&mut self, callee: &ast::Expr, args: &[ast::Expr], span: Span) -> Expr {
        if let ast::ExprKind::Name(id) = &callee.kind {
            match self.lookup(&id.name).cloned() {
                Some(Sym::Routine(r)) => {
                    if self.routines[r.0 as usize].kind == RoutineKind::Procedure {
                        self.error(span, format!("procedure `{}` used as a value", id.name));
                        return self.error_expr(span);
                    }
                    return self.call_expr_routine(r, args, span);
                }
                Some(Sym::Type(t)) => return self.transfer(t, args, span),
                Some(Sym::Builtin(b)) => return self.builtin_fn(b, &id.name, args, span),
                None => {
                    self.error(callee.span, format!("undefined identifier `{}`", id.name));
                    return self.error_expr(span);
                }
                _ => {}
            }
        }
        if let ast::ExprKind::Deref(inner) = &callee.kind {
            let p = self.expr(inner);
            if let TypeKind::RoutinePointer(sig) = self.types.kind(p.ty).clone() {
                if self.call_result_deref(&p, span) {
                    return self.error_expr(span);
                }
                let Some(res) = sig.result else {
                    self.error(span, "procedure pointer used as a value");
                    return self.error_expr(span);
                };
                let params = self.sig_params(&sig);
                let call = self.call_args(Callee::Pointer(Box::new(p)), params, args, span);
                return Expr {
                    kind: ExprKind::Call(call),
                    ty: res,
                    value: None,
                    span,
                };
            }
        }
        self.error(span, "function call expected");
        self.error_expr(span)
    }

    /// `addr(r)` (ref p. 4-145): "Although the addr function has been
    /// declared to return a univ_ptr, the compiler adds extra type checking
    /// whenever you try to pass a procedure or function address to a
    /// specific procedure or function pointer" ([`Checker::check_assignable`]).
    /// 3.13.3 (p. 3-50): "You may obtain the addresses only of top-level
    /// procedures and functions; you cannot obtain the addresses of nested
    /// or explicitly declared internal procedures and functions."
    fn routine_addr(&mut self, r: RoutineId, span: Span) -> Expr {
        let rt = &self.routines[r.0 as usize];
        if rt.parent.is_some() {
            self.error(
                span,
                "the address of a nested routine cannot be taken (ref 3.13.3, p. 3-50)",
            );
            return self.error_expr(span);
        }
        if rt.options.internal {
            self.error(
                span,
                "the address of an internal routine cannot be taken (ref 3.13.3, p. 3-50)",
            );
            return self.error_expr(span);
        }
        Expr {
            kind: ExprKind::RoutineAddr(r),
            ty: self.types.builtins.univ_ptr,
            value: None,
            span,
        }
    }

    /// `rf^^` or `getfp^(...)`: a routine pointer returned by a function
    /// call, dereferenced to call it.  Ref "Invoking Procedure and Function
    /// Pointers", p. 4-145: "You cannot use the following assignment
    /// `flag := rf^^;` because you cannot de-reference the return value of
    /// a function call."  Reports the error and returns true.
    fn call_result_deref(&mut self, p: &Expr, span: Span) -> bool {
        if !matches!(p.kind, ExprKind::Call(_)) {
            return false;
        }
        self.error(
            span,
            "the return value of a function call cannot be dereferenced (ref p. 4-145)",
        );
        true
    }

    /// Whether routine `r` matches routine pointer signature `sig` exactly
    /// (ref p. 4-146: "the declaration for proc2 must exactly match the
    /// template for the procedure type of pptr"; 5.3.4.1 p. 5-13: the
    /// parameter types must be name compatible, `pinteger` is not
    /// `integer`; Appendix B messages 180 and 182: the parameter-passing
    /// conventions and `val_param` must agree).
    fn routine_matches(&self, r: RoutineId, sig: &Signature) -> bool {
        let rt = &self.routines[r.0 as usize];
        rt.params.len() == sig.params.len()
            && rt.params.iter().zip(&sig.params).all(|(a, b)| {
                a.mode == b.mode
                    && a.univ == b.univ
                    && self.types.canon(a.ty) == self.types.canon(b.ty)
                    // `[aligned(n)] T` is `T` for compatibility, but an
                    // alignment attribute changes a record's size (ref
                    // 3.15.6.2, Figure 3-21 p. 3-67): a parameter of
                    // another size does not "exactly match" (p. 4-146).
                    && self.types.size(a.ty) == self.types.size(b.ty)
                    && a.conv == b.conv
            })
            && rt.result.as_ref().map(|x| x.ty) == sig.result
            && rt.options.small_by_value() == sig.options.small_by_value()
    }

    fn call_expr_routine(&mut self, r: RoutineId, args: &[ast::Expr], span: Span) -> Expr {
        let rt = &self.routines[r.0 as usize];
        let ty = rt
            .result
            .as_ref()
            .map_or(self.types.builtins.error, |x| x.ty);
        let params = r_sig(rt);
        let call = self.call_args(Callee::Routine(r), params, args, span);
        Expr {
            kind: ExprKind::Call(call),
            ty,
            value: None,
            span,
        }
    }

    /// Parameters of a routine-pointer signature, with their conventions
    /// (decided from the type's options, [`Checker::signature`]).
    fn sig_params(&self, sig: &Signature) -> Vec<(ParamMode, bool, TypeId, conv::ParamConv)> {
        sig.params
            .iter()
            .map(|p| (p.mode, p.univ, p.ty, p.conv))
            .collect()
    }

    fn call_args(
        &mut self,
        callee: Callee,
        params: Vec<(ParamMode, bool, TypeId, conv::ParamConv)>,
        args: &[ast::Expr],
        span: Span,
    ) -> Call {
        let variable = match &callee {
            Callee::Routine(r) => self.routines[r.0 as usize].options.variable,
            Callee::Pointer(_) => false,
        };
        // ref 5.5.5 p. 5-17: `variable` routines may get fewer arguments.
        if args.len() > params.len() || (args.len() < params.len() && !variable) {
            self.error(
                span,
                format!(
                    "wrong number of arguments: {} given, {} expected",
                    args.len(),
                    params.len()
                ),
            );
        }
        let mut out = Vec::new();
        for (i, (a, (mode, univ, ty, c))) in args.iter().zip(params).enumerate() {
            let x = self.expr(a);
            if univ {
                self.univ_conversion_warning(&callee, i, &x, ty, a.span);
            }
            if matches!(mode, ParamMode::Var | ParamMode::Out | ParamMode::InOut)
                && self.in_param_part(&x)
            {
                // ref 5.3.2 p. 5-7; see [`Checker::in_param_part`].
                self.unsupported(
                    a.span,
                    "passing an `in` parameter (or a part of one) to a `var`, `out` or `in out` \
                     parameter",
                );
            }
            if self.is_bit_field(&x) {
                // ref 3.10.3 p. 3-21: "you cannot pass a field from a packed
                // record as an argument to a procedure (including
                // predeclared procedures like read)".  A field that is a
                // whole object at a byte (or, longer, word) address has an
                // address and is passed as usual: the reference itself
                // passes `err_stat.all` of the packed status_$t (9.1.1).
                if matches!(mode, ParamMode::Var | ParamMode::Out | ParamMode::InOut) {
                    self.error(
                        a.span,
                        "a field of a packed record that is not a whole byte, word or longword \
                         cannot be passed to a `var`, `out` or `in out` parameter (ref 3.10.3)",
                    );
                } else if let Err(d) = dpc_syntax::diag::best_effort(Diagnostic::error(
                    a.span,
                    "passing a field of a packed record that is not a whole byte, word or \
                     longword (ref 3.10.3) is not supported yet",
                )) {
                    self.errors.push(d);
                }
                // A value or `in` parameter takes the field's value, as it
                // takes any expression's: by value in its slot, or by
                // address of a copy (Table 5-1, p. 5-5).  Best effort (not
                // byte-identical; bead domain-pascal-eoc6): ref 3.10.3
                // forbids it, the image has no site; accepted so that the
                // kernel's translation compiles.
            }
            if !univ {
                match mode {
                    // ref 5.3.1 p. 5-6: "You must pass a variable as an
                    // argument to a variable parameter."
                    ParamMode::Var | ParamMode::Out | ParamMode::InOut => {
                        if !x.is_designator() && x.transfers_variable() {
                            // As for an assignment ([`Self::assign_target`]).
                            self.unsupported(
                                a.span,
                                "a type transfer of a variable or a field or element of one \
                                 passed to a `var`, `out` or `in out` parameter (whether it is \
                                 a variable: open question 46, ref p. 4-197)",
                            );
                        } else if !x.is_designator() && !self.types.is_error(x.ty) {
                            self.error(a.span, "a variable is needed for this parameter");
                        } else if self.same_type(x.ty, ty)
                            || self.univ_ptr_param(x.ty, ty)
                            // ref Appendix D, ISO 6.6.3.5/6.6.3.6, p. D-3;
                            // clarification Q38: integer and subrange of
                            // integer, same size, accepted quietly.
                            || self.integer_subrange_param(x.ty, ty)
                        {
                            // Compatible.
                        } else if self.integer_subrange_mix(x.ty, ty) {
                            // Q38's mix of another size: the callee would
                            // read or write the wrong bytes.
                            self.error(
                                a.span,
                                format!(
                                    "argument type {} ({}) does not match parameter type {} \
                                     ({}): an integer or subrange argument for a `var`, `out` \
                                     or `in out` parameter must have the parameter's size",
                                    self.types.display(x.ty),
                                    bytes(self.types.size(x.ty)),
                                    self.types.display(ty),
                                    bytes(self.types.size(ty))
                                ),
                            );
                        } else {
                            self.error(
                                a.span,
                                format!(
                                    "argument type {} does not match parameter type {}",
                                    self.types.display(x.ty),
                                    self.types.display(ty)
                                ),
                            );
                        }
                    }
                    ParamMode::Value | ParamMode::In => {
                        self.check_assignable(ty, &x, a.span);
                    }
                }
            } else if matches!(mode, ParamMode::Var | ParamMode::Out | ParamMode::InOut)
                && !x.is_designator()
            {
                self.error(a.span, "a variable is needed for this parameter");
            }
            // A value or `in` argument of another size than the parameter
            // (an `integer32` variable for an `integer` parameter, ref 5.3.2
            // p. 5-7: assignment compatible) cannot be passed by its own
            // address: the callee would read the wrong bytes; the caller
            // must pass a converted copy.
            let other_size = !univ
                && matches!(mode, ParamMode::Value | ParamMode::In)
                && !self.types.is_error(x.ty)
                && self.types.size(x.ty) != self.types.size(ty);
            let needs_temp = c.passing == Passing::ByAddress && (!x.is_designator() || other_size);
            out.push(Arg {
                expr: x,
                conv: c,
                needs_temp,
            });
        }
        Call { callee, args: out }
    }

    /// Ref 5.3.3 p. 5-11 (checked in the PDF): "When you pass an
    /// expression argument (as opposed to a variable argument) to a univ
    /// parameter, Domain Pascal extends the expression to be the same size
    /// as the univ parameter.  In addition, the compiler issues the
    /// following message: Expression passed to UNIV formal NAME was
    /// converted to NEWTYPE." (Appendix B, message 246, a WARNING.)  Given
    /// for an expression smaller than the parameter; whether the original
    /// also warned for an expression of equal size and another type is not
    /// known.  A formal of a routine-pointer type has no name in its
    /// signature: dpc names it by its position (`#2`).
    fn univ_conversion_warning(
        &mut self,
        callee: &Callee,
        index: usize,
        x: &Expr,
        ty: TypeId,
        span: Span,
    ) {
        if x.is_designator()
            || self.types.is_error(x.ty)
            || self.types.size(x.ty) >= self.types.size(ty)
        {
            return;
        }
        let name = match callee {
            Callee::Routine(r) => self.routines[r.0 as usize].params[index].name.clone(),
            Callee::Pointer(_) => format!("#{}", index + 1),
        };
        self.warnings.push(Diagnostic::warning(
            span,
            format!(
                "Expression passed to UNIV formal {name} was converted to {}.",
                self.types.display(ty)
            ),
        ));
    }

    /// `T(x)` (ref "Type Transfer Functions", p. 4-197): "the size of the
    /// argument must be the same as the size of the destination type ...
    /// integer subranges are always compatible regardless of their sizes."
    fn transfer(&mut self, t: TypeId, args: &[ast::Expr], span: Span) -> Expr {
        if args.len() != 1 {
            self.error(span, "a type transfer takes one argument");
            return self.error_expr(span);
        }
        let x = self.expr(&args[0]);
        if self.types.is_error(x.ty) || self.types.is_error(t) {
            return self.error_expr(span);
        }
        let ints = self.types.is_integer(x.ty) && self.types.is_integer(t);
        // An integer *constant* to a pointer type of another size: allowed
        // by the owner's ruling (reference-clarifications, "Type Transfer
        // Functions" (p. 4-197): integer constant to a pointer type; bead
        // domain-pascal-gqp): Apollo's `ec2_$ptr_t( ec2_$always_ready_ec )`
        // (streams/sasc misc.pas), the constant 1 a sentinel.  The value
        // is known, so the pointer is that address.  The reference says
        // nothing of a negative constant (no bits to keep, p. 4-197, once
        // the sizes differ): refused.  A run-time integer of another size
        // stays refused (the p. 4-197 size rule).  An integer32 constant
        // (equal sizes, p. 4-197: the bits are kept, negative or not) is
        // a pointer constant too, so that `ptr_t(16#20000)` goes where
        // `ptr_t(1)` goes (a static variable's initial value).
        let int_const = match &x.value {
            Some(ConstValue::Int(v)) if self.types.is_integer(x.ty) => Some(*v),
            _ => None,
        };
        if let Some(v) = int_const.filter(|_| self.types.is_pointer(t)) {
            if v < 0 && self.types.size(x.ty) != self.types.size(t) {
                self.error(
                    span,
                    format!(
                        "a type transfer of the negative constant {v} to the pointer type {} \
                         (the reference does not say what it means; only constants >= 0 are \
                         accepted, ref p. 4-197)",
                        self.types.display(t)
                    ),
                );
                return self.error_expr(span);
            }
            // A constant, so that it goes where constants go (a stored
            // immediate, a code-section cell passed by address,
            // codegen-patterns 1.5).
            return Expr {
                kind: ExprKind::Transfer(Box::new(x)),
                ty: t,
                value: Some(ConstValue::Int(v)),
                span,
            };
        }
        if !ints && self.types.size(x.ty) != self.types.size(t) {
            self.error(
                span,
                format!(
                    "type transfer needs equal sizes: {} is {} bytes, {} is {}",
                    self.types.display(x.ty),
                    self.types.size(x.ty),
                    self.types.display(t),
                    self.types.size(t)
                ),
            );
            return self.error_expr(span);
        }
        let value = match &x.value {
            Some(ConstValue::Int(v)) if ints => Some(ConstValue::Int(self.reinterpret(*v, t))),
            _ => None,
        };
        Expr {
            kind: ExprKind::Transfer(Box::new(x)),
            ty: t,
            value,
            span,
        }
    }

    /// Integer constant `v` transferred to integer type `t` (ref p. 4-197:
    /// "the type transfer function does not change any bits"): its low
    /// `size(t)` bytes read as `t` reads them, as the code computes it at
    /// run time (`dpc-ir`'s `sign_of`: a subrange stored in fewer bytes
    /// than its integer32 host, e.g. `0..65535`, is unsigned, any other
    /// integer type signed), so `0..65535(-1)` is 65535 and
    /// `integer16(16#18000)` is -32768 (a narrowing truncates, as
    /// p. 4-198's `INTEGER16(x + y)` "ignores the possibility of integer
    /// overflow").
    fn reinterpret(&self, v: i64, t: TypeId) -> i64 {
        let size = self.types.size(t);
        let bits = size * 8;
        if !(1..64).contains(&bits) {
            return v;
        }
        let mut w = v & ((1i64 << bits) - 1);
        let unsigned = self
            .types
            .ordinal_base(t)
            .is_some_and(|host| size < self.types.size(host));
        if !unsigned && w >= 1i64 << (bits - 1) {
            w -= 1i64 << bits;
        }
        w
    }

    fn builtin_fn(&mut self, b: Builtin, name: &str, args: &[ast::Expr], span: Span) -> Expr {
        let bi = self.types.builtins;
        let want = match b {
            Builtin::Disable | Builtin::Enable | Builtin::Discard => {
                self.error(span, format!("`{name}` is a procedure"));
                return self.error_expr(span);
            }
            Builtin::Sizeof | Builtin::Firstof | Builtin::Lastof => {
                return self.type_inquiry(b, name, args, span);
            }
            Builtin::Lshft | Builtin::Rshft | Builtin::Arshft | Builtin::Xor => 2,
            Builtin::Max | Builtin::Min => 2,
            _ => 1,
        };
        if args.len() != want {
            self.error(span, format!("`{name}` takes {want} argument(s)"));
            return self.error_expr(span);
        }
        if b == Builtin::Addr {
            // C.3.5, p. C-7: addr returns "the virtual address of the
            // specified variable or routine".
            if let ast::ExprKind::Name(id) = &args[0].kind {
                if let Some(Sym::Routine(r)) = self.lookup(&id.name) {
                    let r = *r;
                    return self.routine_addr(r, span);
                }
            }
        }
        let saved = self.addr_operand.take();
        if b == Builtin::Addr && args.len() == 1 {
            self.addr_operand = Some(args[0].span);
        }
        let xs: Vec<Expr> = args.iter().map(|a| self.expr(a)).collect();
        self.addr_operand = saved;
        if xs.iter().any(|x| self.types.is_error(x.ty)) {
            return self.error_expr(span);
        }
        let ty = match b {
            // "Addr", p. 4-13: "x Can be a variable declared as any data
            // type ... x can also be a string constant but it cannot be a
            // constant of any type other than string."  "The addr function
            // returns an univ_ptr value."  Of a string constant (p. 4-13)
            // and of a routine (C.3.5, above): not supported yet.
            Builtin::Addr => {
                if matches!(xs[0].value, Some(ConstValue::Str(_) | ConstValue::Char(_))) {
                    self.unsupported(span, "`addr` of a string constant (ref p. 4-13)");
                    return self.error_expr(span);
                }
                if !xs[0].is_designator() && xs[0].transfers_variable() {
                    // Open question 46 ([`Self::assign_target`]).
                    self.unsupported(
                        span,
                        "`addr` of a type transfer of a variable or a field or element of one \
                         (whether it is a variable: open question 46, ref p. 4-197)",
                    );
                    return self.error_expr(span);
                }
                if !xs[0].is_designator() {
                    self.error(span, "`addr` needs a variable (ref p. 4-13)");
                    return self.error_expr(span);
                }
                // `addr` of an `in` parameter, or of a field or element
                // of one, is accepted (bead domain-pascal-mbtx): p. 4-13
                // excepts only routines with the internal attribute (the
                // one `addr` message of chapter 9, "Cannot take the
                // address of internal routine"), and 5.3.2 (p. 5-7)
                // forbids only "us[ing] the parameter on the left side of
                // an assignment statement".  Writing through the address
                // breaks 5.3.2's "not allowed to alter its value" (which
                // callers rely on, [`Checker::in_param_part`]) as writing
                // through any univ_ptr may; nothing checks that.  The
                // address is the parameter's slot value when it is passed
                // by address, the slot's address when by value
                // (`dpc_ir`'s `var_addr`; Table 5-1, p. 5-5).
                if self.is_bit_field(&xs[0]) {
                    self.error(
                        span,
                        "a field of a packed record that is not a whole byte, word or longword \
                         has no address (ref 3.10.3, 3.10.7)",
                    );
                    return self.error_expr(span);
                }
                bi.univ_ptr
            }
            Builtin::Ord => {
                if !self.types.is_ordinal(xs[0].ty) {
                    self.error(span, "`ord` needs an ordinal argument");
                    return self.error_expr(span);
                }
                // p. 4-137: "returns an integer value".  The width is not
                // stated; integer32 arguments keep 32 bits (Uncertain).
                if let Some(o) = xs[0].value.as_ref().and_then(|v| v.ordinal()) {
                    return self.int_const(o, span);
                }
                self.int_result(xs[0].ty, bi.integer16)
            }
            Builtin::Chr => {
                if !self.types.is_integer(xs[0].ty) {
                    self.error(span, "`chr` needs an integer argument");
                    return self.error_expr(span);
                }
                // p. 4-39: "number & 16#FF".
                if let Some(ConstValue::Int(v)) = &xs[0].value {
                    let c = (*v & 0xFF) as u8;
                    return self.konst(ConstValue::Char(c), bi.char, span);
                }
                bi.char
            }
            Builtin::Odd => {
                if !self.types.is_integer(xs[0].ty) {
                    self.error(span, "`odd` needs an integer argument");
                    return self.error_expr(span);
                }
                if let Some(ConstValue::Int(v)) = &xs[0].value {
                    return self.konst(ConstValue::Bool(v & 1 == 1), bi.boolean, span);
                }
                bi.boolean
            }
            // "Abs", p. 4-12: "an integer value if number is an integer";
            // reals are out of scope (CLAUDE.md 1).
            Builtin::Abs => {
                if !self.types.is_integer(xs[0].ty) {
                    self.error(
                        span,
                        "`abs` needs an integer argument (reals are not supported)",
                    );
                    return self.error_expr(span);
                }
                if let Some(ConstValue::Int(v)) = &xs[0].value {
                    return self.int_const(v.abs(), span);
                }
                self.abs_result(xs[0].ty)
            }
            // "Sqr", p. 4-184: "returns an integer if n is an integer";
            // "calculates n * n".  Reals are out of scope (CLAUDE.md 1).
            // A computed argument is not lowered yet (dpc-ir: no image
            // pattern catalogued); a constant one folds (ref 2.2.2.2,
            // p. 2-12, names sqr), out of range an error (ref 3.2.3).
            Builtin::Sqr => {
                if !self.types.is_integer(xs[0].ty) {
                    self.error(
                        span,
                        "`sqr` needs an integer argument (reals are not supported)",
                    );
                    return self.error_expr(span);
                }
                if let Some(ConstValue::Int(v)) = &xs[0].value {
                    return self.int_const(v * v, span);
                }
                self.int_result(xs[0].ty, bi.integer16)
            }
            Builtin::Lshft | Builtin::Rshft | Builtin::Arshft | Builtin::Xor => {
                if !xs.iter().all(|x| self.types.is_integer(x.ty)) {
                    self.error(span, format!("`{name}` needs integer arguments"));
                    return self.error_expr(span);
                }
                // Constant arguments (ref 2.2.2.2, p. 2-12: a const value
                // may contain "Any predefined Domain Pascal function (for
                // example, chr, sqr, lshft, sizeof, but only if the
                // argument to the function is a constant)") fold within
                // the constants' 32-bit representation (ref 3.2.3, p. 3-5),
                // as `&`, `!` and `~` of constants do.  Apollo's X11
                // x.ins.pas relies on it: `ExposureMask = LSHFT(1,15)`,
                // `OwnerGrabButtonMask = LSHFT(1,24)` (X11's 1<<15, 1<<24).
                if let (Some(ConstValue::Int(a)), Some(ConstValue::Int(n))) =
                    (&xs[0].value, &xs[1].value)
                {
                    if let Some(v) = self.fold_bits(b, *a, *n) {
                        return self.int_const(v, span);
                    }
                }
                // xor: the wider operand (Uncertain).  Shifts:
                // [`Checker::shift_result`].
                match b {
                    Builtin::Xor => self.int_result(xs[0].ty, xs[1].ty),
                    Builtin::Arshft => self.int_result(xs[0].ty, bi.integer16),
                    _ => self.shift_result(&xs[0]),
                }
            }
            // Appendix E, p. E-5: "oldsr := set_sr(newsr); {set_sr is a
            // function.}", expanded as "move.w SR,d0 / move.w newsr,SR /
            // move.w d0,oldsr": a word.  TODO(ref-question): the entry
            // gives no types for newsr and oldsr; a 2-byte integer
            // argument and an integer16 result are the conservative
            // reading of the `move.w`s (reference-clarifications Q16).
            Builtin::SetSr => {
                let word = match xs[0].value.as_ref().and_then(|v| v.ordinal()) {
                    Some(k) => (-32768..=65535).contains(&k),
                    None => self.types.size(xs[0].ty) == 2,
                };
                if !self.types.is_integer(xs[0].ty) || !word {
                    self.error(
                        span,
                        "`set_sr` needs a 16-bit integer argument (ref Appendix E, p. E-5)",
                    );
                    return self.error_expr(span);
                }
                bi.integer16
            }
            // "Succ", p. 4-191 / "Pred", p. 4-147: x "must be an integer,
            // Boolean, char, or enumerated expression"; the result has
            // "the same data type as x"; "Domain Pascal does not report an
            // error" out of range (also Appendix D, p. D-5: "Succ, pred,
            // and chr do not detect overflow").
            Builtin::Succ | Builtin::Pred => {
                if !self.types.is_ordinal(xs[0].ty) {
                    self.error(span, format!("`{name}` needs an ordinal argument"));
                    return self.error_expr(span);
                }
                let up = b == Builtin::Succ;
                if self.types.is_boolean(xs[0].ty) {
                    // "True--even if x already equals true" (p. 4-191);
                    // "False--even if x already equals false" (p. 4-147).
                    if xs[0].value.is_some() {
                        return self.konst(ConstValue::Bool(up), xs[0].ty, span);
                    }
                } else if let Some(k) = xs[0].value.as_ref().and_then(|v| v.ordinal()) {
                    let k = if up { k + 1 } else { k - 1 };
                    let v = match xs[0].value.as_ref().unwrap() {
                        ConstValue::Int(_) => ConstValue::Int(k),
                        ConstValue::Char(_) => ConstValue::Char(k as u8),
                        ConstValue::Enum { ty, .. } => ConstValue::Enum { ty: *ty, ord: k },
                        _ => unreachable!(),
                    };
                    return self.konst(v, xs[0].ty, span);
                }
                xs[0].ty
            }
            // "Max", p. 4-111 / "Min", p. 4-113: the arguments "must be the
            // same type or must be convertible to the same type"; unsigned
            // scalars and pointers compare unsigned, signed integers
            // signed.  Reals are out of scope (CLAUDE.md 1).
            Builtin::Max | Builtin::Min => {
                let (a, c) = (&xs[0], &xs[1]);
                let ty = if self.types.is_integer(a.ty) && self.types.is_integer(c.ty) {
                    self.int_result(a.ty, c.ty)
                } else if self.types.is_ordinal(a.ty) && self.ordinal_compatible(a.ty, c.ty) {
                    self.types.ordinal_base(a.ty).unwrap()
                } else if self.types.is_pointer(a.ty)
                    && self.types.is_pointer(c.ty)
                    && self.pointers_compatible(a.ty, c.ty)
                {
                    a.ty
                } else {
                    self.error(
                        span,
                        format!("`{name}` needs two ordinal or pointer arguments of one type"),
                    );
                    return self.error_expr(span);
                };
                if let (Some(x), Some(y)) = (
                    a.value.as_ref().and_then(|v| v.ordinal()),
                    c.value.as_ref().and_then(|v| v.ordinal()),
                ) {
                    let pick = if (b == Builtin::Max) == (x >= y) {
                        a.value.clone().unwrap()
                    } else {
                        c.value.clone().unwrap()
                    };
                    if self.types.is_integer(ty) {
                        let k = pick.ordinal().unwrap();
                        return self.int_const(k, span);
                    }
                    return self.konst(pick, ty, span);
                }
                ty
            }
            // "In_range", p. 4-104: x has "a scalar (i.e., integer, Boolean,
            // char, enumerated, or subrange) data type"; "returns true if
            // the value of x is within its defined range".
            Builtin::InRange => {
                let Some((lo, hi)) = self.types.ordinal_range(xs[0].ty) else {
                    self.error(span, "`in_range` needs a scalar argument (ref p. 4-104)");
                    return self.error_expr(span);
                };
                if let Some(k) = xs[0].value.as_ref().and_then(|v| v.ordinal()) {
                    return self.konst(ConstValue::Bool(lo <= k && k <= hi), bi.boolean, span);
                }
                bi.boolean
            }
            Builtin::Disable
            | Builtin::Enable
            | Builtin::Discard
            | Builtin::Sizeof
            | Builtin::Firstof
            | Builtin::Lastof => unreachable!(),
        };
        Expr {
            kind: ExprKind::Builtin {
                builtin: b,
                args: xs,
            },
            ty,
            value: None,
            span,
        }
    }

    /// `lshft`, `rshft`, `arshft` or `xor` of integer constants `a` and
    /// `n`, folded in 32 bits (ref 3.2.3, p. 3-5: "When you declare an
    /// integer constant, Domain Pascal internally represents the value as a
    /// 32-bit integer").  `None`, left to run time, where the reference
    /// gives no constant value:
    ///
    /// - a shift count outside 0..=31: "Results are unpredictable if sh is
    ///   negative" ("Lshft", p. 4-109); counts of 32 or more are not
    ///   described;
    /// - `rshft` of a negative constant: p. 4-171 gives both the 32-bit
    ///   result ("if num is negative and is a 32-bit integer, the result
    ///   of an rshft is always positive") and, for a 16-bit num stored in
    ///   16 bits, the narrowed one (`rshft(-9, 3)` is -2); which one a
    ///   negative *constant* gets is not stated.
    fn fold_bits(&self, b: Builtin, a: i64, n: i64) -> Option<i64> {
        let a32 = a as i32;
        match b {
            // "Xor", p. 4-221: the bitwise exclusive or.
            Builtin::Xor => Some((a32 ^ (n as i32)) as i64),
            _ if !(0..=31).contains(&n) => None,
            // "Lshft", p. 4-109: zeros shifted in on the right; bits
            // shifted out on the left are lost.
            Builtin::Lshft => Some(((a32 as u32) << n) as i32 as i64),
            // "Rshft", p. 4-171: zeros shifted in from the left.
            Builtin::Rshft if a >= 0 => Some(((a32 as u32) >> n) as i64),
            // "Arshft", p. 4-29: the sign bit preserved.
            Builtin::Arshft => Some((a32 >> n) as i64),
            _ => None,
        }
    }

    /// `sizeof(x)`, `firstof(x)`, `lastof(x)`: `x` is a type name or a
    /// variable (and for `sizeof` also a constant or a string); the value
    /// is always a compile-time constant.
    fn type_inquiry(&mut self, b: Builtin, name: &str, args: &[ast::Expr], span: Span) -> Expr {
        if args.is_empty() {
            self.error(span, format!("`{name}` takes 1 argument"));
            return self.error_expr(span);
        }
        if args.len() > 1 {
            if b == Builtin::Sizeof {
                // "Sizeof", p. 4-181: "sizeof(x, tag1, ... tagN)" gives the
                // size of a variant record for the given tags.  How the
                // size of one variant is rounded is not stated.
                self.unsupported(
                    span,
                    "`sizeof` of a variant record with tags (ref p. 4-182)",
                );
            } else {
                self.error(span, format!("`{name}` takes 1 argument"));
            }
            return self.error_expr(span);
        }
        // A type name, or an expression.
        let (ty, value) = match &args[0].kind {
            ast::ExprKind::Name(id) if matches!(self.lookup(&id.name), Some(Sym::Type(_))) => {
                let Some(Sym::Type(t)) = self.lookup(&id.name).cloned() else {
                    unreachable!()
                };
                (t, None)
            }
            _ => {
                let x = self.expr(&args[0]);
                if self.types.is_error(x.ty) {
                    return self.error_expr(span);
                }
                // "Firstof", p. 4-87 / "Lastof", p. 4-106: "x Is either a
                // variable or the name of a data type"; "Sizeof", p. 4-181:
                // "a type ..., a variable, a constant, or a string".
                let sizeof_const = b == Builtin::Sizeof && x.value.is_some();
                if !(x.is_designator() || sizeof_const) {
                    self.error(span, format!("`{name}` needs a type name or a variable"));
                    return self.error_expr(span);
                }
                // A packed field that fills a byte has the one-byte view
                // of its type ([`Self::byte_field_type`]); `sizeof` of it
                // stays the size of the declared type, as for any other
                // packed field (a bit field's is its type's too).
                let declared = match &x.kind {
                    ExprKind::Field { bits: Some(_), .. } => self
                        .byte_fields
                        .iter()
                        .find_map(|(&t, &v)| (v == x.ty).then_some(t)),
                    _ => None,
                };
                (declared.unwrap_or(x.ty), x.value)
            }
        };
        if self.types.is_error(ty) {
            return self.error_expr(span);
        }
        if b == Builtin::Sizeof {
            // "Sizeof returns an integer equal to the number of bytes that
            // the program uses to store x" (p. 4-181); of a string
            // constant, its length ("SIZEOF('wildebeest')" is 10, p. 4-182).
            let n = match (&value, self.types.kind(ty)) {
                (Some(ConstValue::Str(s)), _) => s.len() as u32,
                _ => self.types.size(ty),
            };
            return self.int_const(n as i64, span);
        }
        let first = b == Builtin::Firstof;
        // The tables of "Firstof" (p. 4-87) and "Lastof" (p. 4-106).
        let (v, vty) = match self.types.kind(ty).clone() {
            TypeKind::Integer16 | TypeKind::Integer32 if first => {
                // TODO(ref-question): p. 4-87 (checked in the PDF, p. 220)
                // gives -32767 for integer/integer16 and -2147483647 for
                // integer32, one more than the types' least values
                // (reference-clarifications Q17).  Refused until settled.
                self.unsupported(
                    span,
                    "`firstof` of integer, integer16 or integer32 (ref p. 4-87 gives -32767 and \
                     -2147483647; open question)",
                );
                return self.error_expr(span);
            }
            TypeKind::Integer16 => (ConstValue::Int(32767), ty),
            TypeKind::Integer32 => (ConstValue::Int(2147483647), ty),
            TypeKind::Char => (ConstValue::Char(if first { 0 } else { 255 }), ty),
            TypeKind::Boolean => (ConstValue::Bool(!first), ty),
            TypeKind::Enum { consts } => (
                ConstValue::Enum {
                    ty,
                    ord: if first { 0 } else { consts.len() as i64 - 1 },
                },
                ty,
            ),
            // Not in the tables; "returns the first [final] possible value
            // of x" (pp. 4-87, 4-106): the bound of the subrange.
            TypeKind::Subrange { base, lo, hi } => {
                let k = if first { lo } else { hi };
                let v = match self.types.kind(base) {
                    TypeKind::Char => ConstValue::Char(k as u8),
                    TypeKind::Boolean => ConstValue::Bool(k != 0),
                    TypeKind::Enum { .. } => ConstValue::Enum { ty: base, ord: k },
                    _ => ConstValue::Int(k),
                };
                (v, ty)
            }
            // "The lower [upper] bound of the subrange that defines the
            // array's size": a value of the index type.
            TypeKind::Array { index, .. } => {
                let Some((lo, hi)) = self.types.ordinal_range(index) else {
                    return self.error_expr(span);
                };
                let k = if first { lo } else { hi };
                let base = self.types.ordinal_base(index).unwrap();
                let v = match self.types.kind(base) {
                    TypeKind::Char => ConstValue::Char(k as u8),
                    TypeKind::Boolean => ConstValue::Bool(k != 0),
                    TypeKind::Enum { .. } => ConstValue::Enum { ty: base, ord: k },
                    _ => ConstValue::Int(k),
                };
                (v, index)
            }
            // "varying array: 1" (p. 4-87); "The maximum length of the
            // array" (p. 4-106).
            TypeKind::Varying { max, .. } => {
                return self.int_const(if first { 1 } else { max as i64 }, span);
            }
            _ => {
                self.error(
                    span,
                    format!(
                        "`{name}` of {} (ref pp. 4-87, 4-106: not a record, file, set, \
                         floating-point or pointer type)",
                        self.types.display(ty)
                    ),
                );
                return self.error_expr(span);
            }
        };
        if let ConstValue::Int(k) = v {
            if self.types.kind(vty) == &TypeKind::Integer16
                || self.types.kind(vty) == &TypeKind::Integer32
            {
                return self.int_const(k, span);
            }
        }
        self.konst(v, vty, span)
    }

    // -----------------------------------------------------------------
    // Compatibility (ISO 7185 6.4.5/6.4.6, which the reference assumes;
    // Domain extensions cited where used)
    // -----------------------------------------------------------------

    fn same_type(&self, a: TypeId, b: TypeId) -> bool {
        self.types.canon(a) == self.types.canon(b)
            || self.types.is_error(a)
            || self.types.is_error(b)
    }

    /// Ordinal types with the same host type.  integer16 and integer32 (and
    /// their subranges) are one host: Domain Pascal mixes them freely in
    /// expressions (ref 4.3.1 p. 4-5).
    fn ordinal_compatible(&self, a: TypeId, b: TypeId) -> bool {
        if self.types.is_error(a) || self.types.is_error(b) {
            return true;
        }
        match (self.types.ordinal_base(a), self.types.ordinal_base(b)) {
            (Some(x), Some(y)) => x == y || (self.types.is_integer(x) && self.types.is_integer(y)),
            _ => false,
        }
    }

    fn sets_compatible(&self, a: TypeId, b: TypeId) -> bool {
        match (self.types.kind(a), self.types.kind(b)) {
            (TypeKind::Set { elem: x }, TypeKind::Set { elem: y }) => {
                self.ordinal_compatible(*x, *y)
            }
            _ => false,
        }
    }

    /// A pointer argument for a `var`, `out` or `in out` parameter of
    /// another pointer type, one of the two being `univ_ptr`: ref 3.13.2,
    /// p. 3-50, a `univ_ptr` may be used as a "Formal or actual parameter
    /// for any pointer type" (Apollo's ios.ins.pas `ios_$locate`'s `OUT b:
    /// univ_ptr` is passed typed pointers, e.g. domain_examples
    /// enscramble.pas).
    fn univ_ptr_param(&self, arg: TypeId, param: TypeId) -> bool {
        let univ = |t: TypeId| matches!(self.types.kind(self.types.canon(t)), TypeKind::UnivPtr);
        self.types.is_pointer(arg) && self.types.is_pointer(param) && (univ(arg) || univ(param))
    }

    /// An integer subrange and an integer type (`integer`, `integer16`,
    /// `integer32`), or two integer subranges: the mix the reference allows
    /// for `var`, `out` and `in out` arguments.  Ref Appendix D, ISO
    /// 6.6.3.5/6.6.3.6, p. D-3: "Domain Pascal treats integer and subrange
    /// of integer as identical"; clarification Q38 (accepted quietly).
    /// Sizes are not compared here: see [`Checker::integer_subrange_param`].
    fn integer_subrange_mix(&self, arg: TypeId, param: TypeId) -> bool {
        let sub = |t: TypeId| matches!(self.types.kind(t), TypeKind::Subrange { .. });
        self.types.is_integer(arg) && self.types.is_integer(param) && (sub(arg) || sub(param))
    }

    /// [`Checker::integer_subrange_mix`] with the argument's storage size
    /// equal to the parameter's.  The argument is passed by its address, so
    /// a 2-byte variable for a 4-byte parameter (or the reverse) would make
    /// the callee read or write the wrong bytes; that mix stays refused (the
    /// lead's limit in clarification Q38, not settled by the ruling).
    fn integer_subrange_param(&self, arg: TypeId, param: TypeId) -> bool {
        self.integer_subrange_mix(arg, param) && self.types.size(arg) == self.types.size(param)
    }

    /// `nil` and `univ_ptr` are compatible with every pointer (ref 3.13.2
    /// p. 3-50).  Two `^T` types are compatible when they point to the same
    /// type (TODO(ref-question): p. 4-144 says only "compatible pointer";
    /// name equivalence of the pointer types themselves would be stricter).
    fn pointers_compatible(&self, a: TypeId, b: TypeId) -> bool {
        use TypeKind as K;
        match (self.types.kind(a), self.types.kind(b)) {
            (K::Nil, _) | (_, K::Nil) | (K::UnivPtr, _) | (_, K::UnivPtr) => true,
            (K::Pointer { target: x }, K::Pointer { target: y }) => {
                self.types.canon(*x) == self.types.canon(*y) || self.same_type(a, b)
            }
            (K::RoutinePointer(x), K::RoutinePointer(y)) => a == b || x == y,
            _ => false,
        }
    }

    /// A record or an array (not a string literal).
    fn is_structured(&self, t: TypeId) -> bool {
        matches!(
            self.types.kind(self.types.canon(t)),
            TypeKind::Record(_) | TypeKind::Array { .. }
        )
    }

    /// Strings and char arrays of equal length (ref "Array Operations",
    /// p. 4-26).
    fn strings_compatible(&self, a: TypeId, b: TypeId) -> bool {
        let len = |t: TypeId| match self.types.kind(t) {
            TypeKind::StringLit { len } => Some(*len),
            _ => self.types.char_array_len(t),
        };
        matches!((len(a), len(b)), (Some(x), Some(y)) if x == y)
    }

    /// Check that `x` may be assigned to an object of type `dst`; report
    /// error 79 (p. 9-32) if not.  Returns whether it may.
    fn check_assignable(&mut self, dst: TypeId, x: &Expr, span: Span) -> bool {
        let src = x.ty;
        if let (ExprKind::RoutineAddr(r), TypeKind::RoutinePointer(sig)) =
            (&x.kind, self.types.kind(dst))
        {
            // ref p. 4-146: the extra check of a routine's address against
            // a routine pointer type.
            if !self.routine_matches(*r, sig) {
                self.error(
                    span,
                    format!(
                        "`{}` does not match the routine pointer type {} (ref p. 4-146)",
                        self.routines[r.0 as usize].name,
                        self.types.display(dst)
                    ),
                );
                return false;
            }
            return true;
        }
        if self.types.is_error(dst) || self.types.is_error(src) || self.same_type(dst, src) {
            return true;
        }
        let ok = if self.types.is_ordinal(dst) && self.types.is_ordinal(src) {
            if !self.ordinal_compatible(dst, src) {
                false
            } else {
                // A constant must lie in the destination's range, except
                // for an integer subrange: ref Appendix D, ISO 6.4.6,
                // p. D-3, "You may make an assignment from an integer
                // expression that includes a value outside one of the
                // variables' declared subranges"; clarification Q38: the
                // destination keeps the constant's low bytes (as Q41's
                // narrowing).
                let int_subrange = self.types.is_integer(dst)
                    && matches!(self.types.kind(dst), TypeKind::Subrange { .. });
                if let (false, Some(k), Some((lo, hi))) = (
                    int_subrange,
                    x.value.as_ref().and_then(|v| v.ordinal()),
                    self.types.ordinal_range(dst),
                ) {
                    if k < lo || k > hi {
                        self.error(
                            span,
                            format!(
                                "constant {k} is out of range for {}",
                                self.types.display(dst)
                            ),
                        );
                        return false;
                    }
                }
                true
            }
        } else if matches!(self.types.kind(dst), TypeKind::Set { .. }) {
            self.sets_compatible(dst, src)
        } else if self.types.is_pointer(dst) && self.types.is_pointer(src) {
            !matches!(self.types.kind(dst), TypeKind::Nil) && self.pointers_compatible(dst, src)
        } else if let Some(n) = self.types.char_array_len(dst) {
            match self.types.kind(src) {
                // ref p. 4-26: a string no longer than the array.
                TypeKind::StringLit { len } => *len <= n,
                // ... and a char array of the same size.
                _ => self.types.char_array_len(src) == Some(n),
            }
        } else {
            false
        };
        if !ok {
            self.error(
                span,
                format!(
                    "Assignment statement expression is not compatible with the assignment variable ({} := {})",
                    self.types.display(dst),
                    self.types.display(src)
                ),
            );
        }
        ok
    }
}

/// "1 byte", "2 bytes".
fn bytes(n: u32) -> String {
    if n == 1 {
        "1 byte".into()
    } else {
        format!("{n} bytes")
    }
}

/// A routine's parameters as (mode, univ, type, convention).
fn r_sig(r: &Routine) -> Vec<(ParamMode, bool, TypeId, conv::ParamConv)> {
    r.params
        .iter()
        .map(|p| (p.mode, p.univ, p.ty, p.conv))
        .collect()
}

fn merge_options(a: &mut RoutineOptions, b: &RoutineOptions) {
    a.forward |= b.forward;
    a.extern_ |= b.extern_;
    a.internal |= b.internal;
    a.variable |= b.variable;
    a.abnormal |= b.abnormal;
    a.val_param |= b.val_param;
    a.nosave |= b.nosave;
    a.noreturn |= b.noreturn;
    a.d0_return |= b.d0_return;
    a.a0_return |= b.a0_return;
    a.c_param |= b.c_param;
}

fn label_name(l: &ast::Label) -> String {
    match l {
        ast::Label::Int(n, _) => n.to_string(),
        ast::Label::Ident(i) => i.name.clone(),
    }
}
