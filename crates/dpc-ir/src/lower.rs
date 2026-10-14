//! Lowering of the checked program (`dpc_sema::Program`) to the IR.
//!
//! Only the constructs whose code the image has shown are lowered; the
//! rest are reported as "not supported yet" at their source position
//! (never wrong code).  Each accepted construct names the image evidence
//! for the selector pattern that executes it (crate `dpc-m68k`, `isel`).

use dpc_sema::conv::{Passing, ResultReg};
use dpc_sema::{
    Builtin, Call as SCall, Callee as SCallee, Expr as SExpr, ExprKind, Program, Routine,
    RoutineId, Stmt as SStmt, StmtKind as SStmtKind, Storage, VarId,
};
use dpc_syntax::ast::{BinOp, ForDir, ParamMode, UnOp};
use dpc_syntax::{Diagnostic, Span};

use dpc_sema::types::{TypeId, TypeKind, TypeTable};

use dpc_sema::consts::ConstValue;

use crate::{
    Access, Addr, Arg, BinOp as IrBinOp, BitField, Call, Callee, CaseArm, Cond, DataArea,
    DataBaseOptions, DataInit, Expr, ForLoop, Function, InitValue, LinkArg, Module, Nesting,
    ParamCopy, PoolConst, Rel, ResultClass, ResultSpec, Scalar, SetOp, Sign, StaticLink, Stmt,
    StmtKind, UnOp as IrUnOp, ValueParam, Width,
};

type Res<T> = Result<T, Diagnostic>;

fn unsupported(span: Span, what: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(span, format!("{what} not supported yet"))
}

/// Lower `program` to IR with the default data-base policy
/// ([`DataBaseOptions`]).
pub fn lower(program: &Program) -> Result<Module, Vec<Diagnostic>> {
    lower_with(program, &DataBaseOptions::default())
}

/// Lower `program` to IR.  Every construct that cannot be lowered yet is
/// reported; the module is returned only when there is none.  `db` says
/// which routines load the data base (codegen-patterns 4).
pub fn lower_with(program: &Program, db: &DataBaseOptions) -> Result<Module, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    if program.is_program {
        let d = match program.main.as_ref().and_then(|b| b.stmts.first()) {
            Some(s) => unsupported(s.span, "code generation for a main program is"),
            None => {
                Diagnostic::error_nopos("code generation for a main program is not supported yet")
            }
        };
        errors.push(d);
    }
    let data = match data_area(program) {
        Ok(d) => d,
        Err(d) => {
            errors.extend(d);
            Vec::new()
        }
    };
    let mut functions = Vec::new();
    for &id in &program.top_routines {
        let r = program.routine(id);
        if r.external || r.body.is_none() {
            continue;
        }
        lower_family(program, id, &mut functions, &mut errors);
    }
    if errors.is_empty() {
        if let Err(d) = set_static_links(&mut functions) {
            errors.extend(d);
        }
    }
    if errors.is_empty() {
        if let Err(d) = set_loads_data_base(&mut functions, &data, db) {
            errors.extend(d);
        }
    }
    if errors.is_empty() {
        let outside_internal_calls = outside_internal_calls(program, &functions);
        Ok(Module {
            name: program.name.clone(),
            functions,
            data,
            outside_internal_calls,
        })
    } else {
        Err(errors)
    }
}

/// Lower every routine family of `program` that lowers, ignoring the
/// others: the functions, and the diagnostics of the families that did
/// not lower.  For surveys of the IR only (`dpc --dump-locals`,
/// codegen-patterns 12.5): static links and the data-base choice are not
/// decided, so the functions are not fit for code generation.
pub fn lower_each(program: &Program) -> (Vec<Function>, Vec<Diagnostic>) {
    let mut functions = Vec::new();
    let mut errors = Vec::new();
    for &id in &program.top_routines {
        let r = program.routine(id);
        if r.external || r.body.is_none() {
            continue;
        }
        lower_family(program, id, &mut functions, &mut errors);
    }
    (functions, errors)
}

/// The unit's static data areas (codegen-patterns 4): every variable with
/// [`Storage::Static`] storage, unit-level or a routine's `static` one,
/// lives in a data section of the unit at the offset `dpc-sema` gave it.
/// One area per data section, in order of first allocation
/// (`Program::data_sections`): the module heading's (ref 7.1.1, p. 7-2;
/// `.data` by default, ref 5.7.1 p. 5-24) and the ones routines name in
/// `[section(c,d)]` (ref 5.7.1: such a routine's static data goes to
/// `d`).  The image has one block per data section for such a module
/// (PROC2: 0xE7BE84 in .DATA, 0xE35030 in OS_INIT_DATA, 0xE86050 in
/// PROC2_CREATE_DAT; codegen-patterns 11).
///
/// Rejected for now: variables of an overlay section, `var (name)` with a
/// name no routine and no module heading of the unit gives as its data
/// section (ref 3.14, p. 3-52: "just like a named common block in
/// FORTRAN", shared by name between units, ref 7.2.4 p. 7-7), whose
/// addressing has not been catalogued (codegen-patterns 11: the image's
/// sections with no per-module segment, such as PROC2_$DATA 0xEA551C,
/// are reached with `movea.l #<section>,An`, PROC2_$INIT 0xE30466); and
/// the initial values [`init_value`] cannot encode.
fn data_area(p: &Program) -> Result<Vec<DataArea>, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let module_section =
        |name: &str| name == p.data_section || p.routines.iter().any(|r| r.data_section == name);
    let mut used: Vec<&str> = Vec::new();
    let mut globals: Vec<(&str, String, u32)> = Vec::new();
    let mut inits: Vec<(&str, DataInit)> = Vec::new();
    for v in &p.vars {
        let Storage::Static {
            symbol,
            global,
            section,
            offset,
        } = &v.storage
        else {
            continue;
        };
        if !module_section(section) {
            errors.push(unsupported(
                v.span,
                "variables in an overlay section (`var (name)`, ref 3.14 p. 3-52) are",
            ));
        }
        if let Err(d) = check_section_name(section, v.span) {
            errors.push(d);
        }
        if let Some(stores) = &v.init {
            for st in stores {
                match init_value(&p.types, st.ty, &st.value) {
                    Ok(value) => inits.push((
                        section.as_str(),
                        DataInit {
                            offset: offset + st.offset,
                            value,
                        },
                    )),
                    Err(what) => errors.push(unsupported(v.span, what)),
                }
            }
        }
        if !used.contains(&section.as_str()) {
            used.push(section);
        }
        if *global {
            globals.push((section, symbol.clone(), *offset));
        }
    }
    if !errors.is_empty() {
        errors.dedup();
        return Err(errors);
    }
    Ok(p.data_sections
        .iter()
        .filter(|(n, _)| used.contains(&n.as_str()))
        .map(|(n, size)| {
            let mut g: Vec<(String, u32)> = globals
                .iter()
                .filter(|(s, _, _)| s == n)
                .map(|(_, sym, o)| (sym.clone(), *o))
                .collect();
            g.sort_by_key(|&(_, o)| o);
            DataArea {
                section: n.clone(),
                size: *size,
                globals: g,
                init: inits
                    .iter()
                    .filter(|(s, _)| s == n)
                    .map(|(_, i)| i.clone())
                    .collect(),
            }
        })
        .collect())
}

/// The initial value `value` of an object of type `ty` (one store of
/// `dpc_sema::InitStore`), or what is not supported.
///
/// - Integers, subranges and enumerations (ref 3.2.2 p. 3-4; 3.7.1 p.
///   3-13: an enumeration is one word) take their type's size; a boolean
///   is 0xFF or 0 (ref 3.5.3 p. 3-11); a `char` one byte (3.6.2); `nil`
///   a zero pointer (ref 3.13.4 p. 3-51; the image's nil is 0, codegen-
///   patterns 7.1 `tst.l` against `nil`).
/// - A set is its bit string, member k in byte `size-1 - k/8`, bit `k mod
///   8` (ref 3.9.2 p. 3-15, 3.9.3 Figure 3-6 p. 3-16).
/// - A string (or one character) given to an array of `char` fills its
///   first elements only (ref 3.11.1.1 p. 3-38: "assign elements 1
///   through 21 of array s1"; the rest is not initialised, i.e. stays
///   zero here, not blank-padded as an assignment would be, ref p. 4-204).
///
/// Not supported yet: reals (no real constants, CLAUDE.md 1), a string
/// given to a `varying` string (its length word and the trailing null of
/// ref p. 4-205 are not catalogued for initialisers).
fn init_value(types: &TypeTable, ty: TypeId, value: &ConstValue) -> Result<InitValue, String> {
    let size = types.size(ty);
    let int = |v: i64| -> Result<InitValue, String> {
        if matches!(size, 1 | 2 | 4) {
            Ok(InitValue::Int { size, value: v })
        } else {
            Err(format!("an initial value of a {size}-byte scalar is"))
        }
    };
    match (types.kind(ty), value) {
        (TypeKind::Real { .. }, _) => Err("initialising a real variable is".into()),
        (TypeKind::Varying { .. }, _) => {
            Err("initialising a varying string (length word, trailing null) is".into())
        }
        (TypeKind::Set { .. }, ConstValue::Set(members)) => {
            let mut bytes = vec![0u8; size as usize];
            for &k in members {
                let (b, bit) = types.set_bit_location(ty, k as u32);
                bytes[b as usize] |= 1 << bit;
            }
            Ok(InitValue::Bytes(bytes))
        }
        (TypeKind::Array { .. }, ConstValue::Str(chars)) if chars.len() <= size as usize => {
            Ok(InitValue::Bytes(chars.clone()))
        }
        (TypeKind::Array { .. }, ConstValue::Char(c)) => Ok(InitValue::Bytes(vec![*c])),
        (TypeKind::Array { .. } | TypeKind::Record(_) | TypeKind::Set { .. }, _) => {
            Err(format!("this initial value of a {} is", types.display(ty)))
        }
        (_, ConstValue::Bool(b)) => int(if *b { 0xFF } else { 0 }),
        (_, ConstValue::Nil) => int(0),
        (_, ConstValue::Int(_) | ConstValue::Char(_) | ConstValue::Enum { .. }) => {
            int(value.ordinal().unwrap_or(0))
        }
        (_, ConstValue::Str(_) | ConstValue::Set(_)) => {
            Err(format!("this initial value of a {} is", types.display(ty)))
        }
    }
}

/// A section name the output can carry: letters, digits, `_`, `$` and
/// `.` (every section name of the SAU2 map is such: WIRED_PROC, .TEXT,
/// PROCEDURE$, PROC2_CREATE_PRO).  A quoted name may hold anything (ref
/// 3.14, p. 3-52: `"this is a section name"`); how such a name reached
/// the object is not known.
fn check_section_name(name: &str, span: Span) -> Res<()> {
    if !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '.'))
    {
        Ok(())
    } else {
        Err(unsupported(
            span,
            format!(
                "section name `{name}` (only letters, digits, `_`, `$` and `.` are \
                 supported) is"
            ),
        ))
    }
}

/// Lower top-level routine `id` and the routines nested in it (ref 2.4,
/// pp. 2-19..2-21), into `out`
/// in emission order: each nested routine before its enclosing routine,
/// in source order (codegen-patterns 6, "Order": `tools/nestsurvey.py
/// summary`, 83 of 83 nested routines lie before their parent; the 19
/// calls between nested routines of one parent all go to an earlier
/// one, as declaration order requires: PMAP_$FLUSH's 0xE1359C, then
/// 0xE1360C, which calls it).
///
/// Only one level of nesting is supported: the image's deeper routines
/// reach outer frames through chains (DIR 0xE4F034 `movea.l (A6),A0;
/// movea.l (A0),A0; movea.l (-4,A0),A2`; the level-1 routine 0xE4F3BA
/// stores its static link at `(-4,A6)` for them, 0xE4F3C2), not
/// implemented.  A nested routine declared `forward` is refused: whether
/// it is emitted at its heading or at its body is not seen.
fn lower_family(p: &Program, id: RoutineId, out: &mut Vec<Function>, errors: &mut Vec<Diagnostic>) {
    let r = p.routine(id);
    let mut uplevel: Vec<(i32, u32)> = Vec::new();
    for &n in &r.nested {
        let nr = p.routine(n);
        if let Some(&g) = nr.nested.first() {
            errors.push(unsupported(
                p.routine(g).span,
                "a routine nested in a nested routine (outer frames reached through a chain of \
                 links, DIR 0xE4F034, codegen-patterns 6) is",
            ));
            continue;
        }
        if nr.body.is_none() {
            errors.push(unsupported(nr.span, "a nested routine without a body is"));
            continue;
        }
        if nr.options.forward {
            errors.push(unsupported(
                nr.span,
                "a nested routine declared `forward` (emitted at its heading or its body: not \
                 seen) is",
            ));
            continue;
        }
        match lower_routine(p, n, &[]) {
            Ok(f) => {
                for st in &f.body {
                    stmt_enclosing(st, &mut uplevel);
                }
                out.push(f);
            }
            Err(d) => errors.extend(d),
        }
    }
    uplevel.sort_unstable();
    uplevel.dedup();
    match lower_routine(p, id, &uplevel) {
        Ok(f) => out.push(f),
        Err(d) => errors.extend(d),
    }
}

