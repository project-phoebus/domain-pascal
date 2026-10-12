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
    Addr, Arg, Call, Callee, Cond, DataArea, Expr, Function, Module, Rel, ResultClass, ResultSpec,
    Scalar, Sign, Stmt, StmtKind, ValueParam, Width,
};

type Res<T> = Result<T, Diagnostic>;

fn unsupported(span: Span, what: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(span, format!("{what} not supported yet"))
}

/// Lower `program` to IR.  Every construct that cannot be lowered yet is
/// reported; the module is returned only when there is none.
pub fn lower(program: &Program) -> Result<Module, Vec<Diagnostic>> {
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
        if let Err(d) = set_loads_data_base(&mut functions, data.is_some()) {
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
        value_params: value_params(p, r),
        address_params: r
            .params
            .iter()
            .filter(|prm| prm.passing() == Passing::ByAddress)
            .map(|prm| prm.slot_offset)
            .collect(),
        body,
        span: r.span,
    })
}

/// Decide [`Function::loads_data_base`] for every routine of the unit
/// (codegen-patterns 4; on m68k `lea <area>.l,A5` after saving A5).
///
/// Survey of the image (compiled code: every `link` in the map's `I`
/// segments other than `*_ASM`, modules with a `D` segment), where a
/// routine "uses" the data when it has an `(..,A5..)` operand or passes A5
/// on, *directly or through a module-local callee that does not set A5
/// itself*:
///
/// - An exported routine that uses its unit's data loads A5 (585
///   map-named routines, e.g. PROC1_$GET_LOADAV 0xE14BC0,
///   XNS_IDP_$GET_STATS 0xE18FDC).  None was found using it without
///   loading A5.  Through a callee: LOG_$READ 0xE17806 loads A5 although
///   it has no A5 operand of its own; its module-local worker 0xE17778,
///   which does not load A5, reads `(0x14,A5)` (0xE1778C).
/// - A module-local routine called only from its module inherits A5 from
///   its caller (0xE17778).  `internal` routines (ref 5.5.4) are taken to
///   be such workers.  **Uncertain:** 23 routines the map does not name do
///   load A5, but nearly all of them are also called with `jsr` from other
///   modules (exported routines the map omits, e.g. 0xE14B12), and a few
///   RING ones (0xE76048, 0xE7667C) are reached only with `bsr` from
///   their module.
/// - An exported routine that does not use the data is the open case: 204
///   map-named routines load A5 without any A5 operand (e.g.
///   XNS_IDP_$GET_PORT_INFO 0xE18FBE, SMD_$BUSY_WAIT 0xE1D8A4), 251 do
///   not load it (e.g. TIME_$WAIT 0xE1650A).  Map segments seem to merge
///   several compilation units (one segment's routines run in blocks that
///   load A5 and blocks that do not), so "every exported routine of a
///   unit with static data" fits the image too, but the unit boundaries
///   are not known.  Rejected.
fn set_loads_data_base(fs: &mut [Function], unit_has_data: bool) -> Result<(), Vec<Diagnostic>> {
    // Which routines use the data, directly or through internal callees
    // of this unit (which inherit A5); a fixpoint over the call graph.
    let mut uses: Vec<bool> = fs
        .iter()
        .map(|f| f.body.iter().any(stmt_uses_static))
        .collect();
    loop {
        let mut changed = false;
        for i in 0..fs.len() {
            if uses[i] {
                continue;
            }
            let mut callees = Vec::new();
            for st in &fs[i].body {
                local_callees(st, &mut callees);
            }
            let through = callees.iter().any(|sym| {
                fs.iter()
                    .position(|g| &g.symbol == sym)
                    .is_some_and(|j| !fs[j].global && uses[j])
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
    let mut errors = Vec::new();
    for (f, uses) in fs.iter_mut().zip(uses) {
        if !f.global {
            continue;
        }
        if !uses && unit_has_data {
            errors.push(unsupported(
                f.span,
                "an exported routine that does not use its unit's static data, in a unit that has \
                 some (whether it loads the data base is not settled, codegen-patterns 4), is",
            ));
        }
        f.loads_data_base = uses;
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The symbols of this unit's routines that `s` calls ([`Callee::Local`]).
fn local_callees(s: &Stmt, out: &mut Vec<String>) {
    match &s.kind {
        StmtKind::Call(Call {
            callee: Callee::Local(sym),
            ..
        }) => out.push(sym.clone()),
        StmtKind::If { then_, else_, .. } => {
            for st in then_.iter().chain(else_) {
                local_callees(st, out);
            }
        }
        StmtKind::Repeat { body, .. } => {
            for st in body {
                local_callees(st, out);
            }
        }
        _ => {}
    }
}

fn stmt_uses_static(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::SetResult(e) => expr_uses_static(e),
        StmtKind::Store { addr, value } => addr_uses_static(addr) || expr_uses_static(value),
        StmtKind::Copy { dst, src, .. } => addr_uses_static(dst) || addr_uses_static(src),
        StmtKind::Call(c) => c.args.iter().any(|a| match a {
            Arg::Value { value, .. } => expr_uses_static(value),
            Arg::Address { addr, .. } => addr_uses_static(addr),
        }),
        StmtKind::Fill { start, value, .. } => addr_uses_static(start) || expr_uses_static(value),
        StmtKind::If { cond, then_, else_ } => {
            cond_uses_static(cond) || then_.iter().chain(else_).any(stmt_uses_static)
        }
        StmtKind::Repeat { body, until } => {
            body.iter().any(stmt_uses_static) || cond_uses_static(until)
        }
        StmtKind::Return => false,
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

fn expr_uses_static(e: &Expr) -> bool {
    match e {
        Expr::Const { .. } => false,
        Expr::Load { addr, .. } | Expr::AddrOf(addr) => addr_uses_static(addr),
        Expr::SetMember { elem, set, .. } => expr_uses_static(elem) || expr_uses_static(set),
    }
}

fn addr_uses_static(a: &Addr) -> bool {
    match a {
        Addr::Static(_) => true,
        Addr::Frame(_) | Addr::Symbol { .. } => false,
        Addr::Indirect { base, .. } => expr_uses_static(base),
    }
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
                    kind: StmtKind::Call(call(p, self.r, c, s.span)?),
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
                _ => return Err(unsupported(s.span, "this statement is")),
            }
        }
        Ok(out)
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
    if vsize != res.ty.bytes() {
        // Implicit integer widening (integer16 -> integer32) is not
        // catalogued yet.
        return Err(unsupported(
            value.span,
            format!(
                "assigning a {vsize}-byte value to a {}-byte function result is",
                res.ty.bytes()
            ),
        ));
    }
    Ok(Stmt {
        kind: StmtKind::SetResult(value_of(p, value)?),
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
            return Err(unsupported(
                value.span,
                format!(
                    "assigning a {}-byte value to a {}-byte object is",
                    v.ty().bytes(),
                    ty.bytes()
                ),
            ));
        }
        v
    };
    Ok(Stmt {
        kind: StmtKind::Store { addr, value },
        span: s.span,
    })
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
        ExprKind::Binary {
            op: BinOp::In,
            lhs,
            rhs,
        } => {
            // codegen-patterns 5.1: a set that fits in a register is tested
            // with `btst Dn,Dm` (PROC1_$TST_LOCK 0xE148DC, a 4-byte set).
            // Other set sizes (byte-addressed `btst`, MST_$ALLOC_ASID
            // 0xE42D5C) are not implemented yet.
            let set_size = p.types.size(rhs.ty);
            if set_size != 4 {
                return Err(unsupported(
                    e.span,
                    format!("`in` on a {set_size}-byte set is"),
                ));
            }
            Ok(Expr::SetMember {
                elem: Box::new(value_of(p, lhs)?),
                set: Box::new(value_of(p, rhs)?),
                set_width: Width::Long,
            })
        }
        // `addr(x)` (ref p. 4-13): the address of the variable as a value.
        ExprKind::Builtin {
            builtin: Builtin::Addr,
            args,
        } => Ok(Expr::AddrOf(addr_of(p, &args[0])?)),
        _ => Err(unsupported(e.span, "this expression is")),
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
fn call(p: &Program, caller: &Routine, c: &SCall, span: Span) -> Res<Call> {
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
    } else if r.code_section == caller.code_section {
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
