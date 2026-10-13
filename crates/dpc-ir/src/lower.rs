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
use dpc_syntax::ast::{BinOp, ForDir, UnOp};
use dpc_syntax::{Diagnostic, Span};

use dpc_sema::types::{TypeId, TypeKind, TypeTable};

use dpc_sema::consts::ConstValue;

use crate::{
    Addr, Arg, BinOp as IrBinOp, Call, Callee, CaseArm, Cond, DataArea, DataBaseOptions, Expr,
    Function, Module, Rel, ResultClass, ResultSpec, Scalar, Sign, Stmt, StmtKind, UnOp as IrUnOp,
    ValueParam, Width,
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
            None
        }
    };
    let mut functions = Vec::new();
    for &id in &program.top_routines {
        let r = program.routine(id);
        if r.external || r.body.is_none() {
            continue;
        }
        match lower_routine(program, id) {
            Ok(f) => functions.push(f),
            Err(d) => errors.extend(d),
        }
    }
    if errors.is_empty() {
        if let Err(d) = set_loads_data_base(&mut functions, data.is_some(), db) {
            errors.extend(d);
        }
    }
    if errors.is_empty() {
        Ok(Module {
            name: program.name.clone(),
            functions,
            data,
        })
    } else {
        Err(errors)
    }
}

/// The unit's static data area (codegen-patterns 4): every variable with
/// [`Storage::Static`] storage, unit-level or a routine's `static` one,
/// lives in the unit's default data section at the offset `dpc-sema`
/// gave it.
///
/// Rejected for now: a data section named in the module heading (ref
/// 7.1.1, p. 7-2) or in a `var (name)` part (ref 3.14), whose placement
/// and addressing (shared by name between units, ref 7.2.4 p. 7-7) have
/// not been catalogued; and initial values (ref 3.2.2), whose section
/// contents the emitter does not write yet.
fn data_area(p: &Program) -> Result<Option<DataArea>, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let mut any = false;
    let mut globals = Vec::new();
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
        any = true;
        if section != DEFAULT_DATA_SECTION || p.data_section != DEFAULT_DATA_SECTION {
            errors.push(unsupported(
                v.span,
                "variables in a named data section (ref 3.14, 7.1.1) are",
            ));
        }
        if v.init.is_some() {
            errors.push(unsupported(v.span, "initialised static variables are"));
        }
        if *global {
            globals.push((symbol.clone(), *offset));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    if !any {
        return Ok(None);
    }
    globals.sort_by_key(|&(_, o)| o);
    let size = p
        .data_sections
        .iter()
        .find(|(n, _)| n == DEFAULT_DATA_SECTION)
        .map_or(0, |&(_, n)| n);
    Ok(Some(DataArea { size, globals }))
}

/// The default data section (ref 5.7.1, p. 5-24): `dpc_sema::Program`'s
/// `data_section` when the module heading names none.
const DEFAULT_DATA_SECTION: &str = ".data";