/// Decide each nested routine's [`StaticLink`] and the [`LinkArg`] of
/// every call of one (codegen-patterns 6).
///
/// The rule, from every nested routine of the image (`tools/nestsurvey.py
/// summary`; a nested routine is one that is not map-named and reads A1
/// before writing it or moves from `(A6)`): the 11 that take the static
/// link in A1 are each called from a routine other than their enclosing
/// one (4 by themselves: 0xE397D0, 0xE39BC2, 0xE4F3BA, 0xE50D5E; 7 by a
/// sibling: PMAP_$FLUSH's 0xE1359C from 0xE1360C at 0xE13718), and the 75
/// that read `(A6)` each have exactly one caller, their enclosing routine
/// (PMAP_$FLUSH's 0xE1360C; PROC2_$SUSPEND's 0xE4120C).  So the link is
/// passed exactly when the routine needs it (reaches the enclosing frame
/// or passes the link on) and some call comes from elsewhere than the
/// enclosing routine's own body; then the enclosing routine passes its own frame
/// (0xE13928 `movea.l A6,A1`) and a sibling, or the routine itself, its
/// own link (0xE13718 `movea.l A4,A1`; 0xE397D0 recursive).
///
/// Nested routines are local symbols (ref 5.5.4 p. 5-17), so two routines
/// of one name in a unit (nested in different routines) would collide:
/// refused.
fn set_static_links(fs: &mut [Function]) -> Result<(), Vec<Diagnostic>> {
    let mut errors = Vec::new();
    for (j, f) in fs.iter().enumerate() {
        if fs[..j].iter().any(|g| g.symbol == f.symbol) {
            errors.push(unsupported(
                f.span,
                format!(
                    "two routines named `{}` in one unit (nested routines are local symbols) are",
                    f.symbol
                ),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let index = |sym: &str| fs.iter().position(|g| g.symbol == sym);
    let parent_of = |j: usize| fs[j].nesting.as_ref().map(|n| n.parent.as_str());
    // other_caller[j]: some call of nested routine j comes from a routine
    // other than its enclosing one; callees[i]: the unit's routines that
    // routine i calls.
    let mut other_caller = vec![false; fs.len()];
    let mut callees: Vec<Vec<usize>> = vec![Vec::new(); fs.len()];
    for (i, f) in fs.iter().enumerate() {
        let mut calls = Vec::new();
        for st in &f.body {
            stmt_calls(st, &mut calls);
        }
        for c in calls {
            let Callee::Local(sym) = &c.callee else {
                continue;
            };
            if let Some(j) = index(sym) {
                callees[i].push(j);
                if parent_of(j).is_some_and(|p| p != f.symbol) {
                    other_caller[j] = true;
                }
            }
        }
    }
    // A nested routine needs its link when it reaches the enclosing frame
    // or calls a sibling (or itself) that takes the link from it; only
    // then is the link passed, even when a sibling calls it.  The image
    // never loads A1 before a `bsr` to a routine that does not read it
    // (`tools/nestsurvey.py`): ROUTE_$SERVICE's nested 0xE69E40 (between
    // its nested 0xE69CCC and 0xE69EC2, `link #-4`, no frame or link
    // access) is called by the sibling 0xE69EC2 at 0xE69F3C with no A1
    // load; likewise DIR's 0xE4EF42 inside 0xE4F3BA's family, called by
    // 0xE4EF7C and 0xE4F2DA.
    let mut needs: Vec<bool> = fs
        .iter()
        .map(|f| {
            let mut up = Vec::new();
            for st in &f.body {
                stmt_enclosing(st, &mut up);
            }
            f.nesting.is_some() && !up.is_empty()
        })
        .collect();
    let mut passed = vec![false; fs.len()];
    loop {
        for j in 0..fs.len() {
            passed[j] = needs[j] && other_caller[j];
        }
        let mut changed = false;
        for i in 0..fs.len() {
            if needs[i] || parent_of(i).is_none() {
                continue;
            }
            if callees[i]
                .iter()
                .any(|&j| passed[j] && parent_of(j) == parent_of(i))
            {
                needs[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // (symbol, parent, link passed) of the nested routines.
    let nested: Vec<(String, String, bool)> = fs
        .iter()
        .enumerate()
        .filter_map(|(j, f)| {
            f.nesting
                .as_ref()
                .map(|n| (f.symbol.clone(), n.parent.clone(), passed[j]))
        })
        .collect();
    for (j, f) in fs.iter_mut().enumerate() {
        if let Some(n) = &mut f.nesting {
            n.link = if passed[j] {
                StaticLink::Passed
            } else {
                StaticLink::CallerFrame
            };
        }
        let me = f.symbol.clone();
        let my_parent = f.nesting.as_ref().map(|n| n.parent.clone());
        let span = f.span;
        let mut bad = false;
        for st in &mut f.body {
            stmt_calls_mut(st, &mut |c: &mut Call| {
                let Callee::Local(sym) = &c.callee else {
                    return;
                };
                let Some((_, parent, true)) = nested.iter().find(|(s, _, _)| s == sym) else {
                    return;
                };
                c.link = if *parent == me {
                    Some(LinkArg::OwnFrame)
                } else if my_parent.as_ref() == Some(parent) {
                    Some(LinkArg::OwnLink)
                } else {
                    bad = true;
                    None
                };
            });
        }
        if bad {
            errors.push(unsupported(
                span,
                "a call of a nested routine from outside its enclosing routine and its siblings is",
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The objects of the enclosing frame that `s` reaches
/// ([`Addr::Enclosing`]), as (offset, size): a scalar access its width, a
/// block copy its length, an address taken or an indexed array 1 byte at
/// the object's start.
fn stmt_enclosing(s: &Stmt, out: &mut Vec<(i32, u32)>) {
    fn addr(a: &Addr, size: u32, out: &mut Vec<(i32, u32)>) {
        match a {
            Addr::Enclosing(o) => out.push((*o, size)),
            Addr::Frame(_) | Addr::Static(_) | Addr::Symbol { .. } | Addr::Const(_) => {}
            Addr::Volatile { addr: a, .. } => addr(a, size, out),
            Addr::Indirect { base, .. } => expr(base, out),
            Addr::Index { base, index, .. } => {
                addr(base, 1, out);
                expr(index, out);
            }
        }
    }
    fn expr(e: &Expr, out: &mut Vec<(i32, u32)>) {
        match e {
            Expr::Const { .. } => {}
            Expr::Load { addr: a, ty } => addr(a, ty.bytes(), out),
            Expr::Bits { addr: a, field, .. } => addr(a, field.unit.bytes(), out),
            Expr::AddrOf(a) => addr(a, 1, out),
            Expr::SetMember { elem, set, .. } => {
                expr(elem, out);
                expr(set, out);
            }
            Expr::SetMemberAt {
                elem,
                set: a,
                bytes,
            } => {
                expr(elem, out);
                addr(a, *bytes, out);
            }
            Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
                expr(lhs, out);
                expr(rhs, out);
            }
            Expr::Unary { operand: x, .. }
            | Expr::Convert { value: x, .. }
            | Expr::SetSr(x)
            | Expr::SetOf { elem: x, .. } => expr(x, out),
            Expr::Call { call: c, .. } => call(c, out),
            Expr::Test(c) => cond(c, out),
        }
    }
    fn call(c: &Call, out: &mut Vec<(i32, u32)>) {
        for a in &c.args {
            match a {
                Arg::Value { value, .. } | Arg::Temp { value, .. } => expr(value, out),
                Arg::Address { addr: a, .. } => addr(a, 1, out),
            }
        }
        if let Some(x) = c.pointer() {
            expr(x, out);
        }
    }
    fn cond(c: &Cond, out: &mut Vec<(i32, u32)>) {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                expr(lhs, out);
                expr(rhs, out);
            }
            Cond::Test(e) => expr(e, out),
            Cond::Not(x) => cond(x, out),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) => {
                cond(a, out);
                cond(b, out);
            }
            Cond::Block { lhs, rhs, size, .. } => {
                addr(lhs, *size, out);
                addr(rhs, *size, out);
            }
        }
    }
    match &s.kind {
        StmtKind::SetResult(e) | StmtKind::Discard(e) => expr(e, out),
        StmtKind::Store { addr: a, value } => {
            addr(a, value.ty().bytes(), out);
            expr(value, out);
        }
        StmtKind::StoreBits {
            addr: a,
            field,
            value,
            ..
        } => {
            addr(a, field.unit.bytes(), out);
            expr(value, out);
        }
        StmtKind::Copy { dst, src, bytes } => {
            addr(dst, *bytes, out);
            addr(src, *bytes, out);
        }
        StmtKind::Call(c) => call(c, out),
        StmtKind::Fill { start, value, .. } => {
            addr(start, 1, out);
            expr(value, out);
        }
        StmtKind::If {
            cond: c,
            then_,
            else_,
        } => {
            cond(c, out);
            for st in then_.iter().chain(else_) {
                stmt_enclosing(st, out);
            }
        }
        StmtKind::Repeat { body, until } => {
            for st in body {
                stmt_enclosing(st, out);
            }
            cond(until, out);
        }
        StmtKind::While { cond: c, body } => {
            cond(c, out);
            for st in body {
                stmt_enclosing(st, out);
            }
        }
        StmtKind::For(l) => {
            expr(&l.from, out);
            expr(&l.to, out);
            for st in &l.body {
                stmt_enclosing(st, out);
            }
        }
        StmtKind::Case {
            selector,
            arms,
            otherwise,
        } => {
            expr(selector, out);
            for st in arms
                .iter()
                .flat_map(|a| &a.body)
                .chain(otherwise.iter().flatten())
            {
                stmt_enclosing(st, out);
            }
        }
        StmtKind::Return
        | StmtKind::Disable
        | StmtKind::Enable
        | StmtKind::Exit
        | StmtKind::Next
        | StmtKind::Label(_)
        | StmtKind::Goto(_) => {}
    }
}

/// Whether the object of `size` bytes at frame offset `o` overlaps one of
/// `objs` ([`Function::uplevel_refs`]).
fn overlaps(objs: &[(i32, u32)], o: i32, size: u32) -> bool {
    objs.iter()
        .any(|&(p, n)| p < o + size as i32 && o < p + n as i32)
}

/// Apply `f` to every call of `s`, statement calls and function calls in
/// expressions (the mutable twin of [`stmt_calls`]).
fn stmt_calls_mut(s: &mut Stmt, f: &mut dyn FnMut(&mut Call)) {
    fn addr(a: &mut Addr, f: &mut dyn FnMut(&mut Call)) {
        match a {
            Addr::Frame(_)
            | Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::Symbol { .. }
            | Addr::Const(_) => {}
            Addr::Volatile { addr: a, .. } => addr(a, f),
            Addr::Indirect { base, .. } => expr(base, f),
            Addr::Index { base, index, .. } => {
                addr(base, f);
                expr(index, f);
            }
        }
    }
    fn expr(e: &mut Expr, f: &mut dyn FnMut(&mut Call)) {
        match e {
            Expr::Const { .. } => {}
            Expr::Load { addr: a, .. } | Expr::Bits { addr: a, .. } | Expr::AddrOf(a) => addr(a, f),
            Expr::SetMember { elem, set, .. } => {
                expr(elem, f);
                expr(set, f);
            }
            Expr::SetMemberAt { elem, set: a, .. } => {
                expr(elem, f);
                addr(a, f);
            }
            Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
                expr(lhs, f);
                expr(rhs, f);
            }
            Expr::Unary { operand: x, .. }
            | Expr::Convert { value: x, .. }
            | Expr::SetSr(x)
            | Expr::SetOf { elem: x, .. } => expr(x, f),
            Expr::Call { call: c, .. } => call(c, f),
            Expr::Test(c) => cond(c, f),
        }
    }
    fn call(c: &mut Call, f: &mut dyn FnMut(&mut Call)) {
        for a in &mut c.args {
            match a {
                Arg::Value { value, .. } | Arg::Temp { value, .. } => expr(value, f),
                Arg::Address { addr: a, .. } => addr(a, f),
            }
        }
        if let Some(x) = c.pointer_mut() {
            expr(x, f);
        }
        f(c);
    }
    fn cond(c: &mut Cond, f: &mut dyn FnMut(&mut Call)) {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                expr(lhs, f);
                expr(rhs, f);
            }
            Cond::Test(e) => expr(e, f),
            Cond::Not(x) => cond(x, f),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) => {
                cond(a, f);
                cond(b, f);
            }
            Cond::Block { lhs, rhs, .. } => {
                addr(lhs, f);
                addr(rhs, f);
            }
        }
    }
    match &mut s.kind {
        StmtKind::SetResult(e) | StmtKind::Discard(e) => expr(e, f),
        StmtKind::Store { addr: a, value } | StmtKind::StoreBits { addr: a, value, .. } => {
            addr(a, f);
            expr(value, f);
        }
        StmtKind::Copy { dst, src, .. } => {
            addr(dst, f);
            addr(src, f);
        }
        StmtKind::Call(c) => call(c, f),
        StmtKind::Fill { start, value, .. } => {
            addr(start, f);
            expr(value, f);
        }
        StmtKind::If {
            cond: c,
            then_,
            else_,
        } => {
            cond(c, f);
            for st in then_.iter_mut().chain(else_.iter_mut()) {
                stmt_calls_mut(st, f);
            }
        }
        StmtKind::Repeat { body, until } => {
            for st in body.iter_mut() {
                stmt_calls_mut(st, f);
            }
            cond(until, f);
        }
        StmtKind::While { cond: c, body } => {
            cond(c, f);
            for st in body.iter_mut() {
                stmt_calls_mut(st, f);
            }
        }
        StmtKind::For(l) => {
            expr(&mut l.from, f);
            expr(&mut l.to, f);
            for st in l.body.iter_mut() {
                stmt_calls_mut(st, f);
            }
        }
        StmtKind::Case {
            selector,
            arms,
            otherwise,
        } => {
            expr(selector, f);
            for a in arms.iter_mut() {
                for st in a.body.iter_mut() {
                    stmt_calls_mut(st, f);
                }
            }
            if let Some(o) = otherwise {
                for st in o.iter_mut() {
                    stmt_calls_mut(st, f);
                }
            }
        }
        StmtKind::Return
        | StmtKind::Disable
        | StmtKind::Enable
        | StmtKind::Exit
        | StmtKind::Next
        | StmtKind::Label(_)
        | StmtKind::Goto(_) => {}
    }
}

/// Give every expression passed by address ([`Arg::Temp`]) its frame
/// temporary, and return the frame size with them (`frame` without).
///
/// The temporaries lie below every other frame object ("locals, result,
/// then temporaries", codegen-patterns 2: TONE_$ENABLE 0xE1ACE8 `link
/// #-8`, its status local at -4 and the temporary at -8).  A call's
/// temporaries are allocated in push order, downward (SIO_$INIT
/// 0xE32C28/0xE32C42 `-8` then `-12`; PCHIST_$CNTL 0xE5CF0A/0xE5CF16
/// `-40` then `-44`), each word aligned, a longword longword aligned (the
/// frame alignment rule, codegen-patterns 2); calls reuse the same
/// slots (GET_BUILD_TIME 0xE38052: `(-6,A6)` for five calls;
/// `tools/pcsurvey.py temps`).  The temporaries of a call made inside
/// another call's arguments go below the outer call's.  Uncertain: where
/// temporaries go when the frame has a gap; whether a call nested in the
/// arguments reuses the outer call's slots.
fn assign_temps(body: &mut [Stmt], frame: u32) -> u32 {
    fn call(c: &mut Call, mut size: u32, max: &mut u32) {
        for a in c.args.iter_mut().rev() {
            if let Arg::Temp { value, offset, .. } = a {
                let n = value.ty().bytes();
                let align = if n >= 4 { 4 } else { 2 };
                size = (size + n).div_ceil(align) * align;
                *offset = -(size as i32);
                *max = (*max).max(size);
            }
        }
        for a in c.args.iter_mut() {
            match a {
                Arg::Value { value, .. } | Arg::Temp { value, .. } => expr(value, size, max),
                Arg::Address { addr: a, .. } => addr(a, size, max),
            }
        }
        if let Some(x) = c.pointer_mut() {
            expr(x, size, max);
        }
    }
    fn addr(a: &mut Addr, size: u32, max: &mut u32) {
        match a {
            Addr::Volatile { addr: a, .. } => addr(a, size, max),
            Addr::Indirect { base, .. } => expr(base, size, max),
            Addr::Index { base, index, .. } => {
                addr(base, size, max);
                expr(index, size, max);
            }
            Addr::Frame(_)
            | Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::Symbol { .. }
            | Addr::Const(_) => {}
        }
    }
    fn expr(e: &mut Expr, size: u32, max: &mut u32) {
        match e {
            Expr::Const { .. } => {}
            Expr::Load { addr: a, .. } | Expr::Bits { addr: a, .. } | Expr::AddrOf(a) => {
                addr(a, size, max)
            }
            Expr::SetMember {
                elem: x, set: y, ..
            }
            | Expr::Binary { lhs: x, rhs: y, .. }
            | Expr::SetOp { lhs: x, rhs: y, .. } => {
                expr(x, size, max);
                expr(y, size, max);
            }
            Expr::SetMemberAt { elem, set: a, .. } => {
                expr(elem, size, max);
                addr(a, size, max);
            }
            Expr::Unary { operand: x, .. }
            | Expr::Convert { value: x, .. }
            | Expr::SetSr(x)
            | Expr::SetOf { elem: x, .. } => expr(x, size, max),
            Expr::Call { call: c, .. } => call(c, size, max),
            Expr::Test(c) => cond(c, size, max),
        }
    }
    fn cond(c: &mut Cond, size: u32, max: &mut u32) {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                expr(lhs, size, max);
                expr(rhs, size, max);
            }
            Cond::Test(e) => expr(e, size, max),
            Cond::Not(x) => cond(x, size, max),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) => {
                cond(a, size, max);
                cond(b, size, max);
            }
            Cond::Block { lhs, rhs, .. } => {
                addr(lhs, size, max);
                addr(rhs, size, max);
            }
        }
    }
    let mut max = frame;
    for s in body.iter_mut() {
        // `stmt_calls_mut` visits a call nested in another's arguments
        // before the outer one, which then places it again, below its own
        // temporaries: the last placement stands.
        stmt_calls_mut(s, &mut |c| call(c, frame, &mut max));
    }
    max
}

/// Lower one routine.  `uplevel`: the objects of its frame that its nested
/// routines reach ([`Function::uplevel_refs`]).
fn lower_routine(
    p: &Program,
    id: RoutineId,
    uplevel: &[(i32, u32)],
) -> Result<Function, Vec<Diagnostic>> {
    let r = p.routine(id);
    let mut errors = Vec::new();
    for name in [&r.code_section, &r.data_section] {
        if let Err(d) = check_section_name(name, r.span) {
            errors.push(d);
        }
    }
    // The data section whose block the routine's data base points at: a
    // nested routine inherits its enclosing routine's base (codegen-
    // patterns 4), so its top-level routine's.
    let mut top = r;
    while let Some(pid) = top.parent {
        top = p.routine(pid);
    }
    if let Some(pid) = r.parent {
        // ref 5.7.1, p. 5-25: nested routines inherit their outer
        // routine's sections unless they name their own.  One in another
        // code section would be reached by `jsr` with its static link,
        // which the image does not show.
        if p.routine(pid).code_section != r.code_section {
            errors.push(unsupported(
                r.span,
                "a nested routine in another code section than its enclosing routine is",
            ));
        }
    }
    let data_section = top.data_section.clone();
    for &(v, span) in &r.static_refs {
        if let Storage::Static { section, .. } = &p.var(v).storage {
            if *section != data_section {
                // codegen-patterns 4, 11: such data is reached from its
                // block's absolute address (PROC1_$INIT 0xE2F95C in
                // OS_INIT_PROC: `movea.l #0xE254E8,A0`, PROC1_'s WIRED_DATA
                // block, then `(0xc40,A0)`, ...; PROC2_$INIT 0xE3048E:
                // `movea.l #0xE7BE84,A1`, PROC2's .DATA block); when that
                // register is loaded and reloaded is not settled.
                errors.push(unsupported(
                    span,
                    format!(
                        "a static variable of data section `{section}` in a routine whose \
                         data section is `{data_section}` is"
                    ),
                ));
            }
        }
    }
    let o = &r.options;
    for (on, name) in [
        (o.abnormal, "options(abnormal)"),
        (o.noreturn, "options(noreturn)"),
        (o.nosave, "options(nosave)"),
    ] {
        if on {
            errors.push(unsupported(r.span, format!("{name} is")));
        }
    }
    let result = match &r.result {
        None => None,
        Some(res) => match result_spec(p, r, res) {
            Ok(s) => Some(s),
            Err(d) => {
                errors.push(d);
                None
            }
        },
    };
    // A function whose result type was just reported is not lowered
    // further: its result assignment would only repeat that error.
    let mut body = if r.result.is_some() && result.is_none() {
        Vec::new()
    } else {
        match lower_body(p, id, r, result, uplevel) {
            Ok(b) => b,
            Err(d) => {
                errors.push(d);
                Vec::new()
            }
        }
    };
    let frame_size = assign_temps(&mut body, r.frame_size);
    set_call_sections(p, &r.code_section, &mut body);
    let param_copies = param_copies(p, r).unwrap_or_else(|d| {
        errors.push(d);
        Vec::new()
    });
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Function {
        symbol: r.symbol.clone(),
        global: r.global,
        code_section: r.code_section.clone(),
        data_section,
        frame_size,
        result,
        result_slot: r.result.as_ref().and_then(|res| res.var).and_then(|v| {
            match p.var(v).storage {
                dpc_sema::Storage::Frame { offset } => Some(offset),
                _ => None,
            }
        }),
        // Decided once every routine is lowered (`set_loads_data_base`).
        loads_data_base: false,
        data_absolute: false,
        value_params: value_params(p, r),
        address_params: r
            .params
            .iter()
            .filter(|prm| prm.passing() == Passing::ByAddress)
            .map(|prm| prm.slot_offset)
            .collect(),
        scalar_locals: scalar_locals(p, r, uplevel),
        nesting: r.parent.map(|pid| Nesting {
            parent: p.routine(pid).symbol.clone(),
            // Decided once every routine is lowered (`set_static_links`).
            link: StaticLink::CallerFrame,
        }),
        uplevel_refs: uplevel.to_vec(),
        param_copies,
        body,
        span: r.span,
    })
}

/// Turn every call in `body` of a routine of this unit in another code
/// section than `caller_section` into [`Callee::Section`] (`jsr abs.l`):
/// `bsr` never crosses a map segment in the image, and all 191 calls
/// between the segments of one module are `jsr abs.l` (codegen-patterns
/// 1.6, 11; PROC1_$INIT 0xE2F9EC, AST_$ACTIVATE_ASTE_CANNED 0xE2F350).
/// Statement calls and calls in expressions alike (bead
/// domain-pascal-86o).
fn set_call_sections(p: &Program, caller_section: &str, body: &mut [Stmt]) {
    for s in body.iter_mut() {
        stmt_calls_mut(s, &mut |c| {
            if let Callee::Local(sym) = &c.callee {
                let other = p.routines.iter().any(|r| {
                    &r.symbol == sym
                        && !r.external
                        && r.body.is_some()
                        && r.code_section != caller_section
                });
                if other {
                    c.callee = Callee::Section(sym.clone());
                }
            }
        });
    }
}

/// Decide [`Function::loads_data_base`] and [`Function::data_absolute`]
/// for every routine of the unit (codegen-patterns 4; on m68k `lea
/// <area>.l,A5` after saving A5).
///
/// Survey of the image (`tools/a5survey.py`; compiled code only), where a
/// routine "uses" the data when it has an `(..,A5..)` operand, directly or
/// through a module-local callee that does not load A5 itself (a call to
/// a routine that loads A5 itself is not a use):
///
/// - An exported routine that uses its unit's data loads A5 (628 `S`
///   routines, e.g. PROC1_$GET_LOADAV 0xE14BC0); through a callee:
///   LOG_$READ 0xE17806, whose worker 0xE17778 reads `(0x14,A5)`.
/// - Module-local routines inherit A5 (0xE17778); `internal` routines
///   (ref 5.5.4) are taken to be such workers.
/// - An exported routine that does not use the data: in "used mode" it
///   does not load A5 (196 `n`, PROC1_$TST_LOCK 0xE148CA), in "always
///   mode" it does (213 `s`, SMD_$BUSY_WAIT 0xE1D8A4).  The mode runs
///   over stretches of a segment and is not recorded in the source, so
///   it is an option ([`DataBaseOptions`]): used mode by default.
/// - A bare-`rts` routine never loads A5 (SMD_$PUTC 0xE1D89C).
///
/// Errors: a name that is not a routine of the unit; always mode in a
/// unit without static data (A5 would point at a 4-byte block with A5 at
/// its end, which is not covered); a routine in both the `always` and
/// the `absolute` list; an empty procedure in either; an `internal`
/// routine in the `absolute` list (not seen); an absolute routine that
/// does not address the data, or reaches it through a callee that
/// inherits A5 (which would see the caller's A5).
fn set_loads_data_base(
    fs: &mut [Function],
    data: &[DataArea],
    db: &DataBaseOptions,
) -> Result<(), Vec<Diagnostic>> {
    let fsr: &[Function] = fs;
    let mut errors = Vec::new();
    let find = |name: &str| fsr.iter().position(|f| f.symbol.eq_ignore_ascii_case(name));
    // Whether routine `i`'s data section has a block of this unit.
    let has_data = |i: usize| data.iter().any(|a| a.section == fsr[i].data_section);
    let mut always = vec![false; fsr.len()];
    let mut absolute = vec![false; fsr.len()];
    for (opt, names, set) in [
        ("--a5-always", &db.always, &mut always),
        ("--a5-absolute", &db.absolute, &mut absolute),
    ] {
        for name in names {
            match find(name) {
                Some(i) => set[i] = true,
                None => errors.push(Diagnostic::error_nopos(format!(
                    "{opt}: no routine `{name}` in this unit"
                ))),
            }
        }
    }
    // "Always mode" from each named routine on, in the unit's order within
    // its code section: one switch per map segment (codegen-patterns 4),
    // and a code section of a unit is one segment.
    let mut always_mode = vec![false; fsr.len()];
    let mut switched: Vec<&str> = Vec::new();
    for name in &db.always_from {
        let Some(k) = find(name) else {
            errors.push(Diagnostic::error_nopos(format!(
                "--a5-always-from: no routine `{name}` in this unit"
            )));
            continue;
        };
        let sec = fsr[k].code_section.as_str();
        if switched.contains(&sec) {
            errors.push(Diagnostic::error_nopos(format!(
                "--a5-always-from: two routines of code section `{sec}`; each code section \
                 switches at most once"
            )));
        }
        switched.push(sec);
        if !has_data(k) {
            // In the image such a unit's A5 is the end of a 4-byte block
            // no code reads (OS_INIT_DATA AST_ 0xE35000 -> 0xE35004); dpc
            // does not lay out that block.
            errors.push(Diagnostic::error_nopos(format!(
                "--a5-always-from: the unit has no static data in the data section of `{}`, so \
                 there is no data area to load (a unit without data that loads A5 points it past a 4-byte \
                 block, codegen-patterns 4; not supported)",
                fsr[k].symbol
            )));
        }
        for i in k..fsr.len() {
            if fsr[i].code_section == sec {
                always_mode[i] = true;
            }
        }
    }
    for (i, f) in fsr.iter().enumerate() {
        if always[i] && !has_data(i) {
            errors.push(Diagnostic::error_nopos(format!(
                "--a5-always: the unit has no static data in the data section of `{}`, so there \
                 is no data area to load (a unit without data that loads A5 points it past a 4-byte block, \
                 codegen-patterns 4; not supported)",
                f.symbol
            )));
        }
        if always[i] && absolute[i] {
            errors.push(Diagnostic::error_nopos(format!(
                "`{}` is in both --a5-always and --a5-absolute",
                f.symbol
            )));
        }
        if (always[i] || absolute[i]) && f.is_empty_procedure() {
            errors.push(Diagnostic::error(
                f.span,
                format!(
                    "{}: `{}` is an empty procedure, compiled to a bare `rts`, which never loads \
                     or addresses the data base (codegen-patterns 2)",
                    if always[i] {
                        "--a5-always"
                    } else {
                        "--a5-absolute"
                    },
                    f.symbol
                ),
            ));
        }
        if absolute[i] && !f.global {
            errors.push(Diagnostic::error(
                f.span,
                format!(
                    "--a5-absolute: `{}` is an internal routine (it inherits the data base; \
                     absolute addressing is seen only in exported routines, codegen-patterns 4)",
                    f.symbol
                ),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    // Which routines use the data, directly or through internal callees
    // that inherit it; a fixpoint over the call graph.
    let direct: Vec<bool> = fs
        .iter()
        .map(|f| f.body.iter().any(stmt_uses_static))
        .collect();
    let callees: Vec<Vec<String>> = fs
        .iter()
        .map(|f| {
            let mut c = Vec::new();
            for st in &f.body {
                local_callees(st, &mut c);
            }
            c
        })
        .collect();
    let index = |sym: &str| fsr.iter().position(|g| g.symbol == sym);
    // An `internal` top-level routine that a routine of another data
    // section calls cannot inherit the caller's base, which points at
    // another block (or at none): it sets up its own like an exported
    // routine.  The image's unnamed routines that load A5 are called with
    // `jsr abs.l` from another code section of their module (21 of 26,
    // codegen-patterns 4: 0xE14B12 in WIRED_PROC, from PROC1_$INIT
    // 0xE2F9EC in OS_INIT_PROC, loads PROC1_'s WIRED_DATA block 0xE254E8;
    // 0xE0250C, from AST_$ACTIVATE_ASTE_CANNED, loads AST_'s 0xE1DC80).
    let mut entry = vec![false; fsr.len()];
    for (i, cs) in callees.iter().enumerate() {
        for sym in cs {
            if let Some(j) = index(sym) {
                if fsr[j].nesting.is_none() && fsr[j].data_section != fsr[i].data_section {
                    entry[j] = true;
                }
            }
        }
    }
    // A callee inherits the data base unless it is exported, in the
    // `always` list or such an entry (it loads its own; a call to it is
    // not a use).
    let inherits = |j: usize| !fsr[j].global && !always[j] && !entry[j];
    let mut uses = direct.clone();
    loop {
        let mut changed = false;
        for i in 0..fsr.len() {
            if uses[i] {
                continue;
            }
            let through = callees[i]
                .iter()
                .any(|sym| index(sym).is_some_and(|j| inherits(j) && uses[j]));
            if through {
                uses[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for i in 0..fsr.len() {
        if !absolute[i] {
            continue;
        }
        let f = &fsr[i];
        let through = callees[i]
            .iter()
            .any(|sym| index(sym).is_some_and(|j| inherits(j) && uses[j]));
        if through {
            errors.push(Diagnostic::error(
                f.span,
                format!(
                    "--a5-absolute: `{}` calls an internal routine that uses the unit's data \
                     through the inherited data base, which it would not have loaded",
                    f.symbol
                ),
            ));
        } else if !direct[i] {
            errors.push(Diagnostic::error(
                f.span,
                format!(
                    "--a5-absolute: `{}` does not address the unit's static data",
                    f.symbol
                ),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let has: Vec<bool> = (0..fsr.len()).map(has_data).collect();
    for (i, f) in fs.iter_mut().enumerate() {
        f.data_absolute = absolute[i];
        f.loads_data_base = if f.is_empty_procedure() || absolute[i] {
            false
        } else if always[i] {
            true
        } else if !f.global && !entry[i] {
            false
        } else {
            uses[i] || (always_mode[i] && has[i])
        };
    }
    Ok(())
}

/// [`Module::outside_internal_calls`]: the calls of top-level `internal`
/// routines with no body in the unit (ref 5.2, p. 5-4: "Internal routines
/// can be called only within the same compilation unit in which they are
/// defined"), the first per symbol, with the innermost statement that
/// holds it.
fn outside_internal_calls(p: &Program, functions: &[Function]) -> Vec<(String, Span)> {
    let names: Vec<&str> = p
        .routines
        .iter()
        .filter(|r| r.parent.is_none() && r.options.internal && r.body.is_none())
        .map(|r| r.symbol.as_str())
        .collect();
    fn walk(body: &[Stmt], names: &[&str], out: &mut Vec<(String, Span)>) {
        for s in body {
            let mut all = Vec::new();
            stmt_calls(s, &mut all);
            let mut inner = Vec::new();
            for b in s.kind.bodies() {
                for st in b {
                    stmt_calls(st, &mut inner);
                }
            }
            for c in all {
                if inner.iter().any(|i| std::ptr::eq(*i, c)) {
                    continue;
                }
                if let Callee::External(sym) = &c.callee {
                    if names.contains(&sym.as_str()) && !out.iter().any(|(n, _)| n == sym) {
                        out.push((sym.clone(), s.span));
                    }
                }
            }
            for b in s.kind.bodies() {
                walk(b, names, out);
            }
        }
    }
    let mut out = Vec::new();
    if !names.is_empty() {
        for f in functions {
            walk(&f.body, &names, &mut out);
        }
    }
    out
}

/// The symbols of this unit's routines that `s` calls ([`Callee::Local`]),
/// as statements or inside expressions.
fn local_callees(s: &Stmt, out: &mut Vec<String>) {
    let mut calls = Vec::new();
    stmt_calls(s, &mut calls);
    for c in calls {
        if let Callee::Local(sym) | Callee::Section(sym) = &c.callee {
            out.push(sym.clone());
        }
    }
}

/// Every call `s` makes, statement calls and function calls in
/// expressions.
pub(crate) fn stmt_calls<'a>(s: &'a Stmt, out: &mut Vec<&'a Call>) {
    let e = |x: &'a Expr, out: &mut Vec<&'a Call>| expr_calls(x, out);
    match &s.kind {
        StmtKind::SetResult(x) => e(x, out),
        StmtKind::Store { addr, value } | StmtKind::StoreBits { addr, value, .. } => {
            addr_calls(addr, out);
            e(value, out);
        }
        StmtKind::Copy { dst, src, .. } => {
            addr_calls(dst, out);
            addr_calls(src, out);
        }
        StmtKind::Call(c) => {
            out.push(c);
            call_arg_calls(c, out);
        }
        StmtKind::Fill { start, value, .. } => {
            addr_calls(start, out);
            e(value, out);
        }
        StmtKind::If { cond, then_, else_ } => {
            cond_calls(cond, out);
            for st in then_.iter().chain(else_) {
                stmt_calls(st, out);
            }
        }
        StmtKind::Repeat { body, until } => {
            for st in body {
                stmt_calls(st, out);
            }
            cond_calls(until, out);
        }
        StmtKind::While { cond, body } => {
            cond_calls(cond, out);
            for st in body {
                stmt_calls(st, out);
            }
        }
        StmtKind::For(l) => {
            e(&l.from, out);
            e(&l.to, out);
            for st in &l.body {
                stmt_calls(st, out);
            }
        }
        StmtKind::Case {
            selector,
            arms,
            otherwise,
        } => {
            e(selector, out);
            for st in arms
                .iter()
                .flat_map(|a| &a.body)
                .chain(otherwise.iter().flatten())
            {
                stmt_calls(st, out);
            }
        }
        StmtKind::Discard(x) => e(x, out),
        StmtKind::Return
        | StmtKind::Disable
        | StmtKind::Enable
        | StmtKind::Exit
        | StmtKind::Next
        | StmtKind::Label(_)
        | StmtKind::Goto(_) => {}
    }
}

fn call_arg_calls<'a>(c: &'a Call, out: &mut Vec<&'a Call>) {
    for a in &c.args {
        match a {
            Arg::Value { value, .. } | Arg::Temp { value, .. } => expr_calls(value, out),
            Arg::Address { addr, .. } => addr_calls(addr, out),
        }
    }
    if let Some(x) = c.pointer() {
        expr_calls(x, out);
    }
}

fn cond_calls<'a>(c: &'a Cond, out: &mut Vec<&'a Call>) {
    match c {
        Cond::Compare { lhs, rhs, .. } => {
            expr_calls(lhs, out);
            expr_calls(rhs, out);
        }
        Cond::Test(e) => expr_calls(e, out),
        Cond::Not(x) => cond_calls(x, out),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) => {
            cond_calls(a, out);
            cond_calls(b, out);
        }
        Cond::Block { lhs, rhs, .. } => {
            addr_calls(lhs, out);
            addr_calls(rhs, out);
        }
    }
}

fn expr_calls<'a>(e: &'a Expr, out: &mut Vec<&'a Call>) {
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
            addr_calls(addr, out)
        }
        Expr::SetMember { elem, set, .. } => {
            expr_calls(elem, out);
            expr_calls(set, out);
        }
        Expr::SetMemberAt { elem, set, .. } => {
            expr_calls(elem, out);
            addr_calls(set, out);
        }
        Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
            expr_calls(lhs, out);
            expr_calls(rhs, out);
        }
        Expr::Unary { operand: x, .. }
        | Expr::Convert { value: x, .. }
        | Expr::SetSr(x)
        | Expr::SetOf { elem: x, .. } => expr_calls(x, out),
        Expr::Call { call, .. } => {
            out.push(call);
            call_arg_calls(call, out);
        }
        Expr::Test(c) => cond_calls(c, out),
    }
}

fn addr_calls<'a>(a: &'a Addr, out: &mut Vec<&'a Call>) {
    match a {
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::Static(_)
        | Addr::Symbol { .. }
        | Addr::Const(_) => {}
        Addr::Volatile { addr, .. } => addr_calls(addr, out),
        Addr::Indirect { base, .. } => expr_calls(base, out),
        Addr::Index { base, index, .. } => {
            addr_calls(base, out);
            expr_calls(index, out);
        }
    }
}

fn stmt_uses_static(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::SetResult(e) => expr_uses_static(e),
        StmtKind::Store { addr, value } | StmtKind::StoreBits { addr, value, .. } => {
            addr_uses_static(addr) || expr_uses_static(value)
        }
        StmtKind::Copy { dst, src, .. } => addr_uses_static(dst) || addr_uses_static(src),
        StmtKind::Call(c) => call_uses_static(c),
        StmtKind::Fill { start, value, .. } => addr_uses_static(start) || expr_uses_static(value),
        StmtKind::If { cond, then_, else_ } => {
            cond_uses_static(cond) || then_.iter().chain(else_).any(stmt_uses_static)
        }
        StmtKind::Repeat { body, until } => {
            body.iter().any(stmt_uses_static) || cond_uses_static(until)
        }
        StmtKind::While { cond, body } => {
            cond_uses_static(cond) || body.iter().any(stmt_uses_static)
        }
        StmtKind::For(l) => {
            expr_uses_static(&l.from)
                || expr_uses_static(&l.to)
                || l.body.iter().any(stmt_uses_static)
        }
        StmtKind::Case {
            selector,
            arms,
            otherwise,
        } => {
            expr_uses_static(selector)
                || arms
                    .iter()
                    .flat_map(|a| &a.body)
                    .chain(otherwise.iter().flatten())
                    .any(stmt_uses_static)
        }
        StmtKind::Discard(e) => expr_uses_static(e),
        StmtKind::Return
        | StmtKind::Disable
        | StmtKind::Enable
        | StmtKind::Exit
        | StmtKind::Next
        | StmtKind::Label(_)
        | StmtKind::Goto(_) => false,
    }
}

fn cond_uses_static(c: &Cond) -> bool {
    match c {
        Cond::Compare { lhs, rhs, .. } => expr_uses_static(lhs) || expr_uses_static(rhs),
        Cond::Test(e) => expr_uses_static(e),
        Cond::Not(x) => cond_uses_static(x),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) => cond_uses_static(a) || cond_uses_static(b),
        Cond::Block { lhs, rhs, .. } => addr_uses_static(lhs) || addr_uses_static(rhs),
    }
}

/// The pointer of a call through a routine pointer is a use when it is
/// read off A5 (RING_INTERRUPT 0xE0AB1C `movea.l (A5),A0`); the call
/// itself is not (the callee is a top-level routine and sets its own A5,
/// codegen-patterns 4).
fn call_uses_static(c: &Call) -> bool {
    c.args.iter().any(|a| match a {
        Arg::Value { value, .. } | Arg::Temp { value, .. } => expr_uses_static(value),
        Arg::Address { addr, .. } => addr_uses_static(addr),
    }) || c.pointer().is_some_and(expr_uses_static)
}

fn expr_uses_static(e: &Expr) -> bool {
    match e {
        Expr::Const { .. } => false,
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
            addr_uses_static(addr)
        }
        Expr::SetMember { elem, set, .. } => expr_uses_static(elem) || expr_uses_static(set),
        Expr::SetMemberAt { elem, set, .. } => expr_uses_static(elem) || addr_uses_static(set),
        Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
            expr_uses_static(lhs) || expr_uses_static(rhs)
        }
        Expr::Unary { operand: x, .. }
        | Expr::Convert { value: x, .. }
        | Expr::SetSr(x)
        | Expr::SetOf { elem: x, .. } => expr_uses_static(x),
        Expr::Call { call, .. } => call_uses_static(call),
        Expr::Test(c) => cond_uses_static(c),
    }
}

