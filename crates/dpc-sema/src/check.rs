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
    round_up, Field, RecordType, SigParam, Signature, TypeId, TypeKind, TypeTable, VariantArm,
    VariantPart, MAX_OBJECT_SIZE,
};

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
    Label,
}

#[derive(Debug, Default)]
struct Scope {
    names: HashMap<String, Sym>,
}

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
    loop_depth: u32,
    /// `%natural_alignment` in effect (ref 4.7 p. 4-57); push stack.
    natural: bool,
    align_stack: Vec<bool>,
    /// String-literal types, by length.
    string_types: HashMap<u32, TypeId>,
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
            loop_depth: 0,
            natural: false,
            align_stack: Vec::new(),
            string_types: HashMap::new(),
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
    /// The order and packing of the original compiler's frame slots are
    /// not known yet (codegen-patterns 2, Uncertain); this allocates in
    /// declaration order, downward.
    fn alloc_frame(&mut self, ty: TypeId) -> i32 {
        let (size, align) = {
            let i = self.types.get(ty);
            (i.size.max(1), i.align)
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
        let mut unit_vars = Vec::new();
        let top = self.decls(&unit.decls, None, &mut unit_vars);
        let main = unit.body.as_ref().map(|b| self.block(b));
        for p in &unit.trailing_pragmas {
            self.pragma(p);
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
                    self.unsupported(a.span, "the attribute declaration part (ref 3.16)")
                }
                Decl::RoutineOption(ro) => {
                    for def in &ro.defs {
                        self.routine_option_def(def);
                    }
                }
                Decl::Routine(r) => {
                    if let Some(id) = self.routine(r, owner) {
                        if !routines.contains(&id) {
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
        self.declare(id, Sym::Label);
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
                match form {
                    ast::RecordForm::Packed => {
                        self.unsupported(t.span, "a packed record (ref 3.10.7)");
                        return err;
                    }
                    ast::RecordForm::Aligned => {
                        self.unsupported(
                            t.span,
                            "an aligned record (natural alignment, ref 3.10.6)",
                        );
                        return err;
                    }
                    // ref 3.10.6 p. 3-34: unaligned records always use the
                    // word alignment rules, which are also the default.
                    ast::RecordForm::Plain | ast::RecordForm::Unaligned => {}
                }
                if self.natural && *form == ast::RecordForm::Plain {
                    self.unsupported(t.span, "a record under %natural_alignment (ref 3.15.6)");
                    return err;
                }
                let r = self.record(fields, variant.as_deref());
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
                for a in attrs {
                    match a.name.name.as_str() {
                        // ref 3.15.1 / 3.15.2: volatile and atomic restrict
                        // optimisation; they do not change layout, and this
                        // compiler does not optimise.
                        "volatile" | "atomic" if a.args.is_none() => {}
                        n => {
                            let what = format!("the `{n}` attribute (ref 3.15)");
                            self.unsupported(a.span, &what);
                        }
                    }
                }
                self.type_expr(ty)
            }
        }
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
    /// 3.10.5.5 p. 3-26).
    fn record(&mut self, fields: &[ast::FieldDecl], variant: Option<&ast::VariantPart>) -> TypeId {
        let mut seen = HashSet::new();
        let mut fixed = self.field_list(fields, &mut seen);
        let end = self.types.layout_fields(&mut fixed, 0);
        let variant = variant.map(|v| Box::new(self.variant_part(v, end, &mut seen)));
        self.new_type(TypeKind::Record(RecordType {
            fields: fixed,
            variant,
        }))
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
            }];
            arm_start = self.types.layout_fields(&mut f, start);
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
            let end = self.types.layout_fields(&mut fields, arm_start);
            let variant = arm
                .variant
                .as_deref()
                .map(|sub| Box::new(self.variant_part(sub, end, seen)));
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
        let mut params = Vec::new();
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
                        params.push(SigParam {
                            mode: *mode,
                            univ: *univ,
                            ty: t,
                        });
                    }
                }
                ast::Param::Routine { span, .. } => {
                    self.unsupported(*span, "a routine parameter (ref 5.4)");
                }
            }
        }
        let result = sig.result.as_ref().map(|r| self.type_expr(r));
        Signature { params, result }
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
                if !self.check_assignable(ty, &x, e.span) {
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
                        if let (Some(l), Some(h)) = (l, h) {
                            members.extend(l..=h);
                        }
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
                    if rec.variant.is_some() {
                        self.unsupported(*span, "initialising a variant record");
                        return;
                    }
                    let mut next = 0usize;
                    for it in items {
                        match it {
                            ast::InitItem::Value(v) => {
                                let Some(f) = rec.fields.get(next) else {
                                    self.error(*span, "too many initial values for the record");
                                    return;
                                };
                                self.init(f.ty, base + f.offset, v, out);
                                next += 1;
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
                                match rec.fields.iter().position(|f| f.name == id.name) {
                                    Some(i) => {
                                        let f = &rec.fields[i];
                                        self.init(f.ty, base + f.offset, value, out);
                                        next = i + 1;
                                    }
                                    None => self.error(id.span, format!("no field `{}`", id.name)),
                                }
                            }
                            _ => self.error(*span, "malformed record initialiser"),
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
            self.routines[p.0 as usize].nested.push(id);
        }
        id
    }

    fn routine_body(&mut self, id: RoutineId, body: &ast::RoutineBody) {
        let level = self.routines[id.0 as usize].level;
        self.ctx.push(RoutineCtx {
            id,
            level,
            frame: 0,
        });
        self.scopes.push(Scope::default());

        // Frame order (Uncertain, codegen-patterns 2): the function result
        // variable, then local copies of value parameters, then locals.
        if let Some(res) = self.routines[id.0 as usize].result.clone() {
            let offset = self.alloc_frame(res.ty);
            let name = self.routines[id.0 as usize].name.clone();
            let span = self.routines[id.0 as usize].span;
            let var = self.add_var(Var {
                name,
                ty: res.ty,
                storage: Storage::Frame { offset },
                owner: Some(id),
                init: None,
                span,
            });
            let rt = &mut self.routines[id.0 as usize];
            rt.result.as_mut().unwrap().var = Some(var);
            rt.frame_vars.push(var);
        }
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
                if !matches!(self.lookup(&name), Some(Sym::Label)) {
                    self.error(label.span(), format!("label `{name}` is not declared"));
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
                if !matches!(self.lookup(&name), Some(Sym::Label)) {
                    self.error(l.span(), format!("label `{name}` is not declared"));
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
        }
        t
    }

    fn call_stmt(&mut self, e: &ast::Expr) -> StmtKind {
        let (callee, args): (&ast::Expr, &[ast::Expr]) = match &e.kind {
            ast::ExprKind::Call { callee, args } => (callee, args),
            _ => (e, &[]),
        };
        if let ast::ExprKind::Name(id) = &callee.kind {
            match self.lookup(&id.name).cloned() {
                Some(Sym::Builtin(b @ (Builtin::Disable | Builtin::Enable))) => {
                    if !args.is_empty() {
                        self.error(e.span, format!("`{}` takes no arguments", id.name));
                    }
                    return StmtKind::Builtin {
                        builtin: b,
                        args: Vec::new(),
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
            Sym::Options(_) | Sym::Label => {
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
        Expr {
            kind: ExprKind::Var { var: v, uplevel },
            ty: var.ty,
            value: None,
            span,
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
                let (ty, offset) = (f.ty, f.offset);
                Expr {
                    kind: ExprKind::Field {
                        base: Box::new(base),
                        name: field.name.clone(),
                        offset,
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
                } else {
                    self.strings_compatible(l.ty, r.ty)
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
    /// decided as for a routine heading without options.
    fn sig_params(&mut self, sig: &Signature) -> Vec<(ParamMode, bool, TypeId, conv::ParamConv)> {
        sig.params
            .iter()
            .map(|p| {
                let c =
                    conv::param_conv(p.mode, p.univ, p.ty, false, false, &self.types, self.target);
                (p.mode, p.univ, p.ty, c)
            })
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
        for (a, (mode, univ, ty, c)) in args.iter().zip(params) {
            let x = self.expr(a);
            if !univ {
                match mode {
                    // ref 5.3.1 p. 5-6: "You must pass a variable as an
                    // argument to a variable parameter."
                    ParamMode::Var | ParamMode::Out | ParamMode::InOut => {
                        if !x.is_designator() && !self.types.is_error(x.ty) {
                            self.error(a.span, "a variable is needed for this parameter");
                        } else if !self.same_type(x.ty, ty) {
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
            let needs_temp = c.passing == Passing::ByAddress && !x.is_designator();
            out.push(Arg {
                expr: x,
                conv: c,
                needs_temp,
            });
        }
        Call { callee, args: out }
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
        let value = if ints { x.value.clone() } else { None };
        Expr {
            kind: ExprKind::Transfer(Box::new(x)),
            ty: t,
            value,
            span,
        }
    }

    fn builtin_fn(&mut self, b: Builtin, name: &str, args: &[ast::Expr], span: Span) -> Expr {
        let bi = self.types.builtins;
        let want = match b {
            Builtin::Disable | Builtin::Enable => {
                self.error(span, format!("`{name}` is a procedure"));
                return self.error_expr(span);
            }
            Builtin::Lshft | Builtin::Rshft | Builtin::Arshft | Builtin::Xor => 2,
            _ => 1,
        };
        if args.len() != want {
            self.error(span, format!("`{name}` takes {want} argument(s)"));
            return self.error_expr(span);
        }
        let xs: Vec<Expr> = args.iter().map(|a| self.expr(a)).collect();
        if xs.iter().any(|x| self.types.is_error(x.ty)) {
            return self.error_expr(span);
        }
        let ty = match b {
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
            Builtin::Disable | Builtin::Enable => unreachable!(),
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

    // -----------------------------------------------------------------
    // Compatibility (ISO 7185 6.4.5/6.4.6, which the reference assumes;
    // Domain extensions cited where used)
    // -----------------------------------------------------------------

    fn same_type(&self, a: TypeId, b: TypeId) -> bool {
        a == b || self.types.is_error(a) || self.types.is_error(b)
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

    /// `nil` and `univ_ptr` are compatible with every pointer (ref 3.13.2
    /// p. 3-50).  Two `^T` types are compatible when they point to the same
    /// type (TODO(ref-question): p. 4-144 says only "compatible pointer";
    /// name equivalence of the pointer types themselves would be stricter).
    fn pointers_compatible(&self, a: TypeId, b: TypeId) -> bool {
        use TypeKind as K;
        match (self.types.kind(a), self.types.kind(b)) {
            (K::Nil, _) | (_, K::Nil) | (K::UnivPtr, _) | (_, K::UnivPtr) => true,
            (K::Pointer { target: x }, K::Pointer { target: y }) => x == y || a == b,
            (K::RoutinePointer(x), K::RoutinePointer(y)) => a == b || x == y,
            _ => false,
        }
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
        if self.types.is_error(dst) || self.types.is_error(src) || dst == src {
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