fn lower_routine(p: &Program, id: RoutineId) -> Result<Function, Vec<Diagnostic>> {
    let r = p.routine(id);
    let mut errors = Vec::new();
    if let Some(&n) = r.nested.first() {
        // codegen-patterns 6 (static link in A1 or the dynamic link).
        errors.push(unsupported(p.routine(n).span, "nested routines are"));
    }
    if r.code_section != ".text" {
        // ref 5.7.1 / 7.1.1; how named sections map to the output sections
        // `../source` links (codegen-patterns 11) is not settled.
        errors.push(unsupported(r.span, "named code sections are"));
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
    for prm in &r.params {
        if prm.conv.local_copy {
            // Table 5-1, no mode, default: the callee copies the argument
            // on entry.  The copy code has not been catalogued.
            errors.push(unsupported(
                p.var(prm.var).span,
                "value parameters passed by address (callee-made local copy) are",
            ));
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
    let body = if r.result.is_some() && result.is_none() {
        Vec::new()
    } else {
        match lower_body(p, id, r, result) {
            Ok(b) => b,
            Err(d) => {
                errors.push(d);
                Vec::new()
            }
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Function {
        symbol: r.symbol.clone(),
        global: r.global,
        code_section: r.code_section.clone(),
        frame_size: r.frame_size,
        result,
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
        scalar_locals: scalar_locals(p, r),
        body,
        span: r.span,
    })
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
    unit_has_data: bool,
    db: &DataBaseOptions,
) -> Result<(), Vec<Diagnostic>> {
    let fsr: &[Function] = fs;
    let mut errors = Vec::new();
    let find = |name: &str| fsr.iter().position(|f| f.symbol.eq_ignore_ascii_case(name));
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
    // Emission order: code section by code section, in order of first
    // use, the unit's order within each (as `dpc_m68k::emit` lays out).
    let mut sections: Vec<&str> = Vec::new();
    for f in fsr.iter() {
        if !sections.contains(&f.code_section.as_str()) {
            sections.push(&f.code_section);
        }
    }
    let order: Vec<usize> = sections
        .iter()
        .flat_map(|&sec| (0..fsr.len()).filter(move |&i| fsr[i].code_section == sec))
        .collect();
    let mut always_mode = vec![false; fsr.len()];
    if let Some(name) = &db.always_from {
        match order
            .iter()
            .position(|&i| fsr[i].symbol.eq_ignore_ascii_case(name))
        {
            Some(k) => {
                for &i in &order[k..] {
                    always_mode[i] = true;
                }
            }
            None => errors.push(Diagnostic::error_nopos(format!(
                "--a5-always-from: no routine `{name}` in this unit"
            ))),
        }
    }
    if !unit_has_data && (db.always_from.is_some() || always.contains(&true)) {
        // In the image such a unit's A5 is the end of a 4-byte block no
        // code reads (OS_INIT_DATA AST_ 0xE35000 -> 0xE35004); dpc does
        // not lay out that block.
        errors.push(Diagnostic::error_nopos(
            "--a5-always / --a5-always-from: the unit has no static data, so there is no data \
             area to load (a unit without data that loads A5 points it past a 4-byte block, \
             codegen-patterns 4; not supported)",
        ));
    }
    for (i, f) in fsr.iter().enumerate() {
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
    // A callee inherits the data base unless it is exported or in the
    // `always` list (it loads its own; a call to it is not a use).
    let inherits = |j: usize| !fsr[j].global && !always[j];
    let mut uses = direct.clone();
    loop {
        let mut changed = false;
        for i in 0..fsr.len() {
            if uses[i] {
                continue;
            }
            let through = callees[i].iter().any(|sym| {
                fsr.iter()
                    .position(|g| &g.symbol == sym)
                    .is_some_and(|j| inherits(j) && uses[j])
            });
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
        let through = callees[i].iter().any(|sym| {
            fsr.iter()
                .position(|g| &g.symbol == sym)
                .is_some_and(|j| inherits(j) && uses[j])
        });
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
    for (i, f) in fs.iter_mut().enumerate() {
        f.data_absolute = absolute[i];
        f.loads_data_base = if f.is_empty_procedure() || absolute[i] {
            false
        } else if always[i] {
            true
        } else if !f.global {
            false
        } else {
            uses[i] || (always_mode[i] && unit_has_data)
        };
    }
    Ok(())
}

/// The symbols of this unit's routines that `s` calls ([`Callee::Local`]),
/// as statements or inside expressions.
fn local_callees(s: &Stmt, out: &mut Vec<String>) {
    let mut calls = Vec::new();
    stmt_calls(s, &mut calls);
    for c in calls {
        if let Callee::Local(sym) = &c.callee {
            out.push(sym.clone());
        }
    }
}

/// Every call `s` makes, statement calls and function calls in
/// expressions.
fn stmt_calls<'a>(s: &'a Stmt, out: &mut Vec<&'a Call>) {
    let e = |x: &'a Expr, out: &mut Vec<&'a Call>| expr_calls(x, out);
    match &s.kind {
        StmtKind::SetResult(x) => e(x, out),
        StmtKind::Store { addr, value } => {
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
        StmtKind::Return | StmtKind::Disable | StmtKind::Enable => {}
    }
}

fn call_arg_calls<'a>(c: &'a Call, out: &mut Vec<&'a Call>) {
    for a in &c.args {
        match a {
            Arg::Value { value, .. } => expr_calls(value, out),
            Arg::Address { addr, .. } => addr_calls(addr, out),
        }
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
    }
}

fn expr_calls<'a>(e: &'a Expr, out: &mut Vec<&'a Call>) {
    match e {
        Expr::Const { .. } => {}
        Expr::Load { addr, .. } | Expr::AddrOf(addr) => addr_calls(addr, out),
        Expr::SetMember { elem, set, .. } => {
            expr_calls(elem, out);
            expr_calls(set, out);
        }
        Expr::Binary { lhs, rhs, .. } => {
            expr_calls(lhs, out);
            expr_calls(rhs, out);
        }
        Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
            expr_calls(x, out)
        }
        Expr::Call { call, .. } => {
            out.push(call);
            call_arg_calls(call, out);
        }
        Expr::Test(c) => cond_calls(c, out),
    }
}

fn addr_calls<'a>(a: &'a Addr, out: &mut Vec<&'a Call>) {
    match a {
        Addr::Frame(_) | Addr::Static(_) | Addr::Symbol { .. } => {}
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
        StmtKind::Store { addr, value } => addr_uses_static(addr) || expr_uses_static(value),
        StmtKind::Copy { dst, src, .. } => addr_uses_static(dst) || addr_uses_static(src),
        StmtKind::Call(c) => call_uses_static(c),
        StmtKind::Fill { start, value, .. } => addr_uses_static(start) || expr_uses_static(value),
        StmtKind::If { cond, then_, else_ } => {
            cond_uses_static(cond) || then_.iter().chain(else_).any(stmt_uses_static)
        }
        StmtKind::Repeat { body, until } => {
            body.iter().any(stmt_uses_static) || cond_uses_static(until)
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
        StmtKind::Return | StmtKind::Disable | StmtKind::Enable => false,
    }
}

fn cond_uses_static(c: &Cond) -> bool {
    match c {
        Cond::Compare { lhs, rhs, .. } => expr_uses_static(lhs) || expr_uses_static(rhs),
        Cond::Test(e) => expr_uses_static(e),
        Cond::Not(x) => cond_uses_static(x),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) => cond_uses_static(a) || cond_uses_static(b),
    }
}

fn call_uses_static(c: &Call) -> bool {
    c.args.iter().any(|a| match a {
        Arg::Value { value, .. } => expr_uses_static(value),
        Arg::Address { addr, .. } => addr_uses_static(addr),
    })
}

fn expr_uses_static(e: &Expr) -> bool {
    match e {
        Expr::Const { .. } => false,
        Expr::Load { addr, .. } | Expr::AddrOf(addr) => addr_uses_static(addr),
        Expr::SetMember { elem, set, .. } => expr_uses_static(elem) || expr_uses_static(set),
        Expr::Binary { lhs, rhs, .. } => expr_uses_static(lhs) || expr_uses_static(rhs),
        Expr::Unary { operand: x, .. } | Expr::Convert { value: x, .. } | Expr::SetSr(x) => {
            expr_uses_static(x)
        }
        Expr::Call { call, .. } => call_uses_static(call),
        Expr::Test(c) => cond_uses_static(c),
    }
}

fn addr_uses_static(a: &Addr) -> bool {
    match a {
        Addr::Static(_) => true,
        Addr::Frame(_) | Addr::Symbol { .. } => false,
        Addr::Indirect { base, .. } => expr_uses_static(base),
        Addr::Index { base, index, .. } => addr_uses_static(base) || expr_uses_static(index),
    }
}

/// The routine's own scalar local variables ([`Function::scalar_locals`]):
/// the frame variables other than the function result variable (parameter
/// copies are not frame variables), of 1, 2 or 4 bytes.
///
/// None for a routine with nested routines: they may read or write its
/// locals through the frame (ref 2.4), which the selector's per-body
/// analysis (codegen-patterns 12.5) does not see.
fn scalar_locals(p: &Program, r: &Routine) -> Vec<ValueParam> {
    if !r.nested.is_empty() {
        return Vec::new();
    }
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
        // codegen-patterns 3: pointer results in A0 (MST_$MAPS, ...); the
        // code that moves a computed pointer there is not catalogued yet.
        ResultReg::Address | ResultReg::AddressAndData => {
            return Err(unsupported(
                r.span,
                "pointer (address-register) results are",
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
) -> Res<Vec<Stmt>> {
    let block = r.body.as_ref().expect("routine with a body");
    let cx = Cx { p, id, r, result };
    let out = cx.seq(&block.stmts, true)?;
    if r.result.is_some() {
        let assigned = out.iter().any(|s| matches!(s.kind, StmtKind::SetResult(_)));
        if !assigned {
            return Err(unsupported(
                r.span,
                "a function whose last statement is not the result assignment is",
            ));
        }
    }
    Ok(out)
}

/// What statement lowering needs to know about the routine.
struct Cx<'a> {
    p: &'a Program,
    id: RoutineId,
    r: &'a Routine,
    result: Option<ResultSpec>,
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
    /// A statement sequence.  `top`: the routine's own statement part
    /// (a function's result assignment must be its last statement).
    fn seq(&self, stmts: &[SStmt], top: bool) -> Res<Vec<Stmt>> {
        let p = self.p;
        let mut flat = Vec::new();
        flatten(stmts, &mut flat);
        let mut out = Vec::new();
        for (i, s) in flat.iter().enumerate() {
            let last = top && i + 1 == flat.len();
            match &s.kind {
                SStmtKind::Assign { target, value } if matches!(target.kind, ExprKind::Result { routine, uplevel: 0 } if routine == self.id) =>
                {
                    out.push(result_assignment(p, s, value, last, self.result)?);
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
                    kind: StmtKind::Call(call(p, &self.r.code_section, c, s.span)?),
                    span: s.span,
                }),
                SStmtKind::For {
                    var,
                    from,
                    dir,
                    to,
                    body,
                } => out.push(fill(p, s, var, from, *dir, to, body)?),
                SStmtKind::If {
                    cond,
                    then_branch,
                    else_branch,
                } => {
                    let then_ = self.seq(std::slice::from_ref(then_branch), false)?;
                    if then_.is_empty() {
                        // What the original emitted for an empty `then`
                        // part (a branch on the negated condition?) is not
                        // known.
                        return Err(unsupported(s.span, "an `if` with an empty `then` part is"));
                    }
                    let else_ = match else_branch {
                        Some(e) => self.seq(std::slice::from_ref(e), false)?,
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
                        body: self.seq(body, false)?,
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
                // codegen-patterns 7: a `while` loop is compiled with its
                // test at the bottom, entered by a `bra` to the test
                // (0xE01A8E..0xE01A98; 213 such loops in compiled code).
                // Whether the head after that `bra` gets the 0x2048
                // filler is not settled: in aligning routines 3 such heads
                // have it, 2 are 4-aligned and 3 are at 2 mod 4.
                SStmtKind::While { .. } => {
                    return Err(unsupported(
                        s.span,
                        "a `while` loop (head alignment after the entry `bra` not settled, \
                         codegen-patterns 7) is",
                    ))
                }
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
                _ => return Err(unsupported(s.span, "this statement is")),
            }
        }
        Ok(out)
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
            ExprKind::Call(c) if x.value.is_none() => {
                StmtKind::Call(call(self.p, &self.r.code_section, c, x.span)?)
            }
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
                body: self.seq(std::slice::from_ref(&arm.stmt), false)?,
            });
        }
        let otherwise = match otherwise {
            Some(ss) => {
                let o = self.seq(ss, false)?;
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
    if l.ty() != r.ty() {
        return Err(unsupported(
            e.span,
            format!(
                "comparing a {}-byte {} value with a {}-byte {} one (implicit widening) is",
                l.ty().bytes(),
                sign_name(l.ty()),
                r.ty().bytes(),
                sign_name(r.ty())
            ),
        ));
    }
    Ok(Cond::Compare {
        op: rel,
        lhs: l,
        rhs: r,
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

/// `f := e` inside `f`.
///
/// Image (PROC1_$TST_LOCK 0xE148CA..0xE148E4): the assignment is the last
/// statement; the value is computed straight into the result register
/// (`move.b D1,D0` at 0xE148E0) and the frame slot `dpc-sema` reserves for
/// the result variable is never touched, although the frame has room for
/// it (`link #-4`, codegen-patterns 2).
///
/// TODO(codegen): what the original does when the result is assigned
/// before the last statement, assigned more than once, or read back, is
/// not known (store to the frame slot and reload before the return, or
/// keep it in D0?).  Those cases are rejected.
fn result_assignment(
    p: &Program,
    s: &SStmt,
    value: &SExpr,
    last: bool,
    result: Option<ResultSpec>,
) -> Res<Stmt> {
    if !last {
        return Err(unsupported(
            s.span,
            "a function result assignment that is not the routine's last statement is",
        ));
    }
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
    let ty = scalar_of(p, target)?;
    let addr = addr_of(p, target)?;
    let value = if let Some(v) = &value.value {
        const_of(v, ty, value.span)?
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
fn const_of(v: &ConstValue, ty: Scalar, span: Span) -> Res<Expr> {
    let value = match v {
        ConstValue::Bool(b) => -(*b as i64),
        _ => v
            .ordinal()
            .ok_or_else(|| unsupported(span, "this kind of constant is"))?,
    };
    Ok(Expr::Const { value, ty })
}

/// The value of `e`.
fn value_of(p: &Program, e: &SExpr) -> Res<Expr> {
    if let Some(v) = &e.value {
        return const_of(v, scalar_of(p, e)?, e.span);
    }
    match &e.kind {
        ExprKind::Var { .. } | ExprKind::Field { .. } | ExprKind::Deref(_) => Ok(Expr::Load {
            addr: addr_of(p, e)?,
            ty: scalar_of(p, e)?,
        }),
        // `addr(x)` (ref p. 4-13): the address of the variable as a value.
        ExprKind::Builtin {
            builtin: Builtin::Addr,
            args,
        } => Ok(Expr::AddrOf(addr_of(p, &args[0])?)),
        ExprKind::Index { .. } => Ok(Expr::Load {
            addr: addr_of(p, e)?,
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
        ExprKind::Call(c) => {
            let ty = scalar_of(p, e)?;
            Ok(Expr::Call {
                call: call(p, ".text", c, e.span)?,
                ty,
            })
        }
        _ => Err(unsupported(e.span, "this expression is")),
    }
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
                _ => {
                    return Err(unsupported(
                        e.span,
                        format!("`in` on a {set_size}-byte set is"),
                    ))
                }
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

/// The address of designator `e`.
fn addr_of(p: &Program, e: &SExpr) -> Res<Addr> {
    match &e.kind {
        ExprKind::Var { var, uplevel } => {
            if *uplevel != 0 {
                return Err(unsupported(e.span, "up-level variable references are"));
            }
            var_addr(p, *var, e.span)
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
            let ix = value_of(p, index)?;
            if !matches!(ix.ty().width, Width::Word | Width::Long) {
                // A byte index (char, boolean, a byte subrange) would need
                // a zero- or sign-extension first: not catalogued.
                return Err(unsupported(index.span, "an array index of one byte is"));
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
                // By address: the slot holds a pointer (LOG_$READ 0xE17810
                // `movea.l (0xc,A6),A0; move.w (A0),...`).  A value
                // parameter with a local copy was rejected earlier.
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
/// - The callee: a routine of this unit with a body and in the caller's
///   code section is called PC-relative ([`Callee::Local`], `bsr`), any
///   other declared routine through its symbol ([`Callee::External`],
///   `jsr abs.l`): codegen-patterns 1.6 (0xE00F02 `bsr.s` to the
///   module-local 0xE00EB0; 0xE00EF6 `jsr ML_$LOCK`).  `bsr` never
///   crosses a map segment in the image (codegen-patterns 7), so a callee
///   in another code section is not supported yet; nor are nested
///   routines (static link, codegen-patterns 6) and calls through
///   routine pointers (`jsr (An)`, 0xE0B3F0).
/// - The arguments, in parameter order, each with its parameter's
///   convention from `dpc-sema` (ref 5.2 Table 5-1): by value, the value
///   at the parameter's width (a constant takes the parameter's type);
///   by address, the address of a designator.  A non-designator by
///   address (a constant pooled in the code section, `pea (d,PC)`, or an
///   expression stored to a frame temporary; codegen-patterns 1.5) is not
///   supported yet.
/// - The pad: the callee's `arg_pad` (codegen-patterns 1.4).  A
///   `variable` routine called with fewer arguments (ref 5.5.5) is not
///   supported yet: its pad and slots are not catalogued.
fn call(p: &Program, caller_section: &str, c: &SCall, span: Span) -> Res<Call> {
    let SCallee::Routine(id) = &c.callee else {
        return Err(unsupported(span, "a call through a routine pointer is"));
    };
    let r = p.routine(*id);
    if r.parent.is_some() {
        return Err(unsupported(span, "a call of a nested routine is"));
    }
    if c.args.len() != r.params.len() {
        return Err(unsupported(
            span,
            "a call with fewer arguments than parameters (ref 5.5.5) is",
        ));
    }
    let callee = if r.external || r.body.is_none() {
        Callee::External(r.symbol.clone())
    } else if r.code_section == caller_section {
        Callee::Local(r.symbol.clone())
    } else {
        return Err(unsupported(
            span,
            "a call of a routine of this unit in another code section is",
        ));
    };
    let mut args = Vec::new();
    for (a, prm) in c.args.iter().zip(&r.params) {
        let slot = a.conv.slot_size;
        args.push(match a.conv.passing {
            Passing::ByValue => {
                let size = p.types.size(prm.ty);
                let width = Width::of_size(size).ok_or_else(|| {
                    unsupported(a.expr.span, format!("a {size}-byte value argument is"))
                })?;
                let ty = Scalar {
                    width,
                    sign: sign_of(&p.types, prm.ty),
                };
                let value = match &a.expr.value {
                    Some(v) => const_of(v, ty, a.expr.span)?,
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
                if a.needs_temp || !a.expr.is_designator() {
                    return Err(unsupported(
                        a.expr.span,
                        "passing a constant or an expression by address (codegen-patterns 1.5) is",
                    ));
                }
                Arg::Address {
                    addr: addr_of(p, &a.expr)?,
                    slot,
                }
            }
        });
    }
    Ok(Call {
        callee,
        args,
        pad: r.arg_pad,
    })
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
/// Everything else about `for` (other bodies, `downto`, bounds known only
/// at run time: `subq.w #1; bmi` 0xE177DC; an index used in the body) is
/// not supported yet.
fn fill(
    p: &Program,
    s: &SStmt,
    var: &SExpr,
    from: &SExpr,
    dir: ForDir,
    to: &SExpr,
    body: &SStmt,
) -> Res<Stmt> {
    let shape = || {
        unsupported(
            s.span,
            "a `for` loop other than `for i := c1 to c2 do a[i] := c` (codegen-patterns 7) is",
        )
    };
    let ExprKind::Var {
        var: index,
        uplevel: 0,
    } = var.kind
    else {
        return Err(shape());
    };
    if !matches!(p.var(index).storage, Storage::Frame { .. }) || dir != ForDir::To {
        return Err(shape());
    }
    let (Some(lo), Some(hi)) = (
        from.value.as_ref().and_then(ConstValue::ordinal),
        to.value.as_ref().and_then(ConstValue::ordinal),
    ) else {
        return Err(shape());
    };
    let body = match &body.kind {
        SStmtKind::Compound(b) if b.stmts.len() == 1 => &b.stmts[0],
        _ => body,
    };
    let SStmtKind::Assign { target, value } = &body.kind else {
        return Err(shape());
    };
    let ExprKind::Index {
        base,
        index: ix,
        lo: array_lo,
        elem_size,
    } = &target.kind
    else {
        return Err(shape());
    };
    let is_index = |e: &SExpr| matches!(e.kind, ExprKind::Var { var, .. } if var == index);
    if !is_index(ix) || mentions_var(base, index) {
        return Err(shape());
    }
    let Some(c) = &value.value else {
        return Err(shape());
    };
    let ty = scalar_of(p, target)?;
    if ty.bytes() != *elem_size {
        return Err(shape());
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
    Ok(Stmt {
        kind: StmtKind::Fill {
            start: addr_of(p, base)?.offset_by(first),
            count,
            value: const_of(c, ty, value.span)?,
        },
        span: s.span,
    })
}

/// Whether `e` reads variable `v` anywhere.
fn mentions_var(e: &SExpr, v: VarId) -> bool {
    match &e.kind {
        ExprKind::Var { var, .. } => *var == v,
        ExprKind::Const | ExprKind::Result { .. } | ExprKind::WithRecord(_) => false,
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