fn addr_uses_static(a: &Addr) -> bool {
    match a {
        Addr::Static(_) => true,
        Addr::Frame(_) | Addr::Enclosing(_) | Addr::Symbol { .. } | Addr::Const(_) => false,
        Addr::Volatile { addr, .. } => addr_uses_static(addr),
        Addr::Indirect { base, .. } => expr_uses_static(base),
        Addr::Index { base, index, .. } => addr_uses_static(base) || expr_uses_static(index),
    }
}

/// The routine's own scalar local variables ([`Function::scalar_locals`]):
/// the frame variables other than the function result variable (parameter
/// copies are not frame variables), of 1, 2 or 4 bytes, of an ordinal,
/// set or pointer type.
///
/// Not a record or an array, even one of 4 bytes or less used only as a
/// whole: the image keeps `status_$t` locals (a packed record,
/// base.ins.pas) in the frame in routines with registers to spare
/// (MST_$ALLOC_ASID 0xE42DB2 and FLP_$CINIT 0xE3E0D6, `status` at
/// `(-4,A6)`, stored and tested there with three and two of D2-D7 saved;
/// `tools/localsurvey.py c`, bead domain-pascal-iyd).
///
/// Not the locals its nested routines reach (`uplevel`): they read and
/// write them through the frame (ref 2.4), which the selector's per-body
/// analysis (codegen-patterns 12.5) does not see.  The image keeps such
/// locals in memory: PMAP_$FLUSH's word at `(-0x50,A6)` (0xE13784 `clr.w`,
/// 0xE13938 `addq.w #1`, 0xE1398C `tst.w`), which its nested 0xE1360C reads
/// as `(-0x50,A4)` (0xE1362E), while its boolean in D3 (0xE137A8 `sne D3`,
/// read at 0xE138F8 and 0xE139C0), which no nested routine reads, is held
/// in a register (bead domain-pascal-thc).
fn scalar_locals(p: &Program, r: &Routine, uplevel: &[(i32, u32)]) -> Vec<ValueParam> {
    let result = r.result.as_ref().and_then(|res| res.var);
    r.frame_vars
        .iter()
        .filter(|&&v| Some(v) != result)
        .filter_map(|&v| {
            let var = p.var(v);
            let dpc_sema::Storage::Frame { offset } = var.storage else {
                return None;
            };
            let width = Width::of_size(p.types.size(var.ty))?;
            let mut t = var.ty;
            while let dpc_sema::types::TypeKind::Subrange { base, .. } = p.types.kind(t) {
                t = *base;
            }
            use dpc_sema::types::TypeKind as K;
            let scalar = matches!(
                p.types.kind(t),
                K::Integer16
                    | K::Integer32
                    | K::Boolean
                    | K::Char
                    | K::Enum { .. }
                    | K::Set { .. }
                    | K::Pointer { .. }
                    | K::UnivPtr
                    | K::RoutinePointer(_)
            );
            if !scalar || overlaps(uplevel, offset, width.bytes()) {
                return None;
            }
            // A volatile local is referenced in its frame slot as written
            // (ref 3.15.1, p. 3-56: `i: [volatile] integer`), never held
            // in a register.
            if p.types.has_volatile_part(var.ty) {
                return None;
            }
            Some(ValueParam {
                offset,
                ty: Scalar {
                    width,
                    sign: sign_of(&p.types, var.ty),
                },
            })
        })
        .collect()
}

/// The value parameters the callee copies on entry, in parameter order
/// (ref 5.2 Table 5-1, p. 5-5, "no mode", default convention; 5.2 p. 5-4
/// "by making a local copy of the argument").  `dpc-sema` gave each copy
/// its frame slot (`Param::copy_offset`); the body refers to the copy
/// ([`var_addr`]).
///
/// A record or array with volatile or device parts is refused, as its
/// assignment is ([`copy`]): the copy moves longwords, merging device
/// references (ref 3.15.3 p. 3-57), and its shape for such objects is
/// not seen.
fn param_copies(p: &Program, r: &Routine) -> Res<Vec<ParamCopy>> {
    let mut out = Vec::new();
    for prm in &r.params {
        let Some(copy) = prm.copy_offset else {
            continue;
        };
        if p.types.has_volatile_part(prm.ty)
            && matches!(
                p.types.kind(prm.ty),
                TypeKind::Record(_) | TypeKind::Array { .. }
            )
        {
            return Err(unsupported(
                p.var(prm.var).span,
                "copying a value parameter that is a record or array with volatile or device \
                 parts (ref 3.15.1-3.15.3) is",
            ));
        }
        out.push(ParamCopy {
            slot: prm.slot_offset,
            copy,
            bytes: p.types.size(prm.ty),
            span: p.var(prm.var).span,
        });
    }
    Ok(out)
}

/// The parameters passed by value, in parameter order (ref 5.2 Table 5-1,
/// p. 5-5).  Only scalar-sized ones: a by-value parameter of another size
/// cannot be read as one value anyway.
fn value_params(p: &Program, r: &Routine) -> Vec<ValueParam> {
    r.params
        .iter()
        .filter(|prm| prm.passing() == Passing::ByValue && !prm.conv.local_copy)
        .filter_map(|prm| {
            Width::of_size(p.types.size(prm.ty)).map(|width| ValueParam {
                offset: prm.value_offset,
                ty: Scalar {
                    width,
                    sign: sign_of(&p.types, prm.ty),
                },
            })
        })
        .collect()
}

fn result_spec(p: &Program, r: &Routine, res: &dpc_sema::FnResult) -> Res<ResultSpec> {
    let class = match res.reg {
        ResultReg::Data => ResultClass::Data,
        // codegen-patterns 3, "Pointer results": in A0 by default (ref
        // 5.5.10, p. 5-20); which forms are compiled is the selector's
        // business.
        ResultReg::Address => ResultClass::Address,
        // `d0_return` (ref 5.5.10, p. 5-20: "Puts the value of the
        // returned pointer in A0 and also in data register D0"): no
        // compiled routine of the image is known to have it, so how the
        // value reaches both registers is not seen.
        ResultReg::AddressAndData => {
            return Err(unsupported(
                r.span,
                "a pointer result in A0 and D0 (`d0_return`, `c_param`) is",
            ))
        }
    };
    let size = p.types.size(res.ty);
    let width = Width::of_size(size)
        .ok_or_else(|| unsupported(r.span, format!("a {size}-byte function result is")))?;
    Ok(ResultSpec {
        class,
        ty: Scalar {
            width,
            sign: sign_of(&p.types, res.ty),
        },
    })
}

/// The statements of `r`'s body.
fn lower_body(
    p: &Program,
    id: RoutineId,
    r: &Routine,
    result: Option<ResultSpec>,
    uplevel: &[(i32, u32)],
) -> Res<Vec<Stmt>> {
    let block = r.body.as_ref().expect("routine with a body");
    let labels = routine_labels(&block.stmts)?;
    let cx = Cx {
        p,
        id,
        r,
        result,
        labels,
    };
    let out = cx.seq(&block.stmts)?;
    check_uplevel_indices(&out, uplevel)?;
    check_for_indices(&out)?;
    if r.result.is_some() {
        // ISO requires an assignment to the function variable (the
        // reference's error "No assignment to function variable"); a body
        // with none is not compiled.
        let mut assigned = false;
        crate::visit_stmts(&out, &mut |s| {
            assigned |= matches!(s.kind, StmtKind::SetResult(_));
        });
        if !assigned {
            return Err(unsupported(
                r.span,
                "a function with no assignment to its result is",
            ));
        }
    }
    Ok(out)
}

/// No `for` loop of `body` has an index that a nested routine reaches
/// (bead domain-pascal-thc).  The selector keeps an index in a register
/// for its loop (codegen-patterns 7.4), where a nested routine called from
/// the body would not see it.  The image keeps such an index in the frame:
/// RINGLOG_$CNTL 0xE72226, whose nested 0xE721CC runs its own loop over
/// the same variable through the static link (0xE721F0 `move.w
/// #1,(-2,A2)`, 0xE7220A `addq.w #1,(-2,A2)`), steps its loop at
/// 0xE72296..0xE722B0 in memory (`clr.w (-2,A6)`, `muls.w (-2,A6),D1`,
/// `addq.w #1,(-2,A6)`, beside the `dbf` count in D0): that form is not
/// implemented.
fn check_uplevel_indices(body: &[Stmt], uplevel: &[(i32, u32)]) -> Res<()> {
    if uplevel.is_empty() {
        return Ok(());
    }
    for st in body {
        let inner: Vec<&[Stmt]> = match &st.kind {
            StmtKind::For(l) => {
                if overlaps(uplevel, l.index.offset, 2) {
                    return Err(unsupported(
                        st.span,
                        "a `for` loop whose index a nested routine reaches (kept in the frame, \
                         RINGLOG_$CNTL 0xE72296..0xE722B0, codegen-patterns 7.4) is",
                    ));
                }
                vec![&l.body]
            }
            StmtKind::If { then_, else_, .. } => vec![then_, else_],
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => vec![body],
            StmtKind::Case {
                arms, otherwise, ..
            } => arms
                .iter()
                .map(|a| a.body.as_slice())
                .chain(otherwise.as_deref())
                .collect(),
            _ => vec![],
        };
        for b in inner {
            check_uplevel_indices(b, uplevel)?;
        }
    }
    Ok(())
}

