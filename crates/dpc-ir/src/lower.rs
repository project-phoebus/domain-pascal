//! Lowering of the checked program (`dpc_sema::Program`) to the IR.
//!
//! Only the constructs whose code the image has shown are lowered; the
//! rest are reported as "not supported yet" at their source position
//! (never wrong code).  Each accepted construct names the image evidence
//! for the selector pattern that executes it (crate `dpc-m68k`, `isel`).

use dpc_sema::conv::{Passing, ResultReg};
use dpc_sema::{
    Expr as SExpr, ExprKind, Program, Routine, RoutineId, Stmt as SStmt, StmtKind as SStmtKind,
    Storage, VarId,
};
use dpc_syntax::ast::BinOp;
use dpc_syntax::{Diagnostic, Span};

use dpc_sema::types::{TypeId, TypeKind, TypeTable};

use dpc_sema::consts::ConstValue;

use crate::{
    Addr, Expr, Function, Module, ResultClass, ResultSpec, Scalar, Sign, Stmt, StmtKind,
    ValueParam, Width,
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
    // Module data needs the A5 data-base convention (codegen-patterns 4),
    // which is not implemented yet.
    for &v in &program.unit_vars {
        let var = program.var(v);
        if matches!(var.storage, Storage::Static { .. }) {
            errors.push(unsupported(
                var.span,
                "variables allocated in this unit's data sections are",
            ));
        }
    }
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
        Ok(Module {
            name: program.name.clone(),
            functions,
        })
    } else {
        Err(errors)
    }
}

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
        value_params: value_params(p, r),
        body,
        span: r.span,
    })
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
    let stmts: Vec<&SStmt> = block
        .stmts
        .iter()
        .filter(|s| !matches!(s.kind, SStmtKind::Empty))
        .collect();
    let mut out = Vec::new();
    for (i, s) in stmts.iter().enumerate() {
        let last = i + 1 == stmts.len();
        match &s.kind {
            SStmtKind::Assign { target, value } if matches!(target.kind, ExprKind::Result { routine, uplevel: 0 } if routine == id) =>
            {
                out.push(result_assignment(p, s, value, last, result)?);
            }
            SStmtKind::Assign { target, .. } if matches!(target.kind, ExprKind::Result { .. }) => {
                return Err(unsupported(
                    target.span,
                    "assignment to an enclosing function's result is",
                ));
            }
            SStmtKind::Assign { target, value } => out.push(store(p, s, target, value)?),
            _ => return Err(unsupported(s.span, "this statement is")),
        }
    }
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
        Storage::Static { .. } => Err(unsupported(
            span,
            "variables in this unit's data sections (A5-relative) are",
        )),
    }
}
