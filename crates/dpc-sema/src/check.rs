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
    round_up, DeviceAccess, Field, RecordType, SigParam, Signature, TypeAttrs, TypeId, TypeKind,
    TypeTable, VariantArm, VariantPart, MAX_OBJECT_SIZE,
};

/// Smallest alignment of an object in a routine's frame, in bytes: the
/// original compiler word-aligns byte locals (codegen-patterns 2, "Frame
/// layout"; Verified by survey, bead domain-pascal-ek7).
const FRAME_MIN_ALIGN: u32 = 2;

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
            next_with: 0,
            in_withs: HashSet::new(),
            loop_depth: 0,
            natural: false,
            align_stack: Vec::new(),
            string_types: HashMap::new(),
            is_module: false,
            label_defs: HashMap::new(),
            label_gotos: Vec::new(),
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

    /// Allocate `ty` in static data section `section`.  The order of
    /// statics in a module's data area is not verified against the image
    /// (Uncertain); declaration order at freestanding alignment is used.
    fn alloc_static(&mut self, section: &str, ty: TypeId) -> u32 {
        let (size, align) = {
            let i = self.types.get(ty);
            (i.size, i.align)
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
        match e.value {
            Some(value) => self.declare(&def.name, Sym::Const { value, ty: e.ty }),
            None => {
                if !self.types.is_error(e.ty) {
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
                if *packed && !self.packed_elem_same_as_unpacked(elem_ty) {
                    self.unsupported(
                        t.span,
                        "a packed array of this element type (ref 3.11.4.2 Table 3-5)",
                    );
                    return err;
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
            "bit" | "byte" | "word" | "long" | "quad" => {
                let what = format!("the `{name}` size attribute (ref 3.15.5)");
                self.unsupported(a.span, &what);
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
            },
        )
    }

    /// ref 3.11.4.2 Table 3-5 p. 3-47: element types whose packed storage
    /// is the same as unpacked.
    fn packed_elem_same_as_unpacked(&self, elem: TypeId) -> bool {
        match self.types.kind(elem) {
            TypeKind::Integer16
            | TypeKind::Integer32
            | TypeKind::Char
            | TypeKind::Real { .. }
            | TypeKind::Pointer { .. }
            | TypeKind::UnivPtr
            | TypeKind::Record(_)
            | TypeKind::Error => true,
            // Subrange of char: 8 bits, byte aligned.
            TypeKind::Subrange { base, .. } => matches!(self.types.kind(*base), TypeKind::Char),
            // Arrays of non-characters: "Exact number of words required".
            // Character arrays: "Exact number of bytes required".  Both
            // equal the unpacked size for the element types accepted here.
            TypeKind::Array { elem, .. } => self.packed_elem_same_as_unpacked(*elem),
            _ => false,
        }
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
    /// specify an anonymous data type").
    fn param_type(&mut self, ty: &ast::TypeExpr) -> TypeId {
        match &ty.kind {
            ast::TypeKind::Named(id) => self.named_type(id),
            _ => {
                self.error(
                    ty.span,
                    "a parameter's type must be a type name (ref 5.1, p. 5-2)",
                );
                self.types.builtins.error
            }
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
            let init = match &vd.init {
                None => None,
                Some(init) => {
                    if matches!(storage, Storage::Frame { .. }) {
                        // ref 3.2.2 p. 3-4: a routine's variable can be
                        // initialised only if it is static.
                        self.error(
                            init.span(),
                            "a routine's variable can be initialised only if it is static (ref 3.2.2, p. 3-4)",
                        );
                        None
                    } else {
                        let mut stores = Vec::new();
                        self.init(ty, 0, init, &mut stores);
                        Some(stores)
                    }
                }
            };
            let frame_var = matches!(storage, Storage::Frame { .. });
            let id = self.add_var(Var {
                name: name.name.clone(),
                ty,
                storage,
                owner: if frame_var { owner } else { None },
                init,
                span: name.span,
            });
            if let (true, Some(r)) = (frame_var, owner) {
                self.routines[r.0 as usize].frame_vars.push(id);
            }
            vars_out.push(id);
            self.declare(name, Sym::Var(id));
        }
    }

    /// `define x [:= v];` (ref 7.2.2, p. 7-5).
    fn define_item(&mut self, item: &ast::DefineItem) {
        let Some(sym) = self.lookup(&item.name.name).cloned() else {
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
                let section = self.data_section.clone();
                let new = match storage {
                    Storage::Extern { symbol } => Storage::Static {
                        offset: self.alloc_static(&section, ty),
                        symbol,
                        global: true,
                        section,
                    },
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
            // ref 7.3.4 p. 7-11: a define part may also name routines.
            Sym::Routine(_) => {}
            _ => self.error(
                item.name.span,
                format!("`{}` cannot be defined", item.name.name),
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
                match x.value {
                    Some(value) => out.push(InitStore {
                        offset: base,
                        ty,
                        value,
                    }),
                    None => self.error(e.span, "initial value must be a constant"),
                }
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
                        value: ConstValue::Set(members),
                    });
                }
                TypeKind::Array { index, elem, .. } => {
                    let (lo, hi) = self.types.ordinal_range(index).unwrap_or((0, -1));
                    let esize = self.types.size(elem);
                    let mut next = lo;
                    let at = |k: i64| base + ((k - lo) as u32) * esize;
                    for it in items {
                        match it {
                            ast::InitItem::Value(v) => {
                                if next > hi {
                                    self.error(*span, "too many initial values for the array");
                                    return;
                                }
                                self.init(elem, at(next), v, out);
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
                                    self.init(elem, at(next), value, out);
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
                                    self.init(elem, at(j), value, out);
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
                        // A packed record's field is initialised only when
                        // it is whole bytes holding the whole object of its
                        // type (status_$t's `all', ref 3.10.7 Table 3-3);
                        // values in bit fields are not encoded yet.
                        if let Some(bits) = f.bits {
                            if bits.whole_bytes() != Some(self.types.size(f.ty)) {
                                self.unsupported(
                                    *span,
                                    "initialising a packed record's field that is not whole bytes",
                                );
                                continue;
                            }
                        }
                        self.init(f.ty, base + f.offset, v, out);
                    }
                }
                _ => self.error(
                    *span,
                    "a bracketed initialiser needs a set, array or record",
                ),
            },
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
        let result_var = if let Some(res) = self.routines[id.0 as usize].result.clone() {
            let name = self.routines[id.0 as usize].name.clone();
            let span = self.routines[id.0 as usize].span;
            let var = self.add_var(Var {
                name,
                ty: res.ty,
                storage: Storage::Frame { offset: 0 },
                owner: Some(id),
                init: None,
                span,
            });
            let rt = &mut self.routines[id.0 as usize];
            rt.result.as_mut().unwrap().var = Some(var);
            rt.frame_vars.push(var);
            Some((var, res.ty))
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
            if !r.is_designator() && !self.types.is_error(r.ty) {
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
                let mut b = self.expr(base);
                for ix in indices {
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

    fn index(&mut self, base: Expr, ix: &ast::Expr, span: Span) -> Expr {
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
                    self.error(ix.span, "index out of range");
                }
            }
        }
        let elem_size = self.types.size(elem);
        Expr {
            kind: ExprKind::Index {
                base: Box::new(base),
                index: Box::new(i),
                lo,
                elem_size,
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
                } else {
                    self.unsupported(
                        a.span,
                        "passing a field of a packed record that is not a whole byte, word or \
                         longword (ref 3.10.3)",
                    );
                }
            }
            if !univ {
                match mode {
                    // ref 5.3.1 p. 5-6: "You must pass a variable as an
                    // argument to a variable parameter."
                    ParamMode::Var | ParamMode::Out | ParamMode::InOut => {
                        if !x.is_designator() && !self.types.is_error(x.ty) {
                            self.error(a.span, "a variable is needed for this parameter");
                        } else if !self.same_type(x.ty, ty) && !self.univ_ptr_param(x.ty, ty) {
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
        let xs: Vec<Expr> = args.iter().map(|a| self.expr(a)).collect();
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
                if !xs[0].is_designator() {
                    self.error(span, "`addr` needs a variable (ref p. 4-13)");
                    return self.error_expr(span);
                }
                if self.in_param_part(&xs[0]) {
                    // ref 5.3.2 p. 5-7; see [`Checker::in_param_part`].
                    self.unsupported(span, "`addr` of an `in` parameter (or a part of one)");
                    return self.error_expr(span);
                }
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
            Builtin::Abs => {
                if !self.types.is_integer(xs[0].ty) {
                    self.error(span, "`abs` needs an integer argument");
                    return self.error_expr(span);
                }
                if let Some(ConstValue::Int(v)) = &xs[0].value {
                    return self.int_const(v.abs(), span);
                }
                self.int_result(xs[0].ty, bi.integer16)
            }
            Builtin::Lshft | Builtin::Rshft | Builtin::Arshft | Builtin::Xor => {
                if !xs.iter().all(|x| self.types.is_integer(x.ty)) {
                    self.error(span, format!("`{name}` needs integer arguments"));
                    return self.error_expr(span);
                }
                // Shifts: the result has the width of the shifted operand
                // (p. 4-109 example shifts an integer16 within 16 bits).
                // xor: the wider operand (Uncertain).
                if b == Builtin::Xor {
                    self.int_result(xs[0].ty, xs[1].ty)
                } else {
                    self.int_result(xs[0].ty, bi.integer16)
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
                (x.ty, x.value)
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
                // A constant must lie in the destination's range.
                if let (Some(k), Some((lo, hi))) = (
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