/// The index variables of the [`StmtKind::For`] loops of `body` are read
/// only inside loops over them, whole, and never assigned or addressed
/// there (ref p. 4-89: "You may not assign a value to index_variable
/// within the body of the for loop").  The selector keeps an index in a
/// register for its loop only (codegen-patterns 7.4); reading it after
/// the loop (legal after `exit`, ref p. 4-90; MST_$ALLOC_ASID 0xE42D7C
/// pushes D2 after leaving its loop) is not supported yet.
fn check_for_indices(body: &[Stmt]) -> Res<()> {
    struct Check {
        /// Every `for` index of the body.
        all: Vec<ValueParam>,
        /// The indices of the loops being walked.
        active: Vec<i32>,
        span: Span,
    }
    impl Check {
        fn hit(&self, o: i32, size: u32) -> bool {
            self.all
                .iter()
                .any(|l| o < l.offset + 2 && l.offset < o + size as i32)
        }
        fn bad(&self) -> Diagnostic {
            unsupported(
                self.span,
                "the index variable of a `for` loop read outside the loop, assigned or \
                 addressed (it is kept in a register for the loop only, codegen-patterns 7.4) \
                 is",
            )
        }
        fn addr(&self, a: &Addr, size: u32) -> Res<()> {
            match a {
                Addr::Frame(o) if self.hit(*o, size) => Err(self.bad()),
                Addr::Frame(_)
                | Addr::Enclosing(_)
                | Addr::Static(_)
                | Addr::Symbol { .. }
                | Addr::Const(_) => Ok(()),
                Addr::Volatile { addr, .. } => self.addr(addr, size),
                // The whole object is reached through the base.
                Addr::Indirect { base, .. } => self.expr(base),
                Addr::Index { base, index, .. } => {
                    self.addr(base, 1)?;
                    self.expr(index)
                }
            }
        }
        fn expr(&self, e: &Expr) -> Res<()> {
            match e {
                Expr::Load {
                    addr: Addr::Frame(o),
                    ty,
                } if self.active.contains(o) && ty.width == Width::Word => Ok(()),
                Expr::Load { addr, ty } => self.addr(addr, ty.bytes()),
                Expr::Bits { addr, field, .. } => self.addr(addr, field.unit.bytes()),
                Expr::AddrOf(a) => self.addr(a, 1),
                Expr::Const { .. } => Ok(()),
                Expr::SetMember { elem, set, .. } => {
                    self.expr(elem)?;
                    self.expr(set)
                }
                Expr::SetMemberAt { elem, set, bytes } => {
                    self.expr(elem)?;
                    self.addr(set, *bytes)
                }
                Expr::Binary { lhs, rhs, .. } | Expr::SetOp { lhs, rhs, .. } => {
                    self.expr(lhs)?;
                    self.expr(rhs)
                }
                Expr::Unary { operand: x, .. }
                | Expr::Convert { value: x, .. }
                | Expr::SetSr(x)
                | Expr::SetOf { elem: x, .. } => self.expr(x),
                Expr::Call { call, .. } => self.call(call),
                Expr::Test(c) => self.cond(c),
            }
        }
        fn call(&self, c: &Call) -> Res<()> {
            for a in &c.args {
                match a {
                    Arg::Value { value, .. } | Arg::Temp { value, .. } => self.expr(value)?,
                    Arg::Address { addr, .. } => self.addr(addr, 1)?,
                }
            }
            if let Some(x) = c.pointer() {
                self.expr(x)?;
            }
            Ok(())
        }
        fn cond(&self, c: &Cond) -> Res<()> {
            match c {
                Cond::Compare { lhs, rhs, .. } => {
                    self.expr(lhs)?;
                    self.expr(rhs)
                }
                Cond::Test(e) => self.expr(e),
                Cond::Not(x) => self.cond(x),
                Cond::AndThen(a, b) | Cond::OrElse(a, b) => {
                    self.cond(a)?;
                    self.cond(b)
                }
                Cond::Block { lhs, rhs, size, .. } => {
                    self.addr(lhs, *size)?;
                    self.addr(rhs, *size)
                }
            }
        }
        fn stmts(&mut self, body: &[Stmt]) -> Res<()> {
            for st in body {
                self.span = st.span;
                match &st.kind {
                    StmtKind::SetResult(e) | StmtKind::Discard(e) => self.expr(e)?,
                    StmtKind::Store { addr, value } => {
                        self.addr(addr, value.ty().bytes())?;
                        self.expr(value)?;
                    }
                    StmtKind::StoreBits {
                        addr, field, value, ..
                    } => {
                        self.addr(addr, field.unit.bytes())?;
                        self.expr(value)?;
                    }
                    StmtKind::Copy { dst, src, bytes } => {
                        self.addr(dst, *bytes)?;
                        self.addr(src, *bytes)?;
                    }
                    StmtKind::Call(c) => self.call(c)?,
                    StmtKind::Fill { start, value, .. } => {
                        self.addr(start, 1)?;
                        self.expr(value)?;
                    }
                    StmtKind::If { cond, then_, else_ } => {
                        self.cond(cond)?;
                        self.stmts(then_)?;
                        self.stmts(else_)?;
                    }
                    StmtKind::Repeat { body, until } => {
                        self.stmts(body)?;
                        self.cond(until)?;
                    }
                    StmtKind::While { cond, body } => {
                        self.cond(cond)?;
                        self.stmts(body)?;
                    }
                    StmtKind::For(l) => {
                        self.expr(&l.from)?;
                        self.expr(&l.to)?;
                        if self.active.contains(&l.index.offset) {
                            // A loop over the index of an enclosing loop
                            // assigns it.
                            return Err(self.bad());
                        }
                        self.active.push(l.index.offset);
                        self.stmts(&l.body)?;
                        self.active.pop();
                    }
                    StmtKind::Case {
                        selector,
                        arms,
                        otherwise,
                    } => {
                        self.expr(selector)?;
                        for a in arms {
                            self.stmts(&a.body)?;
                        }
                        if let Some(o) = otherwise {
                            self.stmts(o)?;
                        }
                    }
                    StmtKind::Return
                    | StmtKind::Disable
                    | StmtKind::Enable
                    | StmtKind::Exit
                    | StmtKind::Next
                    | StmtKind::Label(_)
                    | StmtKind::Goto(_) => {}
                }
            }
            Ok(())
        }
    }
    fn collect(body: &[Stmt], out: &mut Vec<ValueParam>) {
        for st in body {
            match &st.kind {
                StmtKind::For(l) => {
                    out.push(l.index);
                    collect(&l.body, out);
                }
                StmtKind::If { then_, else_, .. } => {
                    collect(then_, out);
                    collect(else_, out);
                }
                StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => collect(body, out),
                StmtKind::Case {
                    arms, otherwise, ..
                } => {
                    for a in arms {
                        collect(&a.body, out);
                    }
                    if let Some(o) = otherwise {
                        collect(o, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut all = Vec::new();
    collect(body, &mut all);
    if all.is_empty() {
        return Ok(());
    }
    Check {
        all,
        active: Vec::new(),
        span: body[0].span,
    }
    .stmts(body)
}

/// What statement lowering needs to know about the routine.
struct Cx<'a> {
    p: &'a Program,
    id: RoutineId,
    r: &'a Routine,
    result: Option<ResultSpec>,
    /// The labels prefixing statements of the routine, by name, and
    /// their numbers ([`StmtKind::Label`]; [`routine_labels`]).
    labels: Vec<String>,
}

/// A structured statement on the way from a routine's statement part to
/// a statement inside it ([`routine_labels`]): the statement (its
/// address), which of its parts (an `if`'s `then`/`else`, a `case`'s arm,
/// `otherwise` last), and its kind.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Part {
    stmt: usize,
    part: usize,
    kind: Entered,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Entered {
    If,
    Case,
    /// `for`, `while`, `repeat`.
    Loop,
    With,
}

/// The labels prefixing statements of a routine's statement part `stmts`,
/// in source order (their numbers in [`StmtKind::Label`]), after checking
/// the routine's `goto` statements (ref "Goto", pp. 4-95..4-97):
///
/// - a `goto` to a label of an enclosing routine (a non-local `goto`,
///   legal by p. 4-97) is not supported yet: its frame unwinding (A6, SP
///   and the saved registers of the routines left) has not been looked
///   for in the image;
/// - a jump into a `for`, `while`, `repeat`, `with` or `case` statement
///   from outside it is refused: p. 4-95 forbids it ("It is illegal to
///   use goto to jump inside a structured statement (for example, a for,
///   while, case, with, or repeat) from outside that statement"), while
///   Appendix D, p. D-4 (ISO 6.8.1) says "Domain Pascal permits jumps
///   between branches of a case statement and from one structured
///   statement into the middle of another" (open question 30 in
///   docs/reference-clarifications.md);
/// - allowed: a jump out of any statement, within one statement
///   sequence, between the arms of one `case` (p. D-4), and into an `if`
///   statement's parts (p. 6-23: illegal only under `-iso`, the default
///   being `-niso`).  A compound statement only groups (ref "Begin",
///   p. 4-31): jumping into one is jumping to its statement.
fn routine_labels(stmts: &[SStmt]) -> Res<Vec<String>> {
    type Path = Vec<Part>;
    fn walk(
        stmts: &[SStmt],
        path: &mut Path,
        defs: &mut Vec<(String, Path)>,
        gotos: &mut Vec<(String, Path, Span)>,
    ) {
        for s in stmts {
            one(s, path, defs, gotos);
        }
    }
    fn one(
        s: &SStmt,
        path: &mut Path,
        defs: &mut Vec<(String, Path)>,
        gotos: &mut Vec<(String, Path, Span)>,
    ) {
        let id = s as *const SStmt as usize;
        let mut inside = |part: usize, kind: Entered, body: &[SStmt], path: &mut Path| {
            path.push(Part {
                stmt: id,
                part,
                kind,
            });
            walk(body, path, defs, gotos);
            path.pop();
        };
        match &s.kind {
            SStmtKind::Labeled { label, stmt } => {
                defs.push((label.clone(), path.clone()));
                one(stmt, path, defs, gotos);
            }
            SStmtKind::Compound(b) => walk(&b.stmts, path, defs, gotos),
            SStmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                inside(0, Entered::If, std::slice::from_ref(then_branch), path);
                if let Some(e) = else_branch {
                    inside(1, Entered::If, std::slice::from_ref(e), path);
                }
            }
            SStmtKind::Case {
                arms, otherwise, ..
            } => {
                for (i, a) in arms.iter().enumerate() {
                    inside(i, Entered::Case, std::slice::from_ref(&a.stmt), path);
                }
                if let Some(o) = otherwise {
                    inside(arms.len(), Entered::Case, o, path);
                }
            }
            SStmtKind::While { body, .. } | SStmtKind::For { body, .. } => {
                inside(0, Entered::Loop, std::slice::from_ref(body), path)
            }
            SStmtKind::Repeat { body, .. } => inside(0, Entered::Loop, body, path),
            SStmtKind::With { body, .. } => {
                inside(0, Entered::With, std::slice::from_ref(body), path)
            }
            SStmtKind::Goto(name) => gotos.push((name.clone(), path.clone(), s.span)),
            _ => {}
        }
    }
    let (mut defs, mut gotos) = (Vec::new(), Vec::new());
    walk(stmts, &mut Vec::new(), &mut defs, &mut gotos);
    for (name, from, span) in &gotos {
        let Some((_, to)) = defs.iter().find(|(n, _)| n == name) else {
            return Err(unsupported(
                *span,
                "a `goto` to a label of an enclosing routine (a non-local `goto`, ref p. 4-97; \
                 its frame unwinding has not been looked for in the image) is",
            ));
        };
        let common = to.iter().zip(from).take_while(|(a, b)| a == b).count();
        let mut entered = &to[common..];
        if let (Some(t), Some(f)) = (entered.first(), from.get(common)) {
            // Another part of a statement the `goto` is in: between the
            // arms of a `case` (p. D-4) or the parts of an `if` (p. 6-23).
            if t.stmt == f.stmt {
                entered = &entered[1..];
            }
        }
        // TODO(ref-question): open question 30 (p. 4-95 against p. D-4);
        // refused until the owner rules.
        if entered.iter().any(|p| p.kind != Entered::If) {
            return Err(unsupported(
                *span,
                "a `goto` into a `for`, `while`, `repeat`, `with` or `case` statement from \
                 outside it (illegal by ref p. 4-95, permitted by Appendix D p. D-4: open \
                 question 30) is",
            ));
        }
    }
    Ok(defs.into_iter().map(|(n, _)| n).collect())
}

/// The statements of a statement sequence, with `begin ... end` blocks
/// flattened into it (a compound statement only groups, ref "Begin",
/// p. 4-31) and empty statements dropped.
fn flatten<'s>(stmts: &'s [SStmt], out: &mut Vec<&'s SStmt>) {
    for s in stmts {
        match &s.kind {
            SStmtKind::Empty => {}
            SStmtKind::Compound(b) => flatten(&b.stmts, out),
            _ => out.push(s),
        }
    }
}

impl Cx<'_> {
    /// The number of the label `name` of the routine ([`routine_labels`]).
    fn label(&self, name: &str) -> u32 {
        self.labels
            .iter()
            .position(|n| n == name)
            .expect("labels and local gotos are checked by routine_labels") as u32
    }

    /// A statement sequence.
    fn seq(&self, stmts: &[SStmt]) -> Res<Vec<Stmt>> {
        let p = self.p;
        let mut flat = Vec::new();
        flatten(stmts, &mut flat);
        let mut out = Vec::new();
        for s in &flat {
            match &s.kind {
                SStmtKind::Assign { target, value } if matches!(target.kind, ExprKind::Result { routine, uplevel: 0 } if routine == self.id) =>
                {
                    out.push(result_assignment(p, s, value, self.result)?);
                }
                SStmtKind::Assign { target, .. }
                    if matches!(target.kind, ExprKind::Result { .. }) =>
                {
                    return Err(unsupported(
                        target.span,
                        "assignment to an enclosing function's result is",
                    ));
                }
                SStmtKind::Assign { target, value } if is_block(p, target) => {
                    out.push(copy(p, s, target, value)?)
                }
                SStmtKind::Assign { target, value } => out.push(store(p, s, target, value)?),
                SStmtKind::Call(c) => out.push(Stmt {
                    kind: StmtKind::Call(call(p, c, s.span)?),
                    span: s.span,
                }),
                SStmtKind::For {
                    var,
                    from,
                    dir,
                    to,
                    body,
                } => out.push(self.for_loop(s, var, from, *dir, to, body)?),
                SStmtKind::If {
                    cond,
                    then_branch,
                    else_branch,
                } => {
                    let then_ = self.seq(std::slice::from_ref(then_branch))?;
                    if then_.is_empty() {
                        // What the original emitted for an empty `then`
                        // part (a branch on the negated condition?) is not
                        // known.
                        return Err(unsupported(s.span, "an `if` with an empty `then` part is"));
                    }
                    let else_ = match else_branch {
                        Some(e) => self.seq(std::slice::from_ref(e))?,
                        None => Vec::new(),
                    };
                    out.push(Stmt {
                        kind: StmtKind::If {
                            cond: cond_of(p, cond)?,
                            then_,
                            else_,
                        },
                        span: s.span,
                    });
                }
                SStmtKind::Repeat { body, cond } => out.push(Stmt {
                    kind: StmtKind::Repeat {
                        body: self.seq(body)?,
                        until: cond_of(p, cond)?,
                    },
                    span: s.span,
                }),
                SStmtKind::Return => out.push(Stmt {
                    kind: StmtKind::Return,
                    span: s.span,
                }),
                SStmtKind::Case {
                    expr,
                    arms,
                    otherwise,
                } => out.push(self.case(s, expr, arms, otherwise.as_deref())?),
                // codegen-patterns 7.4: test at the bottom, entered by a
                // `bra` to the test.
                SStmtKind::While { cond, body } => {
                    let body = self.seq(std::slice::from_ref(body))?;
                    if body.is_empty() {
                        // No such loop has been looked for in the image.
                        return Err(unsupported(s.span, "a `while` loop with an empty body is"));
                    }
                    out.push(Stmt {
                        kind: StmtKind::While {
                            cond: cond_of(p, cond)?,
                            body,
                        },
                        span: s.span,
                    });
                }
                // Ref pp. 4-77, 4-123; `dpc-sema` rejects them outside a
                // loop.
                SStmtKind::Exit => out.push(Stmt {
                    kind: StmtKind::Exit,
                    span: s.span,
                }),
                SStmtKind::Next => out.push(Stmt {
                    kind: StmtKind::Next,
                    span: s.span,
                }),
                // Appendix E, pp. E-3, E-4.
                SStmtKind::Builtin {
                    builtin: b @ (Builtin::Disable | Builtin::Enable),
                    ..
                } => out.push(Stmt {
                    kind: if *b == Builtin::Disable {
                        StmtKind::Disable
                    } else {
                        StmtKind::Enable
                    },
                    span: s.span,
                }),
                SStmtKind::Builtin {
                    builtin: Builtin::Discard,
                    args,
                } => out.push(self.discard(s, &args[0])?),
                // `L: s` is the label's point, then `s` (ref "Goto",
                // p. 4-95: control goes "to the statement immediately
                // following lbl").
                SStmtKind::Labeled { label, stmt } => {
                    out.push(Stmt {
                        kind: StmtKind::Label(self.label(label)),
                        span: s.span,
                    });
                    out.extend(self.seq(std::slice::from_ref(stmt))?);
                }
                // Non-local and refused jumps were refused by
                // [`routine_labels`].
                SStmtKind::Goto(name) => out.push(Stmt {
                    kind: StmtKind::Goto(self.label(name)),
                    span: s.span,
                }),
                _ => return Err(unsupported(s.span, "this statement is")),
            }
        }
        Ok(out)
    }

    /// A `for` statement (ref "For", pp. 4-88..4-90): the
    /// [`StmtKind::Fill`] shape when it is one (`fill`), else a
    /// [`StmtKind::For`].
    ///
    /// The index variable must be a word-sized local of this routine,
    /// other than the function result variable: the selector keeps it in
    /// a register for the loop and never stores it (codegen-patterns 7.4:
    /// 307 of 320 `dbf` loops with an index step it in a register; the
    /// other 13, `addq.w #1,(d,A6)` or through a static link, are not
    /// reproduced).  Ref p. 4-89 lets the index be declared in another
    /// scope; such loops (0xE721F8, through the parent's frame) are not
    /// supported yet.  Whether the index is read outside its loops, or
    /// assigned or addressed inside them, is checked over the whole body
    /// afterwards ([`check_for_indices`]).
    fn for_loop(
        &self,
        s: &SStmt,
        var: &SExpr,
        from: &SExpr,
        dir: ForDir,
        to: &SExpr,
        body: &SStmt,
    ) -> Res<Stmt> {
        let p = self.p;
        if p.types.attrs(var.ty).is_volatile() {
            // The index is held in a register (codegen-patterns 7.4).
            return Err(unsupported(var.span, "a volatile `for` index variable is"));
        }
        if let Some(st) = fill(p, s, var, from, dir, to, body)? {
            return Ok(st);
        }
        let ExprKind::Var {
            var: index,
            uplevel: 0,
        } = var.kind
        else {
            return Err(unsupported(
                var.span,
                "a `for` index variable that is not a local of the routine (ref p. 4-89) is",
            ));
        };
        let result = self.r.result.as_ref().and_then(|res| res.var);
        let offset =
            match p.var(index).storage {
                Storage::Frame { offset }
                    if self.r.frame_vars.contains(&index) && Some(index) != result =>
                {
                    offset
                }
                _ => return Err(unsupported(
                    var.span,
                    "a `for` index variable that is not a local of the routine (ref p. 4-89) is",
                )),
            };
        let vty = p.var(index).ty;
        if p.types.size(vty) != 2 {
            // `dbf` counts a word; byte and longword indices are not
            // catalogued.
            return Err(unsupported(
                var.span,
                "a `for` index variable that is not a word (integer16 or a subrange stored \
                 in a word) is",
            ));
        }
        let ty = Scalar {
            width: Width::Word,
            sign: sign_of(&p.types, vty),
        };
        let bound = |e: &SExpr| -> Res<Expr> {
            match &e.value {
                Some(c) => const_of(c, ty, e.span),
                // The bound is converted to the index's type as in an
                // assignment (`store`).
                None => {
                    let v = value_of(p, e)?;
                    if v.ty().width == Width::Word {
                        Ok(v)
                    } else if v.ty().bytes() < 2 && p.types.is_integer(e.ty) {
                        Ok(convert(v, ty))
                    } else {
                        narrow(&v, ty).ok_or_else(|| {
                            unsupported(e.span, "a `for` bound of another width than the index is")
                        })
                    }
                }
            }
        };
        let from = bound(from)?;
        let to = bound(to)?;
        let body = self.seq(std::slice::from_ref(body))?;
        if body.is_empty() {
            return Err(unsupported(s.span, "a `for` loop with an empty body is"));
        }
        Ok(Stmt {
            kind: StmtKind::For(ForLoop {
                index: ValueParam { offset, ty },
                from,
                to,
                down: dir == ForDir::Downto,
                body,
            }),
            span: s.span,
        })
    }

    /// `discard(x)` ("Discard", p. 4-62): "explicitly throws away the
    /// value of its exp".
    ///
    /// - A function call is called like a procedure, its result register
    ///   left unread: a call statement (ref 5.2; the result register,
    ///   codegen-patterns 3).
    /// - `set_sr(x)` (Appendix E, p. E-5) installs `x`; the old status word
    ///   is read into a register that is not used (PROC1_$SET_VT 0xE149F2
    ///   `move.w SR,D1; move.w D3,SR`; PROC1_$TS_END_CALLBACK 0xE14AF8
    ///   `move.w SR,D0; move.w D3,SR`).
    /// - Any other expression: what the original emitted (nothing, or the
    ///   evaluation) is not known.
    fn discard(&self, s: &SStmt, x: &SExpr) -> Res<Stmt> {
        let kind = match &x.kind {
            ExprKind::Call(c) if x.value.is_none() => StmtKind::Call(call(self.p, c, x.span)?),
            ExprKind::Builtin {
                builtin: Builtin::SetSr,
                ..
            } => StmtKind::Discard(value_of(self.p, x)?),
            _ => {
                return Err(unsupported(
                    x.span,
                    "`discard` of an expression that is not a function call or `set_sr` is",
                ))
            }
        };
        Ok(Stmt { kind, span: s.span })
    }

    /// `case expr of ... otherwise ... end` (ref "Case", pp. 4-36..4-37).
    ///
    /// The selector is a word or longword value; a `char` (or other
    /// unsigned byte) selector is widened to a word first, `clr.w Dn;
    /// move.b x,Dn` (codegen-patterns 7.3: module-local 0xE1CC6C,
    /// ACL_$SERVER 0xE495C8, REM_FILE_$SERVER 0xE6429A).  A signed byte
    /// (a boolean, a `-128..127` subrange) is not seen as a selector.
    /// Label ranges are rejected by the parser (reference-clarifications
    /// Q7, open).
    fn case(
        &self,
        s: &SStmt,
        expr: &SExpr,
        arms: &[dpc_sema::CaseArm],
        otherwise: Option<&[SStmt]>,
    ) -> Res<Stmt> {
        let p = self.p;
        if expr.value.is_some() {
            // SMD_$INIT 0xE34D50 `clr.w D0; cmpi.w #4,D0`: a constant
            // selector is dispatched at run time there, but one site.
            return Err(unsupported(expr.span, "a constant `case` selector is"));
        }
        let mut selector = value_of(p, expr)?;
        let ty = selector.ty();
        match (ty.width, ty.sign) {
            (Width::Byte, Sign::Unsigned) => {
                selector = Expr::Convert {
                    value: Box::new(selector),
                    ty: Scalar::unsigned(Width::Word),
                }
            }
            (Width::Byte, Sign::Signed) => {
                return Err(unsupported(
                    expr.span,
                    "a signed byte (boolean, `-128..127`) `case` selector is",
                ))
            }
            _ => {}
        }
        let mut out = Vec::new();
        for arm in arms {
            for &l in &arm.labels {
                if !fits(l, ty) {
                    // ref 9, error 45 (p. 9-11): "(Constant) is outside the
                    // subrange of the CASE".
                    return Err(Diagnostic::error(
                        arm.stmt.span,
                        format!("case label {l} is outside the selector's range"),
                    ));
                }
            }
            out.push(CaseArm {
                labels: arm.labels.clone(),
                body: self.seq(std::slice::from_ref(&arm.stmt))?,
            });
        }
        let otherwise = match otherwise {
            Some(ss) => {
                let o = self.seq(ss)?;
                if o.is_empty() {
                    // Whether the original dispatched to the end, and how
                    // the last arm ended, is not seen.
                    return Err(unsupported(s.span, "an empty `otherwise` part is"));
                }
                Some(o)
            }
            None => None,
        };
        Ok(Stmt {
            kind: StmtKind::Case {
                selector,
                arms: out,
                otherwise,
            },
            span: s.span,
        })
    }
}

/// The condition `e` (a boolean expression) as a [`Cond`]: comparisons,
/// boolean values, `not`, `and then`, `or else`.
///
/// - A comparison takes the type of its operands; a constant operand
///   takes the other operand's type and must fit it (LOG_$READ's worker
///   0xE1779A compares its `0..65535` word with `cmpi.w #0x400`, not at
///   the 32 bits of the subrange's host).  Operands of different widths
///   or signs (implicit widening, codegen-patterns 5) are not supported
///   yet; nor are sets and strings.
/// - `nil` is the pointer 0 (`tst.l (0x14,A5)` at 0xE1778C tests a
///   pointer against nil).
/// - `and`/`or` without `then`/`else` may evaluate both operands in any
///   order (ref pp. 4-18..4-19); what the original did is not catalogued:
///   not supported yet.
fn cond_of(p: &Program, e: &SExpr) -> Res<Cond> {
    if e.value.is_some() {
        return Err(unsupported(e.span, "a constant condition is"));
    }
    match &e.kind {
        ExprKind::Unary {
            op: UnOp::Not,
            operand,
        } => Ok(Cond::Not(Box::new(cond_of(p, operand)?))),
        ExprKind::Binary { op, lhs, rhs } => {
            let rel = match op {
                BinOp::AndThen => {
                    return Ok(Cond::AndThen(
                        Box::new(cond_of(p, lhs)?),
                        Box::new(cond_of(p, rhs)?),
                    ))
                }
                BinOp::OrElse => {
                    return Ok(Cond::OrElse(
                        Box::new(cond_of(p, lhs)?),
                        Box::new(cond_of(p, rhs)?),
                    ))
                }
                BinOp::And | BinOp::Or => {
                    return Err(unsupported(
                        e.span,
                        "`and`/`or` in a condition (evaluation not catalogued; `and then`/`or \
                         else` are supported) is",
                    ))
                }
                BinOp::Eq => Rel::Eq,
                BinOp::Ne => Rel::Ne,
                BinOp::Lt => Rel::Lt,
                BinOp::Le => Rel::Le,
                BinOp::Gt => Rel::Gt,
                BinOp::Ge => Rel::Ge,
                _ => return Ok(Cond::Test(bool_value(p, e)?)),
            };
            compare(p, e, rel, lhs, rhs)
        }
        _ => Ok(Cond::Test(bool_value(p, e)?)),
    }
}

/// A boolean value used as a condition.
fn bool_value(p: &Program, e: &SExpr) -> Res<Expr> {
    if !p.types.is_boolean(e.ty) {
        return Err(unsupported(e.span, "this condition is"));
    }
    value_of(p, e)
}

/// `lhs rel rhs` (see [`cond_of`]).
fn compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    if is_set(p, lhs.ty) && is_set(p, rhs.ty) {
        return set_compare(p, e, rel, lhs, rhs);
    }
    if matches!(p.types.kind(lhs.ty), TypeKind::Record(_)) {
        return record_compare(p, e, rel, lhs, rhs);
    }
    let comparable = |x: &SExpr| p.types.is_ordinal(x.ty) || p.types.is_pointer(x.ty);
    if !comparable(lhs) || !comparable(rhs) {
        return Err(unsupported(
            e.span,
            "comparing values other than ordinals and pointers is",
        ));
    }
    if p.types.is_boolean(lhs.ty) || p.types.is_boolean(rhs.ty) {
        // `b = true`, `b1 <> b2`: compiled code tests booleans with
        // `tst.b` and the sign (codegen-patterns 7); a compare of a byte
        // with 0xFF occurs twice (0xE1B96C, 0xE53FDE, not shown to be
        // booleans), so what the original did here is not catalogued.
        return Err(unsupported(e.span, "comparing boolean values is"));
    }
    let operand = |x: &SExpr, other: Option<Scalar>| -> Res<Expr> {
        match (&x.value, other) {
            (Some(ConstValue::Nil), _) => Ok(Expr::Const {
                value: 0,
                ty: Scalar::POINTER,
            }),
            (Some(v), Some(ty)) => {
                let c = const_of(v, ty, x.span)?;
                let Expr::Const { value, .. } = c else {
                    unreachable!()
                };
                if !fits(value, ty) {
                    return Err(unsupported(
                        x.span,
                        "comparing with a constant outside the other operand's type is",
                    ));
                }
                Ok(c)
            }
            (Some(_), None) => Err(unsupported(e.span, "comparing two constants is")),
            (None, _) => value_of(p, x),
        }
    };
    let (l, r) = match (&lhs.value, &rhs.value) {
        (None, Some(_)) => {
            let l = operand(lhs, None)?;
            let r = operand(rhs, Some(l.ty()))?;
            (l, r)
        }
        (Some(_), None) => {
            let r = operand(rhs, None)?;
            let l = operand(lhs, Some(r.ty()))?;
            (l, r)
        }
        _ => (operand(lhs, None)?, operand(rhs, None)?),
    };
    let (l, r) = widen_operands(e, l, r)?;
    Ok(Cond::Compare {
        op: rel,
        lhs: l,
        rhs: r,
    })
}

/// The operands of a comparison brought to one type (ref 4.3.1 p. 4-5:
/// operands smaller than 32 bits are computed "with 32 bits of precision
/// when necessary to achieve correct arithmetic"; p. 9-32, message 250:
/// the compiler knows a negative number never equals an unsigned subrange
/// value, so comparisons have their mathematical meaning).
///
/// - A signed word and an unsigned one (`integer` against `0..65535`):
///   both widened to longwords, compared signed (codegen-patterns 7.1,
///   "Mixed widths and signs": TTY_$K_PUT 0xE67C42..0xE67C4A `move.w
///   D2,D0; clr.l D1; ext.l D0; move.w (A4),D1; cmp.l D1,D0; bge.b`;
///   tty_$i_put_chars 0xE1B050..0xE1B058; tty_$i_err 0xE1BD7C..0xE1BD8A;
///   26 `cmp.l` of two widened words in compiled code, every ordered
///   one with a signed branch; `tools/cmpsurvey.py`, widen).
/// - A word against a longword: the word widened by its own sign, when
///   the longword is signed or both are unsigned (`ext.l Dn; cmp.l
///   <ea>,Dn`, 0xE179BC; `clr.l Dn; move.w; cmp.l <ea>,Dn; bls`,
///   0xE0D7CE; 38 sites, `tools/cmpsurvey.py`, widen1).
///
/// Anything else that mixes widths or signs (a byte against a word, a
/// signed and an unsigned longword, a signed word against an unsigned
/// longword) is not supported yet.
fn widen_operands(e: &SExpr, l: Expr, r: Expr) -> Res<(Expr, Expr)> {
    let (lt, rt) = (l.ty(), r.ty());
    if lt == rt {
        return Ok((l, r));
    }
    let long = Scalar::signed(Width::Long);
    match (lt.width, rt.width) {
        (Width::Word, Width::Word) => return Ok((convert(l, long), convert(r, long))),
        (Width::Word, Width::Long) if rt.sign == Sign::Signed || lt.sign == rt.sign => {
            return Ok((convert(l, rt), r))
        }
        (Width::Long, Width::Word) if lt.sign == Sign::Signed || lt.sign == rt.sign => {
            return Ok((l, convert(r, lt)))
        }
        _ => {}
    }
    Err(unsupported(
        e.span,
        format!(
            "comparing a {}-byte {} value with a {}-byte {} one (implicit widening) is",
            lt.bytes(),
            sign_name(lt),
            rt.bytes(),
            sign_name(rt)
        ),
    ))
}

/// `lhs = rhs` / `lhs <> rhs` of two records (ref Table 4-1, p. 4-2:
/// `=` and `<>` for "All types"), compared in memory ([`Cond::Block`];
/// codegen-patterns 7.1, "Records").  The image compares every byte
/// (`cmpm`), so bytes no field covers (alignment padding) would take part;
/// the reference does not say what equality of records with such bytes
/// means, and the kernel sites found compare UIDs and clock values, which
/// have none: a record with uncovered bytes is not supported yet.  Arrays
/// and strings are not supported yet (the byte loop at 0xE6C278 compares
/// a 6-character array with `cmpm.b`, not `cmpm.w` as the 6-byte records
/// are, so the unit depends on more than the size).
fn record_compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    if !matches!(rel, Rel::Eq | Rel::Ne) {
        return Err(unsupported(e.span, "ordering records is"));
    }
    if lhs.value.is_some() || rhs.value.is_some() {
        return Err(unsupported(e.span, "comparing a record constant is"));
    }
    let size = p.types.size(lhs.ty);
    if !covers(p, lhs.ty, size) {
        return Err(unsupported(
            e.span,
            "comparing records with bytes no field covers (padding; the reference gives no \
             rule, codegen-patterns 7.1) is",
        ));
    }
    Ok(Cond::Block {
        op: rel,
        lhs: addr_of(p, lhs)?,
        rhs: addr_of(p, rhs)?,
        size,
    })
}

/// Whether every byte of an object of type `ty` (`size` bytes) belongs to
/// some field or element: a record whose fields (all variants included)
/// leave no gap, in bytes or, in a packed record (ref 3.10.7), in bits,
/// and are covered themselves; an array of such elements with no room
/// between them; or a scalar that fills its size.  Sets (whose unused
/// bits are not values) are not taken to be covered.
fn covers(p: &Program, ty: TypeId, size: u32) -> bool {
    fn arms<'a>(v: &'a dpc_sema::types::VariantPart, out: &mut Vec<&'a dpc_sema::types::Field>) {
        out.extend(v.tag.iter());
        for arm in &v.arms {
            out.extend(arm.fields.iter());
            if let Some(w) = arm.variant.as_deref() {
                arms(w, out);
            }
        }
    }
    match p.types.kind(ty) {
        TypeKind::Record(r) => {
            let mut fs: Vec<&dpc_sema::types::Field> = r.fields.iter().collect();
            if let Some(v) = r.variant.as_deref() {
                arms(v, &mut fs);
            }
            // Bit spans: a field of an unpacked record, or a whole object
            // in a packed one, is covered when its type is; a bit field
            // covers its bits.
            let mut spans: Vec<(u32, u32)> = Vec::new();
            for f in fs {
                let n = p.types.size(f.ty);
                let (bit, width) = match f.bits {
                    Some(b) if !b.is_whole_object(n) => (b.bit, b.width),
                    Some(b) => (b.bit, n * 8),
                    None => (f.offset * 8, n * 8),
                };
                if (f.bits.is_none() || width == n * 8) && !covers(p, f.ty, n) {
                    return false;
                }
                spans.push((bit, bit + width));
            }
            spans.sort();
            let mut end = 0;
            for (a, b) in spans {
                if a > end {
                    return false;
                }
                end = end.max(b);
            }
            end >= size * 8
        }
        TypeKind::Array { elem, index, .. } => {
            let n = p.types.size(*elem);
            let count = p
                .types
                .ordinal_range(*index)
                .map_or(0, |(lo, hi)| (hi - lo + 1) as u32);
            count * n == size && covers(p, *elem, n)
        }
        TypeKind::Set { .. } | TypeKind::Varying { .. } | TypeKind::Error => false,
        _ => true,
    }
}

/// `s = []` and `s <> []` (ref "Set Operations", p. 4-176) for a
/// register-sized set: the set's value against 0.  Image: `tst.w` of a
/// set variable (TTY_$K_GET 0xE1C5D4 `tst.w (10,A2); beq`), the flags of
/// the `and` for an intersection or a difference (TTY 0xE1BFFE `moveq
/// #7,D1; and.w (8,A2),D1; beq.s`; PROC2_$SIGPAUSE 0xE3FAC6 `move.l
/// (-108,A4),D1; not.l D1; and.l (-100,A4),D1; beq.s`).  Other set
/// comparisons (`=` with a non-empty set, subset `<=`, superset `>=`) are
/// not catalogued.
fn set_compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    if p.types.size(lhs.ty) > 4 {
        // MSG_$CLOSEI 0xE59490..0xE5949C: `tst.l (A1)+; bne` per longword.
        return Err(unsupported(
            e.span,
            "comparing a set of more than a longword (`tst.l (An)+` per longword, \
             codegen-patterns 5.1) is",
        ));
    }
    let empty = |x: &SExpr| matches!(&x.value, Some(ConstValue::Set(m)) if m.is_empty());
    if !matches!(rel, Rel::Eq | Rel::Ne) || !empty(rhs) || lhs.value.is_some() {
        return Err(unsupported(
            e.span,
            "a set comparison other than `s = []` / `s <> []` is",
        ));
    }
    let x = lhs;
    let ty = scalar_of(p, x)?;
    let l = set_value(p, x, ty)?;
    Ok(Cond::Compare {
        op: rel,
        lhs: l,
        rhs: Expr::Const { value: 0, ty },
    })
}

fn sign_name(t: Scalar) -> &'static str {
    match t.sign {
        Sign::Signed => "signed",
        Sign::Unsigned => "unsigned",
    }
}

/// Whether `v` is a value of IR type `ty`.
fn fits(v: i64, ty: Scalar) -> bool {
    let bits = ty.bytes() * 8;
    match ty.sign {
        Sign::Signed => (-(1i64 << (bits - 1))..(1i64 << (bits - 1))).contains(&v),
        Sign::Unsigned => (0..(1i64 << bits)).contains(&v),
    }
}

/// `f := e` inside `f`, anywhere in its body.
///
/// The value goes to the result register, or to the register the result
/// variable is held in (codegen-patterns 3, "Result variable"):
///
/// - as the last statement, straight to the result register (PROC1_$TST_LOCK
///   0xE148CA..0xE148E4, `move.b D1,D0` at 0xE148E0); the frame slot
///   `dpc-sema` reserves for the result variable is never touched,
///   although the frame has room for it (`link #-4`, codegen-patterns 2);
/// - before other statements, on several paths, or before a `return`: into
///   one register that holds the variable up to the epilogue, a
///   callee-saved one when a call follows (PKT_$NEXT_ID 0xE124AC `move.w
///   (0x5c,A5),D2` ... 0xE124D0 `move.w D2,D0`).  The selector decides.
///
/// Reading the result variable is not a thing of the language: the
/// function's name in an expression calls it (ref 5.8, p. 5-26,
/// `factorial(n-1)`; `dpc-sema`).  Assigning an enclosing function's
/// result from a nested routine is refused (`Cx::seq`).
fn result_assignment(
    p: &Program,
    s: &SStmt,
    value: &SExpr,
    result: Option<ResultSpec>,
) -> Res<Stmt> {
    let res = result.expect("lower_routine skips the body when the result type failed");
    let vsize = p.types.size(value.ty);
    if let Some(v) = &value.value {
        return Ok(Stmt {
            kind: StmtKind::SetResult(const_of(v, res.ty, value.span)?),
            span: s.span,
        });
    }
    let v = value_of(p, value)?;
    let v = if vsize == res.ty.bytes() {
        v
    } else if vsize < res.ty.bytes() && p.types.is_integer(value.ty) {
        // Widening, as for a store (`store`).
        convert(v, res.ty)
    } else if let Some(n) = narrow(&v, res.ty) {
        n
    } else {
        return Err(unsupported(
            value.span,
            format!(
                "assigning a {vsize}-byte value to a {}-byte function result is",
                res.ty.bytes()
            ),
        ));
    };
    Ok(Stmt {
        kind: StmtKind::SetResult(v),
        span: s.span,
    })
}

/// `d := e` for a scalar designator `d` (ref "Assignment", p. 4-25).
///
/// A constant value takes the designator's type (the checker has made
/// sure it fits); any other value must have the designator's width, since
/// implicit integer widening (`integer16` to `integer32`) has not been
/// catalogued yet.
fn store(p: &Program, s: &SStmt, target: &SExpr, value: &SExpr) -> Res<Stmt> {
    if let Some((addr, field)) = bit_field(p, target)? {
        return store_bits(p, s, target, value, addr, field);
    }
    if is_set(p, target.ty) && p.types.size(target.ty) > 4 {
        // Sets of more than a longword are built and combined in frame
        // temporaries: `clr.l (A0)+` per longword, the member's byte
        // `bset`, `dbf` loops of `not.l`/`and.l` through `(An)+`, then a
        // copy (MSG_$CLOSEI 0xE59454..0xE5948C; MSG_$OPENI 0xE59248..
        // 0xE5926E; codegen-patterns 5.1).
        return Err(unsupported(
            s.span,
            "an assignment to a set of more than a longword (frame temporaries and `dbf` \
             loops, codegen-patterns 5.1) is",
        ));
    }
    let ty = scalar_of(p, target)?;
    let addr = access_of(p, target, false)?;
    let value = if let Some(v) = &value.value {
        const_of(v, ty, value.span)?
    } else if is_set(p, target.ty) {
        // A set expression takes the target's size (its constructors have
        // their own, ref 3.9.3).
        set_value(p, value, ty)?
    } else {
        let v = value_of(p, value)?;
        if v.ty().width != ty.width {
            // Widening an integer (ref 4.3.1, p. 4-5) is a conversion of
            // the value; narrowing (an integer32 value into an integer16
            // object) has not been catalogued.
            if v.ty().bytes() < ty.bytes() && p.types.is_integer(value.ty) {
                convert(v, ty)
            } else if let Some(n) = narrow(&v, ty) {
                n
            } else {
                return Err(unsupported(
                    value.span,
                    format!(
                        "assigning a {}-byte value to a {}-byte object is",
                        v.ty().bytes(),
                        ty.bytes()
                    ),
                ));
            }
        } else {
            v
        }
    };
    Ok(Stmt {
        kind: StmtKind::Store { addr, value },
        span: s.span,
    })
}

/// `e`, wider than `ty`, computed at `ty`'s width when that gives the
/// same low bytes: integer `+`, `-`, `*`, `&`, `!`, `xor`, `lshft`, `-x`,
/// `~x` (arithmetic modulo 2^bits), operands narrowed in turn, widening
/// conversions of operands already of `ty`'s width dropped, constants
/// truncated.  Ref 4.3.1, p. 4-5: "the compiler tries to avoid the extra
/// code produced by data expansion".  Image: LOG_$READ's worker 0xE177C4
/// `move.w #0x400,D3; sub.w D0,D3; move.w D3,(A0)` stores `16#400 -
/// offset`, `offset` a `0..65535` word whose expression is integer32, as
/// a word subtraction.  A longword `div`/`mod` is kept as it is, inside a
/// narrowing [`Expr::Convert`] (to a word only), for the selector.  `None`
/// when a part cannot be narrowed (a load of a wider object, a right
/// shift, a call).
fn narrow(e: &Expr, ty: Scalar) -> Option<Expr> {
    match e {
        Expr::Const { value, .. } => {
            let bits = ty.bytes() * 8;
            let mask = (1i64 << bits) - 1;
            let mut v = value & mask;
            if ty.is_signed() && v >= 1i64 << (bits - 1) {
                v -= 1i64 << bits;
            }
            Some(Expr::Const { value: v, ty })
        }
        Expr::Convert { value, .. } if value.ty().width == ty.width => Some(convert(
            (**value).clone(),
            Scalar {
                width: ty.width,
                sign: value.ty().sign,
            },
        )),
        Expr::Binary {
            op: op @ (IrBinOp::Add | IrBinOp::Sub | IrBinOp::Mul | IrBinOp::And | IrBinOp::Or),
            lhs,
            rhs,
            ..
        }
        | Expr::Binary {
            op: op @ IrBinOp::Xor,
            lhs,
            rhs,
            ..
        } => Some(Expr::Binary {
            op: *op,
            lhs: Box::new(narrow(lhs, ty)?),
            rhs: Box::new(narrow(rhs, ty)?),
            ty,
        }),
        Expr::Binary {
            op: IrBinOp::Shl,
            lhs,
            rhs,
            ..
        } => Some(Expr::Binary {
            op: IrBinOp::Shl,
            lhs: Box::new(narrow(lhs, ty)?),
            rhs: rhs.clone(),
            ty,
        }),
        Expr::Unary { op, operand, .. } => Some(Expr::Unary {
            op: *op,
            operand: Box::new(narrow(operand, ty)?),
            ty,
        }),
        // A longword `div`/`mod` stored into a word: divided in 32 bits and
        // the low word used (codegen-patterns 8: 0xE10CAE `divs.w #250` on a
        // longword difference passed as a word; PMAP_$PURIFIER_L 0xE13AFC
        // `divu.w #50` stored with `move.w`; 0xE39C2E `move.w D0,D1` after
        // `M$DIS$LLL`).  The selector decides the instructions.
        Expr::Binary {
            op: IrBinOp::Div | IrBinOp::Mod,
            ..
        } if ty.width == Width::Word => Some(Expr::Convert {
            value: Box::new(e.clone()),
            ty,
        }),
        _ => None,
    }
}

/// Whether an assignment to `target` copies a block: a record or an array
/// whose size is not a scalar width.  A record or array of 1, 2 or 4
/// bytes is moved like a scalar of that size.  **Uncertain:** no such
/// assignment has been catalogued.
fn is_block(p: &Program, target: &SExpr) -> bool {
    matches!(
        p.types.kind(target.ty),
        TypeKind::Record(_) | TypeKind::Array { .. }
    ) && Width::of_size(p.types.size(target.ty)).is_none()
}

/// `d := s` for a record or array `d` that is not scalar-sized (ref
/// "Assignment", p. 4-25): a block copy of the type's size.  The value
/// must be a designator; how the original assigned a constant or a
/// function result to a record has not been catalogued.  The selector
/// decides the instructions (codegen-patterns 5.2).
fn copy(p: &Program, s: &SStmt, target: &SExpr, value: &SExpr) -> Res<Stmt> {
    if !value.is_designator() || value.value.is_some() {
        return Err(unsupported(
            value.span,
            "assigning a record or array that is not a variable is",
        ));
    }
    let bytes = p.types.size(target.ty);
    if p.types.size(value.ty) != bytes {
        return Err(unsupported(
            value.span,
            "assigning a record or array of another size is",
        ));
    }
    if volatile_access(p, target).is_some()
        || volatile_access(p, value).is_some()
        || p.types.has_volatile_part(target.ty)
        || p.types.has_volatile_part(value.ty)
    {
        // A block copy (`move.l (An)+,(Am)+`, a `dbf` loop) of an object
        // with volatile or device parts: how the original copied those is
        // not seen.
        return Err(unsupported(
            s.span,
            "assigning a record or array with volatile or device parts (ref 3.15.1-3.15.3) is",
        ));
    }
    // A byte-aligned array may lie at an odd address; the block copy
    // moves words and longwords.
    check_even(p, target)?;
    check_even(p, value)?;
    Ok(Stmt {
        kind: StmtKind::Copy {
            dst: addr_of(p, target)?,
            src: addr_of(p, value)?,
            bytes,
        },
        span: s.span,
    })
}

/// The constant `v` as a value of IR type `ty`.  A boolean is its byte:
/// 0xFF for true, read as the signed byte -1 (ref 3.5.3, p. 3-11: "The
/// system sets all eight bits to 1 for true and sets all eight bits to 0
/// for false").
/// The code-section cell of constant `v` passed by address as type `ty`
/// (codegen-patterns 1.5): the value big-endian in the type's size, a
/// byte in the high byte of a word (`ff00` for `true`: 17 sites name an
/// earlier `ff00` cell, e.g. PROC2_$COMPLETE_VFORK's 0xE735F0; ref 3.5.3
/// p. 3-11).  Scalars of 1, 2 or 4 bytes only; strings, sets, records and
/// `nil` are not supported yet.
fn pool_const(p: &Program, v: &ConstValue, ty: TypeId, span: Span) -> Res<PoolConst> {
    let size = p.types.size(ty);
    let width = match Width::of_size(size) {
        Some(w) if v.ordinal().is_some() => w,
        _ => {
            return Err(unsupported(
                span,
                format!("a constant of {size} bytes or of this kind passed by address is"),
            ))
        }
    };
    let Expr::Const { value, .. } = const_of(v, Scalar::signed(width), span)? else {
        unreachable!("const_of gives a constant")
    };
    let be = (value as i32).to_be_bytes();
    let bytes = match width {
        Width::Byte => vec![be[3], 0],
        Width::Word => be[2..].to_vec(),
        Width::Long => be.to_vec(),
    };
    Ok(PoolConst { bytes, key: ty.0 })
}

/// The type of the cell of a constant of type `cty` passed by address to
/// a `univ` parameter of type `fty` (bead domain-pascal-4be).  Ref 5.3.3
/// p. 5-11 (checked in the PDF): "When you pass an expression argument
/// (as opposed to a variable argument) to a univ parameter, Domain Pascal
/// extends the expression to be the same size as the univ parameter",
/// with the message "Expression passed to UNIV formal NAME was converted
/// to NEWTYPE" (also diagnostic 246, Appendix B).
///
/// - Equal sizes: the constant's own type (the bytes are the same either
///   way; the cell's key stays the constant's type).
/// - An integer constant for a larger integer parameter: the value as the
///   parameter's type, i.e. sign-extended to its size ("converted to
///   NEWTYPE").  A cell of the constant's size would leave the callee
///   reading past it.
/// - Anything else is not supported yet: a constant larger than the
///   parameter (the reference only speaks of extending; p. 5-10/5-11
///   warns against passing an argument larger than the parameter), and
///   the extension of a non-integer constant (boolean, character,
///   enumeration) or to a non-integer parameter (array, record, ...),
///   whose converted bytes the reference does not give.
///
/// Image: no site settles the cell's size against a `univ` formal; the
/// nearest, LOG_$SHUTDN's `pea (0x66,PC)` at 0xE175A0 of a longword
/// `0000 0000` cell for LOG_$ADD's data (C `void *data`, length 0), fits
/// both an `integer32` constant and an extension to a 4-byte formal.
fn univ_const_type(p: &Program, cty: TypeId, fty: TypeId, span: Span) -> Res<TypeId> {
    let (cs, fs) = (p.types.size(cty), p.types.size(fty));
    if cs == fs {
        Ok(cty)
    } else if cs < fs && p.types.is_integer(cty) && p.types.is_integer(fty) {
        Ok(fty)
    } else {
        Err(unsupported(
            span,
            format!(
                "passing a {cs}-byte constant of this type to a {fs}-byte `univ` parameter \
                 (ref 5.3.3 p. 5-11) is"
            ),
        ))
    }
}

fn const_of(v: &ConstValue, ty: Scalar, span: Span) -> Res<Expr> {
    let value = match v {
        ConstValue::Bool(b) => -(*b as i64),
        // A register-sized set (ref 3.9.3, p. 3-16, Figure 3-6): member k
        // is bit k of the word or longword, so the constant is its bit
        // mask (`[]` is 0).  TTY_$I_INIT 0xE33290 `move.l #0x1ffef,(0x20,A0)`
        // stores [0..3, 5..16]; TTY_$K_SET_DELAY 0xE67A1A `moveq #0x1f,D1`
        // is [0..4] (codegen-patterns 5.1).
        ConstValue::Set(members) => {
            let bits = ty.bytes() as i64 * 8;
            if members.iter().any(|&k| !(0..bits).contains(&k)) {
                return Err(unsupported(
                    span,
                    format!("a set constant with a member outside the {bits} bits of its set is"),
                ));
            }
            let mask = members.iter().fold(0u64, |m, &k| m | (1u64 << k));
            imm_value(mask, ty)
        }
        _ => v
            .ordinal()
            .ok_or_else(|| unsupported(span, "this kind of constant is"))?,
    };
    Ok(Expr::Const { value, ty })
}

/// The number a bit pattern of `ty`'s width stands for under `ty`'s sign
/// (an [`Expr::Const`] holds the value, not the bits).
fn imm_value(bits: u64, ty: Scalar) -> i64 {
    match (ty.width, ty.sign) {
        (Width::Byte, Sign::Signed) => bits as u8 as i8 as i64,
        (Width::Word, Sign::Signed) => bits as u16 as i16 as i64,
        (Width::Long, Sign::Signed) => bits as u32 as i32 as i64,
        (Width::Byte, Sign::Unsigned) => bits as u8 as i64,
        (Width::Word, Sign::Unsigned) => bits as u16 as i64,
        (Width::Long, Sign::Unsigned) => bits as u32 as i64,
    }
}

/// Whether `ty` is a set type.
fn is_set(p: &Program, ty: TypeId) -> bool {
    matches!(p.types.kind(ty), TypeKind::Set { .. })
}

/// A set expression `e` of register size (ref 3.9.3: one word or one
/// longword) as a value of the set type `ty`: a constant (its bit mask,
/// [`const_of`]), a variable, `[x]` with one run-time member
/// ([`Expr::SetOf`]), or `+`, `*`, `-` of such ([`Expr::SetOp`]; ref "Set
/// Operations", pp. 4-173..4-175).  A constructor's own type is narrower
/// or wider than the set it is used with (`[7]` is a `set of 0..7`, ref
/// 3.9.3 sizes it by its high end), so constants and constructors take
/// the type of the operation; variables must have it.
///
/// Refused: a constructor of several run-time members or of a run-time
/// range (`[a..b]`: the image has the routine PAS_$SET_BUILD 0xE11FA8,
/// which adds a range to a set in memory, but no compiled call of it, so
/// the compiler's code for one is not catalogued, codegen-patterns 5.1).
fn set_value(p: &Program, e: &SExpr, ty: Scalar) -> Res<Expr> {
    if let Some(v) = &e.value {
        return const_of(v, ty, e.span);
    }
    match &e.kind {
        ExprKind::SetCons(items) => match items.as_slice() {
            [(x, None)] => Ok(Expr::SetOf {
                elem: Box::new(set_element(p, x)?),
                ty,
            }),
            _ => Err(unsupported(
                e.span,
                "a set constructor with a run-time range or several run-time members (codegen-\
                 patterns 5.1: PAS_$SET_BUILD 0xE11FA8 is never called from compiled code) is",
            )),
        },
        ExprKind::Binary {
            op: op @ (BinOp::Add | BinOp::Mul | BinOp::Sub),
            lhs,
            rhs,
        } if is_set(p, e.ty) => {
            let op = match op {
                BinOp::Add => SetOp::Union,
                BinOp::Mul => SetOp::Inter,
                _ => SetOp::Diff,
            };
            Ok(Expr::SetOp {
                op,
                lhs: Box::new(set_value(p, lhs, ty)?),
                rhs: Box::new(set_value(p, rhs, ty)?),
                ty,
            })
        }
        _ => {
            let v = value_of(p, e)?;
            if v.ty().width != ty.width {
                return Err(unsupported(
                    e.span,
                    format!(
                        "a set operation on sets of {} and {} bytes is",
                        v.ty().bytes(),
                        ty.bytes()
                    ),
                ));
            }
            Ok(v)
        }
    }
}

/// A run-time set member as a word (ref 3.9.3: ordinals 0..1023): an
/// enumeration or integer16 value as it is, a char or boolean byte
/// zero-extended.
fn set_element(p: &Program, x: &SExpr) -> Res<Expr> {
    let v = value_of(p, x)?;
    match v.ty().width {
        Width::Word => Ok(v),
        Width::Byte => Ok(convert(v, Scalar::unsigned(Width::Word))),
        Width::Long => Err(unsupported(
            x.span,
            "a set member of a 4-byte type (narrowed to a word: not catalogued) is",
        )),
    }
}

/// `lhs in rhs` for a set of `bytes` bytes, more than a register holds,
/// in memory ([`Expr::SetMemberAt`]; codegen-patterns 5.1: the byte
/// `(bits-1 - k) >> 3` and `btst k` on it, MST_$ALLOC_ASID 0xE42D5C for 64
/// bits, the TTY module-local 0xE1B740..0xE1B750 for a `set of char`).
///
/// Only when every value of the element's type is a member of the set's
/// bits (ref 3.9.3, p. 3-16: one bit per ordinal 0..bits-1): the image
/// tests such elements unguarded (a `char` in a 256-bit set at 0xE1B750,
/// an ASID in a 64-bit one at 0xE42D64), but guards the others with `bcs`
/// on the subtraction and branches with `ble`/`bgt` after the `btst`
/// (NAME_$INIT 0xE31836..0xE3184A on a 96-bit set; UNMAP_CASE
/// 0xE54166..0xE54178): that form is refused.  A constant set of more
/// than a longword (a cell in the code, UNMAP_CASE's `lea (d,PC)`) is
/// refused too.
fn set_member_at(p: &Program, e: &SExpr, lhs: &SExpr, rhs: &SExpr, bytes: u32) -> Res<Expr> {
    if rhs.value.is_some()
        || !matches!(
            rhs.kind,
            ExprKind::Var { .. }
                | ExprKind::Field { .. }
                | ExprKind::Deref(_)
                | ExprKind::Index { .. }
        )
    {
        return Err(unsupported(
            e.span,
            format!("`in` on a {bytes}-byte set that is not a variable is"),
        ));
    }
    if volatile_access(p, rhs).is_some() {
        return Err(unsupported(
            e.span,
            format!("`in` on a volatile {bytes}-byte set in memory is"),
        ));
    }
    let bits = bytes as i64 * 8;
    match p.types.ordinal_range(lhs.ty) {
        Some((lo, hi)) if lo >= 0 && hi < bits => {}
        _ => {
            return Err(unsupported(
                e.span,
                format!(
                    "`in` on a {bytes}-byte set with an element whose type reaches beyond the \
                     set's {bits} bits (the guarded test, NAME_$INIT 0xE3183E, codegen-patterns \
                     5.1) is"
                ),
            ))
        }
    }
    Ok(Expr::SetMemberAt {
        elem: Box::new(set_element(p, lhs)?),
        set: addr_of(p, rhs)?,
        bytes,
    })
}

/// The value of `e`.
fn value_of(p: &Program, e: &SExpr) -> Res<Expr> {
    if let Some(v) = &e.value {
        return const_of(v, scalar_of(p, e)?, e.span);
    }
    if is_set(p, e.ty) && matches!(e.kind, ExprKind::SetCons(_) | ExprKind::Binary { .. }) {
        let ty = scalar_of(p, e)?;
        return set_value(p, e, ty);
    }
    if let Some((addr, field)) = bit_field(p, e)? {
        check_device_access(p, e, true)?;
        return Ok(Expr::Bits {
            addr,
            field,
            ty: bit_field_type(p, e)?,
        });
    }
    match &e.kind {
        ExprKind::Var { .. } | ExprKind::Field { .. } | ExprKind::Deref(_) => Ok(Expr::Load {
            addr: access_of(p, e, true)?,
            ty: scalar_of(p, e)?,
        }),
        // `addr(x)` (ref p. 4-13): the address of the variable as a value.
        ExprKind::Builtin {
            builtin: Builtin::Addr,
            args,
        } => Ok(Expr::AddrOf(addr_of(p, &args[0])?)),
        ExprKind::Index { .. } => Ok(Expr::Load {
            addr: access_of(p, e, true)?,
            ty: scalar_of(p, e)?,
        }),
        ExprKind::Binary { op, lhs, rhs } => binary(p, e, *op, lhs, rhs),
        ExprKind::Unary { op, operand } => {
            let ty = scalar_of(p, e)?;
            let op = match op {
                UnOp::Neg => IrUnOp::Neg,
                UnOp::BitNot => IrUnOp::Not,
                UnOp::Plus => return Ok(convert(value_of(p, operand)?, ty)),
                // `not b` as a value: a condition materialised.
                UnOp::Not => return Ok(Expr::Test(Box::new(cond_of(p, e)?))),
            };
            Ok(Expr::Unary {
                op,
                operand: Box::new(convert(value_of(p, operand)?, ty)),
                ty,
            })
        }
        ExprKind::Builtin { builtin, args } => builtin_value(p, e, *builtin, args),
        // `addr(r)` of a routine (ref p. 4-145): its symbol's address, an
        // absolute operand like another unit's variable (codegen-patterns
        // 1.6: ATBUS_ 0xE0AB36 `move.l #DISK_INTERRUPT,-(SP)`, MAC_OS
        // 0xE0B9A8 `move.l #MAC_$DEMUX,(-8,A6)`, PROC1_ 0xE14B7A `move.l
        // #PROC1_$TS_END_CALLBACK,(24,A2)`).
        ExprKind::RoutineAddr(r) => Ok(Expr::AddrOf(Addr::Symbol {
            name: p.routine(*r).symbol.clone(),
            offset: 0,
        })),
        ExprKind::Transfer(x) => transfer(p, e, x),
        ExprKind::Call(c) => {
            // A pointer result comes back in A0 (ref 5.5.10, p. 5-20;
            // codegen-patterns 3, "Pointer results"); the caller reads it
            // there (codegen-patterns 3, "Callers of pointer functions":
            // MST_$MAPS 0xE439C8 `move.l A0,(-4,A6)`).  `d0_return` puts
            // it in D0 too, where the caller reads it like a data result
            // (no compiled site; ref 5.5.10).
            let reg = match &c.callee {
                SCallee::Routine(id) => match p.routine(*id).result.as_ref().map(|r| r.reg) {
                    Some(ResultReg::Address) => ResultClass::Address,
                    Some(ResultReg::AddressAndData) => ResultClass::AddressAndData,
                    _ => ResultClass::Data,
                },
                // Through a routine pointer: data results come back in D0
                // like a routine's (TTY_$I_ERR 0xE1BE22 `move.l D0,(-4,A6)`
                // after `jsr (A1)`; IO_$MAP_CSR 0xE1A8CC `move.l D0,D4`).
                // No compiled call through a pointer reads a pointer
                // result, so A0 (ref 5.5.10) is not assumed.
                SCallee::Pointer(_) => {
                    if p.types.is_pointer(e.ty) {
                        return Err(unsupported(
                            e.span,
                            "a pointer result of a call through a routine pointer is",
                        ));
                    }
                    ResultClass::Data
                }
            };
            let ty = scalar_of(p, e)?;
            Ok(Expr::Call {
                call: call(p, c, e.span)?,
                ty,
                reg,
            })
        }
        _ => Err(unsupported(e.span, "this expression is")),
    }
}

/// A type transfer `T(x)` as a value (ref "Type Transfer Functions",
/// p. 4-197: "the type transfer function does not change any bits in the
/// argument.  Domain Pascal just 'sees' the argument as a value of the new
/// type.  The one exception is that integer subranges are always
/// compatible regardless of their sizes").
///
/// - Between integer types (subranges included) the value is converted as
///   in an assignment: widened by its own sign ([`convert`]), or computed
///   at the narrower width where [`narrow`] can (p. 4-198, `INTEGER16(x +
///   y) > 5` "ignores the possibility of integer overflow"); other
///   narrowings are refused, as in an assignment (not catalogued).
/// - Otherwise the sizes are equal (`dpc-sema` checks it) and no code is
///   emitted: a load or a constant is retyped, any other value wrapped in
///   a [`Expr::Convert`] of the same width, which the selector evaluates
///   as its operand.  The register class follows the use: a pointer
///   computed in a data register is moved to an address register where it
///   is used as one (LOG_$READ's worker 0xE177D0..0xE177DA `move.l D1,D3;
///   add.l (0x14,A5),D3; movea.l D3,A1` for `univ_ptr(integer32(p) +
///   offset)`, p. 4-199's `UNIV_PTR(INTEGER32(namepointer) + 2)`;
///   codegen-patterns 5.2, "Pointer arithmetic").
fn transfer(p: &Program, e: &SExpr, x: &SExpr) -> Res<Expr> {
    let ty = scalar_of(p, e)?;
    let v = value_of(p, x)?;
    if v.ty().width != ty.width {
        // Only between integers (dpc-sema checks the sizes otherwise).
        if v.ty().bytes() < ty.bytes() {
            return Ok(convert(v, ty));
        }
        return narrow(&v, ty).ok_or_else(|| {
            unsupported(
                e.span,
                format!(
                    "a type transfer of a {}-byte integer to a {}-byte one (narrowing, as in an \
                     assignment) is",
                    v.ty().bytes(),
                    ty.bytes()
                ),
            )
        });
    }
    Ok(match v {
        Expr::Const { value, .. } => Expr::Const { value, ty },
        Expr::Load { addr, .. } => Expr::Load { addr, ty },
        v if v.ty() == ty => v,
        Expr::Convert { value, .. } if value.ty() == ty => *value,
        v => {
            debug_assert_eq!(v.ty().width, ty.width, "sema checks equal sizes");
            Expr::Convert {
                value: Box::new(v),
                ty,
            }
        }
    })
}

/// `v` as a value of type `ty`: a constant retyped, a value of another
/// width or sign wrapped in [`Expr::Convert`].
///
/// A sum, difference, product or negation widened is computed at the wider
/// width, its operands widened first (ref 4.3.1, p. 4-5: "The compiler
/// computes operands smaller than 32 bits with 32 bits of precision when
/// necessary to achieve correct arithmetic"; widening the word result
/// would lose the carry).  Image: 0xE0863C..0xE08642 `move.w D3,D2; ext.l
/// D3; subq.l #2,D3; move.l D3,(-176,A6)`; 0xE69568..0xE6956E `move.w
/// (-20,A2),D5; ext.l D5; addq.l #1,D5` (a longword argument);
/// 0xE07442..0xE0744A `ext.l D6; lsl.l #2,D6; move.l D6,D7; add.l D7,D7`
/// (×12 at the longword width).  No word `add`/`sub`/`mul` result is
/// widened with `ext.l` in 0xE00800..0xE88834 (review survey: what writes
/// the register an `ext.l` extends is a `move.w` ×355, `andi.w` ×2,
/// `asr.w`, `subq.w` before a `divs.w`, `divs.w`).  The bit operators,
/// `div`, `mod` and the right shifts give the same value either way and
/// stay as they are; a widened `lshft` is refused by the selector.
fn convert(v: Expr, ty: Scalar) -> Expr {
    match v {
        _ if v.ty() == ty => v,
        Expr::Const { value, .. } => Expr::Const { value, ty },
        Expr::Binary {
            op: op @ (IrBinOp::Add | IrBinOp::Sub | IrBinOp::Mul),
            lhs,
            rhs,
            ty: t,
        } if ty.bytes() > t.bytes() => Expr::Binary {
            op,
            lhs: Box::new(convert(*lhs, ty)),
            rhs: Box::new(convert(*rhs, ty)),
            ty,
        },
        Expr::Unary {
            op: IrUnOp::Neg,
            operand,
            ty: t,
        } if ty.bytes() > t.bytes() => Expr::Unary {
            op: IrUnOp::Neg,
            operand: Box::new(convert(*operand, ty)),
            ty,
        },
        v => Expr::Convert {
            value: Box::new(v),
            ty,
        },
    }
}

/// An operand of an integer operator, converted to the operation's type
/// `ty` (ref 4.3.1, p. 4-5: integer16 operands are expanded to 32 bits
/// when the operation is done in 32 bits).
fn operand(p: &Program, x: &SExpr, ty: Scalar) -> Res<Expr> {
    match &x.value {
        Some(v) => {
            let c = const_of(v, ty, x.span)?;
            let Expr::Const { value, .. } = c else {
                unreachable!()
            };
            if !fits(value, ty) {
                return Err(unsupported(
                    x.span,
                    "a constant operand outside the operation's type is",
                ));
            }
            Ok(c)
        }
        None => Ok(convert(value_of(p, x)?, ty)),
    }
}

/// `lhs op rhs` as a value.
fn binary(p: &Program, e: &SExpr, op: BinOp, lhs: &SExpr, rhs: &SExpr) -> Res<Expr> {
    let op = match op {
        BinOp::Add => IrBinOp::Add,
        BinOp::Sub => IrBinOp::Sub,
        BinOp::Mul => IrBinOp::Mul,
        BinOp::Div => IrBinOp::Div,
        BinOp::Mod => IrBinOp::Mod,
        BinOp::BitAnd => IrBinOp::And,
        BinOp::BitOr => IrBinOp::Or,
        BinOp::In => {
            // codegen-patterns 5.1: a set that fits in a register (one
            // word or one longword) is tested in a register (PROC1_$TST_LOCK
            // 0xE148DC `btst D0,D1`; REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A41A
            // `move.w (22,A0),D0; btst #13,D0`).  Larger sets (byte-addressed
            // `btst`, MST_$ALLOC_ASID 0xE42D5C) are not implemented yet.
            let set_size = p.types.size(rhs.ty);
            let set_width = match set_size {
                2 => Width::Word,
                4 => Width::Long,
                _ => return set_member_at(p, e, lhs, rhs, set_size),
            };
            return Ok(Expr::SetMember {
                elem: Box::new(value_of(p, lhs)?),
                set: Box::new(value_of(p, rhs)?),
                set_width,
            });
        }
        BinOp::Eq
        | BinOp::Ne
        | BinOp::Lt
        | BinOp::Le
        | BinOp::Gt
        | BinOp::Ge
        | BinOp::AndThen
        | BinOp::OrElse
        | BinOp::And
        | BinOp::Or => return Ok(Expr::Test(Box::new(cond_of(p, e)?))),
        BinOp::RealDiv | BinOp::Pow => return Err(unsupported(e.span, "this operator is")),
    };
    if !p.types.is_integer(e.ty) {
        // Set union, intersection and difference (codegen-patterns 5.1:
        // not catalogued).
        return Err(unsupported(e.span, "this operator on sets is"));
    }
    let mut ty = scalar_of(p, e)?;
    // The sum, product or bit combination of unsigned run-time values (or
    // of one and a non-negative constant) is unsigned: LOG_$READ's worker
    // compares `offset + actual_len^`, two `0..65535` words zero-extended
    // and added in 32 bits, with `bls` (0xE177B4..0xE177B8), as ref 4.3.3
    // (p. 4-6) says the compiler does for `div` and `mod` with such
    // operands.  **Uncertain:** one site; `-` is left signed (no site).
    let unsigned_operand = |x: &SExpr| match &x.value {
        Some(v) => v.ordinal().is_some_and(|n| n >= 0),
        None => sign_of(&p.types, x.ty) == Sign::Unsigned,
    };
    if matches!(op, IrBinOp::Add | IrBinOp::Mul | IrBinOp::And | IrBinOp::Or)
        && (lhs.value.is_none() || rhs.value.is_none())
        && unsigned_operand(lhs)
        && unsigned_operand(rhs)
    {
        ty.sign = Sign::Unsigned;
    }
    if matches!(op, IrBinOp::Div | IrBinOp::Mod) {
        // Ref 4.3.3, pp. 4-6..4-7: div and mod are unsigned when both
        // operands are unsigned run-time values, or one is and the other
        // is a constant in 0..2147483647; signed otherwise (image:
        // `divu.w` and M$DIU/M$OIU on `0..65535` words, codegen-patterns
        // 8).
        let runtime_unsigned =
            |x: &SExpr| x.value.is_none() && sign_of(&p.types, x.ty) == Sign::Unsigned;
        let non_negative = |x: &SExpr| {
            x.value
                .as_ref()
                .and_then(|v| v.ordinal())
                .is_some_and(|n| (0..=0x7fff_ffff).contains(&n))
        };
        let unsigned = (runtime_unsigned(lhs) || non_negative(lhs))
            && (runtime_unsigned(rhs) || non_negative(rhs))
            && (lhs.value.is_none() || rhs.value.is_none());
        ty.sign = if unsigned {
            Sign::Unsigned
        } else {
            Sign::Signed
        };
        // A word sum, difference or product as the dividend is computed
        // in 32 bits (ref 4.3.1, p. 4-5), and the division with it:
        // CAL_$WEEKDAY 0xE686AE..0xE686C0 `move.w (A1),D0; ext.l D0;
        // addi.l #9,D0; ... jsr M$OIS$WLW` for `(month + 9) mod 12`,
        // 0xE686CC..0xE686D4 `ext.l D0; addq.l #2,D0; divs.w #5,D0`,
        // 0xE686DA..0xE686EE (a sum of two words + 1, `mod 7`, M$OIS$WLW);
        // RIP_$SERVER 0xE68AE2..0xE68AE8 `move.w D2,D5; ext.l D5; subq.l
        // #2,D5; divs.w #6,D5`; TPAD_$DATA 0xE69392..0xE6939A `muls.w`
        // then `divs.w #2200` (the 32-bit product divided).  The
        // quotient or remainder is narrowed back to the word (the
        // selector divides in place, `divs.w`, or calls M$ARITH,
        // codegen-patterns 8).  **Exception (Uncertain):** ACL_$INIT
        // 0xE31150..0xE3116E computes `(i + 1) mod 31` and `(31 + i - 1)
        // mod 31` in 16 bits (`addq.w`, then `ext.l; divs.w #31`), `i` the
        // index of a `for` loop over 0..30 whose range the compiler may
        // have known; `dpc` has no such `for` loops.
        let widened = matches!(
            &lhs.kind,
            ExprKind::Binary {
                op: BinOp::Add | BinOp::Sub | BinOp::Mul,
                ..
            }
        ) && lhs.value.is_none()
            && ty.width == Width::Word;
        if widened {
            let lty = Scalar {
                width: Width::Long,
                sign: ty.sign,
            };
            return Ok(Expr::Convert {
                value: Box::new(Expr::Binary {
                    op,
                    lhs: Box::new(operand(p, lhs, lty)?),
                    rhs: Box::new(operand(p, rhs, lty)?),
                    ty: lty,
                }),
                ty,
            });
        }
    }
    Ok(Expr::Binary {
        op,
        lhs: Box::new(operand(p, lhs, ty)?),
        rhs: Box::new(operand(p, rhs, ty)?),
        ty,
    })
}

/// A predeclared function's value.
fn builtin_value(p: &Program, e: &SExpr, b: Builtin, args: &[SExpr]) -> Res<Expr> {
    let ty = scalar_of(p, e)?;
    match b {
        Builtin::Lshft | Builtin::Rshft | Builtin::Arshft => {
            // Ref pp. 4-29, 4-171: rshft and arshft of an operand smaller
            // than 32 bits expand it to 32 bits first, shift, and convert
            // the result back where it is stored; what the original emitted
            // for a 16-bit signed operand (an `ext.l` and a 32-bit shift?)
            // is not catalogued, so only operands of the result's width are
            // taken (lshft keeps its operand's width, p. 4-109).
            let x = value_of(p, &args[0])?;
            if x.ty() != ty {
                return Err(unsupported(
                    e.span,
                    "a shift of an operand of another width than its result is",
                ));
            }
            let op = match b {
                Builtin::Lshft => IrBinOp::Shl,
                Builtin::Rshft => IrBinOp::Shr,
                _ => IrBinOp::Ashr,
            };
            if op == IrBinOp::Shr && x.ty().width != Width::Long && x.ty().is_signed() {
                // P. 4-171: "rshft expands num to a 32-bit integer, performs
                // the shift, and converts it back", so rshft(-9, 3) is -2 in
                // 16 bits, not what `lsr.w` gives; the original's code for it
                // is not catalogued.
                return Err(unsupported(
                    e.span,
                    "`rshft` of a signed operand smaller than 32 bits (expanded first, ref \
                     p. 4-171) is",
                ));
            }
            let count = match &args[1].value {
                Some(v) => const_of(v, Scalar::signed(Width::Word), args[1].span)?,
                None => value_of(p, &args[1])?,
            };
            Ok(Expr::Binary {
                op,
                lhs: Box::new(x),
                rhs: Box::new(count),
                ty,
            })
        }
        Builtin::Xor => Ok(Expr::Binary {
            op: IrBinOp::Xor,
            lhs: Box::new(operand(p, &args[0], ty)?),
            rhs: Box::new(operand(p, &args[1], ty)?),
            ty,
        }),
        // `ord(x)` (p. 4-137): the ordinal as an integer.  A char is
        // zero-extended, an enumeration word or an integer kept; a boolean's
        // ordinal (1 for its 0xFF byte) is not a plain conversion.
        Builtin::Ord if !p.types.is_boolean(args[0].ty) => Ok(convert(value_of(p, &args[0])?, ty)),
        // `chr(n)` (p. 4-39): "number & 16#FF", the low byte.
        Builtin::Chr => Ok(convert(value_of(p, &args[0])?, ty)),
        // Appendix E, p. E-5: the new status word, a word.
        Builtin::SetSr => {
            let w = Scalar::signed(Width::Word);
            let x = match args[0].value.as_ref().and_then(|v| v.ordinal()) {
                // A constant as the 16 bits it puts in SR (16#2700,
                // 0xE149B8; up to 16#FFFF).
                Some(k) => Expr::Const {
                    value: k as u16 as i16 as i64,
                    ty: w,
                },
                None => operand(p, &args[0], w)?,
            };
            Ok(Expr::SetSr(Box::new(x)))
        }
        // "Succ", p. 4-191 / "Pred", p. 4-147: "The numerical value equal to
        // x + 1" ("x-1"), the next (previous) character or identifier, with
        // no range check (Appendix D, p. D-5); the result has x's type.
        // Booleans: true (false) whatever x is; a constant x is folded by
        // `dpc-sema`, a computed one is not supported (whether x is
        // evaluated is not known).
        Builtin::Succ | Builtin::Pred if !p.types.is_boolean(args[0].ty) => Ok(Expr::Binary {
            op: if b == Builtin::Succ {
                IrBinOp::Add
            } else {
                IrBinOp::Sub
            },
            lhs: Box::new(convert(value_of(p, &args[0])?, ty)),
            rhs: Box::new(Expr::Const { value: 1, ty }),
            ty,
        }),
        // "Max", p. 4-111 / "Min", p. 4-113 / "In_range", p. 4-104: the
        // image's `x := min(x, c)`-shaped sites (codegen-patterns 12.4 R8,
        // 0xE2F08A `move.w #504,D0; cmp.w D0,D2; ble.s; move.w D0,D2`) are
        // indistinguishable from an `if`; no site that must be `max`,
        // `min` or `in_range` is known (codegen-patterns 10.1).
        Builtin::Max | Builtin::Min | Builtin::InRange => Err(unsupported(
            e.span,
            "a run-time `max`, `min` or `in_range` (no image pattern known, codegen-patterns \
             10.1) is",
        )),
        _ => Err(unsupported(e.span, "this predeclared function is")),
    }
}

/// The IR scalar type of `e`'s type.
fn scalar_of(p: &Program, e: &SExpr) -> Res<Scalar> {
    let size = p.types.size(e.ty);
    let width = Width::of_size(size)
        .ok_or_else(|| unsupported(e.span, format!("a {size}-byte value is")))?;
    Ok(Scalar {
        width,
        sign: sign_of(&p.types, e.ty),
    })
}

/// Whether values of type `ty` are read as signed or unsigned numbers.
///
/// - `integer`/`integer16` and `integer32`: signed (ref 3.2.1, p. 3-4).
/// - A subrange stored in 2 bytes although its host is `integer32`, i.e.
///   `lo >= 0` and `hi` in 32768..=65535 (`dpc-sema` `types.rs`, after
///   ref 5.3.4.1 p. 5-13 and Table 7-2 p. 7-30, "unsigned short"):
///   unsigned.  Image: LOG_$READ's worker 0xE17778 zero-extends such
///   words (`clr.l D1; move.w D0,D1` at 0xE177A6..0xE177AA) and compares
///   them unsigned (`cmpi.w #0x400,D1; bcc` at 0xE1779A; `bls` at
///   0xE177BA).  Its C (`log/read_internal.c`) has `uint16_t`; the
///   Pascal type is our inference.
/// - Any other subrange: its host's sign.  **Uncertain:** whether the
///   original treated a non-negative `integer16` subrange (`0..100`) as
///   unsigned in compares; not observed yet.
/// - `char`: unsigned (ordinals 0..255, ref 3.6.4 p. 3-13).
/// - `boolean`: signed ([`Scalar::BOOLEAN`]).
/// - Enumerations: unsigned (ordinals 0..n-1, ref 3.7.1 p. 3-13).
///   **Uncertain:** enumeration compares have not been catalogued.
/// - Pointers, sets and everything else (bit patterns): unsigned.
pub(crate) fn sign_of(types: &TypeTable, ty: TypeId) -> Sign {
    match types.kind(ty) {
        TypeKind::Integer16 | TypeKind::Integer32 => Sign::Signed,
        TypeKind::Boolean => Scalar::BOOLEAN.sign,
        TypeKind::Subrange { base, .. } => {
            let widened_host = matches!(types.kind(*base), TypeKind::Integer32);
            if widened_host && types.size(ty) < types.size(*base) {
                Sign::Unsigned
            } else {
                sign_of(types, *base)
            }
        }
        _ => Sign::Unsigned,
    }
}

/// A bit field of a packed record, `e` (ref 3.10.7): a field that is
/// not a whole object of its type at a byte or word address
/// ([`dpc_sema::types::BitPos::is_whole_object`]; whole ones are read and
/// written like unpacked fields).  Gives the byte, word or longword that
/// holds it, the smallest of the three the field fits in that the 68010
/// can access (a word or longword at an even address), and where the
/// field lies in it.  Image (codegen-patterns 5.3): a byte for a field
/// within one byte (0xE0089A `andi.b #127,(39,A2)`, 0xE158BA `and.b
/// (16,A0),D0`), a word for one in the two bytes of an even word (0xE00994
/// `andi.w #-0x7f81,(40,A2)`, 0xE5BFBE `and.w (16,A0),D1`), a longword
/// for one across an odd byte boundary (0xE0086E `andi.l #-0x7f8001,
/// (38,A2)`, an 8-bit field in bytes 39..40; 0xE0E346 `andi.l
/// #-0xffff01,(4,A2)`, a 16-bit one in bytes 5..6).
fn bit_field(p: &Program, e: &SExpr) -> Res<Option<(Addr, BitField)>> {
    let ExprKind::Field {
        base,
        bits: Some(b),
        ..
    } = &e.kind
    else {
        return Ok(None);
    };
    if b.is_whole_object(p.types.size(e.ty)) {
        return Ok(None);
    }
    let (first, last) = (b.bit / 8, (b.bit + b.width - 1) / 8);
    let (unit, start) = if first == last {
        (Width::Byte, first)
    } else if first % 2 == 0 && last == first + 1 {
        (Width::Word, first)
    } else if last < (first & !1) + 4 {
        (Width::Long, first & !1)
    } else {
        return Err(unsupported(
            e.span,
            "a packed field wider than a longword is",
        ));
    };
    let shift = 8 * (start + unit.bytes()) - (b.bit + b.width);
    let addr = wrap_volatile(p, e, addr_of(p, base)?.offset_by(start as i32));
    Ok(Some((
        addr,
        BitField {
            unit,
            shift,
            width: b.width,
        },
    )))
}

/// The value type of bit field `e` ([`bit_field`]): a boolean of one bit,
/// or an ordinal of non-negative values (an enumeration, a subrange with a
/// low bound of 0 or more), zero-extended.  A subrange with negative
/// values (`-100..40`, ref Figure 3-17) would need its sign extended
/// (not catalogued), a set field is not an operand yet.
fn bit_field_type(p: &Program, e: &SExpr) -> Res<Scalar> {
    let t = &p.types;
    if t.is_boolean(e.ty) {
        return Ok(Scalar::BOOLEAN);
    }
    match t.ordinal_range(e.ty) {
        Some((lo, _)) if lo >= 0 => scalar_of(p, e),
        Some(_) => Err(unsupported(
            e.span,
            "a packed field of a subrange with negative values (its sign extension is not \
             catalogued) is",
        )),
        None => Err(unsupported(
            e.span,
            format!(
                "a packed field of type {} that is not a whole byte, word or longword is",
                t.display(e.ty)
            ),
        )),
    }
}

/// `d := e` for a bit field `d` ([`bit_field`]; codegen-patterns 5.3).
///
/// A value wider than the field's type is kept at its width: the
/// selector reads, or uses, only its low bytes (PACCT's module-local
/// 0xE5A9CA merges a longword held in D0 into a 13-bit field with `move.w
/// D0,D3; or.w D3,(-8,A6)`).  A narrower one is widened like any
/// assignment's value.
///
/// `mask`: the value is masked to the field's bits before it is merged
/// when its type admits negative values (0xE31DF2..0xE31E00: a value
/// sign-extended with `ext.w`, then `lsl.w #4,D1; andi.w #0x3f0,D1; or.w
/// D1,(-6,A6)`), not when its type is non-negative, however wide
/// (0xE5A9CA: an unsigned longword, compared with `bcc`, merged into 13
/// bits unmasked; 0xE1817A `move.b (3,A1),D1; lsl.b #3,D1; or.b
/// D1,(218,A0)`).  **Uncertain:** fitted to these sites; the masked byte
/// merges 0xE38C72 and 0xE5BD6E have values of unknown type.
fn store_bits(
    p: &Program,
    s: &SStmt,
    target: &SExpr,
    value: &SExpr,
    addr: Addr,
    field: BitField,
) -> Res<Stmt> {
    let ty = bit_field_type(p, target)?;
    if let Some(a) = volatile_access(p, target) {
        check_device_access(p, target, false)?;
        // The other bits of the unit are read and written back (`and`/`or`
        // into memory, `bset`/`bclr`): for a device, ref 3.15.3 p. 3-57
        // forbids "gratuitous read-modify-write references"; for any
        // volatile object the field form writes the unit twice (`andi`
        // then `ori`), and whether the original did so for one is not
        // known.
        return Err(unsupported(
            s.span,
            if a.device {
                "storing into a packed field of a device record (a read-modify-write of its \
                 unit, ref 3.15.3) is"
            } else {
                "storing into a packed field of a volatile record (its unit read and written \
                 back, perhaps twice; not seen, ref 3.15.1) is"
            },
        ));
    }
    let (v, mask) = if let Some(c) = &value.value {
        (const_of(c, ty, value.span)?, false)
    } else {
        let v = value_of(p, value)?;
        let v = if v.ty().bytes() < ty.bytes() {
            if !p.types.is_integer(value.ty) {
                return Err(unsupported(
                    value.span,
                    format!(
                        "assigning a {}-byte value to a packed field of a {}-byte type is",
                        v.ty().bytes(),
                        ty.bytes()
                    ),
                ));
            }
            convert(v, ty)
        } else {
            v
        };
        let negative =
            ty != Scalar::BOOLEAN && p.types.ordinal_range(value.ty).is_none_or(|(lo, _)| lo < 0);
        (v, negative)
    };
    Ok(Stmt {
        kind: StmtKind::StoreBits {
            addr,
            field,
            value: v,
            mask,
        },
        span: s.span,
    })
}

/// How designator `e` is referenced when it is volatile (ref
/// 3.15.1-3.15.3), or `None`.  The attributes of `e`'s type, and the
/// `volatile` and `device` of the records and arrays it is a field or an
/// element of (ref 3.15.7 pp. 3-76..3-77: "If you define a record type as
/// volatile or device, all the fields within the record inherit the
/// attribute"; an array of a volatile type is volatile), up to a
/// dereference: "Pointer types do not inherit the device or volatile
/// attributes of their base type.  However, when pointer variables are
/// dereferenced, the system applies any attributes of their base type"
/// (p. 3-77).  `atomic` is the object's own (ref 3.15.2: scalar variables
/// only).
fn volatile_access(p: &Program, e: &SExpr) -> Option<Access> {
    fn inherited(p: &Program, e: &SExpr) -> (bool, bool) {
        let a = p.types.attrs(e.ty);
        let (mut v, mut d) = (a.is_volatile(), a.device.is_some());
        if let ExprKind::Field { base, .. } | ExprKind::Index { base, .. } = &e.kind {
            let (bv, bd) = inherited(p, base);
            v |= bv;
            d |= bd;
        }
        (v, d)
    }
    let (v, d) = inherited(p, e);
    v.then(|| Access {
        device: d,
        atomic: p.types.attrs(e.ty).atomic,
    })
}

/// The `device(read)` / `device(write)` restriction of `e` (ref 3.15.3 p.
/// 3-59: "if you attempt to write to this variable, the compiler flags the
/// attempt as invalid"; reading a write-only one likewise), the nearest
/// one up to a dereference.
fn check_device_access(p: &Program, e: &SExpr, read: bool) -> Res<()> {
    let mut x = e;
    loop {
        if let Some(d) = p.types.attrs(x.ty).device {
            if read && !d.read {
                return Err(Diagnostic::error(
                    e.span,
                    "reading a write-only device object (`device(write)`, ref 3.15.3, p. 3-59)",
                ));
            }
            if !read && !d.write {
                return Err(Diagnostic::error(
                    e.span,
                    "writing a read-only device object (`device(read)`, ref 3.15.3, p. 3-59)",
                ));
            }
            return Ok(());
        }
        match &x.kind {
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => x = base,
            _ => return Ok(()),
        }
    }
}

/// `addr`, the address of designator `e`, wrapped in [`Addr::Volatile`]
/// when `e` is volatile.
fn wrap_volatile(p: &Program, e: &SExpr, addr: Addr) -> Addr {
    match volatile_access(p, e) {
        Some(access) => Addr::Volatile {
            addr: Box::new(addr),
            access,
        },
        None => addr,
    }
}

/// The address of designator `e` for a load (`read`) or a store of it:
/// [`addr_of`], wrapped when `e` is volatile ([`Addr::Volatile`]), after
/// the `device(read)`/`device(write)` check.
fn access_of(p: &Program, e: &SExpr, read: bool) -> Res<Addr> {
    check_device_access(p, e, read)?;
    check_even(p, e)?;
    Ok(wrap_volatile(p, e, addr_of(p, e)?))
}

/// Refuse a reference to an object of more than one byte that an
/// alignment attribute lets lie at an odd address (`aligned(0)`, ref
/// 3.15.6.3 p. 3-69: `b, c: [ALIGNED(0)] integer16` at offsets 1 and 3),
/// unless its address is known to be even.  The 68010 traps on a word or
/// longword reference at an odd address; the reference describes split
/// references for objects known not to be aligned only for the Series
/// 10000 (3.15.6.6 p. 3-74), and how the original referenced such an
/// object on the 68000 family is not known.  A record, an array's first
/// element and an object reached through a pointer are taken to be at
/// least word aligned (ref 3.10.5.5 p. 3-26, "The default alignment of the
/// beginning of a record is at least word aligned"; 3.15.6.7 p. 3-75, a
/// pointer's target is assumed naturally aligned, unless the pointer's
/// base type has an alignment attribute), as is every frame
/// object (codegen-patterns 2).  An object inside a byte-aligned array
/// or record counts too.
fn check_even(p: &Program, e: &SExpr) -> Res<()> {
    /// The parity of `e`'s address: `Some(true)` even, `Some(false)` odd,
    /// `None` not known.
    fn even(p: &Program, e: &SExpr) -> Option<bool> {
        match &e.kind {
            ExprKind::Field { base, offset, .. } => Some(even(p, base)? == (offset % 2 == 0)),
            ExprKind::Index {
                base,
                index,
                lo,
                elem_size,
            } => {
                let b = even(p, base)?;
                if elem_size % 2 == 0 {
                    return Some(b);
                }
                let i = index.value.as_ref().and_then(ConstValue::ordinal)?;
                Some(b == ((i - lo) % 2 == 0))
            }
            // The target is aligned as the pointer's base type says (ref
            // 3.15.6.7 pp. 3-75..3-76: `iptr: ^word_int32` "points to a
            // word aligned 4-byte integer"): even unless byte aligned.
            ExprKind::Deref(_) => (p.types.get(e.ty).align >= 2).then_some(true),
            ExprKind::Var { var, .. } => match &p.var(*var).storage {
                Storage::Static { offset, .. } => Some(offset % 2 == 0),
                Storage::Frame { .. } | Storage::Param { .. } => Some(true),
                // Another unit's object: aligned as its type says.
                Storage::Extern { .. } => (p.types.get(e.ty).align >= 2).then_some(true),
            },
            _ => None,
        }
    }
    /// Whether `e` or an array or record it is an element or a field of
    /// (up to a dereference) is byte aligned: an element of a byte-aligned
    /// array (`x: [aligned(0)] array[1..2] of integer16` after a char)
    /// may lie at an odd address although its own type has no attribute.
    fn byte_aligned(p: &Program, e: &SExpr) -> bool {
        if p.types.attrs(e.ty).align == Some(1) {
            return true;
        }
        match &e.kind {
            ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => byte_aligned(p, base),
            _ => false,
        }
    }
    if p.types.get(e.ty).size > 1 && byte_aligned(p, e) && even(p, e) != Some(true) {
        return Err(unsupported(
            e.span,
            "a reference to a byte-aligned object (`aligned(0)`) of more than one byte at an \
             address not known to be even (the 68010 traps on an odd word or longword access; \
             the original's code for one is not known, ref 3.15.6.6) is",
        ));
    }
    Ok(())
}

/// The address of designator `e`.
fn addr_of(p: &Program, e: &SExpr) -> Res<Addr> {
    if bit_field(p, e)?.is_some() {
        // The checker refuses such a field where an address is needed
        // (ref 3.10.3); any other use that would need one is refused here.
        return Err(unsupported(
            e.span,
            "the address of a field of a packed record that is not a whole byte, word or \
             longword is",
        ));
    }
    match &e.kind {
        // A variable of the enclosing routine (ref 2.4), through the static
        // link: its frame objects as [`Addr::Enclosing`] (codegen-patterns
        // 6: 0xE135AA `btst #0,(0x15,A2)`, a by-value parameter of
        // PMAP_$FLUSH; 0xE135B8 `move.l (8,A2),-(SP)`, its by-address slot
        // passed on; 0xE1362E `move.w (-0x50,A4),-(SP)`, its local).
        ExprKind::Var { var, uplevel } => {
            let a = var_addr(p, *var, e.span)?;
            match uplevel {
                0 => Ok(a),
                1 => Ok(enclosing(a)),
                _ => Err(unsupported(
                    e.span,
                    "a reference to a variable more than one routine out (a chain of links, \
                     DIR 0xE4F034, codegen-patterns 6) is",
                )),
            }
        }
        ExprKind::Field { base, offset, .. } => Ok(addr_of(p, base)?.offset_by(*offset as i32)),
        ExprKind::Deref(ptr) => Ok(Addr::Indirect {
            base: Box::new(value_of(p, ptr)?),
            offset: 0,
        }),
        // `a[i]`: the element at `(i - lo) * size` bytes into the array
        // (codegen-patterns 5.2).  A constant index is folded into the
        // displacement; otherwise the lower bound is (`(-2,A5,D0.w)` for
        // a 1-based word array, AUDIT_$IS_PROCESS_AUDITED 0xE70DA8).  No
        // range check is emitted: the image has none (FIM_$FREE_ASID's C,
        // `fim/free_asid.c`, notes "the original performs no bounds check").
        ExprKind::Index {
            base,
            index,
            lo,
            elem_size,
        } => {
            let b = addr_of(p, base)?;
            let big = || unsupported(e.span, "an array offset beyond 32 bits is");
            if let Some(i) = index.value.as_ref().and_then(ConstValue::ordinal) {
                let d = i32::try_from((i - lo) * *elem_size as i64).map_err(|_| big())?;
                return Ok(b.offset_by(d));
            }
            let mut ix = value_of(p, index)?;
            if ix.ty().width == Width::Byte {
                // A byte index of an unsigned type (`char`, a char
                // subrange) is zero-extended to a word, `clr.w Dn; move.b
                // x,Dn`, then scaled and used as a `.w` index (codegen-
                // patterns 5.2, "Byte indices": 21 of the 27 byte indices
                // of compiled code, e.g. tty_$i_enable_crash_func 0xE672AA..
                // 0xE672B4, TTY_$K_SET_INPUT_BREAK_MODE 0xE67874; 5 more
                // with the `clr.w` earlier, one with `clr.l`).  A signed one
                // (a boolean) would need `ext.w`, which no byte index of the
                // image has.
                if ix.ty().sign != Sign::Unsigned {
                    return Err(unsupported(
                        index.span,
                        "an array index of a signed byte type (no `ext.w` index in the image, \
                         codegen-patterns 5.2) is",
                    ));
                }
                ix = convert(ix, Scalar::unsigned(Width::Word));
            }
            if ix.ty().width == Width::Word {
                // A word index is scaled at the word's width and used as
                // `(d,An,Dn.w)`, which sign-extends it (MC68000 brief
                // extension word): right only while every index × size of
                // the array's bounds fits in -32768..32767.  An index of a
                // `0..65535` array above 32767, or a scaled index beyond 16
                // bits, would address the wrong element.  The image scales
                // with `mulu.w` into a `.l` index too (codegen-patterns 5.2:
                // 43 sites, 0xE39364); when it did is not catalogued.
                let fits16 = match p.types.kind(base.ty) {
                    TypeKind::Array { index: it, .. } => {
                        p.types.ordinal_range(*it).is_some_and(|(alo, ahi)| {
                            let s = *elem_size as i64;
                            alo * s >= -0x8000 && ahi * s <= 0x7fff
                        })
                    }
                    _ => false,
                };
                if !fits16 {
                    return Err(unsupported(
                        index.span,
                        "a word index whose scaled value may not fit in 16 bits (a `.w` index \
                         sign-extends; the `.l` form is not catalogued, codegen-patterns 5.2) is",
                    ));
                }
            }
            let offset = i32::try_from(-lo * *elem_size as i64).map_err(|_| big())?;
            Ok(Addr::Index {
                base: Box::new(b),
                index: Box::new(ix),
                scale: *elem_size,
                offset,
            })
        }
        _ => Err(unsupported(e.span, "this designator is")),
    }
}

/// `a`, an address in the enclosing routine's terms, as seen from a
/// routine nested in it: its frame objects through the static link.
fn enclosing(a: Addr) -> Addr {
    match a {
        Addr::Frame(o) => Addr::Enclosing(o),
        Addr::Indirect { base, offset } => match *base {
            Expr::Load {
                addr: Addr::Frame(s),
                ty,
            } => Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Enclosing(s),
                    ty,
                }),
                offset,
            },
            b => Addr::Indirect {
                base: Box::new(b),
                offset,
            },
        },
        other => other,
    }
}

fn var_addr(p: &Program, id: VarId, span: Span) -> Res<Addr> {
    let v = p.var(id);
    match &v.storage {
        Storage::Extern { symbol } => Ok(Addr::Symbol {
            name: symbol.clone(),
            offset: 0,
        }),
        Storage::Frame { offset } => Ok(Addr::Frame(*offset)),
        Storage::Param { index } => {
            let owner = v.owner.expect("a parameter has an owner routine");
            let prm = &p.routine(owner).params[*index];
            match prm.passing() {
                // val_param word at (8,A6): PROC1_$TST_LOCK 0xE148D4.
                Passing::ByValue => Ok(Addr::Frame(prm.value_offset)),
                // A value parameter passed by address: the body refers to
                // the callee's copy (ref 5.2 Table 5-1, p. 5-5, "makes
                // direct references to them"), so an assignment never
                // reaches the caller's object, and `addr` or passing it on
                // by address names the copy (DIR_$GET_ENTRYU 0xE4D55E `pea
                // (-8,A6)`, its UID copied at 0xE4D512..0xE4D51A).
                Passing::ByAddress if prm.copy_offset.is_some() => {
                    Ok(Addr::Frame(prm.copy_offset.unwrap()))
                }
                // By address: the slot holds a pointer (LOG_$READ 0xE17810
                // `movea.l (0xc,A6),A0; move.w (A0),...`).
                Passing::ByAddress => {
                    let width = Width::of_size(prm.conv.slot_size)
                        .ok_or_else(|| unsupported(span, "an address slot of this size is"))?;
                    Ok(Addr::Indirect {
                        base: Box::new(Expr::Load {
                            addr: Addr::Frame(prm.slot_offset),
                            ty: Scalar::unsigned(width),
                        }),
                        offset: 0,
                    })
                }
            }
        }
        // The unit's data area (codegen-patterns 4): `(d,A5)` on m68k.
        // Named data sections were rejected by `data_area`.
        Storage::Static { offset, .. } => i32::try_from(*offset)
            .map(Addr::Static)
            .map_err(|_| unsupported(span, "a static variable at this offset is")),
    }
}

/// A procedure call statement (ref 5.2).
///
/// - The callee: a routine of this unit with a body is called
///   PC-relative ([`Callee::Local`], `bsr`), any other declared routine
///   through its symbol ([`Callee::External`], `jsr abs.l`):
///   codegen-patterns 1.6 (0xE00F02 `bsr.s` to the module-local
///   0xE00EB0; 0xE00EF6 `jsr ML_$LOCK`).  A routine of this unit in
///   another code section than the caller's becomes [`Callee::Section`]
///   once the caller is lowered ([`set_call_sections`]), so that calls
///   in statements and in expressions see the caller's section alike
///   (bead domain-pascal-86o).  Through a routine
///   pointer ([`Callee::Pointer`], `jsr (An)`, 0xE0B3F0): the conventions,
///   modes and pad of the pointer type's signature (`val_param` on the
///   type, ref p. 4-146; [`routine_pointer`]).
/// - The arguments, in parameter order, each with its parameter's
///   convention from `dpc-sema` (ref 5.2 Table 5-1): by value, the value
///   at the parameter's width (a constant takes the parameter's type);
///   by address, the address of a designator; a constant by address is
///   its cell in the code section ([`Addr::Const`], `pea (d,PC)`; for a
///   `univ` parameter see `univ_const_type`), an
///   expression by address its frame temporary ([`Arg::Temp`]):
///   codegen-patterns 1.5 (TERM_$SEND_KBD_STRING 0xE1AB0C, TONE_$ENABLE
///   0xE1ACFC).  A variable of another size than its value or `in`
///   parameter (a converted copy) is not supported yet.
/// - The pad: the callee's `arg_pad` (codegen-patterns 1.4).  A
///   `variable` routine called with fewer arguments (ref 5.5.5) is not
///   supported yet: its pad and slots are not catalogued.
fn call(p: &Program, c: &SCall, span: Span) -> Res<Call> {
    // The parameters as (mode, univ, type), and the pad.
    let (callee, params, pad): (Callee, Vec<(ParamMode, bool, TypeId)>, u32) = match &c.callee {
        SCallee::Routine(id) => {
            let r = p.routine(*id);
            let callee = if r.external || r.body.is_none() {
                Callee::External(r.symbol.clone())
            } else {
                Callee::Local(r.symbol.clone())
            };
            let params = r.params.iter().map(|q| (q.mode, q.univ, q.ty)).collect();
            (callee, params, r.arg_pad)
        }
        SCallee::Pointer(x) => {
            let TypeKind::RoutinePointer(sig) = p.types.kind(x.ty) else {
                unreachable!("a call through a value that is not a routine pointer")
            };
            let params = sig.params.iter().map(|q| (q.mode, q.univ, q.ty)).collect();
            (
                Callee::Pointer(Box::new(routine_pointer(p, x)?)),
                params,
                sig.arg_pad,
            )
        }
    };
    if c.args.len() != params.len() {
        return Err(unsupported(
            span,
            "a call with fewer arguments than parameters (ref 5.5.5) is",
        ));
    }
    let mut args = Vec::new();
    for (a, &(mode, univ, prm_ty)) in c.args.iter().zip(&params) {
        let slot = a.conv.slot_size;
        args.push(match a.conv.passing {
            Passing::ByValue => {
                let size = p.types.size(prm_ty);
                let width = Width::of_size(size).ok_or_else(|| {
                    unsupported(a.expr.span, format!("a {size}-byte value argument is"))
                })?;
                let ty = Scalar {
                    width,
                    sign: sign_of(&p.types, prm_ty),
                };
                let value = match &a.expr.value {
                    Some(v) => const_of(v, ty, a.expr.span)?,
                    // A set expression takes the parameter's size (`[k]`
                    // pushed: DISK_$FORMAT 0xE3D4E2..0xE3D4E6 `clr.w D1;
                    // bset D3,D1; move.w D1,-(SP)`, codegen-patterns 5.1).
                    None if is_set(p, prm_ty) => set_value(p, &a.expr, ty)?,
                    None => {
                        let v = value_of(p, &a.expr)?;
                        if v.ty().width != width {
                            return Err(unsupported(
                                a.expr.span,
                                format!(
                                    "passing a {}-byte value to a {}-byte parameter is",
                                    v.ty().bytes(),
                                    width.bytes()
                                ),
                            ));
                        }
                        v
                    }
                };
                Arg::Value { value, slot }
            }
            Passing::ByAddress => {
                if a.needs_temp && a.expr.is_designator() {
                    return Err(unsupported(
                        a.expr.span,
                        "passing a variable of another size to a value or `in` parameter (a \
                         converted copy, codegen-patterns 1.5) is",
                    ));
                }
                if let Some(v) = &a.expr.value {
                    // A constant: its cell in the code section (codegen-
                    // patterns 1.5, "Constant by address"), holding the
                    // value as the parameter's type (a `univ` one: see
                    // `univ_const_type`).
                    let ty = if univ {
                        univ_const_type(p, a.expr.ty, prm_ty, a.expr.span)?
                    } else {
                        prm_ty
                    };
                    args.push(Arg::Address {
                        addr: Addr::Const(pool_const(p, v, ty, a.expr.span)?),
                        slot,
                        writable: false,
                    });
                    continue;
                }
                if !a.expr.is_designator() {
                    // An expression: stored into a frame temporary, whose
                    // address is passed (codegen-patterns 1.5, TONE_$ENABLE
                    // 0xE1ACFC); the offset is assigned once the body is
                    // lowered (`assign_temps`).
                    let width = Width::of_size(p.types.size(prm_ty)).ok_or_else(|| {
                        unsupported(
                            a.expr.span,
                            format!(
                                "a {}-byte expression passed by address is",
                                p.types.size(prm_ty)
                            ),
                        )
                    })?;
                    let value = value_of(p, &a.expr)?;
                    // The temporary has the parameter's size.  An
                    // expression of another size (to a `univ` parameter:
                    // extended to its size, ref 5.3.3 p. 5-11; to a
                    // larger value or `in` one: assignment compatible,
                    // ref 5.3.2 p. 5-7) needs a conversion whose code is
                    // not catalogued (bead domain-pascal-4be).
                    if value.ty().width != width {
                        return Err(unsupported(
                            a.expr.span,
                            format!(
                                "passing a {}-byte expression by address to a {}-byte parameter is",
                                value.ty().bytes(),
                                width.bytes()
                            ),
                        ));
                    }
                    args.push(Arg::Temp {
                        value,
                        offset: 0,
                        slot,
                    });
                    continue;
                }
                Arg::Address {
                    addr: addr_of(p, &a.expr)?,
                    slot,
                    // ref 5.3.1 p. 5-6; 5.3.2 p. 5-7 (bead domain-pascal-pe9);
                    // through a routine pointer, the mode of the pointer
                    // type's parameter (bead domain-pascal-5aw.16).
                    writable: matches!(mode, ParamMode::Var | ParamMode::Out | ParamMode::InOut),
                }
            }
        });
    }
    Ok(Call {
        callee,
        args,
        pad,
        // Set for a nested callee by `set_static_links`.
        link: None,
    })
}

/// The routine pointer a call goes through, as a value (codegen-patterns
/// 1.6).  Every compiled `jsr (An)` whose register is defined in the
/// straight-line code before it reads a pointer loaded from memory or
/// copied from a register right after the pushes: 69 sites, e.g. KBD_$RCV
/// 0xE1CE5C `movea.l (A2),A0`, RING_INTERRUPT 0xE0AB1C `movea.l (A5),A0`,
/// TTY_$I_ERR 0xE1BE1A `movea.l (0x2c0,A0),A1`.  So the pointer must be a
/// variable, parameter, field or element (a load); a function result as
/// the pointer (`rf^^`, which the reference forbids, p. 4-145) or any
/// other value is refused.
fn routine_pointer(p: &Program, x: &SExpr) -> Res<Expr> {
    let v = value_of(p, x)?;
    // A function call inside the pointer's address (`tbl[f(i)]^(...)`)
    // would run after the pushes, where no compiled call is seen: every
    // load chain after the last push is moves and index arithmetic
    // (codegen-patterns 1.6).  Refused rather than guessed.
    let mut calls = Vec::new();
    expr_calls(&v, &mut calls);
    if !calls.is_empty() {
        return Err(unsupported(
            x.span,
            "a function call in the routine pointer of a call through a pointer is",
        ));
    }
    match &v {
        Expr::Load {
            ty: Scalar {
                width: Width::Long, ..
            },
            ..
        } => Ok(v),
        _ => Err(unsupported(
            x.span,
            "a call through a routine pointer that is not a variable, field or element is",
        )),
    }
}

/// `for v := lo to hi do a[v] := c` where `v` is a local variable used
/// for nothing but the index, `lo`, `hi` and `c` are constants and the
/// element is a scalar: [`StmtKind::Fill`] of `hi - lo + 1` elements
/// from `a[lo]`.
///
/// Image: module-local 0xE00EB0 (`misc/zero_page.c`), 0xE00ECC..0xE00EDA
/// `movea.l #AST_$ZERO_BUFF,A0; move.w #0xFF,D0; movea.l A0,A0; clr.l
/// (A0)+; dbf D0`: 256 longwords cleared, no register or frame slot for
/// an index.  The compiler strength-reduced indexing in loops to a
/// pointer that steps by the element size, with the index's offset
/// folded in (PKT_$DUMP_DATA 0xE12800 `lea (4,A0),A2`, then `(-4,A2)`
/// and `addq.l #4,A2` at 0xE12816/0xE12824, where the index is also used
/// in a compare and kept in D3); an index used for nothing else is not
/// kept at all.  The Pascal source of 0xE00EB0 is not known; this
/// reading of it is ours (codegen-patterns 7, `for` loops).
///
/// `None` when the loop is not of this shape (it is then a
/// [`StmtKind::For`], `Cx::for_loop`).
fn fill(
    p: &Program,
    s: &SStmt,
    var: &SExpr,
    from: &SExpr,
    dir: ForDir,
    to: &SExpr,
    body: &SStmt,
) -> Res<Option<Stmt>> {
    let ExprKind::Var {
        var: index,
        uplevel: 0,
    } = var.kind
    else {
        return Ok(None);
    };
    if !matches!(p.var(index).storage, Storage::Frame { .. }) || dir != ForDir::To {
        return Ok(None);
    }
    let (Some(lo), Some(hi)) = (
        from.value.as_ref().and_then(ConstValue::ordinal),
        to.value.as_ref().and_then(ConstValue::ordinal),
    ) else {
        return Ok(None);
    };
    let body = match &body.kind {
        SStmtKind::Compound(b) if b.stmts.len() == 1 => &b.stmts[0],
        _ => body,
    };
    let SStmtKind::Assign { target, value } = &body.kind else {
        return Ok(None);
    };
    let ExprKind::Index {
        base,
        index: ix,
        lo: array_lo,
        elem_size,
    } = &target.kind
    else {
        return Ok(None);
    };
    let is_index = |e: &SExpr| matches!(e.kind, ExprKind::Var { var, .. } if var == index);
    if !is_index(ix) || mentions_var(base, index) {
        return Ok(None);
    }
    let Some(c) = &value.value else {
        return Ok(None);
    };
    if volatile_access(p, target).is_some() {
        // Each element is stored as written, one statement per iteration.
        return Ok(None);
    }
    // The elements of a byte-aligned array may lie at odd addresses, as
    // for a store to one of them ([`access_of`]).
    check_even(p, target)?;
    let ty = scalar_of(p, target)?;
    if ty.bytes() != *elem_size {
        return Ok(None);
    }
    if hi < lo {
        // The body never runs; what the original emitted is not known.
        return Err(unsupported(s.span, "a `for` loop that never runs is"));
    }
    let count = u32::try_from(hi - lo + 1)
        .ok()
        .filter(|&n| n <= 0x10000)
        .ok_or_else(|| unsupported(s.span, "a `for` loop of more than 65536 iterations is"))?;
    let first = i32::try_from((lo - array_lo) * *elem_size as i64)
        .map_err(|_| unsupported(s.span, "an array offset beyond 32 bits is"))?;
    Ok(Some(Stmt {
        kind: StmtKind::Fill {
            start: addr_of(p, base)?.offset_by(first),
            count,
            value: const_of(c, ty, value.span)?,
        },
        span: s.span,
    }))
}

/// Whether `e` reads variable `v` anywhere.
fn mentions_var(e: &SExpr, v: VarId) -> bool {
    match &e.kind {
        ExprKind::Var { var, .. } => *var == v,
        ExprKind::Const
        | ExprKind::Result { .. }
        | ExprKind::WithRecord(_)
        | ExprKind::RoutineAddr(_) => false,
        ExprKind::Field { base, .. } => mentions_var(base, v),
        ExprKind::Index { base, index, .. } => mentions_var(base, v) || mentions_var(index, v),
        ExprKind::Deref(x) | ExprKind::Unary { operand: x, .. } | ExprKind::Transfer(x) => {
            mentions_var(x, v)
        }
        ExprKind::Binary { lhs, rhs, .. } => mentions_var(lhs, v) || mentions_var(rhs, v),
        ExprKind::SetCons(items) => items
            .iter()
            .any(|(a, b)| mentions_var(a, v) || b.as_ref().is_some_and(|b| mentions_var(b, v))),
        ExprKind::Call(c) => {
            c.args.iter().any(|a| mentions_var(&a.expr, v))
                || matches!(&c.callee, SCallee::Pointer(x) if mentions_var(x, v))
        }
        ExprKind::Builtin { args, .. } => args.iter().any(|a| mentions_var(a, v)),
    }
}
