//! Lowering of the checked program (`dpc_sema::Program`) to the IR.
//!
//! Only the constructs whose code the image has shown are lowered; the
//! rest are reported as "not supported yet" at their source position
//! (never wrong code).  Each accepted construct names the image evidence
//! for the selector pattern that executes it (crate `dpc-m68k`, `isel`).

use dpc_sema::conv::{Passing, ResultReg};
use dpc_sema::{
    Builtin, Call as SCall, Callee as SCallee, Expr as SExpr, ExprKind, InitDatum, Program,
    Routine, RoutineId, StaticAddr, StaticBase, Stmt as SStmt, StmtKind as SStmtKind, Storage,
    VarId,
};
use dpc_syntax::ast::{BinOp, ForDir, ParamMode, UnOp};
use dpc_syntax::{Diagnostic, Span};

use dpc_sema::types::{BitPos, TypeId, TypeKind, TypeTable};

use dpc_sema::consts::ConstValue;

use crate::{
    Access, Addr, Arg, BinOp as IrBinOp, BitField, Call, Callee, CaseArm, Cond, DataArea,
    DataBaseOptions, DataInit, Expr, ForLoop, Function, InitAddr, InitValue, LinkArg, Module,
    Nesting, ParamCopy, PoolConst, Rel, ResultClass, ResultSpec, Scalar, SetOp, Sign, StaticLink,
    Stmt, StmtKind, UnOp as IrUnOp, ValueParam, Width,
};

type Res<T> = Result<T, Diagnostic>;

fn unsupported(span: Span, what: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error(span, format!("{what} not supported yet"))
}

/// The refusal `unsupported(span, what)` of a shape the lowering has a
/// correct path for, refused only because the image's exact form is not
/// settled: an error under `dpc --strict`, else a best-effort warning and
/// `Ok` ([`dpc_syntax::diag::best_effort`]; bead domain-pascal-5kgg).
fn best_effort(span: Span, what: impl std::fmt::Display) -> Res<()> {
    dpc_syntax::diag::best_effort(unsupported(span, what))
}

/// The best-effort narrowing of an integer value `v` of the expression `e`
/// to the narrower `ty` where [`assign_convert`] has no catalogued form:
/// the low bytes of the value computed at its own width (the selector's
/// narrowing [`Expr::Convert`], not in place: a register's low part, or
/// the whole object read at its size), after the warning `what`; `None`
/// when `e` is not an integer narrowed (bead domain-pascal-5kgg).
fn narrow_best_effort(
    p: &Program,
    e: &SExpr,
    v: Expr,
    ty: Scalar,
    span: Span,
    what: impl std::fmt::Display,
) -> Option<Res<Expr>> {
    if !p.types.is_integer(e.ty) || v.ty().pointer || v.ty().bytes() <= ty.bytes() {
        return None;
    }
    Some(best_effort(span, what).map(|()| Expr::Convert {
        value: Box::new(v),
        ty,
        in_place: false,
    }))
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
    let (data, overlays) = match data_area(program) {
        Ok(d) => d,
        Err(d) => {
            errors.extend(d);
            (Vec::new(), Vec::new())
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
        // ref 5.5.8, p. 5-19 ([`Module::nosave_callees`]).
        let nosave_callees = program
            .routines
            .iter()
            .filter(|r| r.options.nosave && r.body.is_none())
            .map(|r| r.symbol.clone())
            .collect();
        Ok(Module {
            name: program.name.clone(),
            functions,
            data,
            overlays,
            outside_internal_calls,
            nosave_callees,
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
/// The second list: the overlay sections, `var (name)` with a name no
/// routine and no module heading of the unit gives as its data section
/// (ref 3.14, p. 3-52: "just like a named common block in FORTRAN",
/// shared by name between units, ref 7.2.4 p. 7-7), allocated by
/// `dpc-sema` like a data section (3.14: "allocates variables ...
/// sequentially within the specified section"; a later `var (name)` part
/// of the same name continues it).  Their variables are addressed as
/// another unit's object ([`var_addr`]; codegen-patterns 11, "Overlay
/// sections").
///
/// An overlay section's `define`d variables are its area's `globals`
/// (ref 3.14: a `define` of an `extern` variable of a `var (name)` part
/// allocates it in the section); the target defines the section then
/// (an object file cannot define a symbol inside a common block).
///
/// Rejected: the initial values [`init_value`] cannot encode; in an
/// overlay section, a name that is no assembler symbol, or that the
/// unit also uses as a symbol (a routine's, an `extern` or `define`d
/// variable's), or two spellings of one name (also of one of the unit's
/// data sections).
fn data_area(p: &Program) -> Result<(Vec<DataArea>, Vec<DataArea>), Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let mut used: Vec<&str> = Vec::new();
    let mut globals: Vec<(&str, String, u32)> = Vec::new();
    let mut inits: Vec<(&str, DataInit)> = Vec::new();
    // The symbols the unit defines or references, for the overlay names.
    let symbols: Vec<&str> = p
        .routines
        .iter()
        .map(|r| r.symbol.as_str())
        .chain(p.vars.iter().filter_map(|v| match &v.storage {
            Storage::Extern { symbol }
            | Storage::Static {
                symbol,
                global: true,
                ..
            } => Some(symbol.as_str()),
            _ => None,
        }))
        .collect();
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
        if is_overlay(p, section) {
            // A `define`d variable here (ref 3.14 p. 3-52: a `define` of
            // an `extern` variable of a `var (name)` part "will be stored
            // in section_name") is one of the area's `globals`: the
            // target defines the section with its symbol at its offset
            // (codegen-patterns 11, "Overlay sections"; bead
            // domain-pascal-m15q).
            if let Err(d) = check_overlay(p, section, &symbols, v.span) {
                errors.push(d);
            }
        } else if let Err(d) = check_section_name(section, v.span) {
            errors.push(d);
        }
        if let Some(stores) = &v.init {
            let first = inits.len();
            for st in stores {
                let value = match (&st.value, st.bits) {
                    (InitDatum::Const(c), Some(b)) => init_bits(b, c),
                    (InitDatum::Const(c), None) => init_value(&p.types, st.ty, c),
                    (InitDatum::Addr(a), None) => init_addr(p, st.ty, a),
                    (InitDatum::Addr(_), Some(_)) => {
                        Err("an address in a packed record's bit field is".into())
                    }
                };
                match value {
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
            let mine: Vec<&DataInit> = inits[first..].iter().map(|(_, d)| d).collect();
            if address_partly_overwritten(&mine) {
                // A later initial value (a variant's field, ref 3.10.4
                // p. 3-22) over part of an address: its bytes would be
                // the address's link-time value.
                errors.push(unsupported(
                    v.span,
                    "an initial value over part of an address initial value is",
                ));
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
    let area = |n: &String, size: u32| {
        let mut g: Vec<(String, u32)> = globals
            .iter()
            .filter(|(s, _, _)| s == n)
            .map(|(_, sym, o)| (sym.clone(), *o))
            .collect();
        g.sort_by_key(|&(_, o)| o);
        DataArea {
            section: n.clone(),
            size,
            globals: g,
            init: inits
                .iter()
                .filter(|(s, _)| s == n)
                .map(|(_, i)| i.clone())
                .collect(),
        }
    };
    let (mut blocks, mut overlays) = (Vec::new(), Vec::new());
    for (n, size) in p
        .data_sections
        .iter()
        .filter(|(n, _)| used.contains(&n.as_str()))
    {
        if is_overlay(p, n) {
            overlays.push(area(n, *size));
        } else {
            blocks.push(area(n, *size));
        }
    }
    Ok((blocks, overlays))
}

/// Whether Pascal data section `name` is an overlay section of unit `p`
/// (ref 3.14, p. 3-52): none of its data sections, which are the module
/// heading's (ref 7.1.1) and those its routines name (`[section(c,d)]`,
/// ref 5.7.1).  A `var (name)` part naming one of those adds to the
/// unit's block there (dpc's reading; srcsurvey's hoisted statics rely
/// on it).
fn is_overlay(p: &Program, name: &str) -> bool {
    name != p.data_section && !p.routines.iter().any(|r| r.data_section == name)
}

/// The checks of an overlay section's name and size ([`data_area`]).
fn check_overlay(p: &Program, section: &str, symbols: &[&str], span: Span) -> Res<()> {
    let sym = crate::overlay_symbol(section);
    let mut chars = section.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '_' | '$'))
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$'))
    {
        return Err(unsupported(
            span,
            format!(
                "overlay section name `{section}` (only a letter, `_` or `$`, then letters, \
                 digits, `_` and `$`, are supported) is"
            ),
        ));
    }
    if let Some(other) = p
        .data_sections
        .iter()
        .find(|(n, _)| n != section && is_overlay(p, n) && crate::overlay_symbol(n) == sym)
    {
        return Err(unsupported(
            span,
            format!(
                "overlay sections `{section}` and `{}`, one symbol `{sym}`, are",
                other.0
            ),
        ));
    }
    // One of the unit's data sections spelt another way (`var
    // ('WIRED_DATA')` in `module m, , wired_data;`): the unit's block
    // there or an overlay section of its own is not known (question 44).
    if let Some(own) = std::iter::once(p.data_section.as_str())
        .chain(p.routines.iter().map(|r| r.data_section.as_str()))
        .find(|n| *n != section && crate::overlay_symbol(n) == sym)
    {
        return Err(unsupported(
            span,
            format!(
                "overlay section `{section}`, the unit's data section `{own}` spelt another \
                 way (docs/reference-clarifications.md, question 44), is"
            ),
        ));
    }
    if symbols.iter().any(|s| s.eq_ignore_ascii_case(&sym)) {
        return Err(unsupported(
            span,
            format!(
                "an overlay section named like a routine or variable symbol of the unit \
                 (`{sym}`) is"
            ),
        ));
    }
    // A section of more than 0x8000 bytes is allocated like any other;
    // how its variables are reached from a base register is the target's
    // (codegen-patterns 5.2, "Overlay sections of more than 0x8000 bytes";
    // [`addr_of`] refuses the one shape whose base the image leaves open).
    Ok(())
}

/// Whether a value of `inits` (one variable's, in order) overwrites
/// part, not all, of an earlier address (4 bytes, [`init_addr`]).
fn address_partly_overwritten(inits: &[&DataInit]) -> bool {
    let size = |d: &DataInit| d.value.size();
    inits.iter().enumerate().any(|(i, a)| {
        matches!(a.value, InitValue::Address(_))
            && inits[i + 1..].iter().any(|b| {
                let (lo, hi) = (b.offset, b.offset + size(b));
                let overlaps = lo < a.offset + 4 && hi > a.offset;
                overlaps && !(lo <= a.offset && hi >= a.offset + 4)
            })
    })
}

/// The initial value `addr(...)` of an object of type `ty`, a pointer
/// (one store of `dpc_sema::InitStore`): a symbol or the unit's data area
/// plus an offset, emitted as a relocated long.  Image: compiled modules'
/// data areas hold their routines' entry addresses as plain longs (FLP_ D
/// 0xE7AEF4: FLP_$SHUTDOWN 0xE3E228 at +0x04, FLP_$DINIT, FLP_$REVALIDATE,
/// FLP_$DO_IO at +0x08..+0x10; WIN_ D 0xE2B89C: WIN_$SPIN_DOWN 0xE19BC0 at
/// +0x10).  Whether those came from Pascal initialisers is not known.
fn init_addr(p: &Program, ty: TypeId, a: &StaticAddr) -> Result<InitValue, String> {
    if p.types.size(ty) != 4 {
        return Err(format!(
            "an address as the initial value of a {} is",
            p.types.display(ty)
        ));
    }
    let offset = i32::try_from(a.offset).map_err(|_| "an address offset this large is")?;
    Ok(InitValue::Address(match a.base {
        StaticBase::Routine(r) => InitAddr::Symbol {
            name: p.routine(r).symbol.clone(),
            offset,
        },
        StaticBase::Var(v) => match &p.var(v).storage {
            Storage::Extern { symbol } => InitAddr::Symbol {
                name: symbol.clone(),
                offset,
            },
            // A variable of an overlay section: an offset into the
            // section's symbol ([`var_addr`]).
            Storage::Static {
                section, offset: o, ..
            } if is_overlay(p, section) => InitAddr::Symbol {
                name: crate::overlay_symbol(section),
                offset: *o as i32 + offset,
            },
            Storage::Static {
                section, offset: o, ..
            } => InitAddr::Static {
                section: section.clone(),
                offset: *o as i32 + offset,
            },
            Storage::Frame { .. } | Storage::Param { .. } | Storage::HiddenResult { .. } => {
                return Err("an address of a routine's variable as an initial value is".into())
            }
        },
    }))
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

/// The initial value of a packed record's bit field (one store of
/// `dpc_sema::InitStore` with `bits`, ref 3.10.7): the ordinal's low
/// `width` bits, two's complement for a negative one (sema, `init_bits`).
fn init_bits(b: BitPos, value: &ConstValue) -> Result<InitValue, String> {
    let Some(v) = value.ordinal() else {
        return Err("this initial value of a packed record's bit field is".into());
    };
    if b.width > 32 {
        return Err("an initial value of a packed bit field wider than 32 bits is".into());
    }
    Ok(InitValue::Bits {
        bit: b.bit,
        width: b.width,
        value: (v as u64) & ((1u64 << b.width) - 1),
    })
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
/// Deeper nesting (ref 2.4 allows any depth): a routine reaches the
/// frame of a routine two or more levels out through a chain of static
/// links, each nested routine with nested routines that reach past it
/// storing its own static link in its frame's link slot `(-4,A6)` at
/// entry (DIR's level-1 0xE4F3BA `move.l A1,(-4,A6)` at 0xE4F3C2, read by
/// its children as `movea.l (A6),A0; movea.l (-4,A0),A2`, 0xE4EF8C; the
/// level-3 0xE4F034 `movea.l (A6),A0; movea.l (A0),A0; movea.l
/// (-4,A0),A2`; codegen-patterns 6, "Deeper nesting").  Best effort (not
/// byte-identical): where the image keeps a second link in the deeper
/// routine's own frame (0xE4DFFE `move.l (A6),(-0x1a4,A6); movea.l
/// (-0x1a4,A6),A0; move.l (A0),(-4,A6)`, a display) `dpc` walks the chain
/// at each reference ([`outer`]).  A nested routine declared `forward` is
/// refused: whether it is emitted at its heading or at its body is not
/// seen.
fn lower_family(p: &Program, id: RoutineId, out: &mut Vec<Function>, errors: &mut Vec<Diagnostic>) {
    lower_tree(p, id, out, errors);
}

/// Lower routine `id` after the routines nested in it (recursively),
/// into `out`.  Returns the frame objects of the routines enclosing `id`
/// that `id` and the routines nested in it reach: (levels out from `id`,
/// frame offset, bytes), 1 for `id`'s parent.
fn lower_tree(
    p: &Program,
    id: RoutineId,
    out: &mut Vec<Function>,
    errors: &mut Vec<Diagnostic>,
) -> Vec<(u32, i32, u32)> {
    let r = p.routine(id);
    let mut uplevel: Vec<(i32, u32)> = Vec::new();
    let mut up: Vec<(u32, i32, u32)> = Vec::new();
    for &n in &r.nested {
        let nr = p.routine(n);
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
        for (l, o, b) in lower_tree(p, n, out, errors) {
            if l == 1 {
                uplevel.push((o, b));
            } else {
                up.push((l - 1, o, b));
            }
        }
    }
    // A nested routine whose link slot a deeper routine reads ([`outer`])
    // stores its static link there first.
    let store_link = r.parent.is_some() && uplevel.iter().any(|&(o, _)| o == LINK_SLOT);
    if store_link {
        best_effort(
            r.span,
            "a routine nested in a nested routine (the outer frames reached through the chain \
             of links at each reference, where DIR 0xE4DFFE keeps them in its frame; \
             codegen-patterns 6, \"Deeper nesting\") is",
        )
        .unwrap_or_else(|d| errors.push(d));
    }
    // The address of the enclosing frame itself (the link stored) is no
    // object of it.
    uplevel.retain(|&(o, _)| o != 0);
    uplevel.sort_unstable();
    uplevel.dedup();
    OUTER_REFS.with(|o| o.borrow_mut().clear());
    match lower_routine(p, id, &uplevel) {
        Ok(mut f) => {
            if store_link {
                f.body.insert(
                    0,
                    Stmt {
                        kind: StmtKind::Store {
                            addr: Addr::Frame(LINK_SLOT),
                            value: Expr::AddrOf(Addr::Enclosing(0)),
                        },
                        span: f.span,
                    },
                );
            }
            if r.parent.is_some() {
                let mut own = Vec::new();
                for st in &f.body {
                    stmt_enclosing(st, &mut own);
                }
                up.extend(own.into_iter().map(|(o, b)| (1, o, b)));
                up.extend(OUTER_REFS.with(|o| o.borrow_mut().split_off(0)));
            }
            out.push(f);
        }
        Err(d) => errors.extend(d),
    }
    up
}

/// The slot of a nested routine's frame that holds its static link when
/// a deeper routine needs it (codegen-patterns 6, "Frame: a 4-byte
/// static-link slot": 0xE4F3BA `move.l A1,(-4,A6)`).
const LINK_SLOT: i32 = -4;

thread_local! {
    /// The frame objects of routines two or more levels out that the
    /// routine being lowered reaches ([`outer`]): (levels out, frame
    /// offset, bytes), the link slots of the routines between included.
    static OUTER_REFS: std::cell::RefCell<Vec<(u32, i32, u32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// `a`, an address in the terms of the routine `levels` (2 or more)
/// out, as seen from a routine nested in it: through the chain of links,
/// the parent's link slot, then each one's (codegen-patterns 6, "Deeper
/// nesting": DIR 0xE4EF8C `movea.l (A6),A0; movea.l (-4,A0),A2`).
/// Records what it reaches in [`OUTER_REFS`].
fn outer(a: Addr, levels: u32, var_bytes: u32) -> Addr {
    let mut chain = Expr::Load {
        addr: Addr::Enclosing(LINK_SLOT),
        ty: Scalar::POINTER,
    };
    for l in 2..levels {
        OUTER_REFS.with(|o| o.borrow_mut().push((l, LINK_SLOT, 4)));
        chain = Expr::Load {
            addr: Addr::Indirect {
                base: Box::new(chain),
                offset: LINK_SLOT,
            },
            ty: Scalar::POINTER,
        };
    }
    let at = |o: i32| Addr::Indirect {
        base: Box::new(chain.clone()),
        offset: o,
    };
    match a {
        Addr::Frame(o) => {
            OUTER_REFS.with(|r| r.borrow_mut().push((levels, o, var_bytes)));
            at(o)
        }
        Addr::Indirect { base, offset } => match *base {
            Expr::Load {
                addr: Addr::Frame(s),
                ty,
            } => {
                OUTER_REFS.with(|r| r.borrow_mut().push((levels, s, 4)));
                Addr::Indirect {
                    base: Box::new(Expr::Load { addr: at(s), ty }),
                    offset,
                }
            }
            b => Addr::Indirect {
                base: Box::new(b),
                offset,
            },
        },
        other => other,
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
/// Nested routines are local symbols (ref 5.5.4 p. 5-17); `dpc-sema`
/// gives two of one name distinct symbols (`NAME.2`), so a collision left
/// here is an internal error.
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
            Addr::Frame(_)
            | Addr::Static(_)
            | Addr::OtherStatic { .. }
            | Addr::Symbol { .. }
            | Addr::Const(_) => {}
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
            Expr::BitElem {
                array: a,
                bytes,
                index,
                ..
            } => {
                expr(index, out);
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
                Arg::Call { call: c, .. } => call(c, out),
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
            Cond::Test(e) | Cond::Odd(e) => expr(e, out),
            Cond::Not(x) => cond(x, out),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                cond(a, out);
                cond(b, out);
            }
            Cond::Block { lhs, rhs, size, .. } => {
                addr(lhs, *size, out);
                addr(rhs, *size, out);
            }
            Cond::SetEmpty { set, size } => addr(set, *size, out),
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
        StmtKind::StoreBitElem {
            array: a,
            bytes,
            index,
            ..
        }
        | StmtKind::StoreSetMember {
            set: a,
            bytes,
            elem: index,
            ..
        } => {
            addr(a, *bytes, out);
            expr(index, out);
        }
        StmtKind::StoreSetConst { set: a, bits } => addr(a, bits.len() as u32, out),
        StmtKind::StoreSetRange {
            set: a,
            bytes,
            lo,
            hi,
            ..
        } => {
            addr(a, *bytes, out);
            expr(hi, out);
            expr(lo, out);
        }
        StmtKind::Copy {
            dst, src, bytes, ..
        } => {
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
            if let Some(u) = until {
                cond(u, out);
            }
        }
        StmtKind::While { cond: c, body } => {
            cond(c, out);
            for st in body {
                stmt_enclosing(st, out);
            }
        }
        StmtKind::For(l) => {
            if let Some(h) = &l.home {
                addr(h, l.index.ty.bytes(), out);
            }
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
            | Addr::OtherStatic { .. }
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
            Expr::SetMemberAt { elem, set: a, .. }
            | Expr::BitElem {
                index: elem,
                array: a,
                ..
            } => {
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
                Arg::Call { call: c, .. } => call(c, f),
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
            Cond::Test(e) | Cond::Odd(e) => expr(e, f),
            Cond::Not(x) => cond(x, f),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                cond(a, f);
                cond(b, f);
            }
            Cond::Block { lhs, rhs, .. } => {
                addr(lhs, f);
                addr(rhs, f);
            }
            Cond::SetEmpty { set, .. } => addr(set, f),
        }
    }
    match &mut s.kind {
        StmtKind::SetResult(e) | StmtKind::Discard(e) => expr(e, f),
        StmtKind::Store { addr: a, value }
        | StmtKind::StoreBits { addr: a, value, .. }
        | StmtKind::StoreBitElem {
            array: a,
            index: value,
            ..
        }
        | StmtKind::StoreSetMember {
            set: a,
            elem: value,
            ..
        } => {
            addr(a, f);
            expr(value, f);
        }
        StmtKind::StoreSetConst { set: a, .. } => addr(a, f),
        StmtKind::StoreSetRange { set: a, lo, hi, .. } => {
            addr(a, f);
            expr(hi, f);
            expr(lo, f);
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
            if let Some(u) = until {
                cond(u, f);
            }
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

/// The frame of `frame` bytes of locals (and result variable) with a
/// 4-byte slot for each of `n` array-element `with` temporaries
/// ([`Function::with_temps`], the [`Addr::Index`] case of
/// [`Cx::with_stmt`]) below them, longword aligned as a pointer local
/// is: the selector holds each temporary in an address register and
/// never addresses its slot, but the original compiler reserved one
/// (codegen-patterns 2, "Frame size", and 5.2, "`with`"; bead
/// domain-pascal-s9g0).  Fitted to ../source's Pascal, stand-ins
/// removed: of its routines that compile, the 12 with element
/// temporaries all come closer to the image's `link` and 9 meet it
/// (MMAP_$AVAIL 0xE0CC64, MMAP_$WIRE 0xE0CCBC, MMAP_$UNWIRE,
/// MMAP_$GET_WS_SIZ, MMAP_$FREE, MAC_$CLOSE, TPAD_$SET_PUNCH_IMPACT,
/// TPAD_$INQ_PUNCH_IMPACT, TERM_$SET_REAL_LINE_DISCIPLINE with two);
/// the frame of no other routine changes; NETLOG_$LOG_IT 0xE71B38 (18
/// bytes of locals, `link #-24`) and XPD_$FIND_INDEX 0xE5AF38 (a word,
/// `link #-8`) fit too.  A pointer's temporary (`with p^`) gets no
/// slot: the one such site, HINT_$ADD_NET 0xE49C76, has `link #0`
/// while it loads the pointer into A0 for two stores (Uncertain, one
/// site; bead domain-pascal-njy0).  Where the code stores the
/// temporary, its slot is below the declared locals (MAC_$SEND
/// 0xE0BB44 `move.l A0,(-0x80,A6)`, `link #-128`); whether above or
/// below the call temporaries is not visible: here above them
/// ([`assign_temps`]), Uncertain.
pub(crate) fn with_slots(frame: u32, n: usize) -> u32 {
    if n == 0 {
        frame
    } else {
        frame.div_ceil(4) * 4 + 4 * n as u32
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
/// arguments reuses the outer call's slots; the order in which later
/// calls reuse them (bead domain-pascal-tf4x).
///
/// The temporaries receiving hidden-address results (`results`, by
/// number, [`RESULT_TEMPS`]) are placed the same way, as their call's
/// first argument, and so is a call passed on by address ([`Arg::Call`])
/// with its own temporaries, at its place in the outer call's push order;
/// their pseudo offsets, in the calls and in the copies out of them, are
/// replaced once all are placed.
fn assign_temps(body: &mut [Stmt], frame: u32, results: &[u32]) -> u32 {
    /// The frame size so far with temporaries, and the hidden-address
    /// result temporaries ([`RESULT_TEMPS`]): their sizes and where they
    /// are placed.
    struct St<'r> {
        max: u32,
        results: &'r [u32],
        placed: Vec<Option<i32>>,
    }
    /// Places `c`'s temporaries below `size`; returns the size below
    /// them.
    fn call(c: &mut Call, mut size: u32, st: &mut St) -> u32 {
        for a in c.args.iter_mut().rev() {
            match a {
                Arg::Temp { value, offset, .. } => {
                    let n = value.ty().bytes();
                    let align = if n >= 4 { 4 } else { 2 };
                    size = (size + n).div_ceil(align) * align;
                    *offset = -(size as i32);
                    st.max = st.max.max(size);
                }
                // The temporary of a hidden-address result (`call`), the
                // call's first argument: placed as the others, in push
                // order, at a multiple of 4 (TIME_$SET_ITIMER
                // 0xE58EA8, 0xE58EB8: two 6-byte results at -8 and
                // -0x10).  The pseudo offset stays until every call is
                // placed, the last placement standing (below).
                Arg::Address {
                    addr: Addr::Frame(o),
                    ..
                } if result_temp_index(*o).is_some() => {
                    let k = result_temp_index(*o).unwrap();
                    size = (size + st.results[k]).div_ceil(4) * 4;
                    st.placed[k] = Some(-(size as i32));
                    st.max = st.max.max(size);
                }
                // A hidden-address result passed on by address: the
                // inner call's temporaries, its result's among them, are
                // the outer call's at this point of the push order, so
                // that two such arguments get two places (TIME_$SET_ITIMER
                // 0xE58EA8 -8, 0xE58EB8 -0x10).
                Arg::Call { call: ic, .. } => size = call(ic, size, st),
                _ => {}
            }
        }
        for a in c.args.iter_mut() {
            match a {
                Arg::Value { value, .. } | Arg::Temp { value, .. } => {
                    expr(value, size, st);
                }
                Arg::Address { addr: a, .. } => addr(a, size, st),
                Arg::Call { .. } => {}
            }
        }
        if let Some(x) = c.pointer_mut() {
            expr(x, size, st);
        }
        size
    }
    fn addr(a: &mut Addr, size: u32, st: &mut St) {
        match a {
            Addr::Volatile { addr: a, .. } => addr(a, size, st),
            Addr::Indirect { base, .. } => expr(base, size, st),
            Addr::Index { base, index, .. } => {
                addr(base, size, st);
                expr(index, size, st);
            }
            Addr::Frame(_)
            | Addr::Enclosing(_)
            | Addr::Static(_)
            | Addr::OtherStatic { .. }
            | Addr::Symbol { .. }
            | Addr::Const(_) => {}
        }
    }
    fn expr(e: &mut Expr, size: u32, st: &mut St) {
        match e {
            Expr::Const { .. } => {}
            Expr::Load { addr: a, .. } | Expr::Bits { addr: a, .. } | Expr::AddrOf(a) => {
                addr(a, size, st)
            }
            Expr::SetMember {
                elem: x, set: y, ..
            }
            | Expr::Binary { lhs: x, rhs: y, .. }
            | Expr::SetOp { lhs: x, rhs: y, .. } => {
                expr(x, size, st);
                expr(y, size, st);
            }
            Expr::SetMemberAt { elem, set: a, .. }
            | Expr::BitElem {
                index: elem,
                array: a,
                ..
            } => {
                expr(elem, size, st);
                addr(a, size, st);
            }
            Expr::Unary { operand: x, .. }
            | Expr::Convert { value: x, .. }
            | Expr::SetSr(x)
            | Expr::SetOf { elem: x, .. } => expr(x, size, st),
            Expr::Call { call: c, .. } => {
                call(c, size, st);
            }
            Expr::Test(c) => cond(c, size, st),
        }
    }
    fn cond(c: &mut Cond, size: u32, st: &mut St) {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                expr(lhs, size, st);
                expr(rhs, size, st);
            }
            Cond::Test(e) | Cond::Odd(e) => expr(e, size, st),
            Cond::Not(x) => cond(x, size, st),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                cond(a, size, st);
                cond(b, size, st);
            }
            Cond::Block { lhs, rhs, .. } => {
                addr(lhs, size, st);
                addr(rhs, size, st);
            }
            Cond::SetEmpty { set, .. } => addr(set, size, st),
        }
    }
    let mut st = St {
        max: frame,
        results,
        placed: vec![None; results.len()],
    };
    for s in body.iter_mut() {
        // `stmt_calls_mut` visits a call nested in another's arguments
        // before the outer one, which then places it again, below its own
        // temporaries: the last placement stands.
        stmt_calls_mut(s, &mut |c| {
            call(c, frame, &mut st);
        });
    }
    // The result temporaries' pseudo offsets replaced: in the calls and in
    // the copies out of them.
    fn place(o: &mut i32, placed: &[Option<i32>]) {
        if let Some(k) = result_temp_index(*o) {
            *o = placed[k].expect("a result temporary is placed with its call");
        }
    }
    fn copies(body: &mut [Stmt], placed: &[Option<i32>]) {
        for s in body.iter_mut() {
            stmt_calls_mut(s, &mut |c| {
                for a in &mut c.args {
                    if let Arg::Address {
                        addr: Addr::Frame(o),
                        ..
                    } = a
                    {
                        place(o, placed);
                    }
                }
            });
        }
        for s in body {
            match &mut s.kind {
                StmtKind::Copy {
                    src: Addr::Frame(o),
                    ..
                } => place(o, placed),
                StmtKind::If { then_, else_, .. } => {
                    copies(then_, placed);
                    copies(else_, placed);
                }
                StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                    copies(body, placed)
                }
                StmtKind::For(l) => copies(&mut l.body, placed),
                StmtKind::Case {
                    arms, otherwise, ..
                } => {
                    for a in arms {
                        copies(&mut a.body, placed);
                    }
                    if let Some(o) = otherwise {
                        copies(o, placed);
                    }
                }
                _ => {}
            }
        }
    }
    if !results.is_empty() {
        copies(body, &st.placed);
    }
    st.max
}

/// Lower one routine.  `uplevel`: the objects of its frame that its nested
/// routines reach ([`Function::uplevel_refs`]).
fn lower_routine(
    p: &Program,
    id: RoutineId,
    uplevel: &[(i32, u32)],
) -> Result<Function, Vec<Diagnostic>> {
    // For the driver's internal-error backstop (`diag::set_context`).
    let outer = dpc_syntax::diag::set_context(Some(p.routine(id).span));
    let f = lower_routine_in_context(p, id, uplevel);
    dpc_syntax::diag::set_context(outer);
    f
}

fn lower_routine_in_context(
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
    // A static variable of another data section than the routine's is
    // [`Addr::OtherStatic`] ([`var_addr`]; codegen-patterns 4, 11: such
    // data is reached from its block's absolute address, PROC1_$INIT
    // 0xE2F95C in OS_INIT_PROC `movea.l #0xE254E8,A0`, PROC1_'s WIRED_DATA
    // block, then `(0xc40,A0)`).
    let _scope = DataSectionScope::enter(&data_section);
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
    // A result through a hidden address (codegen-patterns 3, "Record
    // results over 4 bytes") comes back in no register: for the selector
    // the function is a procedure whose first argument slot holds the
    // result's address (listed in [`Function::address_params`]).
    let hidden = is_hidden_result(r);
    if hidden && r.parent.is_some() {
        errors.push(unsupported(r.span, HIDDEN_NESTED));
    }
    let result = match &r.result {
        None => None,
        Some(_) if hidden => None,
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
    let (mut body, with_temps, elem_temps, result_temps) =
        if r.result.is_some() && result.is_none() && !hidden {
            (Vec::new(), Vec::new(), 0, Vec::new())
        } else {
            match lower_body(p, id, r, result, uplevel) {
                Ok(b) => b,
                Err(d) => {
                    errors.push(d);
                    (Vec::new(), Vec::new(), 0, Vec::new())
                }
            }
        };
    let frame_size = assign_temps(
        &mut body,
        with_slots(r.frame_size, elem_temps),
        &result_temps,
    );
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
        has_data_base: false,
        data_absolute: false,
        value_params: value_params(p, r),
        // The hidden result address first: it is argument 1.
        address_params: r
            .result
            .as_ref()
            .and_then(|res| res.var)
            .and_then(|v| match p.var(v).storage {
                dpc_sema::Storage::HiddenResult { slot } => Some(slot),
                _ => None,
            })
            .into_iter()
            .chain(
                r.params
                    .iter()
                    .filter(|prm| prm.passing() == Passing::ByAddress)
                    .map(|prm| prm.slot_offset),
            )
            .collect(),
        scalar_locals: scalar_locals(p, r, uplevel),
        nesting: r.parent.map(|pid| Nesting {
            parent: p.routine(pid).symbol.clone(),
            // Decided once every routine is lowered (`set_static_links`).
            link: StaticLink::CallerFrame,
        }),
        uplevel_refs: uplevel.to_vec(),
        param_copies,
        with_temps,
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
        // A routine that inherits the base and uses the data has callers
        // that use it too (`uses` is closed over the calls of inheriting
        // callees), so each of them has it: by induction, the loading
        // routine up the chain has loaded it.
        f.has_data_base =
            f.loads_data_base || (uses[i] && has[i] && !absolute[i] && !f.is_empty_procedure());
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
        StmtKind::Store { addr, value }
        | StmtKind::StoreBits { addr, value, .. }
        | StmtKind::StoreBitElem {
            array: addr,
            index: value,
            ..
        }
        | StmtKind::StoreSetMember {
            set: addr,
            elem: value,
            ..
        } => {
            addr_calls(addr, out);
            e(value, out);
        }
        StmtKind::StoreSetConst { set, .. } => addr_calls(set, out),
        StmtKind::StoreSetRange { set, lo, hi, .. } => {
            addr_calls(set, out);
            e(hi, out);
            e(lo, out);
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
            if let Some(u) = until {
                cond_calls(u, out);
            }
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
            Arg::Call { call, .. } => {
                out.push(call);
                call_arg_calls(call, out);
            }
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
        Cond::Test(e) | Cond::Odd(e) => expr_calls(e, out),
        Cond::Not(x) => cond_calls(x, out),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
            cond_calls(a, out);
            cond_calls(b, out);
        }
        Cond::Block { lhs, rhs, .. } => {
            addr_calls(lhs, out);
            addr_calls(rhs, out);
        }
        Cond::SetEmpty { set, .. } => addr_calls(set, out),
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
        Expr::SetMemberAt { elem, set, .. }
        | Expr::BitElem {
            index: elem,
            array: set,
            ..
        } => {
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
        | Addr::OtherStatic { .. }
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
        StmtKind::Store { addr, value }
        | StmtKind::StoreBits { addr, value, .. }
        | StmtKind::StoreBitElem {
            array: addr,
            index: value,
            ..
        }
        | StmtKind::StoreSetMember {
            set: addr,
            elem: value,
            ..
        } => addr_uses_static(addr) || expr_uses_static(value),
        StmtKind::StoreSetConst { set, .. } => addr_uses_static(set),
        StmtKind::StoreSetRange { set, lo, hi, .. } => {
            addr_uses_static(set) || expr_uses_static(hi) || expr_uses_static(lo)
        }
        StmtKind::Copy { dst, src, .. } => addr_uses_static(dst) || addr_uses_static(src),
        StmtKind::Call(c) => call_uses_static(c),
        StmtKind::Fill { start, value, .. } => addr_uses_static(start) || expr_uses_static(value),
        StmtKind::If { cond, then_, else_ } => {
            cond_uses_static(cond) || then_.iter().chain(else_).any(stmt_uses_static)
        }
        StmtKind::Repeat { body, until } => {
            body.iter().any(stmt_uses_static) || until.as_ref().is_some_and(cond_uses_static)
        }
        StmtKind::While { cond, body } => {
            cond_uses_static(cond) || body.iter().any(stmt_uses_static)
        }
        StmtKind::For(l) => {
            l.home.as_ref().is_some_and(addr_uses_static)
                || expr_uses_static(&l.from)
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
        Cond::Test(e) | Cond::Odd(e) => expr_uses_static(e),
        Cond::Not(x) => cond_uses_static(x),
        Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
            cond_uses_static(a) || cond_uses_static(b)
        }
        Cond::Block { lhs, rhs, .. } => addr_uses_static(lhs) || addr_uses_static(rhs),
        Cond::SetEmpty { set, .. } => addr_uses_static(set),
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
        Arg::Call { call, .. } => call_uses_static(call),
    }) || c.pointer().is_some_and(expr_uses_static)
}

fn expr_uses_static(e: &Expr) -> bool {
    match e {
        Expr::Const { .. } => false,
        Expr::Load { addr, .. } | Expr::Bits { addr, .. } | Expr::AddrOf(addr) => {
            addr_uses_static(addr)
        }
        Expr::SetMember { elem, set, .. } => expr_uses_static(elem) || expr_uses_static(set),
        Expr::SetMemberAt { elem, set, .. }
        | Expr::BitElem {
            index: elem,
            array: set,
            ..
        } => expr_uses_static(elem) || addr_uses_static(set),
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
        Addr::Frame(_)
        | Addr::Enclosing(_)
        | Addr::OtherStatic { .. }
        | Addr::Symbol { .. }
        | Addr::Const(_) => false,
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
                ty: scalar_of_type(&p.types, var.ty, width),
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
/// A record or array with volatile or device parts is refused: no entry
/// copy of one is seen, and [`ParamCopy`] has no volatile marking for
/// the selector.  (An assignment of one is the ordinary block copy, its
/// moves marked volatile, [`copy`]; codegen-patterns 5.5.)
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
        // The copy moves words and longwords from the argument's address
        // ([`ParamCopy`]), which a byte-aligned parameter type lets be odd
        // (ref 3.15.6.8 p. 3-76; the 68010 traps; see [`check_even`]):
        // a scalar, and a record with an `aligned(0)` of its own or a
        // 2-byte one that inherits it ([`type_is_even`], as for a
        // reference through the parameter; bead domain-pascal-cr4a: an
        // `[aligned(0)]` packed record argument at an odd offset was
        // copied with `move.w (A0),(-2,A6)`).  No byte copy of a parameter
        // at entry is seen (below), so these are refused.
        if p.types.size(prm.ty) > 1
            && (p.types.attrs(prm.ty).align == Some(1) || p.types.get(prm.ty).align_attr == Some(1))
            && !type_is_even(p, prm.ty)
        {
            return Err(unsupported(
                p.var(prm.var).span,
                "copying a value parameter of a byte-aligned type (`aligned(0)`) of more than \
                 one byte, whose argument may be at an odd address (the 68010 traps on an odd \
                 word or longword access; the original's code for one is not known, ref \
                 3.15.6.6) is",
            ));
        }
        // No byte copy of compiled code copies a parameter's argument to
        // the frame at entry (of the 50 `move.b (Am)+,(An)+` copies of
        // codegen-patterns 5.2, "Arrays of char", the one at a routine's
        // start, ACL_ 0xE45134, copies into an `out` parameter), so the
        // callee's copy of an array of char is not seen.
        if byte_copy(p, prm.ty) != ByteCopy::No {
            return Err(unsupported(
                p.var(prm.var).span,
                "copying a value parameter that is an array of char or of one-byte elements (the \
                 original's entry copy of one is not seen: codegen-patterns 5.2 \"Arrays of \
                 char\", bead domain-pascal-4lf1) is",
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
                ty: scalar_of_type(&p.types, prm.ty, width),
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
        // A 4-byte record: in D0 as a longword (codegen-patterns 3,
        // "Record results"); the result variable stays in its frame slot
        // ([`record_result_assignment`]).
        ResultReg::Record => ResultClass::Data,
        // Through a hidden address, no register (`lower_routine_in_context`
        // gives such a function no [`ResultSpec`]).
        ResultReg::Hidden => unreachable!("a hidden-address result has no result register"),
        ResultReg::Unknown => {
            return Err(Diagnostic::error(
                r.span,
                dpc_sema::conv::unknown_result_message(res.ty, &p.types),
            ))
        }
    };
    // The result variable of a volatile type (`function f: [volatile]
    // integer32`, ref 3.15.1): the selector may hold the result variable in
    // a register, which a volatile object never is (ref 3.15.1 p. 3-56;
    // [`scalar_locals`]); how the original treated one is not seen.
    if p.types.has_volatile_part(res.ty) {
        return Err(unsupported(
            r.span,
            "a function result of a volatile, atomic or device type (ref 3.15.1-3.15.3) is",
        ));
    }
    let size = p.types.size(res.ty);
    let width = Width::of_size(size)
        .ok_or_else(|| unsupported(r.span, format!("a {size}-byte function result is")))?;
    let mut ty = scalar_of_type(&p.types, res.ty, width);
    // A 4-byte integer result in A0 (`a0_return`, ref 5.5.11 p. 5-20:
    // "Puts into A0 any values returned from that routine that should go
    // into a register"): returned as the selector returns a pointer.  The
    // image's TTY_$I_GET_DESC 0xE667B0 and SIO_$I_GET_DESC 0xE66808 (read
    // as integer32 `a0_return` functions by ../source, after their callers'
    // `move.l A0,D0` parking) end with the pointer-result form
    // (codegen-patterns 3, "Pointer results").
    if class == ResultClass::Address && !ty.pointer && width == Width::Long {
        ty = Scalar::POINTER;
    }
    Ok(ResultSpec { class, ty })
}

/// The statements of `r`'s body.
fn lower_body(
    p: &Program,
    id: RoutineId,
    r: &Routine,
    result: Option<ResultSpec>,
    uplevel: &[(i32, u32)],
) -> Res<(Vec<Stmt>, Vec<i32>, usize, Vec<u32>)> {
    let block = r.body.as_ref().expect("routine with a body");
    RESULT_TEMPS.with(|t| t.borrow_mut().clear());
    let labels = routine_labels(&block.stmts)?;
    let cx = Cx {
        p,
        id,
        r,
        result,
        labels,
        uplevel,
        with_temps: Default::default(),
        with_elem_temps: Default::default(),
        for_indices: Default::default(),
        hidden_assigned: Default::default(),
    };
    let mut out = cx.seq(&block.stmts)?;
    let with_temps = cx.with_temps.into_inner();
    let elem_temps = cx.with_elem_temps.get();
    let result_temps = RESULT_TEMPS.with(|t| t.take());
    check_uplevel_indices(&out, uplevel)?;
    check_for_indices(&mut out)?;
    if r.result.is_some() {
        // ISO requires an assignment to the function variable (the
        // reference's error "No assignment to function variable"); a body
        // with none is not compiled.
        let mut assigned = cx.hidden_assigned.get();
        crate::visit_stmts(&out, &mut |s| {
            assigned |= matches!(s.kind, StmtKind::SetResult(_));
        });
        if !assigned {
            return Err(unsupported(
                r.span,
                "a function with no assignment to its result is",
            ));
        }
        if is_record_result(r) {
            check_record_result(&out, r.span)?;
        }
    }
    Ok((out, with_temps, elem_temps, result_temps))
}

/// No `for` loop of `body` has an index that a nested routine reaches
/// (bead domain-pascal-thc).  The selector keeps an index in a register
/// for its loop (codegen-patterns 7.4), where a nested routine called from
/// the body would not see it.  The image keeps such an index in the frame:
/// RINGLOG_$CNTL 0xE72226, whose nested 0xE721CC runs its own loop over
/// the same variable through the static link (0xE721F0 `move.w
/// #1,(-2,A2)`, 0xE7220A `addq.w #1,(-2,A2)`), steps its loop at
/// 0xE72296..0xE722B0 in memory (`clr.w (-2,A6)`, `muls.w (-2,A6),D1`,
/// `addq.w #1,(-2,A6)`, beside the `dbf` count in D0): `Cx::for_loop`
/// gives such a loop a [`ForLoop::home`], so this holds by construction
/// (a check kept for the loops of the register form).
fn check_uplevel_indices(body: &[Stmt], uplevel: &[(i32, u32)]) -> Res<()> {
    if uplevel.is_empty() {
        return Ok(());
    }
    for st in body {
        let inner: Vec<&[Stmt]> = match &st.kind {
            StmtKind::For(l) => {
                if l.home.is_none() && overlaps(uplevel, l.index.offset, l.index.ty.bytes()) {
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
/// only inside loops over them, whole, or after one, and never assigned or
/// addressed outside them (ref p. 4-89: "You may not assign a value to
/// index_variable within the body of the for loop").  The selector keeps an
/// index in a register (codegen-patterns 7.4); inside its loop it may also
/// be passed by address to a value or `in` parameter (its register stored
/// to its slot first, "The index passed by address").
///
/// A read after the loop (legal after `exit`, `goto` or `return`, ref p.
/// 4-90; after normal termination Domain Pascal "forbids the use of the
/// value" and warns, message 235, "Potential illegal use of FOR index
/// variable", Diagnostic Messages p. 9-31) reads the register the loop
/// left: the image keeps the
/// index there, one step past the last value after normal termination
/// (codegen-patterns 7.4, "The index read after its loop").  Accepted
/// where the read lies in the statements after it in the same sequence,
/// with no other loop over the index nested among them and no label there
/// that a `goto` from elsewhere reaches; a loop with a run-time bound has
/// the target set the index before its guard (bead domain-pascal-trdz).
///
/// The refusal, and the index it is about when keeping that index in
/// memory makes the use an ordinary one ([`check_for_indices`]).
fn check_for_indices_once(body: &[Stmt]) -> Result<(), (Diagnostic, Option<i32>)> {
    /// A loop whose index the statements after it may read
    /// ([`Check::after`]).
    struct After {
        offset: i32,
        /// The [`Check::depth`] of the sequence holding the loop.
        depth: usize,
        /// The labels every `goto` to which lies in the loop or after it.
        labels: Vec<u32>,
        /// Why a read here is refused, once something between the loop
        /// and the read makes it so.
        taint: Option<&'static str>,
    }
    struct Check {
        /// Every `for` index of the body.
        all: Vec<ValueParam>,
        /// The indices of the loops being walked.
        active: Vec<i32>,
        /// The loops the statement being walked follows in a sequence.
        after: Vec<After>,
        /// Nesting of statement sequences.
        depth: usize,
        /// `(label, gotos)` of the routine.
        gotos: Vec<(u32, usize)>,
        /// The index of the loop whose bounds are being walked.
        bounding: Vec<i32>,
        /// The homes of the loops being walked whose index is kept in
        /// memory ([`ForLoop::home`]).
        homes: Vec<Addr>,
        span: Span,
        /// The index whose use outside its loops the last refusal was
        /// about, when keeping it in memory instead serves
        /// ([`check_for_indices`]).
        culprit: std::cell::Cell<Option<i32>>,
    }
    /// The `goto`s of `body`, by label.
    fn gotos_of(body: &[Stmt], out: &mut Vec<(u32, usize)>) {
        crate::visit_stmts(body, &mut |s| {
            if let StmtKind::Goto(l) = s.kind {
                match out.iter_mut().find(|e| e.0 == l) {
                    Some(e) => e.1 += 1,
                    None => out.push((l, 1)),
                }
            }
        });
    }
    impl Check {
        fn hit(&self, o: i32, size: u32) -> bool {
            self.all
                .iter()
                .any(|l| o < l.offset + l.ty.bytes() as i32 && l.offset < o + size as i32)
        }
        /// The size of the `for` index at `o` (a byte, a word or a
        /// longword).
        fn index_bytes(&self, o: i32) -> i32 {
            self.all
                .iter()
                .find(|l| l.offset == o)
                .map_or(2, |l| l.ty.bytes() as i32)
        }
        /// Whether a load of `ty` at `o` reads a `for` index whole.
        fn whole(&self, o: i32, ty: &Scalar) -> bool {
            self.all
                .iter()
                .any(|l| l.offset == o && l.ty.width == ty.width)
        }
        fn bad(&self, o: i32, size: u32) -> Diagnostic {
            self.culprit.set(
                self.all
                    .iter()
                    .find(|l| o < l.offset + l.ty.bytes() as i32 && l.offset < o + size as i32)
                    .map(|l| l.offset),
            );
            unsupported(
                self.span,
                "the index variable of a `for` loop assigned, addressed or read in part outside \
                 its loops \
                 (another live range of the variable, in another register: AREA_$FREE_\
                 SEGMENTS 0xE0863C `move.w D3,D2` for `n := first_n` after a loop over n in \
                 D5; not modelled, codegen-patterns 7.4) is",
            )
        }
        /// A whole read of the index at `o` outside its loops.
        fn read(&self, o: i32) -> Res<()> {
            // Every refusal of [`Self::read_at`] is of a read that an index
            // kept in memory makes an ordinary one.
            let r = self.read_at(o);
            if r.is_err() {
                self.culprit.set(Some(o));
            }
            r
        }
        fn read_at(&self, o: i32) -> Res<()> {
            if self.bounding.contains(&o) {
                // `for i := 1 to i do` after an earlier loop over i: the
                // bound would read the register that loop left while the
                // new loop takes the variable over (no site seen).
                return Err(unsupported(
                    self.span,
                    "a `for` loop whose bound reads its own index variable (the value an earlier \
                     loop left; no site seen, codegen-patterns 7.4) is",
                ));
            }
            let Some(a) = self.after.iter().rev().find(|a| a.offset == o) else {
                return Err(unsupported(
                    self.span,
                    "the index variable of a `for` loop read before its loop or outside the \
                     statements that follow the loop in its sequence (codegen-patterns 7.4) is",
                ));
            };
            // A loop with a run-time bound: the target sets the index
            // before the guard, so that a loop the guard skips leaves it
            // `from` (XNS_IDP_$REGISTER_ADDR 0xE1905E reads D1, which the
            // image sets at 0xE1902A after the guard; best effort, the
            // target's warning; bead domain-pascal-trdz).
            match a.taint {
                Some(m) => Err(unsupported(self.span, m)),
                None => Ok(()),
            }
        }
        fn addr(&self, a: &Addr, size: u32) -> Res<()> {
            match a {
                Addr::Frame(o)
                    if self
                        .active
                        .iter()
                        .any(|&l| *o < l + self.index_bytes(l) && l < *o + size as i32) =>
                {
                    Err(unsupported(
                        self.span,
                        "the index variable of a `for` loop assigned, addressed or read in part \
                         in its loop (ref p. 4-89: \"You may not assign a value to \
                         index_variable within the body of the for loop\") is",
                    ))
                }
                Addr::Frame(o) if self.hit(*o, size) => Err(self.bad(*o, size)),
                Addr::Frame(_)
                | Addr::Enclosing(_)
                | Addr::Static(_)
                | Addr::OtherStatic { .. }
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
                } if self.active.contains(o) && self.whole(*o, ty) => Ok(()),
                Expr::Load {
                    addr: Addr::Frame(o),
                    ty,
                } if self.whole(*o, ty) => self.read(*o),
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
                Expr::BitElem {
                    array,
                    bytes,
                    index,
                    ..
                } => {
                    self.expr(index)?;
                    self.addr(array, *bytes)
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
                    // The index of a loop being walked passed to a value or
                    // `in` parameter by address: the selector stores its
                    // register to the variable's slot first
                    // (codegen-patterns 7.4, "The index passed by
                    // address").  To a `var`, `out` or `in out` one the
                    // callee could assign it (ref p. 4-89: "You may not
                    // assign a value to index_variable within the body").
                    Arg::Address {
                        addr: Addr::Frame(o),
                        writable: false,
                        ..
                    } if self.active.contains(o) => {}
                    Arg::Address { addr, .. } => self.addr(addr, 1)?,
                    Arg::Call { call, .. } => self.call(call)?,
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
                Cond::Test(e) | Cond::Odd(e) => self.expr(e),
                Cond::Not(x) => self.cond(x),
                Cond::AndThen(a, b) | Cond::OrElse(a, b) | Cond::And(a, b) | Cond::Or(a, b) => {
                    self.cond(a)?;
                    self.cond(b)
                }
                Cond::Block { lhs, rhs, size, .. } => {
                    self.addr(lhs, *size)?;
                    self.addr(rhs, *size)
                }
                Cond::SetEmpty { set, size } => self.addr(set, *size),
            }
        }
        fn stmts(&mut self, body: &[Stmt]) -> Res<()> {
            self.depth += 1;
            let base = self.after.len();
            for (k, st) in body.iter().enumerate() {
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
                    StmtKind::StoreBitElem {
                        array,
                        bytes,
                        index,
                        ..
                    }
                    | StmtKind::StoreSetMember {
                        set: array,
                        bytes,
                        elem: index,
                        ..
                    } => {
                        self.addr(array, *bytes)?;
                        self.expr(index)?;
                    }
                    StmtKind::StoreSetConst { set, bits } => self.addr(set, bits.len() as u32)?,
                    StmtKind::StoreSetRange {
                        set, bytes, lo, hi, ..
                    } => {
                        self.addr(set, *bytes)?;
                        self.expr(hi)?;
                        self.expr(lo)?;
                    }
                    StmtKind::Copy {
                        dst, src, bytes, ..
                    } => {
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
                        if let Some(u) = until {
                            self.cond(u)?;
                        }
                    }
                    StmtKind::While { cond, body } => {
                        self.cond(cond)?;
                        self.stmts(body)?;
                    }
                    // An index kept in memory: its bounds are evaluated
                    // before it is set (`Selector::for_wide`), it is read
                    // in memory after the loop; only a loop over it nested
                    // in another one is refused, as below.
                    StmtKind::For(ForLoop {
                        home: Some(h),
                        from,
                        to,
                        body: inner,
                        ..
                    }) => {
                        self.expr(from)?;
                        self.expr(to)?;
                        if self.homes.contains(h) {
                            return Err(unsupported(
                                self.span,
                                "a `for` loop over the index of an enclosing `for` loop (an \
                                 assignment of its index, ref p. 4-89) is",
                            ));
                        }
                        self.homes.push(h.clone());
                        self.stmts(inner)?;
                        self.homes.pop();
                    }
                    StmtKind::For(l) => {
                        self.bounding.push(l.index.offset);
                        let bounds = self.expr(&l.from).and_then(|()| self.expr(&l.to));
                        self.bounding.pop();
                        bounds?;
                        if self.active.contains(&l.index.offset) {
                            // A loop over the index of an enclosing loop
                            // assigns it.
                            return Err(unsupported(
                                self.span,
                                "a `for` loop over the index of an enclosing `for` loop (an \
                                 assignment of its index, ref p. 4-89) is",
                            ));
                        }
                        let off = l.index.offset;
                        // A loop over the index among the statements after
                        // an earlier loop over it: a read after both may be
                        // reached from either (refused); a later loop of
                        // the same sequence replaces the earlier.
                        let depth = self.depth;
                        self.after.retain(|a| a.offset != off || a.depth != depth);
                        for a in &mut self.after {
                            if a.offset == off {
                                a.taint.get_or_insert(
                                    "the index variable of a `for` loop read after it where \
                                     another loop over it, nested among the statements \
                                     between, may have set it (two live ranges meeting; not \
                                     modelled, codegen-patterns 7.4) is",
                                );
                            }
                        }
                        self.active.push(off);
                        self.stmts(&l.body)?;
                        self.active.pop();
                        let mut mine = Vec::new();
                        gotos_of(&l.body, &mut mine);
                        gotos_of(&body[k + 1..], &mut mine);
                        let labels = mine
                            .iter()
                            .filter(|(lab, n)| self.gotos.contains(&(*lab, *n)))
                            .map(|&(lab, _)| lab)
                            .collect();
                        self.after.push(After {
                            offset: off,
                            depth,
                            labels,
                            taint: None,
                        });
                    }
                    StmtKind::Label(lab) => {
                        for a in &mut self.after {
                            if !a.labels.contains(lab) {
                                a.taint.get_or_insert(
                                    "the index variable of a `for` loop read after it past a \
                                     label a `goto` from before the loop reaches (the index \
                                     not set on that path; not modelled, codegen-patterns 7.4) \
                                     is",
                                );
                            }
                        }
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
                    | StmtKind::Goto(_) => {}
                }
            }
            self.after.truncate(base);
            self.depth -= 1;
            Ok(())
        }
    }
    fn collect(body: &[Stmt], out: &mut Vec<ValueParam>) {
        for st in body {
            match &st.kind {
                StmtKind::For(l) => {
                    // An index kept in memory ([`ForLoop::home`]) is an
                    // ordinary variable outside its loop.
                    if l.home.is_none() {
                        out.push(l.index);
                    }
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
    let mut homes = false;
    crate::visit_stmts(body, &mut |s| {
        homes |= matches!(&s.kind, StmtKind::For(l) if l.home.is_some())
    });
    if all.is_empty() && !homes {
        return Ok(());
    }
    let mut gotos = Vec::new();
    gotos_of(body, &mut gotos);
    let mut check = Check {
        all,
        active: Vec::new(),
        after: Vec::new(),
        depth: 0,
        gotos,
        bounding: Vec::new(),
        homes: Vec::new(),
        span: body[0].span,
        culprit: Default::default(),
    };
    check.stmts(body).map_err(|d| (d, check.culprit.get()))
}

/// [`check_for_indices_once`] until it holds: an index used outside its
/// loops in a way the check refuses for an index held in a register
/// (assigned, addressed or read in part there, as AREA_$FREE_SEGMENTS's
/// `n := first_n` after a loop over n; read before its loop, or after it
/// where another loop over it or a label intervenes) is kept in memory by
/// every loop over it instead ([`ForLoop::home`]), where any use of it is
/// an ordinary variable's.  Best effort (not byte-identical; bead
/// domain-pascal-trdz): the image holds such a variable in registers, one
/// per live range (AREA_$FREE_SEGMENTS 0xE0863C `move.w D3,D2` for `n :=
/// first_n` after a loop over n in D5), which `dpc` does not model.
fn check_for_indices(body: &mut [Stmt]) -> Res<()> {
    loop {
        match check_for_indices_once(body) {
            Ok(()) => return Ok(()),
            Err((d, Some(o))) => {
                if !rehome_index(body, o) {
                    return Err(d);
                }
                dpc_syntax::diag::best_effort(d)?;
            }
            Err((d, None)) => return Err(d),
        }
    }
}

/// Keep the index of every `for` loop of `body` over the local at frame
/// offset `off` in memory, at its own slot ([`check_for_indices`]);
/// whether there was one.
fn rehome_index(body: &mut [Stmt], off: i32) -> bool {
    let mut hit = false;
    for s in body {
        match &mut s.kind {
            StmtKind::For(l) => {
                if l.home.is_none() && l.index.offset == off {
                    l.home = Some(Addr::Frame(off));
                    l.index.offset = 0;
                    hit = true;
                }
                hit |= rehome_index(&mut l.body, off);
            }
            StmtKind::If { then_, else_, .. } => {
                hit |= rehome_index(then_, off);
                hit |= rehome_index(else_, off);
            }
            StmtKind::Repeat { body, .. } | StmtKind::While { body, .. } => {
                hit |= rehome_index(body, off)
            }
            StmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms {
                    hit |= rehome_index(&mut a.body, off);
                }
                if let Some(o) = otherwise {
                    hit |= rehome_index(o, off);
                }
            }
            _ => {}
        }
    }
    hit
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
    /// The objects of the routine's frame its nested routines reach
    /// ([`Function::uplevel_refs`]).
    uplevel: &'a [(i32, u32)],
    /// The `with` temporaries allocated so far ([`Function::with_temps`]).
    with_temps: std::cell::RefCell<Vec<i32>>,
    /// How many of [`Self::with_temps`] hold an array element's address
    /// (the ones given a frame slot: [`with_slots`]).
    with_elem_temps: std::cell::Cell<usize>,
    /// The frame offsets of the indices of the `for` loops whose bodies
    /// are being lowered ([`Cx::with_stmt`]).
    for_indices: std::cell::RefCell<Vec<i32>>,
    /// The body assigns its hidden-address result
    /// ([`hidden_result_assignment`]).
    hidden_assigned: std::cell::Cell<bool>,
}

thread_local! {
    /// The data section of the routine being lowered
    /// ([`lower_routine_in_context`]): a static variable of another is an
    /// [`Addr::OtherStatic`] ([`var_addr`]).
    static DATA_SECTION: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// Sets [`DATA_SECTION`] for the lowering of one routine and restores the
/// previous value however it ends.
struct DataSectionScope(Option<String>);

impl DataSectionScope {
    fn enter(section: &str) -> DataSectionScope {
        DataSectionScope(DATA_SECTION.with(|d| d.replace(Some(section.to_string()))))
    }
}

impl Drop for DataSectionScope {
    fn drop(&mut self) {
        let prev = self.0.take();
        DATA_SECTION.with(|d| *d.borrow_mut() = prev);
    }
}

thread_local! {
    /// The records of the `with` statements whose bodies are being lowered
    /// (ref "With", pp. 4-211..4-214): the [`dpc_sema::WithId`], the
    /// record designator as written, and the address the body's
    /// references to the record stand for ([`Cx::with_stmt`]).
    static WITHS: std::cell::RefCell<Vec<(u32, SExpr, Addr)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Address `a` split into the address with no constant displacement and
/// that displacement, for an array's base under a `with` element
/// ([`Cx::with_stmt`]): a frame, data-area, symbol or pointer base.  An
/// element of an outer array and a volatile wrapper are kept whole.
fn base_displacement(a: Addr) -> (Addr, i32) {
    match a {
        Addr::Frame(o) => (Addr::Frame(0), o),
        Addr::Enclosing(o) => (Addr::Enclosing(0), o),
        Addr::Static(o) => (Addr::Static(0), o),
        Addr::OtherStatic { section, offset } => (Addr::OtherStatic { section, offset: 0 }, offset),
        Addr::Symbol { name, offset } => (Addr::Symbol { name, offset: 0 }, offset),
        Addr::Indirect { base, offset } => (Addr::Indirect { base, offset: 0 }, offset),
        a => (a, 0),
    }
}

/// The designator and address bound to `with` record `w`
/// ([`WITHS`]).
fn with_binding(w: dpc_sema::WithId) -> (SExpr, Addr) {
    try_with_binding(w).expect("a with record is referenced inside its with statement")
}

/// [`with_binding`], or `None` outside the record's `with` statement.
fn try_with_binding(w: dpc_sema::WithId) -> Option<(SExpr, Addr)> {
    WITHS.with(|ws| {
        let ws = ws.borrow();
        ws.iter()
            .rev()
            .find(|(id, _, _)| *id == w.0)
            .map(|(_, e, a)| (e.clone(), a.clone()))
    })
}

/// Pops the bindings a `with` statement pushed ([`WITHS`]), however its
/// lowering ends.
struct WithScope(usize);

impl Drop for WithScope {
    fn drop(&mut self) {
        WITHS.with(|ws| ws.borrow_mut().truncate(self.0));
    }
}

/// The variable `e` designates part of, up to a dereference
/// (`v`, `v.f`, `v[i]`; through a `with` record its designator's).
fn root_var(e: &SExpr) -> Option<VarId> {
    match &e.kind {
        ExprKind::Var { var, uplevel: 0 } => Some(*var),
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => root_var(base),
        // A `with` record is a record, never the pointer variable asked
        // about; one not bound here (another statement's) has no root.
        ExprKind::WithRecord(w) => try_with_binding(*w).and_then(|(e, _)| root_var(&e)),
        _ => None,
    }
}

/// Whether variable `v` may be written by `stmts` (`assign`: assigned, a
/// `for` index) or has its address taken or passed in them (`addr(v)`, an
/// argument passed by address).
fn var_touched(stmts: &[SStmt], v: VarId, assign: bool) -> bool {
    fn expr(e: &SExpr, v: VarId) -> bool {
        match &e.kind {
            ExprKind::Const
            | ExprKind::Var { .. }
            | ExprKind::Result { .. }
            | ExprKind::WithRecord(_)
            | ExprKind::RoutineAddr(_) => false,
            ExprKind::Field { base, .. } => expr(base, v),
            ExprKind::Index { base, index, .. } => expr(base, v) || expr(index, v),
            ExprKind::Deref(x) | ExprKind::Unary { operand: x, .. } | ExprKind::Transfer(x) => {
                expr(x, v)
            }
            ExprKind::Binary { lhs, rhs, .. } => expr(lhs, v) || expr(rhs, v),
            ExprKind::SetCons(items) => items
                .iter()
                .any(|(a, b)| expr(a, v) || b.as_ref().is_some_and(|b| expr(b, v))),
            ExprKind::Call(c) => call(c, v),
            ExprKind::Builtin { builtin, args } => {
                (*builtin == Builtin::Addr && args.iter().any(|a| root_var(a) == Some(v)))
                    || args.iter().any(|a| expr(a, v))
            }
        }
    }
    fn call(c: &SCall, v: VarId) -> bool {
        c.args.iter().any(|a| {
            (a.conv.passing == Passing::ByAddress && root_var(&a.expr) == Some(v))
                || expr(&a.expr, v)
        }) || matches!(&c.callee, SCallee::Pointer(x) if expr(x, v))
    }
    fn stmt(s: &SStmt, v: VarId, assign: bool) -> bool {
        let all = |ss: &[SStmt]| ss.iter().any(|s| stmt(s, v, assign));
        match &s.kind {
            SStmtKind::Empty | SStmtKind::Goto(_) | SStmtKind::Exit | SStmtKind::Next => false,
            SStmtKind::Return => false,
            SStmtKind::Labeled { stmt: x, .. } => stmt(x, v, assign),
            SStmtKind::Assign { target, value } => {
                (assign && root_var(target) == Some(v)) || expr(target, v) || expr(value, v)
            }
            SStmtKind::Call(c) => call(c, v),
            SStmtKind::Builtin { args, .. } => args.iter().any(|a| expr(a, v)),
            SStmtKind::Compound(b) => all(&b.stmts),
            SStmtKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                expr(cond, v)
                    || stmt(then_branch, v, assign)
                    || else_branch.as_ref().is_some_and(|e| stmt(e, v, assign))
            }
            SStmtKind::Case {
                expr: x,
                arms,
                otherwise,
            } => {
                expr(x, v)
                    || arms.iter().any(|a| stmt(&a.stmt, v, assign))
                    || otherwise.as_ref().is_some_and(|o| all(o))
            }
            SStmtKind::While { cond, body } => expr(cond, v) || stmt(body, v, assign),
            SStmtKind::Repeat { body, cond } => all(body) || expr(cond, v),
            SStmtKind::For {
                var,
                from,
                to,
                body,
                ..
            } => {
                (assign && root_var(var) == Some(v))
                    || expr(from, v)
                    || expr(to, v)
                    || stmt(body, v, assign)
            }
            SStmtKind::With { items, body } => {
                items.iter().any(|(_, r)| expr(r, v)) || stmt(body, v, assign)
            }
        }
    }
    stmts.iter().any(|s| stmt(s, v, assign))
}

/// The first offset of the `with` temporaries ([`Function::with_temps`]):
/// far below any frame object, never addressed in memory.
pub const WITH_TEMP_BASE: i32 = -0x4000_0000;

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
        // TODO(ref-question): open question 30 (p. 4-95 against p. D-4).
        // Best effort pending the owner's ruling (bead domain-pascal-sq4a;
        // docs/reference-clarifications.md, question 30): a plain jump to
        // the label, as the original compiler evidently accepted (MST_$TOUCH
        // 0xE0DD68 `bcc.w 0xE0DF12`, a `goto` into a `with`; MST_$INVALIDATE
        // 0xE442B8 likewise).  The statement entered is not set up: a loop's
        // counter and a `with`'s base are whatever they hold, so code at the
        // label that reads them, or reaches the loop's end, is undefined, as
        // the reference leaves it.
        if entered.iter().any(|p| p.kind != Entered::If) {
            // Refused where that undefined state is reached: the code at
            // the label can run on to a `for` loop's end (its `dbf` or
            // step on a count never loaded) or read an entered `with`'s
            // record (its base never evaluated).  The kernel's jumps
            // (MST_$TOUCH, MST_$FORK: leave by `goto` or `return`;
            // MST_$INVALIDATE: into a `with` only, leaving by `exit`) do
            // neither.  A `while` or `repeat` reached at its end tests its
            // condition, which needs no set up; a `case` arm none.
            for p in entered {
                if let Some(why) = entry_unset(stmts, p, name) {
                    return Err(unsupported(*span, why));
                }
            }
            best_effort(
                *span,
                "a `goto` into a `for`, `while`, `repeat`, `with` or `case` statement from \
                 outside it (illegal by ref p. 4-95, permitted by Appendix D p. D-4: open \
                 question 30) is",
            )?;
        }
    }
    Ok(defs.into_iter().map(|(n, _)| n).collect())
}

/// Whether a `goto` to `label` entering statement part `p` from outside
/// ([`routine_labels`]) reaches state the entered statement sets up: for
/// a `for`, the code from the label may run on to the loop's end (the
/// count and index never loaded); for a `with`, it may read one of the
/// statement's records (the base never evaluated).  The reason, or `None`.
/// A `goto` to a label inside the statement is followed (the code from
/// that label is taken too); conservative otherwise: a statement other
/// than `goto`, `return` or `exit` may complete; a loop inside holding
/// the label is taken from the label, then whole (and on after it) when
/// that code may reach its end or leave it by `exit` (MST_$FORK's `fail`
/// in a `with` in a `for` in a `for`, left by `return`).
fn entry_unset(stmts: &[SStmt], p: &Part, label: &str) -> Option<&'static str> {
    fn find(stmts: &[SStmt], id: usize) -> Option<&SStmt> {
        stmts.iter().find_map(|s| find_one(s, id))
    }
    fn find_one(s: &SStmt, id: usize) -> Option<&SStmt> {
        if s as *const SStmt as usize == id {
            return Some(s);
        }
        match &s.kind {
            SStmtKind::Labeled { stmt, .. }
            | SStmtKind::While { body: stmt, .. }
            | SStmtKind::For { body: stmt, .. }
            | SStmtKind::With { body: stmt, .. } => find_one(stmt, id),
            SStmtKind::Compound(b) => find(&b.stmts, id),
            SStmtKind::Repeat { body, .. } => find(body, id),
            SStmtKind::If {
                then_branch,
                else_branch,
                ..
            } => find_one(then_branch, id)
                .or_else(|| else_branch.as_ref().and_then(|e| find_one(e, id))),
            SStmtKind::Case {
                arms, otherwise, ..
            } => arms
                .iter()
                .find_map(|a| find_one(&a.stmt, id))
                .or_else(|| otherwise.as_ref().and_then(|o| find(o, id))),
            _ => None,
        }
    }
    let s = find(stmts, p.stmt)?;
    let (SStmtKind::For { body, .. } | SStmtKind::With { body, .. }) = &s.kind else {
        return None;
    };
    let mut inner = Vec::new();
    labels_in(std::slice::from_ref(s), &mut inner);
    let mut tail = Tail {
        label: label.to_string(),
        seen: Vec::new(),
        loops: 0,
        next: false,
    };
    // The labels reached so far, and whether the code from one runs on
    // to the end of the statement's body.
    let (mut todo, mut done, mut end) = (vec![label.to_string()], Vec::new(), false);
    while let Some(l) = todo.pop() {
        if done.contains(&l) {
            continue;
        }
        tail.label = l.clone();
        done.push(l);
        end |= tail.enter(body) == Some(true);
        let mut gotos = Vec::new();
        for t in &tail.seen {
            gotos_in(t, &mut gotos);
        }
        todo.extend(
            gotos
                .into_iter()
                .filter(|g| inner.contains(g) && !done.contains(g)),
        );
    }
    match &s.kind {
        SStmtKind::For { .. } => (end || tail.next).then_some(
            "a `goto` into a `for` statement whose code at the label can reach the loop's end \
             (its count and index never set up; open question 30) is",
        ),
        SStmtKind::With { items, .. } => {
            let reads = tail.seen.iter().any(|t| {
                stmt_any(t, &|e| {
                    matches!(&e.kind, ExprKind::WithRecord(w) if items.iter().any(|(i, _)| i == w))
                })
            });
            reads.then_some(
                "a `goto` into a `with` statement whose code at the label reads its record (the \
                 record never evaluated; open question 30) is",
            )
        }
        _ => None,
    }
}

/// The labels `goto` statements in `s` name, at any depth.
fn gotos_in(s: &SStmt, out: &mut Vec<String>) {
    match &s.kind {
        SStmtKind::Goto(l) => out.push(l.clone()),
        SStmtKind::Labeled { stmt, .. }
        | SStmtKind::While { body: stmt, .. }
        | SStmtKind::For { body: stmt, .. }
        | SStmtKind::With { body: stmt, .. } => gotos_in(stmt, out),
        SStmtKind::Compound(b) => b.stmts.iter().for_each(|t| gotos_in(t, out)),
        SStmtKind::Repeat { body, .. } => body.iter().for_each(|t| gotos_in(t, out)),
        SStmtKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            gotos_in(then_branch, out);
            if let Some(e) = else_branch {
                gotos_in(e, out);
            }
        }
        SStmtKind::Case {
            arms, otherwise, ..
        } => {
            arms.iter().for_each(|a| gotos_in(&a.stmt, out));
            if let Some(o) = otherwise {
                o.iter().for_each(|t| gotos_in(t, out));
            }
        }
        _ => {}
    }
}

/// The labels defined in `stmts`, at any depth.
fn labels_in(stmts: &[SStmt], out: &mut Vec<String>) {
    for s in stmts {
        match &s.kind {
            SStmtKind::Labeled { label, stmt } => {
                out.push(label.clone());
                labels_in(std::slice::from_ref(stmt), out);
            }
            SStmtKind::While { body, .. }
            | SStmtKind::For { body, .. }
            | SStmtKind::With { body, .. } => labels_in(std::slice::from_ref(body), out),
            SStmtKind::Compound(b) => labels_in(&b.stmts, out),
            SStmtKind::Repeat { body, .. } => labels_in(body, out),
            SStmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                labels_in(std::slice::from_ref(then_branch), out);
                if let Some(e) = else_branch {
                    labels_in(std::slice::from_ref(e), out);
                }
            }
            SStmtKind::Case {
                arms, otherwise, ..
            } => {
                for a in arms {
                    labels_in(std::slice::from_ref(&a.stmt), out);
                }
                if let Some(o) = otherwise {
                    labels_in(o, out);
                }
            }
            _ => {}
        }
    }
}

/// The code a jump to `label` runs inside a statement ([`entry_unset`]):
/// [`Tail::enter`] collects the whole statements it may run in `seen`; a
/// `goto` ends the code there (the code from a label inside the entered
/// statement is taken separately, [`entry_unset`]).
struct Tail<'s> {
    label: String,
    seen: Vec<&'s SStmt>,
    /// The loops inside the entered statement that hold the label and
    /// enclose the code taken: an `exit` there leaves only such a loop.
    loops: usize,
    /// A `next` outside such loops was taken: it goes to the entered
    /// loop's end (its step and `dbf`).
    next: bool,
}

impl<'s> Tail<'s> {
    /// From the label, inside `s`: `None` when `s` does not contain it,
    /// otherwise whether control may complete `s`.
    fn enter(&mut self, s: &'s SStmt) -> Option<bool> {
        match &s.kind {
            SStmtKind::Labeled { label, stmt } if *label == self.label => {
                self.take(stmt);
                Some(self.completes(stmt))
            }
            SStmtKind::Labeled { stmt, .. } => self.enter(stmt),
            SStmtKind::Compound(b) => self.seq(&b.stmts),
            SStmtKind::If {
                then_branch,
                else_branch,
                ..
            } => self
                .enter(then_branch)
                .or_else(|| else_branch.as_ref().and_then(|e| self.enter(e))),
            SStmtKind::Case {
                arms, otherwise, ..
            } => arms
                .iter()
                .find_map(|a| self.enter(&a.stmt))
                .or_else(|| otherwise.as_ref().and_then(|o| self.seq(o))),
            SStmtKind::With { body, .. } => self.enter(body),
            // A loop inside holding the label: the code from it, and when
            // that may run on to the loop's end (or leave it by `exit`),
            // the whole loop again and what follows it.
            SStmtKind::While { body, .. } | SStmtKind::For { body, .. } => {
                self.loops += 1;
                let r = self.enter(body);
                self.loops -= 1;
                self.again(s, r?)
            }
            SStmtKind::Repeat { body, .. } => {
                self.loops += 1;
                let r = self.seq(body);
                self.loops -= 1;
                self.again(s, r?)
            }
            _ => None,
        }
    }

    /// Statement `s` may run: into [`Tail::seen`], and a `next` in it
    /// outside its loops noted ([`Tail::next`]) when no loop inside the
    /// entered statement encloses it.
    fn take(&mut self, s: &'s SStmt) {
        fn free_next(s: &SStmt) -> bool {
            match &s.kind {
                SStmtKind::Next => true,
                SStmtKind::Labeled { stmt, .. } | SStmtKind::With { body: stmt, .. } => {
                    free_next(stmt)
                }
                SStmtKind::Compound(b) => b.stmts.iter().any(free_next),
                SStmtKind::If {
                    then_branch,
                    else_branch,
                    ..
                } => free_next(then_branch) || else_branch.as_ref().is_some_and(|e| free_next(e)),
                SStmtKind::Case {
                    arms, otherwise, ..
                } => {
                    arms.iter().any(|a| free_next(&a.stmt))
                        || otherwise.as_ref().is_some_and(|o| o.iter().any(free_next))
                }
                _ => false,
            }
        }
        if self.loops == 0 && free_next(s) {
            self.next = true;
        }
        self.seen.push(s);
    }

    /// [`Tail::enter`] of loop `s` once the code from the label in its
    /// body may complete (`completes`).
    fn again(&mut self, s: &'s SStmt, completes: bool) -> Option<bool> {
        if completes {
            self.seen.push(s);
        }
        Some(completes)
    }

    /// [`Tail::enter`] for a statement sequence: the statement holding the
    /// label, then those after it while each may complete.
    fn seq(&mut self, stmts: &'s [SStmt]) -> Option<bool> {
        let i = stmts.iter().position(|s| {
            let mut l = Vec::new();
            labels_in(std::slice::from_ref(s), &mut l);
            l.iter().any(|n| *n == self.label)
        })?;
        if !self.enter(&stmts[i])? {
            return Some(false);
        }
        for t in &stmts[i + 1..] {
            self.take(t);
            if !self.completes(t) {
                return Some(false);
            }
        }
        Some(true)
    }

    /// Whether control may leave `s` at its end (conservative).
    fn completes(&self, s: &SStmt) -> bool {
        match &s.kind {
            SStmtKind::Goto(_) | SStmtKind::Return => false,
            // Out of a loop inside holding the label: on after that loop.
            SStmtKind::Exit => self.loops > 0,
            SStmtKind::Labeled { stmt, .. } | SStmtKind::With { body: stmt, .. } => {
                self.completes(stmt)
            }
            SStmtKind::Compound(b) => b.stmts.iter().all(|t| self.completes(t)),
            SStmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.completes(then_branch)
                    || else_branch.as_ref().is_none_or(|e| self.completes(e))
            }
            SStmtKind::Case {
                arms, otherwise, ..
            } => {
                arms.iter().any(|a| self.completes(&a.stmt))
                    || otherwise
                        .as_ref()
                        .is_none_or(|o| o.iter().all(|t| self.completes(t)))
            }
            _ => true,
        }
    }
}

/// Whether `pred` holds of an expression in `s`, at any depth.
fn stmt_any(s: &SStmt, pred: &dyn Fn(&SExpr) -> bool) -> bool {
    fn expr(e: &SExpr, f: &dyn Fn(&SExpr) -> bool) -> bool {
        f(e) || match &e.kind {
            ExprKind::Const
            | ExprKind::Var { .. }
            | ExprKind::Result { .. }
            | ExprKind::WithRecord(_)
            | ExprKind::RoutineAddr(_) => false,
            ExprKind::Field { base, .. } => expr(base, f),
            ExprKind::Index { base, index, .. } => expr(base, f) || expr(index, f),
            ExprKind::Deref(x) | ExprKind::Unary { operand: x, .. } | ExprKind::Transfer(x) => {
                expr(x, f)
            }
            ExprKind::Binary { lhs, rhs, .. } => expr(lhs, f) || expr(rhs, f),
            ExprKind::SetCons(items) => items
                .iter()
                .any(|(a, b)| expr(a, f) || b.as_ref().is_some_and(|b| expr(b, f))),
            ExprKind::Call(c) => call(c, f),
            ExprKind::Builtin { args, .. } => args.iter().any(|a| expr(a, f)),
        }
    }
    fn call(c: &SCall, f: &dyn Fn(&SExpr) -> bool) -> bool {
        c.args.iter().any(|a| expr(&a.expr, f))
            || matches!(&c.callee, SCallee::Pointer(x) if expr(x, f))
    }
    let all = |ss: &[SStmt]| ss.iter().any(|t| stmt_any(t, pred));
    match &s.kind {
        SStmtKind::Empty
        | SStmtKind::Goto(_)
        | SStmtKind::Exit
        | SStmtKind::Next
        | SStmtKind::Return => false,
        SStmtKind::Labeled { stmt, .. } => stmt_any(stmt, pred),
        SStmtKind::Assign { target, value } => expr(target, pred) || expr(value, pred),
        SStmtKind::Call(c) => call(c, pred),
        SStmtKind::Builtin { args, .. } => args.iter().any(|a| expr(a, pred)),
        SStmtKind::Compound(b) => all(&b.stmts),
        SStmtKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            expr(cond, pred)
                || stmt_any(then_branch, pred)
                || else_branch.as_ref().is_some_and(|e| stmt_any(e, pred))
        }
        SStmtKind::Case {
            expr: x,
            arms,
            otherwise,
        } => {
            expr(x, pred)
                || arms.iter().any(|a| stmt_any(&a.stmt, pred))
                || otherwise.as_ref().is_some_and(|o| all(o))
        }
        SStmtKind::While { cond, body } => expr(cond, pred) || stmt_any(body, pred),
        SStmtKind::Repeat { body, cond } => all(body) || expr(cond, pred),
        SStmtKind::For {
            var,
            from,
            to,
            body,
            ..
        } => expr(var, pred) || expr(from, pred) || expr(to, pred) || stmt_any(body, pred),
        SStmtKind::With { items, body } => {
            items.iter().any(|(_, r)| expr(r, pred)) || stmt_any(body, pred)
        }
    }
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
                SStmtKind::Assign { target, value } if matches!(target.kind, ExprKind::Result { routine, uplevel: 0 } if routine == self.id) => {
                    if is_hidden_result(self.r) {
                        out.push(hidden_result_assignment(p, self.r, s, value)?);
                        self.hidden_assigned.set(true);
                    } else if is_record_result(self.r) {
                        out.extend(record_result_assignment(p, self.r, s, value)?);
                    } else {
                        out.push(result_assignment(p, s, value, self.result)?);
                    }
                }
                SStmtKind::Assign { target, .. }
                    if matches!(target.kind, ExprKind::Result { .. }) =>
                {
                    return Err(unsupported(
                        target.span,
                        "assignment to an enclosing function's result is",
                    ));
                }
                SStmtKind::Assign { target, value } if hidden_call(p, value).is_some() => {
                    out.extend(self.hidden_call_assignment(s, target, value)?)
                }
                SStmtKind::Assign { target, value }
                    if is_block(p, target) || byte_array_copy(p, target, value) =>
                {
                    out.push(copy(p, s, target, value)?)
                }
                // The same packed record with and without a `[byte]` size
                // attribute (the 1-byte side is a scalar's size, the
                // 2-byte side a word's): the one byte copied as a block
                // ([`copy`]; bead domain-pascal-7kpw).
                SStmtKind::Assign { target, value }
                    if p.types.canon(target.ty) == p.types.canon(value.ty)
                        && p.types.size(target.ty) != p.types.size(value.ty)
                        && matches!(
                            p.types.kind(p.types.canon(target.ty)),
                            TypeKind::Record(_)
                        ) =>
                {
                    out.push(copy(p, s, target, value)?)
                }
                SStmtKind::Assign { target, value }
                    if is_set(p, target.ty) && p.types.size(target.ty) > 4 =>
                {
                    out.extend(large_set_store(p, s, target, value)?)
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
                    if cond.value.is_some() {
                        // The part a constant condition never runs is dead
                        // code, eliminated at `-opt 1` and above (ref
                        // 6.4.24, p. 6-27); constant expressions are
                        // computed at every level (same page).  A removed
                        // part leaves nothing in the image, and the
                        // kernel's optimisation level is not recorded.
                        return Err(unsupported(
                            cond.span,
                            "an `if` with a constant condition (whether the original removes the \
                             part it never runs is not seen, codegen-patterns 7.1) is",
                        ));
                    }
                    if let (None, Some(SStmtKind::Assign { target, value })) =
                        (else_branch, sole_stmt(then_branch).map(|t| &t.kind))
                    {
                        if narrowing_select(p, cond, target, value) {
                            best_effort(
                                s.span,
                                "a select `if a < b then a := b` that narrows `b` into `a` (the \
                                 compare and the store at the wider width in `a`'s register, \
                                 XNS_ERROR_$SEND 0xE17A04 `move.l (A3),D5`; codegen-patterns 8, \
                                 bead domain-pascal-dzad) is",
                            )?;
                        }
                    }
                    let then_ = self.seq(std::slice::from_ref(then_branch))?;
                    if then_.is_empty() {
                        // An empty `then` and no `else` (ref "Statements", p. 4-188: the
                        // empty statement is a statement): the test and
                        // its branch dropped, the condition's operands
                        // evaluated for nothing ([`empty_if_operands`]).
                        // SAU8 PBU2_$ASSOC 0x3C6169E `bfextu
                        // (0x10,A2){5:3},D5`, never tested, D5 the next
                        // loop's index (`moveq #1,D5`, 0x3C616B0): `if
                        // dcte^.unit_info.bus <> io_$bus_vme then ;`, its
                        // Multibus arm configured out (../source
                        // pascal-translation-notes findings 700, 702; the
                        // SAU7 likewise).  An operand in memory leaves no
                        // instruction ([`StmtKind::Discard`] of a load
                        // reads nothing), a packed field its extraction, a
                        // call its call.  With an `else` part, what the
                        // original emitted (a branch on the negated
                        // condition?) is not known.
                        let ops = match else_branch {
                            None => empty_if_operands(&cond_of(p, cond)?),
                            Some(_) => None,
                        };
                        let Some(ops) = ops else {
                            return Err(unsupported(
                                s.span,
                                "an `if` with an empty `then` part is",
                            ));
                        };
                        out.extend(ops.into_iter().map(|e| Stmt {
                            kind: StmtKind::Discard(e),
                            span: s.span,
                        }));
                        continue;
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
                SStmtKind::Repeat { body, cond } => {
                    // `until false` (any condition whose value is the
                    // constant `false`: constant expressions are computed
                    // at every optimisation level, ref 6.4.24, p. 6-27): a
                    // loop with no test, a `bra` back to its head
                    // (codegen-patterns 7.2, MEM_LITES 0xE0C410).
                    // `until true` runs the body once; whether the
                    // original kept a loop head for it is not seen.
                    let until =
                        match cond.value {
                            Some(ConstValue::Bool(false)) => None,
                            Some(_) => return Err(unsupported(
                                cond.span,
                                "`repeat ... until` with a constant `true` condition (the body \
                                 run once; whether the original keeps a loop is not seen, \
                                 codegen-patterns 7.2) is",
                            )),
                            None => Some(cond_of(p, cond)?),
                        };
                    out.push(Stmt {
                        kind: StmtKind::Repeat {
                            body: self.seq(body)?,
                            until,
                        },
                        span: s.span,
                    })
                }
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
                    if cond.value.is_some() {
                        // `while true do`: the image's loops with no test
                        // do not tell it from `repeat ... until false`
                        // (codegen-patterns 7.2: whether the opening `bra`
                        // to the test stays is not seen); `while false
                        // do`: the body is dead code, eliminated at `-opt
                        // 1` and above (ref 6.4.24, p. 6-27), which leaves
                        // no trace in the image.
                        return Err(unsupported(
                            cond.span,
                            "a `while` loop with a constant condition (`while true` cannot be told \
                             from `repeat ... until false` in the image, `while false` leaves \
                             nothing to see; codegen-patterns 7.2) is",
                        ));
                    }
                    let body = self.seq(std::slice::from_ref(body))?;
                    if body.is_empty() {
                        // No such loop has been looked for in the image.
                        best_effort(s.span, "a `while` loop with an empty body is")?;
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
                SStmtKind::With { items, body } => self.with_stmt(items, body, &mut out)?,
                _ => return Err(unsupported(s.span, "this statement is")),
            }
        }
        Ok(out)
    }

    /// `with r1, ..., rN do body` (ref "With", pp. 4-211..4-214; the
    /// alias form `with r : id`, p. 4-212, C.3.18, binds the same way):
    /// "Domain Pascal evaluates the expression v1 only once, and this
    /// evaluated expression is implied within the body" (p. 4-212); "changing
    /// p does not affect access to the record identified by the with
    /// statement".  Each record is bound, in order, to the address the
    /// body's references to it stand for (codegen-patterns 5.2, "`with`"):
    ///
    /// - a record whose address has no run-time part (a variable, a field
    ///   or a constant-indexed element of one, a by-address parameter's
    ///   object through its slot): that address, no code;
    /// - a record through a pointer variable of this routine (a local or a
    ///   by-value parameter) that the body never writes and whose address
    ///   is never taken or passed in the routine (nor reached by a nested
    ///   routine): the references go through the variable as written,
    ///   which then holds the same pointer for the whole body;
    /// - any other: the run-time part of the address, the pointer or the
    ///   element's address without its constant displacement, computed
    ///   once into a temporary held in an address register for the body
    ///   ([`Function::with_temps`]), the constant displacements folded into
    ///   each reference (NETLOG_$LOG_IT 0xE71BC6..0xE71BFC: `movea.l
    ///   (116,A5),A0; lea (0,A0,D1.w),A0`, then eleven field stores
    ///   `(-26,A0)` .. `(-2,A0)`, the 1-based array's `-26` in each; the
    ///   array's own offset in its record or variable too, SAU5
    ///   MT_PURGE_UCB 0xE75786 `lea (0x0,A0,D0.w),A3` with the units'
    ///   +0x3C in `(0x44,A3)`).
    /// `d := f(args)` for a function `f` whose result goes through a
    /// hidden address ([`ResultReg::Hidden`]; codegen-patterns 3, "Record
    /// results over 4 bytes"): the call with a frame temporary's address
    /// as its first argument, pushed last, then a record copy from the
    /// temporary into `d`.  Every compiled call of such a function stores
    /// into a temporary, never into the destination itself (17 calls
    /// whose result is assigned: PROC2_$UPGID_TO_UID 0xE41030..0xE41046
    /// `move.w (-0xa,A6),-(SP); pea (-0x14,A6); bsr.w 0xE4232A; addq.w
    /// #6,SP; lea (-0x14,A6),A0; move.l (A0)+,(-0x8,A6); move.l
    /// (A0)+,(-0x4,A6)`; TIME_$SET_ITIMER 0xE58ED2..0xE58EE4 `pea (A3); pea
    /// (-0x10,A6); bsr.w 0xE58C02; addq.w #8,SP; move.l (-0x10,A6),(A3);
    /// move.w (-0xc,A6),(0x4,A3)`, the destination its own argument;
    /// ASKNODE_$INTERNET_INFO 0xE648AC..0xE648C4 into a field through a
    /// parameter's pointer).  The temporary is placed as an expression's
    /// passed by address ([`assign_temps`]).  The pop follows the
    /// procedure-call rule: skipped when only the copy and the epilogue
    /// follow (DIR_$OLD_GET_DEFAULT_ACL 0xE56560, TIME_$SET_ITIMER
    /// 0xE58EEE, TIME_$GET_ITIMER 0xE58F4C).
    ///
    /// A destination whose address needs a call (the order of the two
    /// calls is not seen) is refused.
    fn hidden_call_assignment(&self, s: &SStmt, target: &SExpr, value: &SExpr) -> Res<Vec<Stmt>> {
        let p = self.p;
        let c = hidden_call(p, value).expect("a call of a hidden-result function");
        let SCallee::Routine(id) = &c.callee else {
            unreachable!("hidden_call accepts routines only")
        };
        let bytes = p
            .types
            .size(p.routine(*id).result.as_ref().expect("a function").ty);
        if p.types.size(target.ty) != bytes {
            return Err(unsupported(
                value.span,
                "assigning a record or array of another size is",
            ));
        }
        if volatile_access(p, target).is_some() || p.types.has_volatile_part(target.ty) {
            // The temporary's copy into the target would be [`copy`]'s,
            // but its selection marks no volatile references here, and no
            // such assignment is seen.
            return Err(unsupported(
                s.span,
                "assigning a function's record result to a record or array with volatile or \
                 device parts (not seen; ref 3.15.1-3.15.3) is",
            ));
        }
        check_even(p, target)?;
        let call = call(p, c, value.span)?;
        let Some(Arg::Address { addr: tmp, .. }) = call.args.first().cloned() else {
            unreachable!("`call` passes a hidden-address result's temporary first")
        };
        let copy = Stmt {
            kind: StmtKind::Copy {
                dst: addr_of(p, target)?,
                src: tmp,
                bytes,
                chars: false,
            },
            span: s.span,
        };
        let mut inner = Vec::new();
        stmt_calls(&copy, &mut inner);
        if !inner.is_empty() {
            return Err(unsupported(
                target.span,
                "a record function result assigned to a designator whose address needs a call \
                 (the order of the calls is not seen) is",
            ));
        }
        Ok(vec![
            Stmt {
                kind: StmtKind::Call(call),
                span: s.span,
            },
            copy,
        ])
    }

    fn with_stmt(
        &self,
        items: &[(dpc_sema::WithId, SExpr)],
        body: &SStmt,
        out: &mut Vec<Stmt>,
    ) -> Res<()> {
        let p = self.p;
        let _scope = WithScope(WITHS.with(|ws| ws.borrow().len()));
        for (wid, rec) in items {
            let a = addr_of(p, rec)?;
            let bound = if self.with_static(&a) || self.with_stable(rec, body) {
                a
            } else {
                if volatile_access(p, rec).is_some() {
                    best_effort(
                        rec.span,
                        "a `with` statement over a volatile record reached at run time is",
                    )?;
                }
                let (value, offset) = match a {
                    // A pointer's temporary: no frame slot (HINT_$ADD_NET
                    // 0xE49C76 `link #0`; Uncertain, bead
                    // domain-pascal-njy0).
                    Addr::Indirect { base, offset } => (*base, offset),
                    // An element's: a frame slot ([`with_slots`]).
                    Addr::Index {
                        base,
                        index,
                        scale,
                        offset,
                    } => {
                        self.with_elem_temps.set(self.with_elem_temps.get() + 1);
                        // The array's own place in its variable (a field's
                        // offset, the variable's in its data area or frame)
                        // is a constant displacement too: the base's
                        // register alone, `(0,An,Dx)`, every field's
                        // displacement carrying it (SAU5 MT_PURGE_UCB
                        // 0xE75770..0xE757CA, `with mt_$scb.units[u]` with
                        // `units` at +0x3C: `movea.l #MT_$SCB,A0; mulu.w
                        // (A2),D0; lea (0x0,A0,D0.w),A3; bset.b
                        // #0x2,(0x44,A3)`, `pea (0x3c,A3)`, `tst.l
                        // (0x3c,A3)`; the same at 0xE75870, 0xE75B36,
                        // 0xE75C22, 0xE762BC, 0xE767C4, 0xE769A0; bead
                        // domain-pascal-3m13).  Kept in the base when the
                        // folded displacements would not fit 16 bits, and
                        // for an element at an enclosing `for` loop's
                        // index: the selector matches such a temporary to
                        // the loop's stepped array by its base and moves
                        // the array's offset into the references itself
                        // (`dpc-m68k` `with_rebase`, DBUF_$INVALIDATE
                        // 0xE3AA12).
                        let rec_bytes = i64::from(p.types.size(rec.ty));
                        let at_for_index = matches!(&*index,
                            Expr::Load { addr: Addr::Frame(o), .. }
                            if self.for_indices.borrow().contains(o));
                        // An array of an overlay section that the selector
                        // reaches through a biased base or refuses at its
                        // offset ([`Selector::symbol_bias`]: 60000 bytes in
                        // for 0x7C00 and beyond of a section of more than
                        // 0x8000 bytes; between, or 0x7C00 and beyond of a
                        // smaller one, refused): the offset kept in the
                        // base, so that the bias and the refusals apply.
                        let (b, d) = base_displacement(*base);
                        let biased = || {
                            let mut x = rec;
                            loop {
                                match &x.kind {
                                    ExprKind::Field { base, .. } | ExprKind::Transfer(base) => {
                                        x = base
                                    }
                                    _ => break,
                                }
                            }
                            let ExprKind::Index { base: arr, .. } = &x.kind else {
                                return true;
                            };
                            match overlay_var(p, &**arr) {
                                Some((_, size)) if size > 0x8000 => d != 0,
                                Some(_) => d >= 0x7C00,
                                None => false,
                            }
                        };
                        let total = i64::from(d) + i64::from(offset);
                        let fits = i16::try_from(total).is_ok()
                            && i16::try_from(total + rec_bytes).is_ok();
                        let fold = d != 0
                            && !at_for_index
                            && !(matches!(b, Addr::Symbol { .. }) && biased());
                        // An element of a pointer's array indexed from far
                        // beyond 16 bits (the SAU9 MMAP through a pointer,
                        // `array[16#10000..16#1FFFF]` of 16-byte entries):
                        // the element's offset is part of the temporary's
                        // value, not of the fields' displacements
                        // (OS_$COLD_START 0x40034B6 `lea (0,A0,D1.l),A1;
                        // movea.l A1,A3; adda.l #-0x100000,A3; bset
                        // #6,(A3)`, the selector's `lea_into` form; bead
                        // domain-pascal-4bjx).  A symbol's array is
                        // biased instead (`dpc-m68k` `with_bias`).
                        let far_pointer =
                            matches!(b, Addr::Indirect { .. }) && i16::try_from(offset).is_err();
                        if fold && !fits && !far_pointer {
                            // Best effort (not byte-identical): no image site
                            // of a `with` element whose folded displacements
                            // pass 16 bits; the offset kept in the base.
                            best_effort(
                                rec.span,
                                "a `with` over an element of an array whose offset in its \
                                 variable, folded into the fields' displacements, would not fit \
                                 16 bits (the offset kept in the `lea`: no site seen, \
                                 codegen-patterns 5.2, \"`with`\") is",
                            )?;
                        }
                        let (base, elem_offset, offset) = if far_pointer {
                            (b.offset_by(d), offset, 0)
                        } else if fold && fits {
                            (b, 0, total as i32)
                        } else {
                            (b.offset_by(d), 0, offset)
                        };
                        (
                            Expr::AddrOf(Addr::Index {
                                base: Box::new(base),
                                index,
                                scale,
                                offset: elem_offset,
                            }),
                            offset,
                        )
                    }
                    _ => return Err(unsupported(rec.span, "this `with` record is")),
                };
                let mut temps = self.with_temps.borrow_mut();
                let tmp = WITH_TEMP_BASE - 8 * temps.len() as i32;
                temps.push(tmp);
                out.push(Stmt {
                    kind: StmtKind::Store {
                        addr: Addr::Frame(tmp),
                        value,
                    },
                    span: rec.span,
                });
                Addr::Indirect {
                    base: Box::new(Expr::Load {
                        addr: Addr::Frame(tmp),
                        ty: Scalar::POINTER,
                    }),
                    offset,
                }
            };
            WITHS.with(|ws| ws.borrow_mut().push((wid.0, rec.clone(), bound)));
        }
        out.extend(self.seq(std::slice::from_ref(body))?);
        Ok(())
    }

    /// Whether address `a` has no run-time part but a by-address
    /// parameter's slot or an enclosing `with` statement's temporary (`with
    /// p^ : x do with x.sub do`) ([`Self::with_stmt`]).
    fn with_static(&self, a: &Addr) -> bool {
        match a {
            Addr::Frame(_)
            | Addr::Static(_)
            | Addr::OtherStatic { .. }
            | Addr::Symbol { .. }
            | Addr::Enclosing(_) => true,
            Addr::Indirect { base, .. } => {
                matches!(&**base, Expr::Load { addr: Addr::Frame(o), .. }
                if *o <= WITH_TEMP_BASE
                    || self.r.params.iter().any(|prm| prm.passing() == Passing::ByAddress
                        && prm.copy_offset.is_none() && prm.slot_offset == *o))
            }
            _ => false,
        }
    }

    /// Whether `rec` is `v^` (or a field of it) for a pointer variable `v`
    /// of this routine's frame that `body` never writes, whose address the
    /// routine never takes or passes, and which no nested routine reaches
    /// ([`Self::with_stmt`]).
    fn with_stable(&self, rec: &SExpr, body: &SStmt) -> bool {
        let mut x = rec;
        while let ExprKind::Field { base, .. } = &x.kind {
            x = base;
        }
        let ExprKind::Deref(ptr) = &x.kind else {
            return false;
        };
        let ExprKind::Var { var, uplevel: 0 } = ptr.kind else {
            return false;
        };
        let Ok(Addr::Frame(o)) = var_addr(self.p, var, ptr.span) else {
            return false;
        };
        let block = self.r.body.as_ref().expect("routine with a body");
        !self
            .uplevel
            .iter()
            .any(|&(u, n)| u < o + 4 && o < u + n as i32)
            && !var_touched(std::slice::from_ref(body), var, true)
            && !var_touched(&block.stmts, var, false)
    }

    /// A `for` statement (ref "For", pp. 4-88..4-90): the
    /// [`StmtKind::Fill`] shape when it is one (`fill`), else a
    /// [`StmtKind::For`].
    ///
    /// The index variable must be a byte, word or longword local of this
    /// routine, other than the function result variable: the selector keeps it in
    /// a register for the loop and never stores it (codegen-patterns 7.4:
    /// 307 of 320 `dbf` loops with an index step it in a register; the
    /// other 13, `addq.w #1,(d,A6)` or through a static link, are not
    /// reproduced).  Ref p. 4-89 lets the index be declared in another
    /// scope; such an index, and a local a nested routine reaches, is kept
    /// in memory ([`ForLoop::home`]; 0xE721F8, through the parent's
    /// frame).  Whether the index is read outside its loops, or
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
        let vty = var.ty;
        let result = self.r.result.as_ref().and_then(|res| res.var);
        // A local of this routine that no nested routine reaches: held in
        // a register.  Any other index (ref p. 4-89 lets it be declared in
        // another scope) is kept in memory, `ForLoop::home` (RINGLOG_$CNTL
        // 0xE72296..0xE722B0 for its own frame's index, which its nested
        // 0xE721CC reaches and loops over through the static link).
        let local = match var.kind {
            ExprKind::Var {
                var: index,
                uplevel: 0,
            } => match p.var(index).storage {
                Storage::Frame { offset }
                    if self.r.frame_vars.contains(&index)
                        && Some(index) != result
                        && !overlaps(&self.uplevel, offset, p.types.size(vty)) =>
                {
                    Some(offset)
                }
                _ => None,
            },
            _ => None,
        };
        let (offset, home) = match local {
            Some(o) => (o, None),
            None => match addr_of(p, var)? {
                Addr::Volatile { .. } => {
                    return Err(unsupported(var.span, "a volatile `for` index variable is"))
                }
                a => (0, Some(a)),
            },
        };
        // A byte (`char`, a `[byte]` subrange) or longword
        // (`integer32`, `linteger`) index: the target's own loop form
        // (bead domain-pascal-1mtr; codegen-patterns 7.4, "Byte and
        // longword indices").
        let Some(width) = Width::of_size(p.types.size(vty)) else {
            return Err(unsupported(
                var.span,
                "a `for` index variable that is not a byte, a word or a longword is",
            ));
        };
        if p.types.is_boolean(vty) {
            // `true` is stored 0xFF, not `false` + 1 (ref 3.5.3, p. 3-11):
            // stepping the index is not an increment.  No site.
            return Err(unsupported(var.span, "a boolean `for` index variable is"));
        }
        let ty = scalar_of_type(&p.types, vty, width);
        // The constant lower bound of a loop counting up, for the
        // selector's fold of `to = x - c` below.
        let up_from = match (&from.value, dir) {
            (Some(c), ForDir::To) => c.ordinal(),
            _ => None,
        };
        let is_to = |e: &SExpr| std::ptr::eq(e, to);
        let bound = |e: &SExpr| -> Res<Expr> {
            match &e.value {
                Some(c) => const_of(c, ty, e.span),
                // An index other than a signed word held in a register: an
                // integer bound wider than the index is kept at its own
                // width, the target computing the guard and the count there
                // so that a bound outside the index's type gives an empty
                // range rather than a wrapped one (ref p. 4-90; ISO 7185
                // 6.8.3.9; reference-clarifications question 47).
                None if (width != Width::Word || home.is_some() || !ty.is_signed())
                    && p.types.is_integer(e.ty)
                    && p.types.size(e.ty) > ty.bytes() =>
                {
                    value_of(p, e)
                }
                // A byte or longword index: the bound converted as an
                // assignment converts it (`assign_convert`, `store_narrow`).
                None if width != Width::Word => {
                    let v = value_of(p, e)?;
                    let k = v.ty().bytes();
                    if let Some(x) = assign_convert(p, e, v.clone(), ty) {
                        Ok(x)
                    } else if let Some(x) = store_narrow(p, e, v, ty) {
                        x.map_err(|m| unsupported(e.span, m))
                    } else {
                        Err(unsupported(
                            e.span,
                            format!(
                                "a `for` bound of {k} bytes for an index of {} bytes is",
                                ty.bytes()
                            ),
                        ))
                    }
                }
                // The bound is converted to the index's type as in an
                // assignment (`store`).
                None => {
                    let v = value_of(p, e)?;
                    // A signed word index (held in a register) to a bound
                    // computed from unsigned words (`n - 1`, `a + 1` of a
                    // `pinteger`, an `integer32` computation of words):
                    // kept at its own width, as for the other index kinds
                    // above, so that the guard and the count are computed
                    // in a type holding its value (`Selector::for_wide`,
                    // `for_domain`) and `for i := 32767 to n - 1` or `for
                    // i := n - 1 downto 0` run their passes, `a + 1 to b`
                    // with a = 65535 none (bead domain-pascal-3iws).
                    // Except `to = x - c` of an unsigned word variable from
                    // a constant the selector folds `c` into exactly
                    // (`subq.w #k; bcs`, `Selector::for_`).  Best effort:
                    // no site looked at.
                    if ty.is_signed()
                        && home.is_none()
                        && v.ty().width == Width::Long
                        && unsigned_word_arith(&v)
                        && !(is_to(e) && folds_exactly(&v, up_from))
                    {
                        best_effort(
                            e.span,
                            "a `for` bound computed from unsigned words for a signed word index \
                             (computed at its own width, the guard and the count there: no site \
                             looked at, codegen-patterns 7.4) is",
                        )?;
                        return Ok(v);
                    }
                    if v.ty().width == Width::Word {
                        Ok(v)
                    } else if v.ty().bytes() < 2 && p.types.is_integer(e.ty) {
                        Ok(convert(v, ty))
                    } else if let Some(n) = narrow(&v, ty) {
                        Ok(n)
                    } else if matches!(&v, Expr::Load { addr, .. }
                        if !matches!(addr, Addr::Volatile { .. }))
                        && v.ty().bytes() == 4
                        && p.types.is_integer(e.ty)
                    {
                        // A longword variable read as its low word, in
                        // place, as an assignment reads it ([`store_narrow`];
                        // codegen-patterns 7.4, "A longword bound"): 10 of
                        // 10 sites, `move.w (0x155a,A5),D0; subq.w #1,D0;
                        // bmi` for DIR_$MTTAB's count at 0x1558 (DIR_$
                        // FIND_UID 0xE4E46A and seven more DIR_ routines),
                        // EC2_$REGISTER_EC1 0xE42960 `move.w (0xe9a,A5),D0`,
                        // VTOC_$TRUNCATE 0xE3A12C `move.w (-0x32,A6),...`
                        // (a longword at -0x34).
                        Ok(Expr::Convert {
                            value: Box::new(v),
                            ty,
                            in_place: true,
                        })
                    } else {
                        // Best effort (not byte-identical; bead
                        // domain-pascal-trdz): a longword expression or
                        // function result is kept at its own width, as for
                        // the other index kinds above, the target computing
                        // the guard and the count there (`Selector::
                        // for_wide`, `for_domain`): a bound outside the
                        // index's type gives an empty range rather than a
                        // wrapped one (ref p. 4-90; reference-clarifications
                        // question 47).  MST_$INIT 0xE30BF8..0xE30C00
                        // `move.w D2w,D0w; subq.w #1,D0w` computes `words -
                        // 1`, a longword expression, at the word's width
                        // (and has no guard): not followed.
                        let why = "a `for` bound of another width than the index, other than a \
                                   longword variable read as its low word (a longword \
                                   expression or function result narrowed: no site seen, \
                                   codegen-patterns 7.4) is";
                        if !p.types.is_integer(e.ty) || v.ty().pointer {
                            return Err(unsupported(e.span, why));
                        }
                        best_effort(e.span, why)?;
                        Ok(v)
                    }
                }
            }
        };
        let from = bound(from)?;
        let to = bound(to)?;
        self.for_indices.borrow_mut().push(offset);
        let body = self.seq(std::slice::from_ref(body));
        self.for_indices.borrow_mut().pop();
        let body = body?;
        if body.is_empty() {
            best_effort(s.span, "a `for` loop with an empty body is")?;
        }
        Ok(Stmt {
            kind: StmtKind::For(ForLoop {
                index: ValueParam { offset, ty },
                home,
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
        if hidden_call(self.p, x).is_some() {
            // The call with a temporary of its own would do; no site.
            return Err(unsupported(x.span, HIDDEN_CALL_USE));
        }
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
        // A constant selector is dispatched at run time like any other
        // (SMD_$INIT 0xE34D50 `clr.w D0; cmpi.w #4,D0; bcc`, a jump table
        // on the constant 0).
        let mut selector = value_of(p, expr)?;
        let ty = selector.ty();
        match (ty.width, ty.sign) {
            (Width::Byte, Sign::Unsigned) => {
                selector = Expr::Convert {
                    value: Box::new(selector),
                    ty: Scalar::unsigned(Width::Word),
                    in_place: false,
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
                    // the last arm ended, is not seen.  Best effort (not
                    // byte-identical): it does nothing, so it is the `case`
                    // without `otherwise`, an unmatched selector on to the
                    // end (ref "Case", pp. 4-36..4-37).
                    best_effort(s.span, "an empty `otherwise` part is")?;
                    None
                } else {
                    Some(o)
                }
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
                // Both operands evaluated and combined (ref "And", p. 4-19:
                // the system "may evaluate both"; "Or", p. 4-135: it "may
                // have to evaluate both"; the image does, codegen-patterns
                // 7.1, "Plain `and` and `or`").
                BinOp::And | BinOp::Or if p.types.is_boolean(e.ty) => {
                    let (a, b) = (Box::new(cond_of(p, lhs)?), Box::new(cond_of(p, rhs)?));
                    return Ok(if *op == BinOp::And {
                        Cond::And(a, b)
                    } else {
                        Cond::Or(a, b)
                    });
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
        _ => match bool_value(p, e)? {
            // A condition the value is made of (`odd(x)`).
            Expr::Test(c) => Ok(*c),
            v => Ok(Cond::Test(v)),
        },
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
    if matches!(p.types.kind(lhs.ty), TypeKind::Array { .. })
        || matches!(p.types.kind(rhs.ty), TypeKind::Array { .. })
    {
        return array_compare(p, e, rel, lhs, rhs);
    }
    let comparable = |x: &SExpr| p.types.is_ordinal(x.ty) || p.types.is_pointer(x.ty);
    if !comparable(lhs) || !comparable(rhs) {
        return Err(unsupported(
            e.span,
            "comparing values other than ordinals and pointers is",
        ));
    }
    // `b = true`, `b1 <> b2`: compiled code tests booleans with `tst.b`
    // and the sign (codegen-patterns 7); a compare of a byte with 0xFF
    // occurs twice (0xE1B96C, 0xE53FDE, not shown to be booleans).  Two
    // booleans are compared as bytes: SMD_$SHOW_CURSOR 0xE6E32A..0xE6E32E
    // `move.b (-0x1c,A6),D1; cmp.b (0x38,A4),D1; bne` (`visible =
    // cur_hw^.cursor_visible`), SIO2681_$SET_LINE 0xE1D302..0xE1D30E `sne
    // D1; ...; smi D5; cmp.b D5,D1; seq D1` (two comparisons compared).
    // Best effort (not byte-identical; bead domain-pascal-uaq7): as bytes
    // everywhere, the constant `true` as 0xFF (ref 3.5.3).  A boolean is a
    // signed byte here (0xFF is -1), so an ordering (ref: `false < true`,
    // the order of the type's two values) is made by the reversed signed
    // relation.
    let flip = p.types.is_boolean(lhs.ty) || p.types.is_boolean(rhs.ty);
    if flip {
        best_effort(e.span, "comparing boolean values is")?;
    }
    let rel = if flip {
        match rel {
            Rel::Lt => Rel::Gt,
            Rel::Le => Rel::Ge,
            Rel::Gt => Rel::Lt,
            Rel::Ge => Rel::Le,
            r => r,
        }
    } else {
        rel
    };
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
                    let what = "comparing with a constant outside the other operand's type is";
                    // A longword against a constant of 32 bits that its
                    // type does not hold: the constant's 32 bits, compared
                    // with the operand's sign, as the image does
                    // (FILE_$SET_REFCNT 0xE5E410 `cmpi.l #-0x1,D0; bne.b`,
                    // a `0..2147483647` against -1; AST_$UPDATE_ASTE's
                    // nested routine at 0xE01518 `cmpi.l #-0x80000000,
                    // (-0x88,A0); bcs.b`, the same type `>= 16#80000000`,
                    // unsigned).
                    // Any other ordered relation of an unsigned longword
                    // and a negative constant is signed (ref 4.3.3, pp.
                    // 4-6..4-7, clarification 32: unsigned only against a
                    // constant in 0..2147483647): the constant as an
                    // `integer32`, both operands then read signed
                    // ([`widen_operands`]), the unsigned values being the
                    // same signed.  Best effort (not byte-identical): no
                    // site.
                    let ordered = !matches!(rel, Rel::Eq | Rel::Ne);
                    if ty.width == Width::Long
                        && ty.sign == Sign::Unsigned
                        && ordered
                        && value < 0
                        && value != -(1i64 << 31)
                    {
                        best_effort(x.span, what)?;
                        return Ok(Expr::Const {
                            value,
                            ty: Scalar::signed(Width::Long),
                        });
                    }
                    if ty.width == Width::Long && (-(1i64 << 31)..(1i64 << 32)).contains(&value) {
                        let bits = value & 0xffff_ffff;
                        let value = match ty.sign {
                            Sign::Unsigned => bits,
                            Sign::Signed => bits as u32 as i32 as i64,
                        };
                        return Ok(Expr::Const { value, ty });
                    }
                    // A word against a constant beyond it: both compared
                    // as longwords (ref 4.3.1, p. 4-5: the smaller operand
                    // is expanded), so the result is the one the values
                    // give.  Best effort (not byte-identical): the
                    // reference's compiler folds such a test to its
                    // constant result and emits no code for it (p. 9-32,
                    // message 250, `x: 0..65535; IF x = -1`); no site.
                    let long = Scalar::signed(Width::Long);
                    if ty.width == Width::Word && !ty.pointer && fits(value, long) {
                        best_effort(x.span, what)?;
                        return Ok(Expr::Const { value, ty: long });
                    }
                    return Err(unsupported(x.span, what));
                }
                Ok(c)
            }
            (Some(_), None) => Err(unsupported(e.span, "comparing two constants is")),
            // A one-byte integer (`[byte] 0..255`, ref 3.15.5) is compared
            // as a word, an `integer` (ref 4.3.1, p. 4-5: integer
            // operands smaller than the operation are expanded), zero- or
            // sign-extended by its own sign: TPAD_$DATA 0xE691D8..0xE691E0
            // `clr.w D0; move.b (10,A1),D0; cmpi.w #0xdf,D0` (an
            // unsigned byte against a constant; `../source`'s Pascal type).
            // **Uncertain:** a byte compared with another byte or a
            // variable is not catalogued; the same widening is applied.
            (None, _) => {
                let v = value_of(p, x)?;
                if v.ty().width == Width::Byte && p.types.is_integer(x.ty) {
                    Ok(convert(v, Scalar::signed(Width::Word)))
                } else {
                    Ok(v)
                }
            }
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
    // A word sum, difference, product or negation compared: both operands
    // computed in 32 bits, the word operation's own operands widened first
    // ([`convert`]), so that the comparison sees the value the operation
    // has, not its low word.  Ref "Type Transfer Functions", p. 4-198: for
    // `x + y > 5` on integer16 x, y "the compiler expands operands x and y
    // to 32-bit integers and performs 32-bit addition before making the
    // comparison", and `INTEGER16(x + y) > 5` (a transfer, not matched)
    // is the 16-bit form; ref 4.3.1,
    // p. 4-5.  Owner's ruling 2026-11-03 (reference-clarifications 31):
    // follow the reference.  The image has both forms: widened at
    // TPAD_$DATA 0xE69448..0xE6945A `move.w (0x170,A5),D5; ext.l D5;
    // addi.l #50,D5; move.w (-0x1e,A2),D2; ext.l D2; cmp.l D2,D5`,
    // NETWORK_$SET_SERVICE 0xE0F9D0..0xE0F9D8, SAU6 dtty_$gpu_command
    // 0x7801F6E4..0x7801F6F2 (SAU5 0xE1DFF8; ../source finding 921); at
    // the word's width at SHOW_CURSOR 0xE6E2A2 `add.w (-12,A6),D2; cmp.w
    // (80,A4),D2`, SMD_$INVERT_DISP 0xE70576 and others (bead
    // domain-pascal-9js), which a translation writes with the transfer.
    let overflows = |x: &Expr, s: &SExpr| match x {
        _ if matches!(s.kind, ExprKind::Transfer(_)) => false,
        Expr::Binary {
            op: IrBinOp::Add | IrBinOp::Sub | IrBinOp::Mul,
            ty,
            ..
        }
        | Expr::Unary {
            op: IrUnOp::Neg,
            ty,
            ..
        } => ty.width == Width::Word && !ty.pointer,
        _ => false,
    };
    let word_integer = |x: &Expr| x.ty().width == Width::Word && !x.ty().pointer;
    let (l, r) = if (overflows(&l, lhs) || overflows(&r, rhs)) && word_integer(&l) && word_integer(&r) {
        let wide = |x: Expr| {
            let t = Scalar::integer(Width::Long, x.ty().sign);
            convert(x, t)
        };
        (wide(l), wide(r))
    } else {
        (l, r)
    };
    let (l, r) = widen_operands(e, l, r)?;
    // `abs(x) > 0` (`<=`, and with the 0 on the left): unsigned, so the
    // same as `abs(x) <> 0` (`=`), which is how it is compared; best
    // effort (not checked against the image: no such site; an unsigned
    // ordering against 0 is refused by the selector).  `abs(x) >= 0` and
    // `abs(x) < 0` stay refused (constant conditions).
    let unsigned_abs = |x: &Expr| {
        let mut x = x;
        while let Expr::Convert { value, .. } = x {
            x = value;
        }
        matches!(
            x,
            Expr::Unary {
                op: IrUnOp::Abs,
                ..
            }
        ) && x.ty().sign == Sign::Unsigned
    };
    let zero = |x: &Expr| matches!(x, Expr::Const { value: 0, .. });
    let rel = match rel {
        Rel::Gt if zero(&r) && unsigned_abs(&l) && l.ty().sign == Sign::Unsigned => Rel::Ne,
        Rel::Le if zero(&r) && unsigned_abs(&l) && l.ty().sign == Sign::Unsigned => Rel::Eq,
        Rel::Lt if zero(&l) && unsigned_abs(&r) && r.ty().sign == Sign::Unsigned => Rel::Ne,
        Rel::Ge if zero(&l) && unsigned_abs(&r) && r.ty().sign == Sign::Unsigned => Rel::Eq,
        rel => rel,
    };
    // An ordering against 0 of an unsigned operand whose type's values all
    // fit the signed range of its width (`0..32767`, `linteger`
    // `0..2147483647`): a sign test, as the image makes it (BAT_$FREE
    // 0xE3B5B0..0xE3B5B6 `sub.l; bmi.s`, `rel < 0` of a linteger; bead
    // domain-pascal-uaq7).  The operand is seen as signed:
    // a load retyped, another value through a conversion of its width.
    //
    // A word type that uses the top bit (`0..65535`, pinteger) is tested
    // so too: the original compiler branches on the load's or `tst.w`'s
    // flags with the signed condition, so that 16#8000..16#FFFF count as
    // below 0 (bead domain-pascal-wkm5; the operand is unsigned by its
    // `cmpi.w #c; bhi/bls` bound and its zero-extension, `clr.l; move.w`
    // or `andi.l #0xffff`): `x <= 0` `ble` (MT_PURGE_UCB's module-local
    // neighbor, SAU5 0xE75916; MT_$ACQUIRE, SAU5 0xE760B6; SOCK_$PUT_INT,
    // SAU2 0xE161AA, SAU5 0xE167F6), `x > 0` `ble` around the part
    // (PKT_$BLD_INTERNET_HDR 0xE122D6), `x < 0` `bmi` (WIN_$DINIT SAU5
    // 0xE19EDE, PBU_$GET_DCTE SAU5 0xE59212).  No `tst`/load of a word is
    // followed by `bhi`/`bls`/`bcs`/`bcc` in either image's compiled code.
    // One clamp has the `=` form instead (audit_$log_cname_op 0xE4BEFE
    // `tst.w D2; bne`, read `<= 0` by ../source).  Bytes that use the top
    // bit (`0..255`) keep the unsigned relation (no site), which the
    // selector makes `=`/`<>` or a never/always test.
    let (l, r) = if matches!(rel, Rel::Lt | Rel::Le | Rel::Gt | Rel::Ge) {
        let fits = |x: &Expr, s: &SExpr| {
            let t = x.ty();
            let max = match t.width {
                Width::Byte => 0x7f,
                Width::Word => 0xffff,
                Width::Long => 0x7fff_ffff,
            };
            t.sign == Sign::Unsigned
                && matches!(p.types.ordinal_range(s.ty), Some((lo, hi)) if lo >= 0 && hi <= max)
        };
        let signed = |x: Expr| -> Expr {
            let t = Scalar::signed(x.ty().width);
            match x {
                Expr::Load { addr, .. } => Expr::Load { addr, ty: t },
                x => Expr::Convert {
                    value: Box::new(x),
                    ty: t,
                    in_place: false,
                },
            }
        };
        let zero_of = |x: &Expr| Expr::Const {
            value: 0,
            ty: Scalar::signed(x.ty().width),
        };
        match (zero(&l), zero(&r)) {
            (false, true) if fits(&l, lhs) => (signed(l), zero_of(&r)),
            (true, false) if fits(&r, rhs) => (zero_of(&l), signed(r)),
            _ => (l, r),
        }
    } else {
        (l, r)
    };
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
/// - A signed longword and an unsigned one (`integer32` against
///   `0..2147483647`), or a signed word against an unsigned longword:
///   compared signed (ref 4.3.3, pp. 4-6..4-7: `<`, `<=`, `>`, `>=` are
///   "signed except" when both operands are unsigned run-time values or
///   one is and the other a constant in 0..2147483647), the word widened
///   by its own sign (`ext.l`, as above), the unsigned longword read as
///   it is (its values, 0..2^31-1, are the same signed).  **Uncertain:**
///   no image site of these mixes is identified (the image does not
///   show which longwords are `integer32`); the instructions are those
///   of the forms above.
///
/// Anything else that mixes widths or signs (a byte against a word) is
/// not supported yet.
fn widen_operands(e: &SExpr, l: Expr, r: Expr) -> Res<(Expr, Expr)> {
    let (lt, rt) = (l.ty(), r.ty());
    if lt == rt {
        return Ok((l, r));
    }
    let long = Scalar::signed(Width::Long);
    // `abs` of a word (`0..32767`, unsigned) against a signed word: a word
    // compare, signed (TPAD_$DATA 0xE69250 `cmp.w (-0x12,A2),D5w; bge.b`
    // and 0xE692DA, `abs(delta_x) < hysteresis` with an `integer`
    // hysteresis; codegen-patterns 8, "abs"), not the widening of a
    // `0..65535` word below: the values fit the signed word.
    let abs16 = |x: &Expr| {
        matches!(x, Expr::Unary { op: IrUnOp::Abs, ty, .. }
            if ty.width == Width::Word && ty.sign == Sign::Unsigned && !ty.pointer)
    };
    let signed_word = |t: Scalar| t == Scalar::signed(Width::Word);
    if abs16(&l) && signed_word(rt) {
        return Ok((reinterpret(l, rt), r));
    }
    if abs16(&r) && signed_word(lt) {
        return Ok((l, reinterpret(r, lt)));
    }
    match (lt.width, rt.width) {
        (Width::Word, Width::Word) => return Ok((convert(l, long), convert(r, long))),
        (Width::Word, Width::Long) if rt.sign == Sign::Signed || lt.sign == rt.sign => {
            return Ok((convert(l, rt), r))
        }
        (Width::Long, Width::Word) if lt.sign == Sign::Signed || lt.sign == rt.sign => {
            return Ok((l, convert(r, lt)))
        }
        // Here the longword is unsigned and the word signed.
        (Width::Word, Width::Long) => return Ok((convert(l, long), reinterpret(r, long))),
        (Width::Long, Width::Word) => return Ok((reinterpret(l, long), convert(r, long))),
        (Width::Long, Width::Long) => return Ok((reinterpret(l, long), reinterpret(r, long))),
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
    // Compatible records of other sizes: an alignment attribute rounds a
    // record's size (ref 3.15.6.2, Figure 3-21 p. 3-67; Figure 3-26 p.
    // 3-71), and `[attrs] T` is `T` for compatibility.  Comparing `size`
    // bytes would read beyond the smaller one.
    if p.types.size(rhs.ty) != size {
        return Err(unsupported(
            e.span,
            "comparing records of different sizes (an alignment attribute changes a record's \
             size, ref 3.15.6.2) is",
        ));
    }
    // `cmpm` moves words or longwords through both addresses: neither may
    // be odd ([`check_even`]; ref 3.15.6.8 p. 3-76 for a parameter).
    check_even(p, lhs)?;
    check_even(p, rhs)?;
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

/// `lhs = rhs` / `lhs <> rhs` of two arrays, or of an array of `char` and
/// a string constant of its length (ref Table 4-1, p. 4-2: `=` and `<>`
/// apply to "All types"), compared in memory as records are
/// ([`record_compare`]; codegen-patterns 7.1, "Records"): DISK_IO
/// 0xE3D846 `lea (-0x78,A6),A0; lea (0x20,A3),A1; moveq #7,D1; cmpm.l
/// (A1)+,(A0)+; dbne` for `verify_info.l <> req^.header.l`, two arrays of
/// 8 longwords; the module-local DISK_ routine at 0xE6C266 `lea (2,A0),A1;
/// lea (0x44,PC),A3; moveq #5,D2; cmpm.b (A3)+,(A1)+; dbne` for `apollo <>
/// 'APOLLO'`, the constant's cell in the code section (codegen-patterns
/// 1.5), compared in bytes (bead domain-pascal-yqf6).
fn array_compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    let string = |x: &SExpr| {
        matches!(
            (&x.value, p.types.kind(x.ty)),
            (Some(ConstValue::Str(_)), TypeKind::StringLit { .. })
        )
    };
    if !string(lhs) && !string(rhs) {
        // Two arrays: in longwords from 32 bytes as DISK_IO's (0xE3D846);
        // otherwise in the record forms, whose unit (`cmpm.w` for 6 bytes,
        // `cmpm.l` for 8) the image's array compares are not shown to use
        // (an array of `char` against a constant is a `cmpm.b` loop,
        // 0xE6C278).
        let size = p.types.size(lhs.ty);
        let elem = match p.types.kind(lhs.ty) {
            TypeKind::Array { elem, .. } => Some(p.types.size(*elem)),
            _ => None,
        };
        if !(elem == Some(4) && size >= 32 && size % 4 == 0) {
            best_effort(
                e.span,
                "comparing arrays other than of longwords from 32 bytes (in the record forms; \
                 the unit of the image's array loops by element size is not surveyed, \
                 codegen-patterns 7.1) is",
            )?;
        }
        return record_compare(p, e, rel, lhs, rhs);
    }
    let (var, lit) = if string(rhs) { (lhs, rhs) } else { (rhs, lhs) };
    let size = p.types.size(var.ty);
    let TypeKind::Array { elem, .. } = p.types.kind(var.ty) else {
        return Err(unsupported(
            e.span,
            "comparing a string constant with a non-array is",
        ));
    };
    if !p.types.is_char(*elem) || var.value.is_some() || p.types.size(lit.ty) != size {
        return Err(unsupported(
            e.span,
            "comparing a string constant with other than an array of `char` of its length is",
        ));
    }
    if !matches!(rel, Rel::Eq | Rel::Ne) {
        return Err(unsupported(e.span, "ordering arrays is"));
    }
    check_even(p, var)?;
    let cell = Addr::Const(pool_const(
        p,
        lit.value.as_ref().unwrap(),
        lit.ty,
        lit.span,
    )?);
    let v = addr_of(p, var)?;
    let (lhs, rhs) = if string(rhs) { (v, cell) } else { (cell, v) };
    Ok(Cond::Block {
        op: rel,
        lhs,
        rhs,
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
                    Some(b) if !b.is_whole_object(n, p.unaligned_access) => (b.bit, b.width),
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

/// `s = t` and `s <> t` (ref "Set Operations", p. 4-176) for a
/// register-sized set: the two sets' values compared as integers of the
/// set's size (codegen-patterns 5.1, "Set comparisons"; bead
/// domain-pascal-2i6f).
///
/// - `s = []`, `s <> []`: the set's value against 0, `tst` of a set
///   variable (TTY_$K_GET 0xE1C5D4 `tst.w (10,A2); beq`), the flags of the
///   `and` for an intersection or a difference (TTY 0xE1BFFE `moveq
///   #7,D1; and.w (8,A2),D1; beq.s`; PROC2_$SIGPAUSE 0xE3FAC6 `move.l
///   (-108,A4),D1; not.l D1; and.l (-100,A4),D1; beq.s`).
/// - Against a non-empty constant: `cmpi #mask` on the left operand's
///   value: PROC1_$SET_PRIORITY 0xE152AC..0xE152B6 and PROC1_$INIT
///   0xE2F9A8..0xE2F9B2 `moveq #11,D0; and.w (84,An),D0; cmpi.w #8,D0;
///   bne.s` (`flags * [0, 1, 3] = [3]` in ../source's reading); OS_$INIT
///   0xE339EC..0xE33A10 `move.w (-34,A6),D0; andi.w #-0x7ffc,D0; ...;
///   cmpi.w #4,D0; seq D0` (`boot.flags * [2, 15] = [2]` as a value);
///   RING_$SENDP 0xE75C38 `cmpi.w #0x14,D7; bne.s` (a held set, `= [2,
///   4]`).
/// - Two run-time sets: `cmp Dr,Dl`, the left operand's value in Dl:
///   SIO_$K_SET_PARAM 0xE680F6..0xE68102 `moveq #1,D1; moveq #1,D0; and.l
///   (-24,A6),D1; and.l (A2),D0; cmp.l D0,D1; beq.s` (`copy.flags1 * [0]
///   <> params.flags1 * [0]`, 7 times in that routine; the image has the
///   left operand in D1, `dpc` in D0, as 0xE49C2E's integer `and.l; and.l;
///   cmp.l D1,D0`: the register depends on the context, not reproduced
///   there); two set variables, SIO_$K_SET_PARAM 0xE682DC `move.l
///   (8,A2),D1; cmp.l (-0x10,A6),D1; bne.s` (`params.break_mask =
///   copy.break_mask`; the register may differ by context).
/// - **Uncertain:** a set in memory against a constant, `cmpi #mask,<ea>`
///   with no load, has no set site; it is the integer comparison's form
///   (0xE4FE90 `cmpi.w #4,(18,A6)`, 0xE58624 `cmpi.l #0x230001,(A0)`,
///   0xE4BAE0 `cmpi.l #0x400,(16,A3)`), as `tst <ea>` is for `[]`
///   (TTY_$K_GET 0xE1C5D4).
///
/// In each the code is the integer comparison's (codegen-patterns 7.1),
/// and `dpc` lowers the set comparison as one.
///
/// Best effort (not byte-identical; bead domain-pascal-61fd, no site
/// known for any of these):
/// - subset `s <= t` and superset `s >= t` ([`set_inclusion`]);
/// - a constant left operand of `=`/`<>` (`[2] = s`): the operands
///   swapped, `s = [2]` (the relations are symmetric);
/// - a one-byte set against a set other than `[]`: both widened to a
///   word, as for `[]` (acl_$prim_create_internal 0xE45270).
///
/// A set of more than a longword: [`large_set_compare`].
fn set_compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    if p.types.size(lhs.ty) > 4 || p.types.size(rhs.ty) > 4 {
        return large_set_compare(p, e, rel, lhs, rhs);
    }
    if matches!(rel, Rel::Le | Rel::Ge) {
        return set_inclusion(p, rel, lhs, rhs);
    }
    if !matches!(rel, Rel::Eq | Rel::Ne) {
        return Err(unsupported(e.span, "this comparison of sets is"));
    }
    // Best effort (not byte-identical): `c = s` as `s = c`, no site.
    let (lhs, rhs) = if lhs.value.is_some() && rhs.value.is_none() {
        (rhs, lhs)
    } else {
        (lhs, rhs)
    };
    if lhs.value.is_some() {
        return Err(unsupported(e.span, "a comparison of two constant sets is"));
    }
    let empty = |x: &SExpr| matches!(&x.value, Some(ConstValue::Set(m)) if m.is_empty());
    // The operation's type: the left operand's, or a wider run-time right
    // operand's (a constructor's own type is sized by its high end, ref
    // 3.9.3 p. 3-16; [`set_value`]).
    let base = if rhs.value.is_none() && p.types.size(rhs.ty) > p.types.size(lhs.ty) {
        rhs
    } else {
        lhs
    };
    let ty = match scalar_of(p, base)? {
        // A one-byte set compared with `[]`: widened to a word and
        // tested there, acl_$prim_create_internal 0xE45270..0xE45278
        // `clr.w D1; move.b (27,A4),D1; tst.w D1; beq.s` (`prot.
        // world_rights <> []`, ../source's reading; codegen-patterns 5.1,
        // "One-byte sets"; the only site).
        // Best effort (not byte-identical): a one-byte set against a set
        // other than `[]` (no site) is widened the same way.
        t if t.width == Width::Byte => Scalar::unsigned(Width::Word),
        t => t,
    };
    let l = set_value(p, lhs, ty)?;
    let r = if empty(rhs) {
        Expr::Const { value: 0, ty }
    } else {
        set_value(p, rhs, ty)?
    };
    Ok(Cond::Compare {
        op: rel,
        lhs: l,
        rhs: r,
    })
}

/// Subset `s <= t` and superset `s >= t` of register-sized sets (ref "Set
/// Operations", p. 4-176: `s <= t` is true when every member of `s` is
/// one of `t`, `s >= t` when every member of `t` is one of `s`).
///
/// Best effort (not byte-identical; bead domain-pascal-61fd): no site is
/// known (the image cannot tell `s <= t` from `s - t = []`), so `s <= t`
/// is lowered as `s - t = []` and `s >= t` as `t - s = []`, the
/// difference's code (codegen-patterns 5.1, "Values") tested for `[]` on
/// the flags of its `and` (PROC2_$SIGPAUSE 0xE3FAC6 `move.l (-108,A4),D1;
/// not.l D1; and.l (-100,A4),D1; beq.s`).  A one-byte set is widened to a
/// word, as for `[]` (acl_$prim_create_internal 0xE45270).
fn set_inclusion(p: &Program, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    let (sub, sup) = if rel == Rel::Le {
        (lhs, rhs)
    } else {
        (rhs, lhs)
    };
    // The operation's type: a run-time operand's, the wider one's if both
    // are (constants take the operation's type, [`set_value`]).
    let base = if sub.value.is_some()
        || (sup.value.is_none() && p.types.size(sup.ty) > p.types.size(sub.ty))
    {
        sup
    } else {
        sub
    };
    let ty = match scalar_of(p, base)? {
        t if t.width == Width::Byte => Scalar::unsigned(Width::Word),
        t => t,
    };
    Ok(Cond::Compare {
        op: Rel::Eq,
        lhs: Expr::SetOp {
            op: SetOp::Diff,
            lhs: Box::new(set_value(p, sub, ty)?),
            rhs: Box::new(set_value(p, sup, ty)?),
            ty,
        },
        rhs: Expr::Const { value: 0, ty },
    })
}

/// The units a set of `bytes` bytes (more than a longword, a whole number
/// of words, ref 3.9.3) is handled in: its offset and width, longwords
/// first, a last word for a size that is not a multiple of 4.
fn large_set_units(bytes: u32) -> Vec<(i32, Width)> {
    let mut units: Vec<(i32, Width)> = (0..bytes / 4)
        .map(|i| (4 * i as i32, Width::Long))
        .collect();
    if bytes % 4 != 0 {
        units.push((4 * (bytes / 4) as i32, Width::Word));
    }
    units
}

/// The bytes of the set constant `members` in a set of `bytes` bytes (ref
/// 3.9.3 p. 3-16, Figure 3-6: member k is bit k mod 8 of the byte
/// `(bytes*8 - 1 - k) div 8`).
fn large_set_bytes(members: &[i64], bytes: u32, span: Span) -> Res<Vec<u8>> {
    let bits = bytes as i64 * 8;
    let mut out = vec![0u8; bytes as usize];
    for &k in members {
        if !(0..bits).contains(&k) {
            return Err(unsupported(
                span,
                format!("a set constant with a member outside the {bits} bits of its set is"),
            ));
        }
        out[((bits - 1 - k) / 8) as usize] |= 1 << (k % 8);
    }
    Ok(out)
}

/// The unit of `w` at offset `o` of the set constant `b` ([`large_set_bytes`])
/// as a constant of that unit.
fn large_set_unit_const(b: &[u8], o: i32, w: Width) -> Expr {
    let ty = Scalar::unsigned(w);
    let bits = b[o as usize..(o as u32 + w.bytes()) as usize]
        .iter()
        .fold(0u64, |v, &x| (v << 8) | x as u64);
    Expr::Const {
        value: imm_value(bits, ty),
        ty,
    }
}

/// Whether `x` is a set constructor or a set constant (a constructor of
/// constant members is folded into one).
fn cons_like(x: &SExpr) -> bool {
    matches!(x.kind, ExprKind::SetCons(_)) || matches!(x.value, Some(ConstValue::Set(_)))
}

/// The operands of the condition of an `if` with an empty `then` part and
/// no `else`, in evaluation order, each to be evaluated for nothing: those
/// of a comparison or a test that are not constants, of both sides of a
/// plain `and`/`or`, of the left of `and then`/`or else` (and of the
/// right when it calls nothing: a call there runs only on one outcome).
/// `None` for a condition whose effect would be lost (a call on the right
/// of `and then`/`or else`) or with no operand form (a block or set
/// comparison).
fn empty_if_operands(c: &Cond) -> Option<Vec<Expr>> {
    fn go(c: &Cond, out: &mut Vec<Expr>) -> bool {
        match c {
            Cond::Compare { lhs, rhs, .. } => {
                for e in [lhs, rhs] {
                    if !matches!(e, Expr::Const { .. }) {
                        out.push(e.clone());
                    }
                }
                true
            }
            Cond::Test(e) | Cond::Odd(e) => {
                if !matches!(e, Expr::Const { .. }) {
                    out.push(e.clone());
                }
                true
            }
            Cond::Not(c) => go(c, out),
            Cond::And(a, b) | Cond::Or(a, b) => go(a, out) && go(b, out),
            Cond::AndThen(a, b) | Cond::OrElse(a, b) => {
                let mut calls = Vec::new();
                cond_calls(b, &mut calls);
                calls.is_empty() && go(a, out) && go(b, out)
            }
            Cond::Block { .. } | Cond::SetEmpty { .. } => false,
        }
    }
    let mut out = Vec::new();
    go(c, &mut out).then_some(out)
}

/// A run-time element of a set constructor ([`large_set_cons_members`]):
/// a member, or a range with a run-time bound.
enum RunElem<'a> {
    One(&'a SExpr),
    Range(&'a SExpr, &'a SExpr),
}

/// The members of a set constructor (ref "Set Operations", p. 4-173): the
/// constant ones (single members and ranges with constant bounds) and the
/// run-time elements in their order, single members and ranges with a
/// run-time bound (built inline, [`StmtKind::StoreSetRange`]: SAU8
/// MST_$INIT 0x3C33F0E; PAS_$SET_BUILD 0xE11FA8 is never called from
/// compiled code, codegen-patterns 5.1).
fn large_set_cons_members(items: &[(SExpr, Option<SExpr>)]) -> (Vec<i64>, Vec<RunElem<'_>>) {
    let mut konst = Vec::new();
    let mut run = Vec::new();
    for (x, hi) in items {
        let lo = x.value.as_ref().and_then(|v| v.ordinal());
        match hi {
            None => match lo {
                Some(k) => konst.push(k),
                None => run.push(RunElem::One(x)),
            },
            Some(h) => match (lo, h.value.as_ref().and_then(|v| v.ordinal())) {
                (Some(a), Some(b)) => konst.extend(a..=b),
                _ => run.push(RunElem::Range(x, h)),
            },
        }
    }
    (konst, run)
}

/// The statement that sets (`value`) or clears the run-time element `x`
/// of a constructor in the set of `bytes` bytes at `dst`; with `only`,
/// every other member cleared first ([`large_set_into`]).
fn run_elem_stmt(
    p: &Program,
    dst: &Addr,
    bytes: u32,
    x: &RunElem,
    value: bool,
    only: bool,
) -> Res<StmtKind> {
    Ok(match x {
        RunElem::One(e) => StmtKind::StoreSetMember {
            set: dst.clone(),
            bytes,
            elem: set_element(p, e)?,
            value,
            only,
        },
        RunElem::Range(lo, hi) => StmtKind::StoreSetRange {
            set: dst.clone(),
            bytes,
            lo: set_element(p, lo)?,
            hi: set_element(p, hi)?,
            value,
            clear: only,
        },
    })
}

/// `d := e` for a set `d` of more than a longword (ref "Assignment", p.
/// 4-25; "Set Operations", pp. 4-173..4-175; layout ref 3.9.3), as stores
/// into `d` itself:
///
/// - a constant: [`StmtKind::StoreSetConst`], a store of each longword (a
///   last word for a size that is not a multiple of 4), `move.l
///   #imm`/`clr.l`: MSG_$INIT 0xE31BD4
///   `move.l #0x4000000,(0x1e0,A0); clr.l (0x1e4,A0)` (`[58]` of a
///   64-bit set), MST_$INIT 0xE30B98 `clr.l (MST_$ASID_LIST).l; move.l
///   #1,(MST_$ASID_LIST+4).l` (`[0]`);
/// - a set variable: a block copy ([`StmtKind::Copy`]);
/// - `[k]`, `s + [k]`, `[k] + s`, `s - [k]` with run-time members:
///   [`StmtKind::StoreSetMember`] on `d` after the rest is assigned;
///   constant members of `+`/`-` too, each set or cleared on its byte;
///   the rest of
///   `t + [k]` (`t` another set) copied into `d` first;
/// - `+`, `*`, `-` of set variables and constants: each longword (and
///   last word) of `d` stored from the operation on the same units of the
///   operands, as on register-sized sets ([`Expr::SetOp`]): MSG_$FORK
///   0xE73F68 `move.l (A4)+,D1; or.l D1,(A2)+` per longword.  Never a copy
///   of one operand followed by the other `or`ed in: an operand may be
///   `d` itself under another address (a `var` parameter, a pointer's
///   target), which the copy would overwrite before it is read.
///
/// Best effort (not byte-identical): the image forms `[k]` and the
/// operations in a frame temporary and copies it into `d` (MSG_$OPENI
/// 0xE59248..0xE5926E, MSG_$CLOSEI 0xE59454..0xE5948C, MSG_$FORK
/// 0xE73F42..0xE73F6C; codegen-patterns 5.1); `dpc` has no such
/// temporary.  The result is the same: the operands are the same object
/// as `d` or disjoint from it, each unit of `d` is formed from the same
/// unit of the operands, and a run-time member is evaluated after the
/// stores before it (a member's value cannot depend on `d`'s bits but
/// through `in`, which no kernel site does).
fn large_set_store(p: &Program, s: &SStmt, target: &SExpr, value: &SExpr) -> Res<Vec<Stmt>> {
    if volatile_access(p, target).is_some()
        || p.types.has_volatile_part(target.ty)
        || volatile_access(p, value).is_some()
    {
        return Err(unsupported(
            s.span,
            "an assignment to or from a volatile set of more than a longword (ref \
             3.15.1-3.15.3) is",
        ));
    }
    check_even(p, target)?;
    let bytes = p.types.size(target.ty);
    let dst = addr_of(p, target)?;
    let mut out = Vec::new();
    large_set_into(p, s.span, &dst, bytes, value, &mut out)?;
    Ok(out)
}

/// The statements of [`large_set_store`] that make the set of `bytes`
/// bytes at `dst` hold `value`, appended to `out`.
fn large_set_into(
    p: &Program,
    span: Span,
    dst: &Addr,
    bytes: u32,
    value: &SExpr,
    out: &mut Vec<Stmt>,
) -> Res<()> {
    let stmt = |kind| Stmt { kind, span };
    if let Some(v) = &value.value {
        let ConstValue::Set(members) = v else {
            return Err(unsupported(value.span, "this set value is"));
        };
        out.push(stmt(StmtKind::StoreSetConst {
            set: dst.clone(),
            bits: large_set_bytes(members, bytes, value.span)?,
        }));
        return Ok(());
    }
    if value.is_designator() {
        let n = p.types.size(value.ty);
        if !is_set(p, value.ty) || n > bytes {
            return Err(unsupported(
                value.span,
                "assigning a set of another size is",
            ));
        }
        if volatile_access(p, value).is_some() {
            return Err(unsupported(
                value.span,
                "assigning a volatile set of more than a longword is",
            ));
        }
        check_even(p, value)?;
        let src = addr_of(p, value)?;
        if n < bytes {
            // A smaller set widened (ref 3.9.3: member k is bit k of the
            // set's big-endian integer, so the smaller set's bytes are the
            // larger's last ones): its leading bytes cleared, then the
            // smaller set copied after them.  SAU8 MST_$FREE_ASID
            // 0x3C7F664..0x3C7F688 widens a 16-byte set into a 32-byte
            // frame temporary, `lea (-72,A6),A0; clr.l (A0)+` x4, then
            // `move.l (A1)+,(A3)+` x4 to -56 (`private :=
            // mst_$regions_active[asid] - [...]`).  Best effort (not
            // byte-identical): `dpc` widens into the set itself, its
            // leading bytes as a constant store ([`StmtKind::StoreSetConst`]).
            best_effort(
                value.span,
                "a set widened into a larger set (MST_$FREE_ASID 0x3C7F664 widens into a frame \
                 temporary; codegen-patterns 5.1) is",
            )?;
            out.push(stmt(StmtKind::StoreSetConst {
                set: dst.clone(),
                bits: vec![0; (bytes - n) as usize],
            }));
            out.push(stmt(StmtKind::Copy {
                dst: dst.clone().offset_by((bytes - n) as i32),
                src,
                bytes: n,
                chars: false,
            }));
            return Ok(());
        }
        if src != *dst {
            out.push(stmt(StmtKind::Copy {
                dst: dst.clone(),
                src,
                bytes,
                chars: false,
            }));
        }
        return Ok(());
    }
    match &value.kind {
        ExprKind::SetCons(items) => {
            let (konst, run) = large_set_cons_members(items);
            if !run.is_empty() {
                best_effort(
                    value.span,
                    "a set of more than a longword built or updated in place (the image forms it in a frame \
                 temporary and copies it, MSG_$OPENI 0xE59248, MSG_$CLOSEI 0xE59454; codegen-patterns \
                 5.1) is",
                )?;
            }
            let mut run = run.into_iter();
            if konst.is_empty() {
                let first = run.next().expect("a constructor that is not constant");
                out.push(stmt(run_elem_stmt(p, dst, bytes, &first, true, true)?));
            } else {
                out.push(stmt(StmtKind::StoreSetConst {
                    set: dst.clone(),
                    bits: large_set_bytes(&konst, bytes, value.span)?,
                }));
            }
            for x in run {
                out.push(stmt(run_elem_stmt(p, dst, bytes, &x, true, false)?));
            }
            Ok(())
        }
        ExprKind::Binary {
            op: op @ (BinOp::Add | BinOp::Sub),
            lhs,
            rhs,
        } if is_set(p, value.ty)
            && (cons_like(rhs) || (*op == BinOp::Add && cons_like(lhs)))
            && (rhs.value.is_none() || lhs.value.is_none()) =>
        {
            best_effort(
                value.span,
                "a set of more than a longword built or updated in place (the image forms it in a frame \
                 temporary and copies it, MSG_$OPENI 0xE59248, MSG_$CLOSEI 0xE59454; codegen-patterns \
                 5.1) is",
            )?;
            // `x + [..]`, `[..] + x`, `x - [..]`: `x` into the set, then
            // the constructor's members set or cleared, each by its bit
            // (`bset`/`bclr #k mod 8` on its byte for a constant one, as on
            // a word set: TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E `bclr
            // #0,(9,A2)`; codegen-patterns 5.1).
            let (rest, cons) = if cons_like(rhs) {
                (lhs, rhs)
            } else {
                (rhs, lhs)
            };
            let (konst, run) = match (&cons.value, &cons.kind) {
                (Some(ConstValue::Set(m)), _) => (m.clone(), Vec::new()),
                (_, ExprKind::SetCons(items)) => large_set_cons_members(items),
                _ => unreachable!("a constructor or a set constant"),
            };
            large_set_bytes(&konst, bytes, cons.span)?;
            large_set_into(p, span, dst, bytes, rest, out)?;
            let add = *op == BinOp::Add;
            for k in konst {
                out.push(stmt(StmtKind::StoreSetMember {
                    set: dst.clone(),
                    bytes,
                    elem: Expr::Const {
                        value: k,
                        ty: Scalar::signed(Width::Word),
                    },
                    value: add,
                    only: false,
                }));
            }
            for x in run {
                out.push(stmt(run_elem_stmt(p, dst, bytes, &x, add, false)?));
            }
            Ok(())
        }
        ExprKind::Binary {
            op: BinOp::Add | BinOp::Mul | BinOp::Sub,
            ..
        } if is_set(p, value.ty) => {
            best_effort(
                value.span,
                "a set of more than a longword built or updated in place (the image forms it in a frame \
                 temporary and copies it, MSG_$OPENI 0xE59248, MSG_$CLOSEI 0xE59454; codegen-patterns \
                 5.1) is",
            )?;
            for (o, w) in large_set_units(bytes) {
                let v = large_set_unit(p, value, bytes, o, w)?;
                out.push(stmt(StmtKind::Store {
                    addr: dst.clone().offset_by(o),
                    value: v,
                }));
            }
            Ok(())
        }
        _ => Err(unsupported(
            value.span,
            "this value of a set of more than a longword (codegen-patterns 5.1) is",
        )),
    }
}

/// The unit of width `w` at offset `o` of the set value `e` of `bytes`
/// bytes ([`large_set_into`]): of a variable, its load; of a constant, its
/// bits; of `+`, `*`, `-`, the operation on the operands' units.  A
/// run-time constructor is refused here (it has no units in memory).
fn large_set_unit(p: &Program, e: &SExpr, bytes: u32, o: i32, w: Width) -> Res<Expr> {
    let ty = Scalar::unsigned(w);
    if let Some(v) = &e.value {
        let ConstValue::Set(members) = v else {
            return Err(unsupported(e.span, "this set value is"));
        };
        let b = large_set_bytes(members, bytes, e.span)?;
        return Ok(large_set_unit_const(&b, o, w));
    }
    if e.is_designator() {
        let n = p.types.size(e.ty);
        // A smaller set's units are the larger's last ones, its leading
        // ones 0 (ref 3.9.3; widened as [`large_set_into`] widens it); a
        // unit across the boundary is refused.
        let pad = bytes as i32 - n as i32;
        if !is_set(p, e.ty) || pad < 0 || (o < pad && o + w.bytes() as i32 > pad) {
            return Err(unsupported(
                e.span,
                "a set operation on sets of other sizes is",
            ));
        }
        if o < pad {
            return Ok(Expr::Const { value: 0, ty });
        }
        if volatile_access(p, e).is_some() {
            return Err(unsupported(
                e.span,
                "a volatile set of more than a longword as an operand is",
            ));
        }
        check_even(p, e)?;
        return Ok(Expr::Load {
            addr: addr_of(p, e)?.offset_by(o - pad),
            ty,
        });
    }
    match &e.kind {
        ExprKind::Binary {
            op: op @ (BinOp::Add | BinOp::Mul | BinOp::Sub),
            lhs,
            rhs,
        } if is_set(p, e.ty) => Ok(Expr::SetOp {
            op: match op {
                BinOp::Add => SetOp::Union,
                BinOp::Mul => SetOp::Inter,
                _ => SetOp::Diff,
            },
            lhs: Box::new(large_set_unit(p, lhs, bytes, o, w)?),
            rhs: Box::new(large_set_unit(p, rhs, bytes, o, w)?),
            ty,
        }),
        _ => Err(unsupported(
            e.span,
            "this operand of an operation on sets of more than a longword (a run-time \
             constructor: no unit in memory) is",
        )),
    }
}

/// `lhs = rhs`, `lhs <> rhs` for sets of more than a longword (ref "Set
/// Operations", p. 4-176):
///
/// - against `[]`: [`Cond::SetEmpty`] (`tst.l (An)+` per longword,
///   MSG_$OPENI 0xE5920A, MSG_$CLOSEI 0xE59490);
/// - two set variables: [`Cond::Block`], as records of that size (ref
///   Table 4-1, p. 4-2) for the sizes whose `cmpm` form is catalogued
///   (codegen-patterns 7.1, "Records"; 6, 8 and multiples of 4 from 32
///   bytes);
/// - otherwise, and against a non-empty constant: the units compared one
///   by one, `and then` (best effort, not byte-identical: no site);
/// - subset `s <= t` and superset `s >= t`: each unit of `s - t` (`t - s`)
///   against 0, `and then`, as `s - t = []` for register-sized sets
///   ([`set_inclusion`]; best effort, no site).
///
/// Refused: a constant left operand of `=`/`<>`, a computed operand
/// (other than of `<=`/`>=`).
fn large_set_compare(p: &Program, e: &SExpr, rel: Rel, lhs: &SExpr, rhs: &SExpr) -> Res<Cond> {
    if matches!(rel, Rel::Le | Rel::Ge) {
        best_effort(
            e.span,
            "a subset or superset comparison of sets of more than a longword (no site; \
             codegen-patterns 5.1) is",
        )?;
        let (sub, sup) = if rel == Rel::Le {
            (lhs, rhs)
        } else {
            (rhs, lhs)
        };
        let bytes = p.types.size(lhs.ty).max(p.types.size(rhs.ty));
        let mut c: Option<Cond> = None;
        for (o, w) in large_set_units(bytes) {
            let ty = Scalar::unsigned(w);
            let unit = Cond::Compare {
                op: Rel::Eq,
                lhs: Expr::SetOp {
                    op: SetOp::Diff,
                    lhs: Box::new(large_set_unit(p, sub, bytes, o, w)?),
                    rhs: Box::new(large_set_unit(p, sup, bytes, o, w)?),
                    ty,
                },
                rhs: Expr::Const { value: 0, ty },
            };
            c = Some(match c {
                None => unit,
                Some(prev) => Cond::AndThen(Box::new(prev), Box::new(unit)),
            });
        }
        return Ok(c.expect("a set has units"));
    }
    if !matches!(rel, Rel::Eq | Rel::Ne) {
        return Err(unsupported(e.span, "this comparison of sets is"));
    }
    if lhs.value.is_some() || !lhs.is_designator() {
        return Err(unsupported(
            e.span,
            "comparing a set of more than a longword that is not a variable (no site known, \
             codegen-patterns 5.1) is",
        ));
    }
    if volatile_access(p, lhs).is_some() || volatile_access(p, rhs).is_some() {
        return Err(unsupported(
            e.span,
            "comparing a volatile set of more than a longword is",
        ));
    }
    check_even(p, lhs)?;
    let bytes = p.types.size(lhs.ty);
    let set = addr_of(p, lhs)?;
    let wrap = |c: Cond| {
        if rel == Rel::Ne {
            Cond::Not(Box::new(c))
        } else {
            c
        }
    };
    if matches!(&rhs.value, Some(ConstValue::Set(m)) if m.is_empty()) {
        return Ok(wrap(Cond::SetEmpty { set, size: bytes }));
    }
    if rhs.value.is_none() {
        if !rhs.is_designator() || !is_set(p, rhs.ty) || p.types.size(rhs.ty) != bytes {
            return Err(unsupported(
                e.span,
                "comparing a set of more than a longword with a computed set or one of another \
                 size (no site known, codegen-patterns 5.1) is",
            ));
        }
        check_even(p, rhs)?;
        if bytes == 6 || bytes == 8 || (bytes % 4 == 0 && (32..=512).contains(&bytes)) {
            best_effort(
                e.span,
                "comparing two sets of more than a longword (as records of that size: no set \
                 site; codegen-patterns 5.1, 7.1) is",
            )?;
            return Ok(Cond::Block {
                op: rel,
                lhs: set,
                rhs: addr_of(p, rhs)?,
                size: bytes,
            });
        }
    }
    best_effort(
        e.span,
        "comparing a set of more than a longword unit by unit (no site; codegen-patterns 5.1) is",
    )?;
    let mut c: Option<Cond> = None;
    for (o, w) in large_set_units(bytes) {
        let unit = Cond::Compare {
            op: Rel::Eq,
            lhs: Expr::Load {
                addr: set.clone().offset_by(o),
                ty: Scalar::unsigned(w),
            },
            rhs: large_set_unit(p, rhs, bytes, o, w)?,
        };
        c = Some(match c {
            None => unit,
            Some(prev) => Cond::AndThen(Box::new(prev), Box::new(unit)),
        });
    }
    Ok(wrap(c.expect("a set has units")))
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
    if let Some(ConstValue::Nil) = &value.value {
        // `nil`, the pointer 0 (ref 3.13.4, p. 3-51): the selector decides
        // its form by the shape of the body (codegen-patterns 3, "`nil`
        // results"; bead domain-pascal-bdi0): `suba.l An,An` into a
        // result variable held in a register (ROUTE_$FIND_PORTP
        // 0xE15B84), `movea.l #0,A0` as the only assignment and last
        // statement (TIME_$RTE_INT 0xE163D6); the result slot cleared
        // (NET_$FIND_HANDLER 0xE5A178) is refused there.
        return Ok(Stmt {
            kind: StmtKind::SetResult(stored_const(&ConstValue::Nil, res.ty, value.span)?),
            span: s.span,
        });
    }
    if is_set(p, value.ty) {
        // A set result of one word or one longword (codegen-patterns 3,
        // "Set results"): the value as a set of the result's width, a
        // one-byte set widened (acl_$eval_acl_entries 0xE4617A's `f :=
        // prot.owner_rights`, [`set_value`]).
        return Ok(Stmt {
            kind: StmtKind::SetResult(set_value(p, value, res.ty)?),
            span: s.span,
        });
    }
    if let Some(v) = &value.value {
        return Ok(Stmt {
            kind: StmtKind::SetResult(const_of(v, res.ty, value.span)?),
            span: s.span,
        });
    }
    let v = value_of(p, value)?;
    let v = if vsize == res.ty.bytes() {
        v
    } else {
        // Widened or narrowed as for a store (`store`, [`assign_convert`]).
        let what = format!(
            "assigning a {vsize}-byte value to a {}-byte function result is",
            res.ty.bytes()
        );
        match assign_convert(p, value, v.clone(), res.ty) {
            Some(c) => c,
            None => narrow_best_effort(p, value, v, res.ty, value.span, &what)
                .unwrap_or_else(|| Err(unsupported(value.span, &what)))?,
        }
    };
    Ok(Stmt {
        kind: StmtKind::SetResult(v),
        span: s.span,
    })
}

/// Whether `r` returns a 4-byte record ([`ResultReg::Record`]).
fn is_record_result(r: &Routine) -> bool {
    r.result
        .as_ref()
        .is_some_and(|res| res.reg == ResultReg::Record)
}

/// Whether `r` returns its result through a hidden address
/// ([`ResultReg::Hidden`]: a record of more than 4 bytes).
fn is_hidden_result(r: &Routine) -> bool {
    r.result
        .as_ref()
        .is_some_and(|res| res.reg == ResultReg::Hidden)
}

/// The call in `e`, if `e` is a call of a routine whose result goes
/// through a hidden address ([`ResultReg::Hidden`]).
fn hidden_call<'e>(p: &Program, e: &'e SExpr) -> Option<&'e SCall> {
    match &e.kind {
        ExprKind::Call(c) => match &c.callee {
            SCallee::Routine(id) if is_hidden_result(p.routine(*id)) => Some(c),
            _ => None,
        },
        _ => None,
    }
}

/// The refusal of a call of a function whose result goes through a hidden
/// address ([`ResultReg::Hidden`]) used other than in the two forms the
/// image shows (codegen-patterns 3, "Record results over 4 bytes"):
/// assigned to a variable ([`Cx::hidden_call_assignment`]) and passed by
/// address ([`Arg::Call`]).  Not seen: `discard`, a comparison, a field of
/// the result, the result as a `with` record.
const HIDDEN_CALL_USE: &str = "a call of a function whose record result goes through a hidden \
     address, other than assigned to a variable or passed by address (codegen-patterns 3, \
     \"Record results over 4 bytes\"), is";

/// The refusal of a nested function whose result goes through a hidden
/// address, its definition and its calls: no nested routine of the image
/// returns one, so where the address goes beside the static link in A1
/// (codegen-patterns 6) is not seen.
const HIDDEN_NESTED: &str = "a nested function whose record result goes through a hidden \
     address (where the address goes beside the static link is not seen, codegen-patterns 3, \
     \"Record results over 4 bytes\") is";

/// The pseudo frame offset of the `n`th result temporary of a routine
/// ([`Cx::result_temps`]) until [`assign_temps`] places it: outside any
/// frame, below the `with` temporaries' ([`WITH_TEMP_BASE`]).
const RESULT_TEMP_BASE: i32 = -0x6000_0000;

thread_local! {
    /// The frame temporaries of the routine being lowered that receive
    /// the results of calls of functions whose result goes through a
    /// hidden address, by number: their sizes.  Until [`assign_temps`]
    /// places them, each is `Addr::Frame(result_temp(n))` ([`call`]).
    static RESULT_TEMPS: std::cell::RefCell<Vec<u32>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn result_temp(n: usize) -> i32 {
    RESULT_TEMP_BASE - n as i32
}

/// A new result temporary of `bytes` bytes ([`RESULT_TEMPS`]).
fn new_result_temp(bytes: u32) -> Addr {
    RESULT_TEMPS.with(|t| {
        let mut t = t.borrow_mut();
        t.push(bytes);
        Addr::Frame(result_temp(t.len() - 1))
    })
}

/// The number of the result temporary at pseudo offset `o`, if `o` is one
/// ([`result_temp`]).
fn result_temp_index(o: i32) -> Option<usize> {
    (o <= RESULT_TEMP_BASE && o > RESULT_TEMP_BASE - 0x1000_0000)
        .then(|| (RESULT_TEMP_BASE - o) as usize)
}

/// `f := r` inside `f`, for a result through a hidden address
/// ([`ResultReg::Hidden`]; codegen-patterns 3, "Record results over 4
/// bytes"): a record copy into the object the first argument slot points
/// to, as into an `out` parameter's (the module-local 0xE58C3A
/// 0xE58C62..0xE58C6A `movea.l (0x8,A6),A1; move.l (-0xc,A6),(A1); move.w
/// (-0x8,A6),(0x4,A1)`; NAME_$GET_NODE_DATA_UID 0xE58A0C..0xE58A14
/// `movea.l (0x8,A6),A1; lea (A5),A0; move.l (A0)+,(A1); move.l
/// (A0)+,(0x4,A1)`; the module-local 0xE4232A 0xE42344..0xE4234E).  The
/// store is the assignment, with nothing at the epilogue: no image
/// caller reads a register after such a call, and the callees leave D0
/// and A0 as their body left them, so an assignment may come anywhere and
/// more than once (ACL_$DEFAULT_ACL 0xE4787E assigns on three paths).
///
/// Only a record variable is accepted as the value (all the sites); a
/// function's result (`f := g(x)`) is not seen.
fn hidden_result_assignment(p: &Program, r: &Routine, s: &SStmt, value: &SExpr) -> Res<Stmt> {
    let res = r.result.as_ref().expect("a function");
    let var = res
        .var
        .expect("a function with a body has a result variable");
    if !value.is_designator() || value.value.is_some() {
        return Err(unsupported(
            value.span,
            "a record function result assigned other than a record variable (a function's \
             result: no site seen, codegen-patterns 3, \"Record results over 4 bytes\") is",
        ));
    }
    let bytes = p.types.size(res.ty);
    if p.types.size(value.ty) != bytes {
        return Err(unsupported(
            value.span,
            "assigning a record of another size to a record function result is",
        ));
    }
    if volatile_access(p, value).is_some() || p.types.has_volatile_part(value.ty) {
        return Err(unsupported(
            value.span,
            "a record function result assigned from a volatile or device record (ref              3.15.1-3.15.3) is",
        ));
    }
    check_even(p, value)?;
    Ok(Stmt {
        kind: StmtKind::Copy {
            dst: var_addr(p, var, s.span)?,
            src: addr_of(p, value)?,
            bytes,
            chars: false,
        },
        span: s.span,
    })
}

/// `f := r` inside `f`, for a 4-byte record result ([`ResultReg::Record`];
/// codegen-patterns 3, "Record results").  The result variable is a frame
/// object, as a `status_$t` local is ([`scalar_locals`]): the record is
/// copied into the result variable's slot, which is then loaded into the
/// result register, `move.l (-8,A6),(-0x10,A6); move.l (-0x10,A6),D0`
/// (PEB_$GET_STATUS 0xE5AE52, `link #-0x10`, the slot below the locals;
/// VOLX_$SHUTDOWN 0xE6B5B2 `move.l (-12,A6),(-16,A6); move.l
/// (-16,A6),D0`), where a final `f := local` of a scalar result reads the
/// local itself (`tools/resultsurvey.py list frame`: the other 16
/// functions of that class).  Both sites' C (`peb/get_status.c`,
/// `volx/shutdown.c`) returns `status_$t`; that their Pascal did is not
/// known.  So: a [`StmtKind::Store`] into the slot and a
/// [`StmtKind::SetResult`] of the slot, which [`check_record_result`]
/// keeps to the last statement of the body.
///
/// Only a record in memory is accepted: a function's result (`f :=
/// g(x)`; a pointer function's A0 is stored and left in A0, MST_$MAPS
/// 0xE439C8, but no record site is known) and a parameter passed by
/// value (the selector may hold it in a register) are refused.
fn record_result_assignment(p: &Program, r: &Routine, s: &SStmt, value: &SExpr) -> Res<Vec<Stmt>> {
    let res = r.result.as_ref().expect("a function");
    let slot = res
        .var
        .and_then(|v| match p.var(v).storage {
            dpc_sema::Storage::Frame { offset } => Some(offset),
            _ => None,
        })
        .ok_or_else(|| unsupported(r.span, "a record result without a frame slot is"))?;
    if let Some(c) = value.value.as_ref().filter(|_| p.types.size(value.ty) == 4) {
        // A constant (`f := status_$t(integer32(status_$...))`): stored
        // into the result slot as a longword, which is loaded at the end,
        // MST_$ALLOC_TABLE_PAGE 0xE43F72 `move.l #0x4000e,(-0xc,A6); bra`
        // to the epilogue.
        let ty = scalar_of_type(&p.types, res.ty, Width::Long);
        return Ok(vec![
            Stmt {
                kind: StmtKind::Store {
                    addr: Addr::Frame(slot),
                    value: const_of(c, ty, value.span)?,
                },
                span: s.span,
            },
            Stmt {
                kind: StmtKind::SetResult(Expr::Load {
                    addr: Addr::Frame(slot),
                    ty,
                }),
                span: s.span,
            },
        ]);
    }
    if !value.is_designator() || value.value.is_some() {
        if value.value.is_some() || p.types.size(value.ty) != 4 {
            return Err(unsupported(
                value.span,
                "a record function result assigned other than a record variable (a function's \
                 result: no site seen, codegen-patterns 3, \"Record results\") is",
            ));
        }
        // A type transfer of a constant (`status_$t(integer32(k))`) is
        // the constant's form above.  Best effort (not byte-identical)
        // otherwise, a function's result or a type transfer of a computed
        // longword (`f := g(x)`): no site; the value stored into the
        // result slot as a longword, as a constant is
        // (MST_$ALLOC_TABLE_PAGE 0xE43F72), and the slot loaded at the
        // end.
        let v = value_of(p, value)?;
        if !matches!(v, Expr::Const { .. }) {
            best_effort(
                value.span,
                "a record function result assigned other than a record variable or a \
                 constant (a function's result: no site seen, codegen-patterns 3, \"Record \
                 results\") is",
            )?;
        }
        let ty = v.ty();
        return Ok(vec![
            Stmt {
                kind: StmtKind::Store {
                    addr: Addr::Frame(slot),
                    value: v,
                },
                span: s.span,
            },
            Stmt {
                kind: StmtKind::SetResult(Expr::Load {
                    addr: Addr::Frame(slot),
                    ty,
                }),
                span: s.span,
            },
        ]);
    }
    if p.types.size(value.ty) != p.types.size(res.ty) {
        return Err(unsupported(
            value.span,
            "assigning a record of another size to a record function result is",
        ));
    }
    if volatile_access(p, value).is_some() || p.types.has_volatile_part(value.ty) {
        return Err(unsupported(
            value.span,
            "a record function result assigned from a volatile or device record (ref \
             3.15.1-3.15.3) is",
        ));
    }
    let v = value_of(p, value)?;
    if matches!(&v, Expr::Load { addr: Addr::Frame(o), .. } if *o > 0) {
        best_effort(
            value.span,
            "a record function result assigned from a parameter passed by value (no site \
             seen) is",
        )?;
    }
    let ty = v.ty();
    Ok(vec![
        Stmt {
            kind: StmtKind::Store {
                addr: Addr::Frame(slot),
                value: v,
            },
            span: s.span,
        },
        Stmt {
            kind: StmtKind::SetResult(Expr::Load {
                addr: Addr::Frame(slot),
                ty,
            }),
            span: s.span,
        },
    ])
}

/// A function returning a 4-byte record assigns its result once, as the
/// last statement of its body, with no `return` before it: the two sites
/// of [`record_result_assignment`] store into the result slot and load it
/// right before the epilogue; where the load would be with several
/// assignments, or with a `return` (at the epilogue, where `return`
/// branches, or after each store), is not seen.
fn check_record_result(body: &[Stmt], span: Span) -> Res<()> {
    let mut sets = 0;
    let mut returns = false;
    crate::visit_stmts(body, &mut |s| match s.kind {
        StmtKind::SetResult(_) => sets += 1,
        StmtKind::Return => returns = true,
        _ => {}
    });
    let last = matches!(body.last().map(|s| &s.kind), Some(StmtKind::SetResult(_)));
    if sets != 1 || !last || returns {
        // Best effort: each assignment stores the slot and sets the
        // result from it, a longword result as any other (bead
        // domain-pascal-5kgg).
        best_effort(
            span,
            "a record function result assigned other than once, by the last statement of the \
             body with no `return` (where the result slot is loaded is not seen, \
             codegen-patterns 3, \"Record results\") is",
        )?;
    }
    Ok(())
}

/// The copy of `bytes` bytes a best-effort assignment makes where which
/// copy the original made is not seen (bead domain-pascal-5kgg): byte by
/// byte (no alignment needed; the selector's `move.b` `dbf` loop, any
/// size up to 0x8000), else by longwords (the even-address checks then
/// apply).
fn best_effort_chars(bytes: u32) -> bool {
    (1..=0x8000).contains(&bytes)
}

/// `d := e` for a scalar designator `d` (ref "Assignment", p. 4-25).
///
/// A constant value takes the designator's type (the checker has made
/// sure it fits); an integer of another size is widened or narrowed
/// ([`assign_convert`]).
fn store(p: &Program, s: &SStmt, target: &SExpr, value: &SExpr) -> Res<Stmt> {
    if let Some((array, bytes, index, lo)) = bit_element(p, target)? {
        // `true` or `false`: the bit set or cleared through a mask
        // (codegen-patterns 5.2, "Packed arrays of booleans").  A computed
        // value is merged under a mask whose bit number the image keeps
        // in a frame byte (ACL_$INHERIT_SUBSYS 0xE4914C..0xE49182,
        // ACL_$OVERRIDE_LOCAL_LOCKSMITH 0xE4928C..0xE492BC), not modelled.
        let Some(ConstValue::Bool(b)) = &value.value else {
            // Best effort (not byte-identical): `if v then a[i] := true
            // else a[i] := false`, each part the constant's `bset`/`bclr`.
            best_effort(
                value.span,
                "storing a value other than the constant `true` or `false` into an element of \
                 a packed array of booleans (the bit number kept in a frame byte, \
                 codegen-patterns 5.2) is",
            )?;
            let cond = cond_of(p, value)?;
            let part = |value: bool| Stmt {
                kind: StmtKind::StoreBitElem {
                    array: array.clone(),
                    bytes,
                    index: index.clone(),
                    lo,
                    value,
                },
                span: s.span,
            };
            return Ok(Stmt {
                kind: StmtKind::If {
                    cond,
                    then_: vec![part(true)],
                    else_: vec![part(false)],
                },
                span: s.span,
            });
        };
        return Ok(Stmt {
            kind: StmtKind::StoreBitElem {
                array,
                bytes,
                index,
                lo,
                value: *b,
            },
            span: s.span,
        });
    }
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
        stored_const(v, ty, value.span)?
    } else if is_set(p, target.ty) {
        // A set expression takes the target's size (its constructors have
        // their own, ref 3.9.3).
        if ty.width == Width::Byte
            && p.types.size(value.ty) == 2
            && !matches!(value.kind, ExprKind::SetCons(_) | ExprKind::Binary { .. })
        {
            byte_set_narrow(p, value)?
        } else {
            set_value(p, value, ty)?
        }
    } else {
        let v = value_of(p, value)?;
        if v.ty().width != ty.width {
            // Widened or narrowed as an integer (ref 4.3.1, p. 4-5;
            // [`assign_convert`]); the further narrowings of an assignment
            // ([`store_narrow`]).
            let (vb, tb) = (v.ty().bytes(), ty.bytes());
            match assign_convert(p, value, v.clone(), ty) {
                Some(c) => c,
                None => match store_narrow(p, value, v.clone(), ty) {
                    Some(Ok(c)) => c,
                    Some(Err(why)) => narrow_best_effort(p, value, v, ty, value.span, &why)
                        .unwrap_or_else(|| Err(unsupported(value.span, &why)))?,
                    None => {
                        let what = format!("assigning a {vb}-byte value to a {tb}-byte object is");
                        narrow_best_effort(p, value, v, ty, value.span, &what)
                            .unwrap_or_else(|| Err(unsupported(value.span, &what)))?
                    }
                },
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
        // `abs` is not arithmetic modulo 2^bits (the low word of `abs` of a
        // longword in 32768..65535 is not `abs` of its low word): only the
        // `abs` of a word sum, difference or product that [`convert`]
        // widened (its same-type mark) is taken back to the word (the
        // only width [`convert`] widens it from), where the operation's
        // Pascal type was (TPAD_$DATA 0xE69624..0xE69638,
        // SMD_$BLT 0xE6EDA2: word `muls.w`/`sub.w` under a word `abs`).
        // Any other `abs` is computed at its width and its low bytes
        // used ([`assign_convert`]).
        Expr::Convert { value, ty: t, .. }
            if value.ty() == *t
                && ty.width == Width::Word
                && matches!(
                    &**value,
                    Expr::Unary {
                        op: IrUnOp::Abs,
                        ..
                    }
                ) =>
        {
            let Expr::Unary { operand, .. } = &**value else {
                unreachable!()
            };
            Some(Expr::Unary {
                op: IrUnOp::Abs,
                operand: Box::new(narrow(operand, ty)?),
                ty,
            })
        }
        // A value read with another sign ([`reinterpret`], as a shift's
        // result): the same bits.
        Expr::Convert { value, ty: t, .. } if value.ty().width == t.width => narrow(value, ty),
        Expr::Convert { value, .. } if value.ty().width == ty.width => Some(convert(
            (**value).clone(),
            Scalar {
                width: ty.width,
                ..value.ty()
            },
        )),
        // A byte widened to a longword, used as a word: the byte widened
        // to a word (the same low word).
        Expr::Convert { value, ty: t, .. }
            if value.ty().bytes() < ty.bytes() && ty.bytes() < t.bytes() =>
        {
            Some(convert((**value).clone(), ty))
        }
        // A word variable widened to a longword, used as a byte: its low
        // byte, read as [`store_narrow`] reads a narrowed variable
        // (AST_$TOUCH_AREA SAU4 0xF803D74 `add.b D6,D0` for `seg_offset :=
        // i + page - 1` of `integer`/`0..65535` words into a byte; ../source
        // finding 760).
        Expr::Convert { value, ty: t, .. }
            if matches!(**value, Expr::Load { .. })
                && value.ty().bytes() == 2 * ty.bytes()
                && t.bytes() > value.ty().bytes() =>
        {
            Some(Expr::Convert {
                value: value.clone(),
                ty,
                in_place: true,
            })
        }
        // A product used as a byte: by a power of two it is shifted at the
        // byte's width (the selector's `lsl.b`; SAU4 FLP_ 0xF845D06 for
        // `vol.sec_per_track * 4 - 1` into a byte); otherwise, as the
        // 68000 multiplies words only, computed at the word's width and
        // its low byte taken, the same low byte (no site).
        Expr::Binary {
            op: IrBinOp::Mul,
            lhs,
            rhs,
            ..
        } if ty.width == Width::Byte
            && ![lhs, rhs].iter().any(|x| {
                matches!(***x, Expr::Const { value, .. }
                    if value > 0 && (value as u64).is_power_of_two())
            }) =>
        {
            Some(Expr::Convert {
                value: Box::new(narrow(
                    e,
                    Scalar {
                        width: Width::Word,
                        ..ty
                    },
                )?),
                ty,
                in_place: false,
            })
        }
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
        // A logical right shift of an unsigned word (or byte) widened with
        // zeros (`rshft` of a `0..65535` value, [`builtin_value`]) stored
        // as a word: the same low word as the word shifted, so shifted at
        // the word's width: module-local 0xE07386 `move.w D0w,D4w; lsr.w
        // #3,D4w` (`seg_group := rshft(seg_idx, 3)`, both `0..65535`);
        // module-local 0xE085E8..0xE085F2 `move.w (-0xa6,A6),D3w; lsr.w
        // #3,D3w; move.w (-0xa4,A6),D4w; lsr.w #3,D4w` (into `integer`s);
        // VTOC_$GET_UID 0xE3922A and module-local 0xE3845A `lsr.w
        // #2` (bead domain-pascal-4rfc).  Only that operand: a wider
        // value's high bits would be shifted in.
        Expr::Binary {
            op: IrBinOp::Shr,
            lhs,
            rhs,
            ..
        } if matches!(&**lhs, Expr::Convert { value, ty: t, .. }
            if value.ty().bytes() <= ty.bytes()
                && value.ty().sign == Sign::Unsigned
                && !value.ty().pointer
                && t.width == Width::Long) =>
        {
            Some(Expr::Binary {
                op: IrBinOp::Shr,
                lhs: Box::new(narrow(lhs, ty)?),
                rhs: rhs.clone(),
                ty,
            })
        }
        Expr::Unary {
            op: IrUnOp::Abs, ..
        } => None,
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
            in_place: false,
        }),
        _ => None,
    }
}

/// `v`, the value of the integer expression `e`, converted to the integer
/// type `ty` of an assignment's target, a value parameter or a function
/// result (bead domain-pascal-8td; codegen-patterns 8, "Narrowing and
/// widening").  The reference sets no range check: "Currently, Domain
/// Pascal does not support subrange checking" (3.8, p. 3-14) and "Domain
/// Pascal does not detect an overflow or underflow on integer arithmetic"
/// (Appendix D, ISO 6.7.2.2, p. D-4); "you may pass an integer argument
/// that is outside the corresponding parameter's declared subrange"
/// (ISO 6.4.6, p. D-3).  The target keeps the low bytes (not stated by
/// the reference: reference-clarifications 41).
///
/// - Wider: widened by the value's sign ([`convert`]).
/// - Narrower, computable at the narrow width ([`narrow`]): so computed.
/// - Narrower, otherwise, by one step (a longword to a word, a word to a
///   byte; `None` for a longword to a byte, not catalogued):
///   - a variable, a parameter or a field of one: its low bytes read in
///     place, a [`Expr::Convert`] of the load that the selector reads at
///     the narrow width, in memory at the higher address (big-endian;
///     APP_$DEMUX 0xE00AF6 `move.w (0x1a,A0),(-0x14,A6)` for a longword
///     field at 0x18; MMAP_$FREE_PAGES 0xE0CF2C `move.w (-0x12,A6),...`
///     for a longword local at -0x14; PACCT_$LOG 0xE5AB26 `move.b
///     (1,A0),(-0x18e,A6)` for a word `in` parameter) or in its register
///     (LOG_$ADD 0xE1775E `move.w D5,-(SP)`, a longword held in D5; SOCK
///     0xE15ECE `move.b D4,(0x1b,A2)`, a word held in D4).  An array
///     element, a pointer's target or a `with` record (AST_$TOUCH 0xE0350E
///     and AST_$TOUCH_AREA 0xE039FC load an element's longword through a
///     pointer, `move.l (An),Dn; move.w Dn,-(SP)`, both pushed) and a
///     volatile object (ref 3.15.1-3.15.3: an access of another size) are
///     refused here; an assignment's are [`store_narrow`]'s;
///   - a computed value (a right shift, a sum with a longword operand):
///     computed at its own width, the low bytes used, a [`Expr::Convert`]
///     (DBUF_$GET_BLOCK 0xE3A5F2 `move.l D3,D1; lsr.l #5,D1; move.w
///     D1,-(SP)`; DBUF_$INIT 0xE3ABE8..0xE3ABF4 `lsr.l #8; lsr.l #2;
///     lsl.l #4; move.w D0,(0x91a,A5)`; MAC_OS_$SEND 0xE0B650..0xE0B658
///     `move.w (-62,A6),D0; ext.l D0; add.l (A1),D0; move.w
///     D0,(-62,A6)`).  A call in it is refused (no site catalogued; a
///     call alone, in an assignment, is [`store_narrow`]'s).
fn assign_convert(p: &Program, e: &SExpr, v: Expr, ty: Scalar) -> Option<Expr> {
    let (from, to) = (v.ty().bytes(), ty.bytes());
    if from == to {
        return Some(v);
    }
    if !p.types.is_integer(e.ty) {
        return None;
    }
    if from < to {
        return Some(convert(v, ty));
    }
    if let Some(n) = narrow(&v, ty) {
        return Some(n);
    }
    if from != 2 * to {
        return None;
    }
    let ok = match &v {
        Expr::Load { addr, .. } => !matches!(addr, Addr::Volatile { .. }) && plain_designator(e),
        Expr::Binary { .. } | Expr::Unary { .. } | Expr::Convert { .. } => {
            let mut calls = Vec::new();
            expr_calls(&v, &mut calls);
            calls.is_empty()
        }
        _ => false,
    };
    ok.then(|| Expr::Convert {
        value: Box::new(v),
        ty,
        in_place: false,
    })
}

/// The narrowings of an assignment `d := e` beyond [`assign_convert`]'s
/// (bead domain-pascal-d59a; codegen-patterns 8, "Narrowing and
/// widening"), for a store only: `Some(Ok(v))` the value to store,
/// `Some(Err(why))` a refusal naming the class, `None` when the value is
/// none of these (the caller's general refusal).
///
/// - An element (a constant or a run-time index), a pointer's target, a
///   `with` record, a packed record's whole field (or a field of these),
///   one step narrower (a longword into a word, a word into a byte): its
///   low bytes read in place, as a variable's.  4 to 2: MMAP's
///   module-local 0xE0C5AE at 0xE0C602 `move.w (0x6,A0),(-0x1ffa,A2)`
///   (`vpn_array[2]`, an `in` parameter's element), 0xE0C636 and 0xE0C640
///   `move.w (0x2,A1),..`, `move.w (-0x6,A1),..` (elements through the
///   loop's stepped pointer), 0xE0C664 `move.w (-0x6,A0,D0.w),..` (a
///   run-time index); NETBUF_$ADD_PAGES 0xE0EAA8 `move.w (-0x20e,A2),..`
///   (a frame array's element); BAT_$ALLOC_FM 0xE3AD9C `move.w
///   (-0x206,A0),D4` (into a local held in D4); MMAP_$FREE_PAGES 0xE0CFCC
///   `move.w (0xe,A0),(-0x1ffa,A2)` (a `with` record's longword).  2 to
///   1: RINGLOG_$LOGIT 0xE1A3A0, 0xE1A3A6, 0xE1A3E6 `move.b
///   (0x39,A3),(0x3,A2)` (fields of an `in` record, the first two of a
///   packed one) and 0xE1A3F4 `move.b (0x1f,A3,D1.w),(0x3,A2)` (an
///   element at a run-time index); RING's module-local 0xE76048 at
///   0xE760F4 `move.b (0x33,A3),(0x4,A0)` (a pointer's target).  A
///   [`Expr::Convert`] of the load marked `in_place` (bead
///   domain-pascal-dkik): the target reads the low-order bytes where
///   they are (`chr` reads a computed-index element and a longword
///   whole).  **Against** (unexplained, one site): AST_$TOUCH_AREA
///   0xE03974 `move.l (-0x4,A4),D0; move.w D0,(0x2,A2)`.
/// - A function's result one step narrower: its result register at the
///   narrow width.  2 to 1: RIP_$TABLE_D 0xE68F24 `move.b D0,(0xe,A4)`
///   and RIP_$TABLE 0xE68FEC `move.b D0,(0xc,A4)` after
///   ROUTE_$FIND_PORT; RING_$SENDP 0xE75A12 `move.b D0,(0xd,A0)` after
///   HDR_CHKSUM.  4 to 2: NETWORK 0xE0F526, 0xE115BC, 0xE1194E
///   `move.w D0,(0x344,A5)` after MMAP_$REMOTE_POOL (0xE59110 `move.l
///   (0x8,A6),D0`, a longword result).
/// - A longword value parameter's frame slot (a field of one) into a
///   byte: its low byte in place, DISK_IO 0xE3D630 `move.b
///   (0x13,A6),(0x6,A3)` (a longword by value at 0x10), marked `in_place`
///   as above (one site; the other nine frame slots read as a longword at
///   d and a byte at d+3 are packed fields: VTOC_$ALLOCATE 0xE38C72,
///   VTOCE_$READ 0xE39654, FM_$READ 0xE3A33E, SMD_$BLT 0xE6ED52).
///
/// Refused: a volatile object (an access of another size, ref
/// 3.15.1-3.15.3); a longword element, pointer's target, parameter
/// passed by address, local, unit or enclosing routine's variable or
/// function result into a byte (not catalogued).
fn store_narrow(p: &Program, e: &SExpr, v: Expr, ty: Scalar) -> Option<Result<Expr, String>> {
    let (from, to) = (v.ty().bytes(), ty.bytes());
    if from <= to || !p.types.is_integer(e.ty) {
        return None;
    }
    let narrowed = |v: Expr, in_place: bool| Expr::Convert {
        value: Box::new(v),
        ty,
        in_place,
    };
    match &v {
        Expr::Load {
            addr: Addr::Volatile { .. },
            ..
        } => Some(Err(format!(
            "narrowing a volatile or device object ({from} bytes into {to}: an access of \
             another size, ref 3.15.1-3.15.3) is"
        ))),
        Expr::Load { .. } if from == 2 * to => Some(Ok(narrowed(v, true))),
        Expr::Load { addr, .. } if from == 4 && to == 1 => {
            if !plain_designator(e) || matches!(addr, Addr::Index { .. }) {
                Some(Err(
                    "narrowing a longword element, pointer's target or `with` record to a byte \
                     (not catalogued; a variable's is DISK_IO 0xE3D630, codegen-patterns 8) is"
                        .to_string(),
                ))
            } else if matches!(addr, Addr::Indirect { .. }) {
                Some(Err(
                    "narrowing a longword parameter passed by address (or a field of one) to a \
                     byte (not catalogued; a value parameter's slot is DISK_IO 0xE3D630, \
                     codegen-patterns 8) is"
                        .to_string(),
                ))
            } else if !matches!(addr, Addr::Frame(o) if *o > 0) {
                Some(Err(
                    "narrowing a longword local, unit or enclosing routine's variable to a byte \
                     (not catalogued; the one site is a value parameter's slot, DISK_IO \
                     0xE3D630, codegen-patterns 8) is"
                        .to_string(),
                ))
            } else {
                Some(Ok(narrowed(v, true)))
            }
        }
        Expr::Call { .. } if from == 2 * to => Some(Ok(narrowed(v, false))),
        Expr::Call { .. } => Some(Err(
            "narrowing a longword function result to a byte (not catalogued, codegen-patterns \
             8) is"
                .to_string(),
        )),
        _ => None,
    }
}

/// Whether `if cond then target := value` (no `else`) is a select of the
/// shape `if a < b then a := b` whose assignment narrows `value` by
/// [`store_narrow`]: `cond` compares `target` with `value` (either way
/// round, widened).  The image computes such a select at the wider width
/// in the target's own register and stores the whole longword into it:
/// XNS_ERROR_$SEND's nested routine 0xE17960 at 0xE179F6..0xE17A04
/// `move.w (-0x36,A2),D5; ext.l D5; cmp.l (A3),D5; ble.b; move.l (A3),D5`
/// (`chunk := remaining; if chunk > node^.length then chunk :=
/// node^.length`, chunk a word held in D5), 0xE179B2..0xE179C2 the same
/// through A0; MMAP_$ALLOC_FREE 0xE0D8A6's `min` widens its register in
/// place too (bead domain-pascal-dzad).  A narrowing assignment with
/// another statement in the `then` part is the plain one: BAT_$ALLOC_FM
/// 0xE3AD92..0xE3AD9C `move.w D4,D0; ext.l D0; cmp.l (-0x208,A0),D0;
/// bge.b; move.w (-0x206,A0),D4`.
fn narrowing_select(p: &Program, cond: &SExpr, target: &SExpr, value: &SExpr) -> bool {
    if value.value.is_some() || !p.types.is_integer(value.ty) || !p.types.is_integer(target.ty) {
        return false;
    }
    if p.types.size(value.ty) <= p.types.size(target.ty) {
        return false;
    }
    let (Ok(tv), Ok(vv), Ok(c)) = (value_of(p, target), value_of(p, value), cond_of(p, cond))
    else {
        return false;
    };
    let Ok(ty) = scalar_of(p, target) else {
        return false;
    };
    if assign_convert(p, value, vv.clone(), ty).is_some()
        || !matches!(store_narrow(p, value, vv.clone(), ty), Some(Ok(_)))
    {
        return false;
    }
    let Cond::Compare { lhs, rhs, .. } = c else {
        return false;
    };
    let strip = |e: &Expr| match e {
        Expr::Convert { value, .. } => (**value).clone(),
        e => e.clone(),
    };
    let (l, r) = (strip(&lhs), strip(&rhs));
    (l == tv && r == vv) || (l == vv && r == tv)
}

/// `s` itself, or the one statement of a compound `s` (nested, with
/// empty statements around it, `begin x := y; end`); `None` for a
/// compound of none or several ([`narrowing_select`]).
fn sole_stmt(s: &SStmt) -> Option<&SStmt> {
    match &s.kind {
        SStmtKind::Compound(b) => {
            let mut it = b
                .stmts
                .iter()
                .filter(|t| !matches!(t.kind, SStmtKind::Empty));
            match (it.next(), it.next()) {
                (Some(t), None) => sole_stmt(t),
                _ => None,
            }
        }
        _ => Some(s),
    }
}

/// A variable or parameter, or a field of one (through any number of
/// fields): not an element, a pointer's target or a `with` record.
fn plain_designator(e: &SExpr) -> bool {
    match &e.kind {
        ExprKind::Var { .. } => true,
        ExprKind::Field {
            base, bits: None, ..
        } => plain_designator(base),
        _ => false,
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

/// Whether `d := s` copies an array of one-byte elements from a variable
/// as a block copy ([`copy`], [`byte_copy`]) whatever its size, where the
/// caller's [`is_block`] would make one of 1, 2 or 4 bytes a single move:
/// the image copies a 4-byte `cal_$tz_name_t` with four `move.b
/// (A0)+,(A1)+` (CAL_$READ_TIMEZONE 0xE3E592..0xE3E5A0,
/// CAL_$WRITE_TIMEZONE 0xE3E68C..0xE3E69A), not one `move.l`.
fn byte_array_copy(p: &Program, target: &SExpr, value: &SExpr) -> bool {
    byte_copy(p, target.ty) != ByteCopy::No && value.is_designator() && value.value.is_none()
}

/// `d := s` for a record or array `d` that is not scalar-sized (ref
/// "Assignment", p. 4-25): a block copy of the type's size.  The value
/// must be a designator; how the original assigned a constant or a
/// function result to a record has not been catalogued.  The selector
/// decides the instructions (codegen-patterns 5.2).
fn copy(p: &Program, s: &SStmt, target: &SExpr, value: &SExpr) -> Res<Stmt> {
    // A record function result whose convention is not known: said so.
    if let ExprKind::Call(c) = &value.kind {
        refuse_unknown_result(p, c, value.span)?;
    }
    let bytes = p.types.size(target.ty);
    if let (Some(ConstValue::Str(text)), ByteCopy::Chars) = (&value.value, byte_copy(p, target.ty))
    {
        if text.len() as u32 <= bytes
            && volatile_access(p, target).is_none()
            && !p.types.has_volatile_part(target.ty)
        {
            return string_assign(p, s, target, text, bytes);
        }
    }
    if !value.is_designator() || value.value.is_some() {
        if value.value.is_none() && in_place(value) {
            // A record or array selected from a type transfer of a
            // variable (`T(v).f`, ref p. 4-197): its bytes in place, as
            // a scalar field of one is read ([`transfer_view`],
            // codegen-patterns 5.2, "Fields of a type-transferred
            // variable"), copied as a block.  Best effort (not
            // byte-identical; bead domain-pascal-uaq7): no such copy is
            // catalogued (REM_FILE_$CREATE_TYPE's `rem_file_$loc_view_t(
            // loc).addr`, RING_$OPEN_OS's `ring_$pkt_range_t(...)`).
            best_effort(
                value.span,
                "assigning a record or array that is not a variable is",
            )?;
        } else {
            return Err(unsupported(
                value.span,
                "assigning a record or array that is not a variable is",
            ));
        }
    }
    let bytes = if p.types.size(value.ty) == bytes {
        bytes
    } else if p.types.canon(value.ty) == p.types.canon(target.ty)
        && p.types.size(value.ty).min(bytes) == 1
    {
        // The same packed record with and without a `[byte]` size
        // attribute (ref 3.15.5; `dpc-sema` `sized`, bead
        // domain-pascal-7kpw): the one byte is copied, the 2-byte side's
        // other byte left alone (SAU9 SCSI_$C90_INT 0x3C4A57A4 `move.b
        // (16,A0),(-72,A6)` and 0x3C4A57AA `move.b (20,A0),(-68,A6)`, the
        // 53C90's status and interrupt registers into locals the code
        // then tests as words, 0x3C4A57B0 `move.w (-68,A6),D1; btst
        // #8,D1`; Verified at those sites under ../source's reading of
        // the declarations).
        1
    } else {
        return Err(unsupported(
            value.span,
            "assigning a record or array of another size is",
        ));
    };
    // A side with volatile or device parts (ref 3.15.1-3.15.3) is copied
    // with the ordinary block copy's moves, its address wrapped in
    // [`Addr::Volatile`] so that the selector marks each move as a
    // volatile reference ([`copy_side`]).  The sau5 image copies the
    // `[device]` WIN_ window's 12-byte descriptor as three `move.l
    // (An)+,(Am)+` each way (RESTORE 0xE19A1E..0xE19A2A and
    // 0xE19A50..0xE19A5C, SEEK 0xE19B14..0xE19B20 and 0xE19B50..0xE19B5C),
    // its 2-byte ECC pattern with one `move.w` (win_$ecc_correct
    // 0xE19898), as any record (codegen-patterns 5.5).  A part that denies
    // the access (`device(read)` for the destination, `device(write)` for
    // the source) is an error ([`check_device_access`]).
    check_device_access(p, target, false)?;
    check_device_access(p, value, true)?;
    let chars = match (byte_copy(p, target.ty), byte_copy(p, value.ty)) {
        (ByteCopy::Chars, ByteCopy::Chars) => true,
        (ByteCopy::Unsettled(what), _) | (_, ByteCopy::Unsettled(what)) => {
            best_effort(
                s.span,
                format!(
                    "assigning {what} (the original copies an array of char byte by byte, a \
                     record by longwords; how it copies this is not seen, codegen-patterns 5.2 \
                     \"Arrays of char\", bead domain-pascal-4lf1) is"
                ),
            )?;
            best_effort_chars(bytes)
        }
        (ByteCopy::Chars, ByteCopy::No) | (ByteCopy::No, ByteCopy::Chars) => {
            best_effort(
                s.span,
                "assigning between an array of char and a record or array of another kind (which \
                 copy the original makes, by bytes or by longwords, is not seen; bead \
                 domain-pascal-4lf1) is",
            )?;
            best_effort_chars(bytes)
        }
        (ByteCopy::No, ByteCopy::No) => false,
    };
    // A byte-aligned array may lie at an odd address; the block copy
    // moves words and longwords.  An array of char is moved byte by byte.
    if !chars {
        check_even(p, target)?;
        check_even(p, value)?;
    }
    Ok(Stmt {
        kind: StmtKind::Copy {
            dst: copy_side(p, target)?,
            src: copy_side(p, value)?,
            bytes,
            chars,
        },
        span: s.span,
    })
}

/// The address of one side `e` of a record or array copy ([`copy`]),
/// wrapped in [`Addr::Volatile`] when `e` is volatile or has a volatile
/// or device part ([`wrap_volatile`]).
fn copy_side(p: &Program, e: &SExpr) -> Res<Addr> {
    Ok(wrap_volatile(p, e, addr_of(p, e)?))
}

/// `d := 'text'` for an unpacked array of char `d` of `bytes` bytes that
/// is not scalar-sized ([`copy`]): the characters, then blanks to the
/// array's end (ref "Variable-Length String Operations", p. 4-204: for
/// arrays of char "unassigned chars are padded with spaces").  The
/// image stores the text and fills the rest with longwords of blanks,
/// NAME_$INIT 0xE31738..0xE3174A `lea (A5),A0; move.l #0x2f636f6d,(A0)+;
/// move.l #0x20202020,D1; moveq #0x3e,D0; move.l D1,(A0)+; dbf D0`
/// (`path_buf := '/com'`, 256 bytes).  Best effort (not byte-identical;
/// bead domain-pascal-uaq7): a byte copy from a code-section cell of the
/// padded text ([`StmtKind::Copy`] from [`Addr::Const`]).
fn string_assign(p: &Program, s: &SStmt, target: &SExpr, text: &[u8], bytes: u32) -> Res<Stmt> {
    best_effort(
        s.span,
        "assigning a string constant to an array of char longer than a longword (NAME_$INIT \
         0xE31738 stores the text and fills the blanks by longwords; not modelled, \
         codegen-patterns 5.2) is",
    )?;
    let mut cell = text.to_vec();
    cell.resize(bytes as usize, b' ');
    if cell.len() % 2 == 1 {
        cell.push(0);
    }
    Ok(Stmt {
        kind: StmtKind::Copy {
            dst: addr_of(p, target)?,
            src: Addr::Const(PoolConst {
                bytes: cell,
                key: target.ty.0,
                symbol: None,
            }),
            bytes,
            chars: true,
        },
        span: s.span,
    })
}

/// How the original copies an object of a type, as far as its element
/// type decides ([`byte_copy`]; codegen-patterns 5.2, "Arrays of char").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ByteCopy {
    /// Not an array of one-byte elements: a record (whatever its fields:
    /// CAL_$WRITE_TIMEZONE 0xE3E626 copies a `cal_$timezone_rec_t`, a
    /// packed record with a `cal_$tz_name_t` field, by longwords) or an
    /// array of larger elements.
    No,
    /// An unpacked array of `char`, one dimension: copied byte by byte
    /// ([`StmtKind::Copy`]'s `chars`; CAL_$WRITE_TIMEZONE 0xE3E694 four
    /// `move.b (A0)+,(A1)+` for a `cal_$tz_name_t`, Apollo's
    /// /sys/ins/cal.ins.pas `array [1..4] of char`).
    Chars,
    /// An array whose copy is not seen: what it is.
    Unsettled(&'static str),
}

/// [`ByteCopy`] of type `ty`.  The kinds with one-byte elements other
/// than an unpacked array of `char` are not seen in the image with a
/// known type (no Apollo interface declares one that a kernel routine
/// copies), so how the original copies them, by bytes as for char or by
/// longwords as for a record, is not settled.
fn byte_copy(p: &Program, ty: TypeId) -> ByteCopy {
    let TypeKind::Array { elem, packed, .. } = p.types.kind(ty) else {
        return ByteCopy::No;
    };
    match p.types.kind(*elem) {
        TypeKind::Char if *packed => ByteCopy::Unsettled("a packed array of char"),
        TypeKind::Char => ByteCopy::Chars,
        _ if p.types.is_char(*elem) => ByteCopy::Unsettled("an array of a subrange of char"),
        TypeKind::Array { .. } if byte_copy(p, *elem) != ByteCopy::No => {
            ByteCopy::Unsettled("an array of arrays of char or of one-byte elements")
        }
        _ if p.types.size(*elem) == 1 => ByteCopy::Unsettled(
            "an array of one-byte elements other than char (boolean, a `[byte]` subrange)",
        ),
        _ => ByteCopy::No,
    }
}

/// The code-section cell of constant `v` passed by address as type `ty`
/// (codegen-patterns 1.5): the value big-endian in the type's size, a
/// byte in the high byte of a word (`ff00` for `true`: 17 sites name an
/// earlier `ff00` cell, e.g. PROC2_$COMPLETE_VFORK's 0xE735F0; ref 3.5.3
/// p. 3-11).  Scalars of 1, 2 or 4 bytes, and a string constant as its
/// own type (only for a `univ` parameter, [`univ_const_type`]): its
/// characters, an odd count followed by one zero byte (codegen-patterns
/// 1.5, "To a `univ` parameter": 43 of the 101 string cells
/// `tools/pcsurvey.py` finds end in a single `00`, the other 58 in a
/// character, none in `00 00`; 0xE2FFD4, 37 characters and `00`); a
/// string constant as an unpacked array of char, padded with blanks to
/// its size (below).  Sets other than of a word or a longword, records,
/// strings as another type are not supported yet.  `nil` is a longword 0
/// cell keyed as an integer32 0 (below).
fn pool_const(p: &Program, v: &ConstValue, ty: TypeId, span: Span) -> Res<PoolConst> {
    if let (ConstValue::Str(s), TypeKind::StringLit { .. }) = (v, p.types.kind(ty)) {
        let mut bytes = s.clone();
        if bytes.len() % 2 == 1 {
            bytes.push(0);
        }
        return Ok(PoolConst {
            bytes,
            key: ty.0,
            symbol: None,
        });
    }
    let size = p.types.size(ty);
    if let (ConstValue::Str(s), ByteCopy::Chars) = (v, byte_copy(p, ty)) {
        // A string constant as an unpacked array of char that is not
        // `univ` (bead domain-pascal-jgtd): the value an assignment to a
        // variable of the parameter's type gives (ref "Passing
        // Variable-Length Parameters", p. 4-207: "the same as if the
        // argument were being assigned to a variable of the parameter's
        // type", said of varying formals and applied here by analogy: the
        // image settles it, below; for an array of char "unassigned chars are padded with
        // spaces", "Variable-Length String Operations", p. 4-204), so the
        // characters and blanks to the type's size, an odd size followed
        // by one zero byte as a string's own cell (above).  SAU5
        // TAPE_$BOOT's tape_$check_block 0xE33F12 `pea (0x8c,PC)` names
        // 0xE33FA0..0xE34068, 80 characters and 120 blanks for an `in
        // vfmt_$string_t` (200 bytes); one of the formal's own size is
        // its characters, the cell 0xE346DE that TAPE_$BOOT names for the
        // three calls with a 16-character `in` array of char (0xE344C0,
        // 0xE344D4, 0xE344E4: one cell, shared as any equal constant).
        // Keyed by the parameter's type (Uncertain whether a cell of
        // equal bytes for a `univ` parameter, keyed by the string's own
        // type, would be shared: no site).
        if s.len() as u32 <= size {
            let mut bytes = s.clone();
            bytes.resize(size as usize, b' ');
            if bytes.len() % 2 == 1 {
                bytes.push(0);
            }
            return Ok(PoolConst {
                bytes,
                key: ty.0,
                symbol: None,
            });
        }
    }
    if let (ConstValue::Set(_), TypeKind::Set { .. }) = (v, p.types.kind(ty)) {
        // A set constant: its bit mask big-endian in the set's size (ref
        // 3.9.3 p. 3-16, Figure 3-6), in the size group of a scalar cell
        // of that size (codegen-patterns 1.5, "Set constants"; bead
        // domain-pascal-2i6f).  Word sets: TERM_$READ 0xE66864/0xE66870
        // `pea (d,PC)` of 0xE66896 `0001` ([0]) and 0xE66898 `0000` ([]),
        // TTY_$K_GET's options; ROUTE_$SHUTDOWN 0xE6A61A/0xE6A63A, 0xE6A65A
        // `0008` ([3]) and 0xE6A65C `0002` ([1]); NET_IO_$CLEANUP_NIL
        // 0xE74F0A, 0xE74F1C `0008`.  Longword sets: TERM_$INQUIRE's twelve
        // one-member masks for SIO_$K_INQ_PARAM, 0xE671AC `0000 0001` ([0])
        // .. 0xE671D8 `0000 0010` ([4]), first named at 0xE66FAE (../source's
        // reading of the types).  A one-byte set (`[byte] set of 0..7`)
        // and sets of more than a longword: no site known.
        let width = match Width::of_size(size) {
            Some(w @ (Width::Word | Width::Long)) => w,
            _ => {
                return Err(unsupported(
                    span,
                    format!("a {size}-byte set constant passed by address (no site known) is"),
                ))
            }
        };
        let Expr::Const { value, .. } = const_of(v, Scalar::unsigned(width), span)? else {
            unreachable!("const_of gives a constant")
        };
        let be = (value as u32).to_be_bytes();
        let bytes = match width {
            Width::Word => be[2..].to_vec(),
            _ => be.to_vec(),
        };
        return Ok(PoolConst {
            bytes,
            key: ty.0,
            symbol: None,
        });
    }
    if *v == ConstValue::Nil && size == 4 {
        // `nil` by address (bead domain-pascal-dtbg): a longword 0 cell.
        // SAU5 PBU: PBU_$RELEASE_INT 0xE59498, PBU_$SHUTDOWN 0xE59BE0
        // and PBU_$FREE_ASID 0xE5A77A pass `nil` to PBU_$SETDOWN with
        // `pea (d,PC)` of one cell, 0xE594B0 `0000 0000`, placed after
        // PBU_$RELEASE_INT (its first user) behind a `0x2048` filler.
        // Keyed as integer32 0, as a pointer constant is keyed as its
        // integer32 (below, Verified at OS_$INIT), so a module's `nil` and
        // integer32 0 by address share one cell (Uncertain for `nil`: no
        // module is known to pass it and another longword 0 by address;
        // codegen-patterns 1.5).
        return Ok(PoolConst {
            bytes: vec![0; 4],
            key: p.types.builtins.integer32.0,
            symbol: None,
        });
    }
    let width = match Width::of_size(size) {
        Some(w) if v.ordinal().is_some() => w,
        _ => {
            return Err(unsupported(
                span,
                match v {
                    ConstValue::Set(_) => {
                        format!("a {size}-byte set constant passed by address is")
                    }
                    ConstValue::Str(_) => {
                        format!("a string constant passed by address (as a {size}-byte object) is")
                    }
                    _ => format!("a {size}-byte constant passed by address is"),
                },
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
    // Every 4-byte integer cell is keyed as integer32, whatever the
    // integer type it is held as, so equal values share: the module-local
    // 0xE2FF7C's 0 for VFMT_$WRITE2's `a2`/`a1` (Apollo's
    // `vfmt_$generic_signed_arg`, an `[aligned(0)]` integer32) is the cell
    // 0xE2FFFC that LOG_$INIT 0xE30214 names for LOG_$ADD's `univ` data.
    // A pointer constant (`p(n)`, a type transfer of an integer constant)
    // is keyed so too (bead domain-pascal-dtbg): SAU2 OS_$INIT 0xE34130
    // passes `univ_ptr(16#FC0000)` and 0xE3414E the integer32 16#FC0000
    // with `pea` of one cell, 0xE34AA0 `00fc 0000` (Verified;
    // codegen-patterns 1.5).  So `nil` (above) and `p(0)` are one cell.
    let key = if size == 4 && (p.types.is_integer(ty) || p.types.is_pointer(ty)) {
        p.types.builtins.integer32.0
    } else {
        ty.0
    };
    Ok(PoolConst {
        bytes,
        key,
        symbol: None,
    })
}

/// Whether `e` is `addr(x)` of a part of an `extern` variable (another
/// unit's object) or `addr(r)` of a routine, also through a type transfer
/// (`integer32(addr(x))`): the candidates for an address cell
/// ([`symbol_address`]; not an overlay section's variable, whose address
/// is a symbol too).
fn extern_address(p: &Program, e: &SExpr) -> bool {
    match &e.kind {
        ExprKind::RoutineAddr(_) => true,
        ExprKind::Transfer(x) => extern_address(p, x),
        ExprKind::Builtin {
            builtin: Builtin::Addr,
            args,
        } => root_var(&args[0]).is_some_and(|v| matches!(p.var(v).storage, Storage::Extern { .. })),
        _ => false,
    }
}

/// The symbol and offset whose address `e` is: `addr(x)` of another
/// unit's object or of a routine ([`Addr::Symbol`]), also through a type
/// transfer of the same width (`integer32(addr(x))`).  A link-time
/// constant, passed by address as a cell ([`PoolConst::address`]).
fn symbol_address(e: &Expr) -> Option<(String, i32)> {
    match e {
        Expr::AddrOf(Addr::Symbol { name, offset }) => Some((name.clone(), *offset)),
        Expr::Convert { value, ty, .. } if value.ty().width == ty.width => symbol_address(value),
        _ => None,
    }
}

/// The type of the cell of a constant of type `cty` passed by address to
/// a `univ` parameter of type `fty` (beads domain-pascal-4be, -9286,
/// -c5u).  Ref 5.3.3 p. 5-11 (checked in the PDF): "When you pass an
/// expression argument (as opposed to a variable argument) to a univ
/// parameter, Domain Pascal extends the expression to be the same size as
/// the univ parameter", with the message "Expression passed to UNIV
/// formal NAME was converted to NEWTYPE" (also diagnostic 246, Appendix
/// B).  The image does not extend a constant's *cell* to the formal's
/// size, nor cut it (codegen-patterns 1.5, "To a `univ` parameter"):
///
/// - Equal sizes: the constant's own type (the bytes are the same either
///   way; the cell's key stays the constant's type).
/// - An integer constant for a larger integer parameter: the value as the
///   parameter's type, i.e. sign-extended to its size (VFMT_$WRITE2's
///   `a2` and `a1`, Apollo's `univ` `[aligned(0)]` integer32, get a
///   4-byte cell for 0 at the module-local 0xE2FF7C, cell 0xE2FFFC, first
///   named for `a2` at 0xE2FF8A).  A 4-byte integer cell is keyed as
///   integer32 ([`pool_const`]).
/// - An integer constant 0 for a non-integer parameter of 4 bytes or more:
///   an integer32 cell, the constant's 32-bit representation (ref 3.2.3
///   p. 3-5: "Domain Pascal internally represents the value as a 32-bit
///   integer").  Verified: the 0 for a 1024-byte `univ pkt_$bytes_t`
///   (PKT_$SEND_INTERNET, PKT_$SAR_INTERNET, NET_IO_$SEND,
///   PKT_$BLD_INTERNET_HDR: cells 0xE12BB4, 0xE2FDEC, 0xE61D18, 0xE658CC,
///   0xE68E28, 0xE4A584, 0xE1050C, 0xE870D8, each `0000 0000`; three
///   after a `0x2048` filler, so in the longword group), LOG_$ADD's data
///   (0xE17608, 0xE2FFFC), XPD_$POST_EVENT's 4-byte `univ status_$t`
///   (0xE7508C, after the word cell `0003`).
/// - Any other integer constant for a 4-byte non-integer parameter: the
///   same integer32 cell, the value sign-extended to 32 bits, big-endian
///   (bead domain-pascal-45yc).  Ref 5.3.3 p. 5-11 extends the constant
///   to the parameter's 4 bytes, and 3.2.3 p. 3-5 makes it a 32-bit
///   integer: both give the integer32 value.  Verified in SAU5:
///   MBUS_$UNDEF_INT 0xE0AA10 `pea (0xe,PC); jsr CRASH_SYSTEM` (a `univ
///   status_$t` in ../source's reading), cell 0xE0AA20 `0000 002a`.  For
///   a larger parameter (a 1024-byte buffer) only 0 is seen, and the
///   cell is 4 bytes there, not the parameter's size as 5.3.3 would
///   have it; another value is refused.
/// - A string constant: its own type, whatever the parameter's size
///   (0xE2FF7C's 37 and 32 characters for VFMT_$WRITE2's 200-character
///   format, cells 0xE2FFD4 and 0xE30000; its 36 characters for the
///   4-byte `a1`, 0xE30020; PCHIST's `'..'` for TERM_$WRITE, 0xE5CA56).
/// - A character constant for a larger non-integer parameter: its own
///   type, a word cell with the character in the high byte (Reported:
///   KBD_$INIT 0xE3338E, cell 0xE333DA `3200`, an array formal in
///   ../source's reading).
/// - A 4-byte integer constant for a non-integer parameter of 1 to 3
///   bytes: its own type, a 4-byte cell (PROC2's 0xE419E8 and 0xE41A7C,
///   cells 0xE41A1C and 0xE41ABC, for XPD_$WRITE's one-byte buffer).
/// - Anything else is not supported yet (no site known): an integer
///   constant larger than an integer parameter, a 1- or 2-byte integer
///   constant for a non-integer parameter of 1 to 3 bytes other than its
///   own size, a nonzero integer constant for a non-integer parameter of
///   more than 4 bytes, a character for a larger integer parameter, a
///   boolean, enumeration or set constant of another size than the
///   parameter.
fn univ_const_type(
    p: &Program,
    v: &ConstValue,
    cty: TypeId,
    fty: TypeId,
    span: Span,
) -> Res<TypeId> {
    let (cs, fs) = (p.types.size(cty), p.types.size(fty));
    let int = p.types.is_integer(cty);
    let fint = p.types.is_integer(fty);
    if cs == 0 {
        // `''`, a cell of no bytes: DIR_$DO_OP 0xE4C310 `pea (0x6d2,PC)`
        // for dir_$do_op_add_entry's link text, the cell 0xE4C9E4 after
        // the routine's `rts`, where the next routine's `link` follows.
        return Ok(cty);
    }
    if cs == fs || matches!(p.types.kind(cty), TypeKind::StringLit { .. }) {
        Ok(cty)
    } else if cs < fs && int && fint {
        Ok(fty)
    } else if cs <= 4
        && v.ordinal() != Some(0)
        && matches!(p.types.kind(fty), TypeKind::Real { .. })
        && int
    {
        // A real formal: whether Apollo converted the value (7 to 7.0,
        // 0x40E00000) or passed its integer bits is not shown by the image,
        // and reals are out of scope (CLAUDE.md 1); 0 is the same bytes
        // either way.
        Err(unsupported(
            span,
            "a nonzero integer constant for a real `univ` parameter (the value converted or \
             its bits: not settled) is"
                .to_string(),
        ))
    } else if cs <= 4 && (fs == 4 || fs > 4 && v.ordinal() == Some(0)) && int {
        // The integer32 value (ref 5.3.3 p. 5-11; 3.2.3 p. 3-5): SAU5
        // MBUS_$UNDEF_INT's `0000 002a` for CRASH_SYSTEM's 4-byte status;
        // the 0 for a 1024-byte buffer is a 4-byte cell too.
        Ok(p.types.builtins.integer32)
    } else if cs <= 4 && fs > 4 && int {
        // Only 0 is seen for a parameter larger than 4 bytes, in a 4-byte
        // cell, which is not the extension 5.3.3 describes; another value
        // is not settled.
        Err(unsupported(
            span,
            format!(
                "a nonzero integer constant for a {fs}-byte non-integer `univ` parameter (only 0 \
                 is seen, codegen-patterns 1.5) is"
            ),
        ))
    } else if cs < fs && p.types.is_char(cty) && !fint {
        Ok(cty)
    } else if cs == 4 && cs > fs && int && !fint {
        // A 4-byte integer constant for a smaller non-integer parameter: its
        // own type, not cut (PROC2's debug_setup_internal 0xE419E8 passes
        // `integer32(-1)` to XPD_$WRITE's one-byte `univ xpd_$bytes_t`
        // buffer as the 4-byte cell 0xE41A1C `ffff ffff`, its
        // debug_clear_internal 0xE41A7C `integer32(0)` as 0xE41ABC `0000
        // 0000`).
        Ok(cty)
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

/// The constant `v` stored into an object of type `ty` (`store`, the
/// `Fill` of a `for` loop): as [`const_of`], and `nil` too, the pointer 0
/// (ref 3.13.4 p. 3-51), which the selector stores as any longword 0,
/// `clr.l <ea>` (codegen-patterns 13, "Constants stored": no `move.l
/// #0,<mem>` in compiled code; `nil` stored at BAT_$ALLOC_VTOCE 0xE3AEDA
/// `clr.l (-0x14,A6)`, BAT_$ALLOCATE 0xE3B2EE `clr.l (0xd38,A5)`, HINT_$INIT
/// 0xE3139A `clr.l abs.l`, PKT_$SEND_INTERNET 0xE12670 `clr.l (-0x10,A6)`).
fn stored_const(v: &ConstValue, ty: Scalar, span: Span) -> Res<Expr> {
    match v {
        ConstValue::Nil if ty.pointer => Ok(Expr::Const { value: 0, ty }),
        _ => const_of(v, ty, span),
    }
}

/// The constant `v` as a value of IR type `ty`.  A boolean is its byte:
/// 0xFF for true, read as the signed byte -1 (ref 3.5.3, p. 3-11: "The
/// system sets all eight bits to 1 for true and sets all eight bits to 0
/// for false").
/// `nil` and strings are refused here; [`stored_const`] takes `nil`
/// stored into a pointer.
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
        ConstValue::Nil => {
            return Err(unsupported(
                span,
                format!("`nil` as a {}-byte scalar value is", ty.bytes()),
            ))
        }
        ConstValue::Str(s) => {
            return Err(unsupported(
                span,
                format!(
                    "a {}-character string constant as a {}-byte scalar value is",
                    s.len(),
                    ty.bytes()
                ),
            ))
        }
        _ => v.ordinal().expect("an ordinal constant"),
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
            if v.ty().width == Width::Byte && ty.width == Width::Word {
                // A one-byte set (`[byte] set of 0..7`) used as a word
                // set: zero-extended, `clr.w Dn; move.b <ea>,Dn` (member
                // k stays bit k; codegen-patterns 5.1, "One-byte sets"):
                // assigned, ACL_$MIN_RIGHTS 0xE4691C (`rights :=
                // world_rights`, held in D2) and acl_$prim_create_
                // internal 0xE45850 (D3); a `-` operand, 0xE4692A `clr.w
                // D3; move.b (-0x14,A6),D3; not.w D3; and.w D3,D2`; a `+`
                // operand, acl_$eval_acl_entries 0xE462C0 `...; or.w
                // D5,D1`; a `*` operand, 0xE46222 `clr.w D0; move.b
                // (0x1c,A2),D0; and.w (0x1a,A0),D0`; compared with `[]`,
                // 0xE45270 `clr.w D1; move.b (27,A4),D1; tst.w D1`; a
                // word value argument, 0xE45360 `clr.w D0; move.b
                // (0x18,A4),D0; move.w D0,-(SP)`.  Into a longword set:
                // no site, refused below.
                return Ok(convert(v, ty));
            }
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

/// `lhs in rhs` for a set of one word or one longword (`set_width`),
/// tested in a register ([`Expr::SetMember`]; codegen-patterns 5.1).
///
/// A set variable, or a constant set of a longword: the element as it is
/// (TTY_$K_SET_DELAY 0xE67A18 `move.w (A2),D0; moveq #0x1f,D1; btst
/// D0,D1`; PROC1_$TST_LOCK 0xE148D4), a `0..65535` one too (no site
/// shows one widened for a longword set).
///
/// A constant set of one word with a run-time element (codegen-patterns
/// 5.1, "`x in c`, a constant register-sized set"; `tools/setsurvey.py
/// summary`, kind `vtest`):
/// - a `0..65535` element (a word whose type's host is `integer32`,
///   Apollo's `pinteger`) of an integer constructor: zero-extended, the
///   constant a longword.  Image: the 10 `vol_idx in [1..10]` tests of
///   DISK_, AS_IO_SETUP 0xE6B75C..0xE6B76A `clr.l D0; move.l #0x7fe,D1;
///   move.w (A0),D0; btst D0,D1; bne.b` (`dpc-sema` types `[1..10]` `set
///   of 0..10`, one word), the only zero-extended elements of a register
///   `btst` in compiled code; their volume index is compared unsigned in
///   DISK_IO 0xE3D528 and DISK_$READ_MULTI 0xE3CFF4.  **Uncertain:** the
///   element's Pascal type is not visible.  Refused for a constant of a
///   declared set type (`ws([1..10])`: not seen);
/// - any other word element: the constant stays a word; the selector
///   takes it only when `moveq` loads it (`move.w #0xfff` with an
///   `integer` element at FILE_$PRIV_LOCK 0xE5F110..0xE5F11A, against
///   the DISK_ `move.l #0x7fe`: not settled);
/// - a byte element (boolean, char, `[byte]`): refused (no site; a
///   boolean's true is 0xFF, ref 3.5.3, which `btst` would take as bit
///   31); a longword element: refused (no site).
fn set_member(p: &Program, lhs: &SExpr, rhs: &SExpr, set_width: Width) -> Res<Expr> {
    let set = value_of(p, rhs)?;
    let mut elem = value_of(p, lhs)?;
    let Expr::Const { value, .. } = set else {
        return Ok(Expr::SetMember {
            elem: Box::new(elem),
            set: Box::new(set),
            set_width,
        });
    };
    let runtime = lhs.value.is_none();
    // `abs` of a word (`0..32767`, unsigned in the IR): tested as a signed
    // word, an `integer`'s form; best effort, not checked against the
    // image (no `abs` set member seen; codegen-patterns 8, "abs").
    let abs = matches!(elem, Expr::Unary { op: IrUnOp::Abs, ty, .. } if ty.width == Width::Word);
    if abs {
        elem = reinterpret(elem, Scalar::signed(Width::Word));
    }
    // Only on a one-word constant: a longword constant keeps the element
    // as it is (no site shows a `0..65535` element widened for one).
    let wide = runtime
        && !abs
        && set_width == Width::Word
        && elem.ty().width == Width::Word
        && long_host_word(&p.types, lhs.ty);
    if set_width == Width::Word && runtime {
        match elem.ty().width {
            Width::Byte => {
                return Err(unsupported(
                    lhs.span,
                    "`in` on a one-word constant set with a one-byte element (boolean, char, \
                     `[byte]`: no site, codegen-patterns 5.1; a boolean's true is 0xFF) is",
                ))
            }
            Width::Long => {
                return Err(unsupported(
                    lhs.span,
                    "`in` on a one-word constant set with a 4-byte element (no site, \
                     codegen-patterns 5.1) is",
                ))
            }
            Width::Word => {}
        }
        if wide && !integer_constructor(p, rhs) {
            best_effort(
                lhs.span,
                "`in` on a one-word constant set of a declared type with a `0..65535` element \
                 (`move.l #c` seen for integer constructors only, DISK_ 0xE6B75E; \
                 codegen-patterns 5.1) is",
            )?;
        }
    }
    let (set, set_width) = if wide {
        elem = convert(elem, Scalar::unsigned(Width::Long));
        (
            Expr::Const {
                value,
                ty: Scalar::unsigned(Width::Long),
            },
            Width::Long,
        )
    } else {
        (set, set_width)
    };
    Ok(Expr::SetMember {
        elem: Box::new(elem),
        set: Box::new(set),
        set_width,
    })
}

/// A one-word set value `e` assigned to a one-byte set (codegen-patterns
/// 5.1, "One-byte sets"): its low byte, members 8..15 dropped (a `set of
/// 0..15` value with such members is outside the target's type; the
/// image does not check).  A function result, `move.b D0,<ea>`
/// (acl_$convert_image 0xE44F56 and acl_$expand_default_acl 0xE45A50,
/// `acl_$rights_t(acl_$convert_rights(...))` in ../source's reading); a
/// word in memory, its low byte read in place (acl_$find_acl_slot
/// 0xE45F24 `move.b (2061,A2),(27,A1)`, a unit array element's word field
/// at 0x80C, through a type transfer).  Refused: any other value (a
/// computed word set: no site), a volatile word, and a local or value
/// parameter of the routine (`Addr::Frame`: which of them the selector
/// holds in a register is decided later, and neither a held one nor one
/// in the frame has a site).
fn byte_set_narrow(p: &Program, e: &SExpr) -> Res<Expr> {
    let v = set_value(p, e, scalar_of(p, e)?)?;
    let byte = Scalar::unsigned(Width::Byte);
    let call = |x: &Expr| matches!(x, Expr::Call { .. });
    match &v {
        // Best effort (bead domain-pascal-5kgg): the word's low byte,
        // members 0..7, read whole (not in place: the local may be held in
        // a register).
        Expr::Load {
            addr: Addr::Frame(_),
            ..
        } => best_effort(
            e.span,
            "assigning a one-word set local or value parameter to a one-byte set (held in a \
             register or in the frame: no site; codegen-patterns 5.1, \"One-byte sets\") is",
        )
        .map(|()| Expr::Convert {
            value: Box::new(v.clone()),
            ty: byte,
            in_place: false,
        }),
        Expr::Load { addr, .. } if !matches!(addr, Addr::Volatile { .. }) => Ok(Expr::Convert {
            value: Box::new(v),
            ty: byte,
            in_place: true,
        }),
        x if call(x) || matches!(x, Expr::Convert { value, .. } if call(value)) => {
            Ok(Expr::Convert {
                value: Box::new(v),
                ty: byte,
                in_place: false,
            })
        }
        _ => best_effort(
            e.span,
            "assigning a one-word set that is not a function result or a word in memory to a \
             one-byte set (no site; codegen-patterns 5.1, \"One-byte sets\") is",
        )
        .map(|()| Expr::Convert {
            value: Box::new(v.clone()),
            ty: byte,
            in_place: false,
        }),
    }
}

/// `lhs in rhs` for a one-byte set (`[byte] set of 0..7`, ref 3.15.5
/// p. 3-61 and the owner's ruling, reference-clarifications "A size
/// attribute on a set type": member k is bit k of the byte): a constant
/// member, [`Expr::SetMember`] of byte width, which the selector tests
/// on the byte in memory (`btst #k,(d,An)`, the rule of codegen-patterns
/// 5.1 "`in` on sets in memory, constant member", whose last byte is here
/// the whole set).  Image: acl_$setids 0xE46B90 `btst.b #0,(-0x5d,A6)`
/// (`0 in attrs.obj_flags`), ACL_$MIN_RIGHTS 0xE46922 `btst.b
/// #4,(-0x14,A6)` (`4 in owner_rights`), APP_$RECEIVE's `btst.b
/// #3,(7,A3); sne` (`3 in hdr^.rh_flags_07`; ../source app/receive.pas).
/// A run-time member: the same node, which the selector tests with the
/// byte widened into a register (AREA's area_$free_slot_segments
/// 0xE08328..0xE0832E `clr.w D6; ...; move.b (A2),D6; btst.l D5,D6`), a
/// word element there.  Best effort (not byte-identical): a one-byte
/// element (char, `[byte]` subrange) is zero-extended to a word first, so
/// the register `btst` (bit mod 32) reads its value and not the bits
/// above it (no byte element of a byte set seen).
fn byte_set_member(p: &Program, lhs: &SExpr, rhs: &SExpr) -> Res<Expr> {
    if lhs.value.is_none() {
        best_effort(
            lhs.span,
            "`in` on a one-byte set with a run-time element (`clr.w; move.b; btst Dk,Dn` at \
             AREA's area_$free_slot_segments 0xE08328: not reproduced; codegen-patterns 5.1, \
             \"One-byte sets\") is",
        )?;
        if rhs.value.is_some() {
            return Err(unsupported(
                rhs.span,
                "`in` on a one-byte constant set (no site; codegen-patterns 5.1, \"One-byte \
                 sets\") is",
            ));
        }
        if p.types.is_boolean(lhs.ty) {
            // A boolean's true is 0xFF (ref 3.5.3), not its ordinal 1.
            return Err(unsupported(
                lhs.span,
                "`in` on a one-byte set with a boolean element is",
            ));
        }
        let mut elem = value_of(p, lhs)?;
        if elem.ty().width == Width::Byte {
            elem = convert(elem, Scalar::unsigned(Width::Word));
        }
        return Ok(Expr::SetMember {
            elem: Box::new(elem),
            set: Box::new(value_of(p, rhs)?),
            set_width: Width::Byte,
        });
    }
    if rhs.value.is_some() {
        return Err(unsupported(
            rhs.span,
            "`in` on a one-byte constant set (no site; codegen-patterns 5.1, \"One-byte \
             sets\") is",
        ));
    }
    Ok(Expr::SetMember {
        elem: Box::new(value_of(p, lhs)?),
        set: Box::new(value_of(p, rhs)?),
        set_width: Width::Byte,
    })
}

/// `lhs in rhs` for a set field `rhs` of a packed record that is a bit
/// field ([`bit_field`]; ref 3.10.7 Table 3-3, p. 3-36: one bit per
/// element, member k at bit k of the field's value).  Best effort (not
/// byte-identical; bead domain-pascal-eoc6; no site seen):
/// - a constant member k: the member's bit read as a one-bit boolean
///   field, `btst #b` on the byte that holds it (the boolean field's form,
///   0xE16F0C `btst #4,(19,A2); sne D0`; bit 7 `tst.b`, 0xE066B2); a
///   member outside the field's bits is false;
/// - a run-time member: the field's value extracted into a register
///   ([`Expr::Bits`]) and `btst Dk,Dn` on it, as a one-byte set's
///   (AREA's area_$free_slot_segments 0xE0832E `btst D5,D6`); a byte
///   element zero-extended to a word first.
fn bit_field_member(p: &Program, lhs: &SExpr, rhs: &SExpr) -> Res<Expr> {
    let (addr, field) = bit_field(p, rhs)?.expect("a bit field");
    check_device_access(p, rhs, true)?;
    let ty = bit_field_type(p, rhs)?;
    if let Some(k) = lhs.value.as_ref().and_then(|v| v.ordinal()) {
        if k < 0 || k >= i64::from(field.width) {
            return Ok(Expr::Const {
                value: 0,
                ty: Scalar::BOOLEAN,
            });
        }
        let u = field.shift + k as u32;
        let byte = field.unit.bytes() - 1 - u / 8;
        return Ok(Expr::Bits {
            addr: addr.offset_by(byte as i32),
            field: BitField {
                unit: Width::Byte,
                shift: u % 8,
                width: 1,
            },
            ty: Scalar::BOOLEAN,
        });
    }
    if p.types.is_boolean(lhs.ty) {
        // A boolean's true is 0xFF (ref 3.5.3), not its ordinal 1.
        return Err(unsupported(
            lhs.span,
            "`in` on a packed set field with a boolean element is",
        ));
    }
    let mut elem = value_of(p, lhs)?;
    if elem.ty().width == Width::Byte {
        elem = convert(elem, Scalar::unsigned(Width::Word));
    }
    Ok(Expr::SetMember {
        elem: Box::new(elem),
        set: Box::new(Expr::Bits { addr, field, ty }),
        set_width: ty.width,
    })
}

/// Whether `e` is a set constructor of integers as `dpc-sema` types it
/// (`set_cons`: `set of 0..hi`, `hi` its largest member), not a constant
/// of a declared set type (`ws([1..10])`, a named constant of one).
/// **Limit:** a declared `set of 0..hi` constant whose largest member is
/// `hi` cannot be told from a constructor.
fn integer_constructor(p: &Program, e: &SExpr) -> bool {
    if matches!(e.kind, ExprKind::Transfer(_)) {
        return false;
    }
    let (TypeKind::Set { elem }, Some(ConstValue::Set(members))) = (p.types.kind(e.ty), &e.value)
    else {
        return false;
    };
    match p.types.kind(*elem) {
        TypeKind::Subrange { base, lo: 0, hi } => {
            matches!(p.types.kind(*base), TypeKind::Integer16) && members.iter().max() == Some(hi)
        }
        _ => false,
    }
}

/// A 2-byte subrange whose host is `integer32` (`0..65535`).
fn long_host_word(types: &TypeTable, ty: TypeId) -> bool {
    types.size(ty) == 2
        && matches!(types.kind(ty), TypeKind::Subrange { base, .. }
            if matches!(types.kind(*base), TypeKind::Integer32))
}

/// A run-time set member as a word (ref 3.9.3: ordinals 0..1023): an
/// enumeration or integer16 value as it is, a char or boolean byte
/// zero-extended.
fn set_element(p: &Program, x: &SExpr) -> Res<Expr> {
    let v = value_of(p, x)?;
    match v.ty().width {
        // `abs` of a word (unsigned, codegen-patterns 8, "abs"): as a
        // signed word, an `integer`'s form (its values 0..32767 are the
        // same); best effort, not checked against the image (no `abs` set
        // member seen).
        Width::Word
            if matches!(
                v,
                Expr::Unary {
                    op: IrUnOp::Abs,
                    ..
                }
            ) =>
        {
            Ok(reinterpret(v, Scalar::signed(Width::Word)))
        }
        Width::Word => Ok(v),
        Width::Byte => Ok(convert(v, Scalar::unsigned(Width::Word))),
        // A longword expression whose low word [`narrow`] computes at the
        // word's width (ref 4.3.1, p. 4-5), the ordinal of a member
        // 0..1023: the bounds of SAU8 MST_$INIT's ranges, `pinteger`
        // sums, `move.w; add.w; subq.w #1` (0x3C33F0E..0x3C33F20,
        // 0x3C33F54..0x3C33F60; `region_$global_b + region_$global_b_size
        // - 1`, ../source mst/init.pas).
        Width::Long if narrow(&v, Scalar::signed(Width::Word)).is_some() => {
            Ok(narrow(&v, Scalar::signed(Width::Word)).expect("checked"))
        }
        // Best effort: the low word, the ordinal of a member 0..1023 (bead
        // domain-pascal-5kgg).
        Width::Long => best_effort(
            x.span,
            "a set member of a 4-byte type (narrowed to a word: not catalogued) is",
        )
        .map(|()| Expr::Convert {
            value: Box::new(v),
            ty: Scalar::unsigned(Width::Word),
            in_place: false,
        }),
    }
}

/// `lhs in rhs` for a set of `bytes` bytes, more than a register holds,
/// in memory ([`Expr::SetMemberAt`]; codegen-patterns 5.1: the byte
/// `(bits-1 - k) >> 3` and `btst k` on it, MST_$ALLOC_ASID 0xE42D5C for 64
/// bits, the TTY module-local 0xE1B740..0xE1B750 for a `set of char`).
///
/// The image tests unguarded the elements whose every value is a member
/// of the set's bits (ref 3.9.3, p. 3-16: one bit per ordinal
/// 0..bits-1; a `char` in a 256-bit set at 0xE1B750, an ASID in a 64-bit
/// one at 0xE42D64), and an `integer` element of a set variable too
/// (MSG_$FORK 0xE73F2E..0xE73F40 `movea.l D4,A2; moveq #63,D1; move.w
/// (A2),D5; sub.w D5,D1; lsr.w #3,D1; lea (0x1d8,A0),A3; btst
/// D5,(0,A3,D1.w)`, its `in parent_asid: integer`, ../source
/// msg/fork.pas), but guards the elements of constant sets with `bcs` on
/// the subtraction and branches with `ble`/`bgt` after the `btst`
/// (NAME_$INIT 0xE31836..0xE3184A on a 96-bit set; UNMAP_CASE
/// 0xE54166..0xE54178): a constant set of more than a longword (a cell
/// in the code, UNMAP_CASE's `lea (d,PC)`) is refused.  Refused also: an
/// element of another type that reaches beyond the set's bits (no
/// site).
fn set_member_at(p: &Program, e: &SExpr, lhs: &SExpr, rhs: &SExpr, bytes: u32) -> Res<Expr> {
    if let (Some(ConstValue::Set(members)), None) = (&rhs.value, &lhs.value) {
        // A constant set: a cell in the code of the constructor's own size
        // (its high end, ref 3.9.3 p. 3-16), the test guarded by `bcs` on
        // the subtraction and the branch `ble`/`bgt` ([`Expr::SetMemberAt`]
        // on an [`Addr::Const`]): UNMAP_CASE 0xE54164..0xE54178 `clr.w D2;
        // moveq #95,D3; move.b D0,D2; sub.w D2,D3; bcs.s; lsr.w #3,D3; lea
        // (0xe543e0,PC),A3; btst D2,(0,A3,D3.w); ble.s` (a char in a
        // 96-bit constant), NAME_$INIT 0xE31836..0xE3184A.  The element's
        // size is the backend's concern: the MC68010 form takes only a
        // one-byte element (all the sites: chars), as the `ble`/`bgt`
        // after the borrow reads N, which is the borrow only while the
        // subtraction cannot overflow (codegen-patterns 5.1; `dpc-m68k`
        // `set_member_at`); the MC68020 form guards with `cmpi.l`/`bcs`
        // on the complemented element, any size (bead domain-pascal-bkjm).
        let Some(&hi) = members.iter().max() else {
            return Err(unsupported(e.span, "`in` on the constant `[]` is"));
        };
        let size = 2 * ((hi as u32 + 1).div_ceil(16));
        if size <= 4 {
            return Err(unsupported(
                e.span,
                "`in` on a constant set of a word or a longword with a byte element (no site, \
                 codegen-patterns 5.1) is",
            ));
        }
        return Ok(Expr::SetMemberAt {
            elem: Box::new(set_element(p, lhs)?),
            set: Addr::Const(PoolConst {
                bytes: large_set_bytes(members, size, rhs.span)?,
                key: rhs.ty.0,
                symbol: None,
            }),
            bytes: size,
        });
    }
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
    let integer = matches!(p.types.kind(lhs.ty), TypeKind::Integer16)
        || matches!(p.types.kind(lhs.ty), TypeKind::Subrange { base, .. }
            if matches!(p.types.kind(*base), TypeKind::Integer16));
    match p.types.ordinal_range(lhs.ty) {
        Some((lo, hi)) if lo >= 0 && hi < bits => {}
        // Unguarded, as MSG_$FORK 0xE73F2E: an element outside the set's
        // bits reads a byte outside the set, as the original does.  The
        // image does not show the element's Pascal type, so the form stays
        // tracked under `--strict`.
        _ if integer => best_effort(
            e.span,
            format!(
                "`in` on a {bytes}-byte set with an `integer` element that may lie beyond its \
                 {bits} bits (unguarded, MSG_$FORK 0xE73F2E; an element outside reads outside \
                 the set; codegen-patterns 5.1) is"
            ),
        )?,
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
    if let Some(k) = lhs.value.as_ref().and_then(|v| v.ordinal()) {
        // A constant member: its bit on the byte that holds it, as a
        // boolean field (`btst #k mod 8`, `tst.b` for bit 7), the rule of
        // a constant member of a set in memory (codegen-patterns 5.1, "`in`
        // on sets in memory, constant member": the byte that holds bit k,
        // never renumbered; tty_$i_kill_line 0xE1B6B6 `btst
        // #2,(0x1f,A2)`).  Best effort (not byte-identical): no site on a
        // set of more than a longword.
        if !(0..bits).contains(&k) {
            return Err(unsupported(
                e.span,
                format!("`in` of a constant outside a {bytes}-byte set's {bits} bits is"),
            ));
        }
        best_effort(
            e.span,
            "`in` of a constant member of a set of more than a longword (no site; \
             codegen-patterns 5.1) is",
        )?;
        return Ok(Expr::Bits {
            addr: addr_of(p, rhs)?.offset_by(((bits - 1 - k) / 8) as i32),
            field: BitField {
                unit: Width::Byte,
                shift: (k % 8) as u32,
                width: 1,
            },
            ty: Scalar::BOOLEAN,
        });
    }
    Ok(Expr::SetMemberAt {
        elem: Box::new(set_element(p, lhs)?),
        set: addr_of(p, rhs)?,
        bytes,
    })
}

/// The value of `e`.
/// Refuse a call (callee `c`) of a function whose result convention is
/// not known (`ResultReg::Unknown`: sets, reals, records other than
/// 4-byte ones; codegen-patterns 3), whether its value is used or
/// discarded (`discard`): such a result may need caller-provided storage,
/// so even a bare call could be wrong code.  Through a routine pointer the
/// template's result type decides, as `dpc_sema::conv::result_reg` does
/// for a routine (pointers and ordinals are known; a 4-byte record, known
/// for a routine, is refused through a pointer: no site).
fn refuse_unknown_result(p: &Program, c: &SCall, span: Span) -> Res<()> {
    let unknown = match &c.callee {
        SCallee::Routine(id) => p
            .routine(*id)
            .result
            .as_ref()
            .filter(|r| r.reg == ResultReg::Unknown)
            .map(|r| r.ty),
        SCallee::Pointer(x) => match p.types.kind(x.ty) {
            TypeKind::RoutinePointer(sig) => {
                let rec = sig.result.filter(|&t| {
                    matches!(p.types.kind(t), TypeKind::Record(_)) && p.types.size(t) == 4
                });
                if rec.is_some() {
                    // A 4-byte record result is in D0 for a routine
                    // (codegen-patterns 3, "Record results"); no compiled
                    // call through a pointer is known to read one.
                    return Err(unsupported(
                        span,
                        "a record result of a call through a routine pointer (no site seen) is",
                    ));
                }
                sig.result.filter(|&t| {
                    !p.types.is_pointer(t) && !p.types.is_ordinal(t) && !p.types.is_error(t)
                })
            }
            _ => None,
        },
    };
    match unknown {
        Some(ty) => Err(Diagnostic::error(
            span,
            dpc_sema::conv::unknown_result_message(ty, &p.types),
        )),
        None => Ok(()),
    }
}

fn value_of(p: &Program, e: &SExpr) -> Res<Expr> {
    if let Some(v) = &e.value {
        return const_of(v, scalar_of(p, e)?, e.span);
    }
    if is_set(p, e.ty) && matches!(e.kind, ExprKind::SetCons(_) | ExprKind::Binary { .. }) {
        let ty = scalar_of(p, e)?;
        return set_value(p, e, ty);
    }
    if let Some((array, bytes, index, lo)) = bit_element(p, e)? {
        return Ok(Expr::BitElem {
            array,
            bytes,
            index: Box::new(index),
            lo,
        });
    }
    if let Some((addr, field)) = bit_field(p, e)? {
        check_device_access(p, e, true)?;
        let ty = bit_field_type(p, e)?;
        let field = word_sign_field(p, e, &addr, field, ty);
        let bits = Expr::Bits { addr, field, ty };
        if p.types.ordinal_range(e.ty).is_some_and(|(lo, _)| lo < 0) {
            // Best effort (not byte-identical; bead domain-pascal-eoc6): a
            // field of a subrange with negative values (ref Figure 3-17,
            // p. 3-37: two's complement, `-100..40`) has no site.  Its
            // bits zero-extended, then shifted to the top of the value
            // and arithmetically back, `lsl` then `asr` (ref p. 4-29
            // `arshft`), which extends the field's sign bit.
            let n = 8 * ty.bytes() - field.width;
            if n == 0 {
                return Ok(bits);
            }
            let count = Expr::Const {
                value: i64::from(n),
                ty: Scalar::signed(Width::Word),
            };
            return Ok(Expr::Binary {
                op: IrBinOp::Ashr,
                lhs: Box::new(Expr::Binary {
                    op: IrBinOp::Shl,
                    lhs: Box::new(bits),
                    rhs: Box::new(count.clone()),
                    ty,
                }),
                rhs: Box::new(count),
                ty,
            });
        }
        return Ok(bits);
    }
    match &e.kind {
        ExprKind::Var { .. }
        | ExprKind::Field { .. }
        | ExprKind::Deref(_)
        | ExprKind::WithRecord(_) => Ok(Expr::Load {
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
        ExprKind::Call(_) if hidden_call(p, e).is_some() => {
            Err(unsupported(e.span, HIDDEN_CALL_USE))
        }
        ExprKind::Call(c) => {
            // A pointer result comes back in A0 (ref 5.5.10, p. 5-20;
            // codegen-patterns 3, "Pointer results"); the caller reads it
            // there (codegen-patterns 3, "Callers of pointer functions":
            // MST_$MAPS 0xE439C8 `move.l A0,(-4,A6)`).  `d0_return` puts
            // it in D0 too, where the caller reads it like a data result
            // (no compiled site; ref 5.5.10).
            refuse_unknown_result(p, c, e.span)?;
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
///   narrowings are refused (an assignment's further forms,
///   [`assign_convert`], have no transfer site catalogued).
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
        if let Some(n) = narrow(&v, ty) {
            return Ok(n);
        }
        best_effort(
            e.span,
            format!(
                "a type transfer of a {}-byte integer to a {}-byte one (narrowing, as in an \
                 assignment) is",
                v.ty().bytes(),
                ty.bytes()
            ),
        )?;
        return Ok(Expr::Convert {
            value: Box::new(v),
            ty,
            in_place: false,
        });
    }
    Ok(reinterpret(v, ty))
}

/// `v` read as a value of type `ty` of the same width (another sign or
/// class): no code; a load or a constant is retyped, any other value
/// wrapped in a [`Expr::Convert`] of the same width, which the selector
/// evaluates as its operand.
fn reinterpret(v: Expr, ty: Scalar) -> Expr {
    match v {
        Expr::Const { value, .. } => Expr::Const { value, ty },
        Expr::Load { addr, .. } => Expr::Load { addr, ty },
        // A result in A0 (`a0_return`, ref 5.5.11 p. 5-20, or a pointer
        // function) read as another 4-byte type: still the call's A0, so
        // the call is retyped, and its callers see the usual A0 forms
        // (`p := ptr_t(f(...))` for an integer `a0_return` function `f`
        // is `movea.l A0,An`, TTY_$I_SET_RAW 0xE673CC; bead
        // domain-pascal-yzxr).
        Expr::Call {
            call,
            reg: ResultClass::Address,
            ..
        } => Expr::Call {
            call,
            ty,
            reg: ResultClass::Address,
        },
        v if v.ty() == ty => v,
        Expr::Convert { value, .. } if value.ty() == ty => *value,
        v => {
            debug_assert_eq!(v.ty().width, ty.width, "same width only");
            Expr::Convert {
                value: Box::new(v),
                ty,
                in_place: false,
            }
        }
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
/// `div`, `mod` and `arshft` give the same value either way and stay as
/// they are; `lshft` and `rshft` are longwords already ([`builtin_value`]).
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
        // `abs` of a sum, difference or product, widened: the operation
        // computed at the wider width and `abs` there (SUMA_$RCV
        // 0xE1ADCA..0xE1ADDA `clr.l D3; move.b (0x20,A5),D3b; clr.l D0;
        // move.b (0x10,A5),D0b; sub.l D3,D0; bpl.b; neg.l D0`, the same at
        // 0xE1ADF6..0xE1AE06: `abs` of a difference of two bytes compared
        // with a longword; codegen-patterns 8, "abs"), as for `-x` above.
        // The image's operand order there (the subtrahend first) and its
        // bytes zero-extended straight to longwords are not reproduced:
        // best effort, bead domain-pascal-r7hn.  [`narrow`] takes it back
        // to the narrower width (TPAD_$DATA 0xE69624..0xE69638, SMD_$BLT
        // 0xE6EDA2: word `muls.w`/`sub.w` under a word `abs`).  `abs` of
        // any other value is widened after it, by the result's sign
        // (zeros: TPAD_$DATA 0xE69284 `clr.l D0; ...; move.w D5w,D0w`).
        Expr::Unary {
            op: IrUnOp::Abs,
            operand,
            ty: t,
        } if ty.bytes() > t.bytes()
            && matches!(
                *operand,
                Expr::Binary {
                    op: IrBinOp::Add | IrBinOp::Sub | IrBinOp::Mul,
                    ..
                }
            ) =>
        {
            let wide = Scalar {
                width: ty.width,
                ..operand.ty()
            };
            // The provenance mark [`narrow`] needs to take it back: a
            // same-type [`Expr::Convert`] (which [`reinterpret`] never
            // makes) around the widened `abs`.
            Expr::Convert {
                value: Box::new(Expr::Unary {
                    op: IrUnOp::Abs,
                    operand: Box::new(convert(*operand, wide)),
                    ty,
                }),
                ty,
                in_place: false,
            }
        }
        v => Expr::Convert {
            value: Box::new(v),
            ty,
            in_place: false,
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
        None => {
            let v = value_of(p, x)?;
            // An integer of the operation's width and another sign (a
            // `0..2147483647` operand of a signed `-`, `u - 1`): read as
            // it is, a load retyped, so that the forms that take a
            // variable as an operand still match it (`subq.l #1,<ea>`,
            // [`convert`] would wrap it in a `Convert`).
            if v.ty().width == ty.width && !v.ty().pointer && !ty.pointer {
                Ok(reinterpret(v, ty))
            } else {
                Ok(convert(v, ty))
            }
        }
    }
}

/// `lhs op rhs` as a value.
fn binary(p: &Program, e: &SExpr, op: BinOp, lhs: &SExpr, rhs: &SExpr) -> Res<Expr> {
    if op == BinOp::BitAnd {
        // `rshft(w, 8) & c` of a `0..65535` word: DISK_'s module-local
        // 0xE6C55C reads the word's high byte, `moveq #7,D3; and.b
        // (-0x22,A3),D3b` (`../source`'s `rshft(part_dev[part], 8) & 7`),
        // while module-local 0xE1101E and 0xE11164 shift a loaded word,
        // `move.w (d,A2),Dn; lsr.w #8,Dn; andi.w #0xf8,Dn` (operand types
        // not known).  Which the original chose when is not settled (bead
        // domain-pascal-4rfc; codegen-patterns 8, "Shifts of an unsigned
        // word").
        let by8 = |x: &SExpr| {
            widened_unsigned_shift(p, x)
                && matches!(&x.kind, ExprKind::Builtin { builtin: Builtin::Rshft, args }
                    if args[1].value.as_ref().and_then(|v| v.ordinal()) == Some(8))
        };
        if (by8(lhs) && rhs.value.is_some()) || (by8(rhs) && lhs.value.is_some()) {
            best_effort(
                e.span,
                "`rshft(w, 8) & c` of a `0..65535` word (its high byte read, 0xE6C55C, or the \
                 word shifted, 0xE1101E: not settled) is",
            )?;
        }
    }
    let op = match op {
        BinOp::Add => IrBinOp::Add,
        BinOp::Sub => IrBinOp::Sub,
        BinOp::Mul => IrBinOp::Mul,
        BinOp::Div => IrBinOp::Div,
        BinOp::Mod => IrBinOp::Mod,
        BinOp::BitAnd => IrBinOp::And,
        BinOp::BitOr => IrBinOp::Or,
        BinOp::In if bit_field(p, rhs)?.is_some() => {
            return bit_field_member(p, lhs, rhs);
        }
        BinOp::In => {
            // codegen-patterns 5.1: a set that fits in a register (one
            // word or one longword) is tested in a register (PROC1_$TST_LOCK
            // 0xE148DC `btst D0,D1`; REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A41A
            // `move.w (22,A0),D0; btst #13,D0`).  Larger sets (byte-addressed
            // `btst`, MST_$ALLOC_ASID 0xE42D5C) are not implemented yet.
            let set_size = p.types.size(rhs.ty);
            let set_width = match set_size {
                1 => return byte_set_member(p, lhs, rhs),
                2 => Width::Word,
                4 => Width::Long,
                _ => return set_member_at(p, e, lhs, rhs, set_size),
            };
            return set_member(p, lhs, rhs, set_width);
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
    // An `lshft`/`rshft` of an unsigned run-time value is one (its type,
    // `dpc-sema`'s `shift_result`): module-local 0xE09412..0xE0942A `clr.l
    // D2; clr.l D1; move.w D0w,D2w; move.w (A2),D1w; lsl.l #5,D1; add.l
    // D1,D2; ...; cmp.l D1,D2; bls` (`page + lshft(bste, 5)`).
    let unsigned_leaf = |x: &SExpr| match &x.value {
        Some(v) => v.ordinal().is_some_and(|n| n >= 0),
        None => sign_of(&p.types, x.ty) == Sign::Unsigned,
    };
    if matches!(op, IrBinOp::Add | IrBinOp::Mul | IrBinOp::And | IrBinOp::Or)
        && (lhs.value.is_none() || rhs.value.is_none())
        && unsigned_leaf(lhs)
        && unsigned_leaf(rhs)
    {
        ty.sign = Sign::Unsigned;
    }
    if matches!(op, IrBinOp::Div | IrBinOp::Mod) {
        // Ref 4.3.3, pp. 4-6..4-7: div and mod are unsigned when both
        // operands are unsigned run-time values, or one is and the other
        // is a constant in 0..2147483647; signed otherwise (image:
        // `divu.w` and M$DIU/M$OIU on `0..65535` words, codegen-patterns
        // 8).
        // A `div` of an unsigned sum or product ([`unsigned_operand`]) is
        // unsigned (Uncertain; ../source finding 787): PMAP_$PURIFIER_L
        // 0xE1404E..0xE1405E `moveq #11,D4; add.l (MMAP_$WSL+0x70).l,D4`
        // then `M$DIU$LLW` by 12; PMAP_$PURIFIER_R 0xE1420A..0xE1421A
        // `(... + 5)`, `M$DIU$LLW` by 6; MST_$INIT 0xE30C9E..0xE30CAC a
        // product by 10, `M$DIU$LLW` by 100 (`0..2147483647` longwords);
        // DXM_$ADD_CALLBACK `clr.l D5; move.w D4,D5; addq.l #3,D5; lsr.l
        // #2,D5`; AST_$TOUCH_AREA `addi.l #256,D4; lsr.l #8,D4`;
        // RINGLOG_$LOGIT `addi.l #30,D2; lsr.l #1,D2`; FLP_$REVALIDATE
        // `lsl.l #2,D1; lsr.l #1,D1`; TTY_$I_CALC_COLUMN 0xE1B4DE.
        // Against, signed: NET_IO_$COPY_PACKET 0xE0E540 `clr.l D2; move.w
        // D1,D2; addi.l #1023,D2; bpl` (a `for` bound) and MST_$GROW_AREA
        // 0xE436AE (`+ 16#7FFF`, `asr.l`), which ../source's types may
        // explain.  A `mod` keeps its operands' own types: SMD_$LOC_EVENT
        // 0xE6E9EC..0xE6E9FE zero-extends a word, adds 255 and calls
        // `M$OIS$WLW`.
        let runtime_unsigned = |x: &SExpr| {
            x.value.is_none()
                && if op == IrBinOp::Div {
                    unsigned_operand(p, x)
                } else {
                    sign_of(&p.types, x.ty) == Sign::Unsigned
                }
        };
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
            let lty = Scalar::integer(Width::Long, ty.sign);
            return Ok(Expr::Convert {
                value: Box::new(Expr::Binary {
                    op,
                    lhs: Box::new(operand(p, lhs, lty)?),
                    rhs: Box::new(operand(p, rhs, lty)?),
                    ty: lty,
                }),
                ty,
                in_place: false,
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

/// Whether `e` is an `lshft` or `rshft` of an unsigned word run-time value
/// (a `0..65535` like subrange): shifted in 32 bits, the word widened with
/// zeros, the result unsigned ([`builtin_value`]; bead
/// domain-pascal-4rfc).
fn widened_unsigned_shift(p: &Program, e: &SExpr) -> bool {
    let ExprKind::Builtin {
        builtin: Builtin::Lshft | Builtin::Rshft,
        args,
    } = &e.kind
    else {
        return false;
    };
    e.value.is_none()
        && args.len() == 2
        && args[0].value.is_none()
        && p.types.is_integer(args[0].ty)
        && p.types.is_integer(e.ty)
        && p.types.size(args[0].ty) == 2
        && p.types.size(e.ty) == 4
        && sign_of(&p.types, args[0].ty) == Sign::Unsigned
}

/// A predeclared function's value.
fn builtin_value(p: &Program, e: &SExpr, b: Builtin, args: &[SExpr]) -> Res<Expr> {
    let ty = scalar_of(p, e)?;
    match b {
        Builtin::Lshft | Builtin::Rshft => {
            // Owner's ruling (reference-clarifications "Lshft of a 16-bit
            // operand; the sign of a shift of an unsigned word"; "Rshft",
            // p. 4-171): an operand smaller than 32 bits is expanded to 32
            // bits by its own sign, shifted in 32 bits, and the integer32
            // result narrowed only where it is stored into a smaller
            // target ([`narrow`]); `dpc-sema` gives the result its type,
            // unsigned for an unsigned run-time operand.  Image
            // (codegen-patterns 8, "Shifts of a 16-bit operand"; bead
            // domain-pascal-4rfc): a `0..65535` word widened with zeros,
            // AS_$GET_ADDR 0xE58418 `clr.l D0; move.w (0x00e2445c).l,D0w;
            // lsl.l #8,D0; lsl.l #7,D0; move.l D0,(4,A0)`, UID_$GEN
            // 0xE1A04A `clr.l D2; move.w (-0x10,A6),D2w; lsr.l #8,D2;
            // lsr.l #4,D2`, a count in a register at module-local
            // 0xE3C1CE..0xE3C1EA `lsl.l D6,D5`, `lsr.l D7,D6`; an
            // `integer` widened by its sign, PEB_'s module-local 0xE31DEA
            // `move.w D0w,D1w; ext.l D1; lsr.l #7,D1` (`rshft(wcs_addr,
            // 7)`), and `ext.l` then `lsl.l` 212 times in compiled code
            // (with products by powers of two, which look the same).
            let x = value_of(p, &args[0])?;
            let xt = x.ty();
            if xt.pointer || ty.pointer || ty.width != Width::Long {
                return Err(unsupported(
                    e.span,
                    "this shift (internal: not a longword) is",
                ));
            }
            let op = if b == Builtin::Lshft {
                IrBinOp::Shl
            } else {
                IrBinOp::Shr
            };
            let count = match &args[1].value {
                Some(v) => const_of(v, Scalar::signed(Width::Word), args[1].span)?,
                None => value_of(p, &args[1])?,
            };
            let long = Scalar {
                width: Width::Long,
                ..xt
            };
            let shifted = match xt.width {
                Width::Long => Expr::Binary {
                    op,
                    lhs: Box::new(x),
                    rhs: Box::new(count),
                    ty: xt,
                },
                Width::Word => Expr::Binary {
                    op,
                    lhs: Box::new(convert(x, long)),
                    rhs: Box::new(count),
                    ty: long,
                },
                // A one-byte integer (`[byte] 0..255`, ref 3.15.5) shifted
                // left by a constant of at most 8, whose result fits a
                // word: shifted as a word, widened to it by its own sign,
                // and the word widened again where the longword is used
                // (RING_$GET_ID 0xE2FA5C..0xE2FA6C `clr.w D0; move.b
                // (0x14,A1),D0; clr.l D1; lsl.w #8,D0; move.w D0,D1; add.l
                // D1,D2`: a form the selector refuses, not implemented);
                // stored as a word, TPAD_$DATA 0xE6936A..0xE69376 `clr.w D2; ...;
                // move.b (13,A1),D2; ...; lsl.w #6,D2` stored as a word
                // (`../source`'s `lshft(sample.b1, 6)`).  Other shifts of a
                // byte are widened to a longword first, which a word
                // target narrows back to the byte widened to a word and
                // shifted there (as before the ruling); used as a longword
                // the byte's widening to a longword is refused by the
                // selector (`move.b; andi.l #0xff`, not catalogued).
                Width::Byte
                    if op == IrBinOp::Shl
                        && matches!(&count, Expr::Const { value, .. } if (0..=8).contains(value)) =>
                {
                    let word = Scalar {
                        width: Width::Word,
                        ..xt
                    };
                    convert(
                        Expr::Binary {
                            op,
                            lhs: Box::new(convert(x, word)),
                            rhs: Box::new(count),
                            ty: word,
                        },
                        long,
                    )
                }
                Width::Byte => Expr::Binary {
                    op,
                    lhs: Box::new(convert(x, long)),
                    rhs: Box::new(count),
                    ty: long,
                },
            };
            Ok(reinterpret(shifted, ty))
        }
        Builtin::Arshft => {
            // Ref p. 4-29: arshft of an operand smaller than 32 bits
            // expands it to 32 bits first, shifts, and converts the result
            // back where it is stored; its value is that of the 16-bit
            // `asr` widened by its sign, which `dpc` emits (`asr.w`, then
            // `ext.l` where a longword is needed; `dpc-sema` keeps the
            // operand's width for the result).  No `arshft` site is
            // identified in the image.  An unsigned operand of another
            // width or sign than the result is refused.
            let x = value_of(p, &args[0])?;
            if x.ty() != ty {
                return Err(unsupported(
                    e.span,
                    "an `arshft` of an operand of another width or sign than its result (an \
                     unsigned `0..65535` word: no site seen) is",
                ));
            }
            let count = match &args[1].value {
                Some(v) => const_of(v, Scalar::signed(Width::Word), args[1].span)?,
                None => value_of(p, &args[1])?,
            };
            Ok(Expr::Binary {
                op: IrBinOp::Ashr,
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
        // "In_range", p. 4-104: "true if the value of x is within its
        // defined range".  Of an enumeration (ordinals 0..n-1, a word
        // compared unsigned, ref 3.7.1): `x <= n-1`, one unsigned compare
        // against the last ordinal, the lower bound 0 needing none.
        // TTY_$K_SET_FUNC_CHAR 0xE67502 and TTY_$K_INQ_FUNC_CHAR 0xE67588
        // `cmpi.w #0x1f,D1; bhi` (`not in_range(func)`, tty_$func_t's
        // last ordinal 31; ../source tty/k_func.pas).  **Reported:** an
        // `ord(x) > 31` gives the same code (codegen-patterns 10.1).
        // Other types (a subrange, an integer, a char) are not seen.
        Builtin::InRange
            if matches!(
                p.types.kind(p.types.canon(args[0].ty)),
                TypeKind::Enum { .. }
            ) =>
        {
            let x = value_of(p, &args[0])?;
            let (_, hi) = p.types.ordinal_range(args[0].ty).expect("an enumeration");
            let xty = x.ty();
            Ok(Expr::Test(Box::new(Cond::Compare {
                op: Rel::Le,
                lhs: x,
                rhs: Expr::Const { value: hi, ty: xty },
            })))
        }
        Builtin::InRange => in_range_subrange(p, e, &args[0]),
        Builtin::Max | Builtin::Min => min_max(p, e, b, &args[0], &args[1]),
        // "Odd", p. 4-128: "true if i is an odd integer": bit 0 of the
        // integer (codegen-patterns 8, "odd").
        Builtin::Odd => {
            let x = value_of(p, &args[0])?;
            if x.ty().width == Width::Byte {
                return Err(unsupported(e.span, "`odd` of a byte value is"));
            }
            Ok(Expr::Test(Box::new(Cond::Odd(x))))
        }
        Builtin::Abs => abs_value(p, e, &args[0]),
        _ => Err(unsupported(e.span, "this predeclared function is")),
    }
}

/// `abs(x)` of a run-time integer ("Abs", p. 4-12: "the number if it is
/// nonnegative, and the negative of the number if it is negative";
/// codegen-patterns 8, "abs"): [`IrUnOp::Abs`] at `x`'s width, of the
/// non-negative type `dpc-sema` gives it (`0..32767` stored in a word,
/// `0..2147483647`; unsigned in the IR).  Image: 17 sites in compiled
/// code, `x` a signed word or longword (`integer`, `integer32`, a word
/// difference, product or quotient, a longword difference).
///
/// **Best effort (not checked against the image)**, no site: `x`
/// unsigned (`0..65535`, `linteger`, a sum of unsigned values), whose
/// `abs` is `x` itself, converted to the result's type with no code of
/// its own; `x` one byte, widened to a word by its sign and `abs` there.
fn abs_value(p: &Program, e: &SExpr, x: &SExpr) -> Res<Expr> {
    let ty = scalar_of(p, e)?;
    let v = value_of(p, x)?;
    let xt = v.ty();
    if xt.pointer || xt.sign == Sign::Unsigned {
        // Best effort (not checked against the image): non-negative already.
        return Ok(if xt.width == ty.width {
            reinterpret(v, ty)
        } else {
            convert(v, ty)
        });
    }
    let v = if xt.width == Width::Byte && ty.width == Width::Word {
        // Best effort (not checked against the image): `ext.w` first.
        convert(v, Scalar::signed(Width::Word))
    } else {
        v
    };
    const OTHER_WIDTH: &str = "`abs` with a result of another width than its argument is";
    let v = if v.ty().width.bytes() < ty.width.bytes() {
        // Best effort (bead domain-pascal-5kgg): the argument widened by
        // its sign, then `abs` at the result's width (the same value: the
        // wider type holds every absolute value of the narrower one).
        best_effort(e.span, OTHER_WIDTH)?;
        convert(v, Scalar::signed(ty.width))
    } else {
        v
    };
    if v.ty().width != ty.width {
        return Err(unsupported(e.span, OTHER_WIDTH));
    }
    Ok(Expr::Unary {
        op: IrUnOp::Abs,
        operand: Box::new(v),
        ty,
    })
}

/// `in_range(x)` ("In_range", p. 4-104) of an integer subrange `lo..hi`
/// stored in a word, `lo >= 0` (an enumeration is [`builtin_value`]'s):
/// "true if the value of x is within its defined range", one unsigned
/// compare (codegen-patterns 10.1; **Reported**, the reading of
/// ../source's sio/k_set_param.pas):
///
/// - `lo = 0`: `x <= hi` unsigned, the word compared where it is, `cmpi.w
///   #hi,<ea>; bhi` for `not in_range(x)` (SIO_$K_SET_PARAM 0xE68252,
///   0xE682A6, `sio_$parity_t` and `sio_$word_size_t`, 0..3).
/// - `lo > 0`: `x - lo <= hi - lo` unsigned, in 32 bits, the word widened
///   with zeros: `clr.l D1; move.w (0x12,A2),D1; subq.l #1,D1; cmpi.l
///   #2,D1; bhi` (0xE68278..0xE68286, `sio_$stop_bit_t` = 1..3; the only
///   subtract-and-compare of this shape in compiled code).
///
/// Refused: a negative lower bound (the zero extension would put -1 out
/// of `-7..7`, p. 4-104's `small_int`: the original's form is not seen),
/// a subrange stored in a byte or a longword, and integers, chars and
/// booleans (no site).
fn in_range_subrange(p: &Program, e: &SExpr, x: &SExpr) -> Res<Expr> {
    let canon = p.types.canon(x.ty);
    let (lo, hi) = match p.types.kind(canon) {
        TypeKind::Subrange { lo, hi, .. } if p.types.is_integer(canon) => (*lo, *hi),
        _ => {
            return Err(unsupported(
                e.span,
                "`in_range` of a value other than an enumeration or an integer subrange (no \
                 image site, codegen-patterns 10.1) is",
            ))
        }
    };
    let v = value_of(p, x)?;
    if v.ty().width != Width::Word || lo < 0 {
        return Err(unsupported(
            e.span,
            "`in_range` of an integer subrange stored in a byte or a longword, or with a \
             negative lower bound (no image site, codegen-patterns 10.1) is",
        ));
    }
    let uw = Scalar::unsigned(Width::Word);
    let ul = Scalar::unsigned(Width::Long);
    let (lhs, rhs) = if lo == 0 {
        (reinterpret(v, uw), Expr::Const { value: hi, ty: uw })
    } else {
        (
            Expr::Binary {
                op: IrBinOp::Sub,
                lhs: Box::new(convert(reinterpret(v, uw), ul)),
                rhs: Box::new(Expr::Const { value: lo, ty: ul }),
                ty: ul,
            },
            Expr::Const {
                value: hi - lo,
                ty: ul,
            },
        )
    };
    Ok(Expr::Test(Box::new(Cond::Compare {
        op: Rel::Le,
        lhs,
        rhs,
    })))
}

/// `min(x, y)` / `max(x, y)` of run-time integers ("Min", p. 4-113;
/// "Max", p. 4-111; codegen-patterns 10.1): an [`IrBinOp::Min`] or
/// [`IrBinOp::Max`] whose type is that of the comparison.  **Reported**:
/// the image's compare-and-select form, read as `min`/`max` by
/// ../source's Pascal (an `if x > y then x := y` gives the same code).
///
/// - Sign (the reference's entries and 4.3.3, pp. 4-6..4-7): unsigned
///   when both operands are unsigned run-time values ([`sign_of`]; a
///   `0..65535` word, a `0..2147483647` longword, an unsigned sum), or one
///   is and the other a constant in 0..2147483647; signed otherwise.
///   Image: `bls` for MMAP_$ALLOC_FREE 0xE0D8B0 (`0..65535` against
///   `0..2147483647`), MMAP_$GET_IMPURE 0xE0D62A (100 against a
///   longword count), PKT_$BRK_INTERNET_HDR 0xE1246A (two `0..65535`
///   words); `ble` for PKT_$COPY_TO_PA 0xE125AA (1024 against an
///   `integer`), NETBUF_$ADD_PAGES 0xE0E980 (an `integer` against a
///   difference of longwords).
/// - Width: a word when both operands are words (0xE12468 `cmp.w`), a
///   longword when either is one (0xE0D8AC `cmp.l`, the word widened by
///   its own sign: `andi.l #0xffff` or `clr.l; move.w`, `ext.l`), and
///   when either is a word sum, difference or product, which is then
///   computed in 32 bits, its operands widened (ref 4.3.1, p. 4-5, as
///   for `div`'s dividend): NETBUF_$ADD_PAGES 0xE0E95A..0xE0E96C
///   `min(hdr_pages, 176 - netbuf_hdr_alloc)`, two `integer`s, `ext.l`
///   both, `move.l #0xb0,D6; sub.l D4,D6; cmp.l D6,D3`; NETBUF_$DEL_PAGES
///   0xE0EB54, 0xE0EB6E, 0xE0EC10.  A signed word against an unsigned
///   one is widened too, as a comparison is ([`widen_operands`]; not
///   seen for `min`/`max`), and so is an unsigned word against a negative
///   constant (the comparison signed).
/// - The result has the function's type (`dpc-sema`); a longword
///   comparison of word operands is narrowed back (the register's low
///   word: 0xE0EB6A `move.w D1,D4`).
///
/// Refused: bytes, chars, enumerations, booleans and pointers (no site).
fn min_max(p: &Program, e: &SExpr, b: Builtin, x: &SExpr, y: &SExpr) -> Res<Expr> {
    if !p.types.is_integer(x.ty) || !p.types.is_integer(y.ty) {
        return Err(unsupported(
            e.span,
            "`max` or `min` of values other than integers (no image site, codegen-patterns \
             10.1) is",
        ));
    }
    let runtime = |s: &SExpr| -> Res<Option<Expr>> {
        match s.value {
            Some(_) => Ok(None),
            None => value_of(p, s).map(Some),
        }
    };
    let (vx, vy) = (runtime(x)?, runtime(y)?);
    let vals: Vec<&Expr> = [&vx, &vy].into_iter().flatten().collect();
    if vals.iter().any(|v| v.ty().width == Width::Byte) {
        best_effort(
            e.span,
            "`max` or `min` of a one-byte integer (no image site, codegen-patterns 10.1) is",
        )?;
    }
    let non_negative = |s: &SExpr| {
        s.value
            .as_ref()
            .and_then(|v| v.ordinal())
            .is_some_and(|n| (0..=0x7fff_ffff).contains(&n))
    };
    let unsigned = [(x, &vx), (y, &vy)].iter().all(|(s, v)| match v {
        Some(v) => v.ty().sign == Sign::Unsigned,
        None => non_negative(s),
    });
    let word_arith = |v: &Expr| {
        matches!(v, Expr::Binary { op: IrBinOp::Add | IrBinOp::Sub | IrBinOp::Mul, ty, .. }
            if ty.width == Width::Word)
    };
    let long = vals.iter().any(|v| {
        v.ty().width == Width::Long || word_arith(v) || (!unsigned && v.ty().sign == Sign::Unsigned)
    });
    let cty = Scalar::integer(
        if long { Width::Long } else { Width::Word },
        if unsigned {
            Sign::Unsigned
        } else {
            Sign::Signed
        },
    );
    let operand = |s: &SExpr, v: Option<Expr>| -> Res<Expr> {
        match v {
            Some(v) if v.ty().width == cty.width => Ok(reinterpret(v, cty)),
            Some(v) => Ok(convert(v, cty)),
            None => {
                let c = const_of(s.value.as_ref().unwrap(), cty, s.span)?;
                let Expr::Const { value, .. } = c else {
                    unreachable!()
                };
                if !fits(value, cty) {
                    return Err(unsupported(
                        s.span,
                        "a `max` or `min` constant outside the other operand's type is",
                    ));
                }
                Ok(c)
            }
        }
    };
    let v = Expr::Binary {
        op: if b == Builtin::Max {
            IrBinOp::Max
        } else {
            IrBinOp::Min
        },
        lhs: Box::new(operand(x, vx)?),
        rhs: Box::new(operand(y, vy)?),
        ty: cty,
    };
    let rty = scalar_of(p, e)?;
    Ok(if rty.width == cty.width {
        reinterpret(v, rty)
    } else if rty.bytes() > cty.bytes() {
        convert(v, rty)
    } else {
        Expr::Convert {
            value: Box::new(v),
            ty: rty,
            in_place: false,
        }
    })
}

/// The IR scalar type of a value of type `ty` stored in `width`: its
/// sign ([`sign_of`]), and whether it is a pointer ([`Scalar::pointer`]).
pub(crate) fn scalar_of_type(types: &TypeTable, ty: TypeId, width: Width) -> Scalar {
    Scalar {
        width,
        sign: sign_of(types, ty),
        pointer: types.is_pointer(ty),
    }
}

/// The IR scalar type of `e`'s type.
fn scalar_of(p: &Program, e: &SExpr) -> Res<Scalar> {
    let size = p.types.size(e.ty);
    let width = Width::of_size(size)
        .ok_or_else(|| unsupported(e.span, format!("a {size}-byte value is")))?;
    Ok(scalar_of_type(&p.types, e.ty, width))
}

/// Whether `x`, an operand of `div`, counts as unsigned (codegen-patterns
/// 8, "The sign"): a constant that is non-negative, or a run-time value of
/// an unsigned type ([`sign_of`]) or the `+`, `*`, `&` or `!` of such
/// operands, one of them a run-time value (PMAP_$PURIFIER_L 0xE1404E, an
/// unsigned sum divided with `M$DIU$LLW`).  Only for `div`: the operands
/// of `+` itself are checked one level deep ([`binary`]), and `mod` takes
/// its operands' own types.
fn unsigned_operand(p: &Program, x: &SExpr) -> bool {
    if let Some(v) = &x.value {
        return v.ordinal().is_some_and(|n| n >= 0);
    }
    if sign_of(&p.types, x.ty) == Sign::Unsigned {
        return true;
    }
    match &x.kind {
        ExprKind::Binary {
            op: BinOp::Add | BinOp::Mul | BinOp::BitAnd | BinOp::BitOr,
            lhs,
            rhs,
        } if p.types.is_integer(x.ty) => unsigned_operand(p, lhs) && unsigned_operand(p, rhs),
        _ => false,
    }
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
/// - A non-negative subrange stored in 4 bytes (`0..2147483647`, Apollo's
///   `linteger` in `base.ins.pas`; anything from `0..65536` up):
///   unsigned.  Ref 3.4 p. 3-9 ("large unsigned subranges (from 0..2^30
///   up to 0..2^31-1) ... much of the effect of having true unsigned
///   types"), 4.3.3 pp. 4-6..4-7 (`u_type = 0..2147483647`; `u_type(s32)
///   DIV u32` "generates an unsigned divide"), "Max"/"Min" pp. 4-111,
///   4-113 ("The unsigned scalar data types are non-negative subranges of
///   integers").  Image: longwords that are neither pointers nor
///   zero-extended words compared, added and divided unsigned (PMAP_$
///   PURIFIER_L 0xE13EB0..0xE13EB8 `move.l (4,A1),D7; cmp.l (32,A1),D7;
///   bhi`, two working-set counts; 0xE14050..0xE1405E `moveq #11,D4; add.l
///   MMAP_$WSL+0x70,D4` then `M$DIU$LLW` by 12; CAL_$DECODE_TIME
///   0xE68794 `M$DIU$LLW` by 60; EC2_$INIT 0xE42C68 `cmpi.l #1000,D0;
///   bls`; codegen-patterns 5).
/// - Any other subrange: its host's sign.  **Uncertain:** whether the
///   original treated a non-negative `integer16` subrange (`0..100`) as
///   unsigned in compares (the reference's "Max" entry says such
///   subranges are unsigned scalars, but the image cannot tell a
///   `0..100` word from a `0..65535` one, and the mixed compares with
///   `integer` that would change are not catalogued for it).
/// - `char`: unsigned (ordinals 0..255, ref 3.6.4 p. 3-13).
/// - `boolean`: signed ([`Scalar::BOOLEAN`]).
/// - Enumerations: unsigned (ordinals 0..n-1, ref 3.7.1 p. 3-13).
///   **Uncertain:** enumeration compares have not been catalogued.
/// - Pointers, sets and everything else (bit patterns): unsigned.
pub(crate) fn sign_of(types: &TypeTable, ty: TypeId) -> Sign {
    match types.kind(ty) {
        TypeKind::Integer16 | TypeKind::Integer32 => Sign::Signed,
        TypeKind::Boolean => Scalar::BOOLEAN.sign,
        // A one-byte integer subrange (a size attribute, `[byte] 0..255`,
        // ref 3.15.5 p. 3-62: "corresponds to the unsigned char data type
        // in Domain/C"): unsigned when non-negative, signed otherwise
        // (`[byte] -128..127`).  Its values need every bit of the byte
        // either way.  Image: TPAD_$DATA zero-extends the packet's bytes
        // (0xE691D8 `clr.w D0; move.b (10,A1),D0; cmpi.w #0xdf,D0`;
        // 0xE6936A..0xE69372 `clr.w D2; clr.w D3; move.b (13,A1),D2;
        // move.b (12,A1),D3`) and sign-extends the mouse deltas (0xE69228
        // `move.b (12,A1),D1`, 0xE6922E `ext.w D1`); their Pascal types
        // (`tpad_$byte_t`, `tpad_$signed_byte_t`) are `../source`'s
        // inference (`tpad/tpad.ins.pas`).
        // Integer subranges: [`TypeTable::unsigned_integer`].
        TypeKind::Subrange { .. } if types.is_integer(ty) => {
            if types.unsigned_integer(ty) {
                Sign::Unsigned
            } else {
                Sign::Signed
            }
        }
        TypeKind::Subrange { base, .. } => sign_of(types, *base),
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
    if b.is_whole_object(p.types.size(e.ty), p.unaligned_access) {
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
    // A word or longword unit starts at an even offset in the record, so
    // it is odd when the record's address is ([`check_even`]: a packed
    // record that is an element of a byte-aligned array, or reached through
    // a byte-aligned parameter, ref 3.15.6.8 p. 3-76).
    if unit != Width::Byte {
        check_even(p, base)?;
    }
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

/// The field a boolean bit field `e` ([`bit_field`]) is *read* through: a
/// boolean in the top bit of a byte at an even offset in its record, the
/// top bit of the word there, is that word's sign (`unit` the word,
/// `shift` 15; the selector's `tst.w` and `bmi`/`bpl`/`smi`), where the
/// byte rule would give `tst.b` of the byte.  Stores keep the byte (sau5
/// PBU_$INIT 0xE5A680 `andi.b #127,(0x00e91970).l; ...; or.b
/// D0,(0x00e91970).l` for `pbu_$mcb.present := mbus_$present`, then
/// 0xE5A6B0 `tst.w (0x00e91970).l; bmi`).  Survey of both images
/// (codegen-patterns 5.3, "Boolean field, test"; bead domain-pascal-g9kx):
/// sau5 PBU_$MCB (a word packed record, ../source pbu/pbu.ins.pas) 13
/// `tst.w (0x00e91970).l` (MT_$ACQUIRE 0xE75CFE, PBU_$SETUP 0xE5976C,
/// SM_$CINIT 0xE6F2C8, ...); through an address register MSG_$$SEND
/// 0xE0DBBA `tst.w (0x2,A3); bmi` beside the same bit's value store
/// 0xE0DBAC `andi.b #127,(0x2,A3)`, MST_$TOUCH 0xE0DFE2, BAT_$MOUNT
/// 0xE3B77A `tst.w (0x3c,A0); smi`, DBUF_$GET_BLOCK 0xE3A634, both images.
/// The low byte's top bit stays `tst.b` (AST_$INVALIDATE 0xE066B2 `tst.b
/// (185,A0)`).
///
/// Another unit's record (an absolute operand) is read through the word
/// for the other bits of that byte too, bits 14..8 of the word, which the
/// selector loads and tests in a register: sau5 PBU_$WIRE_PBU 0xE5A436
/// `move.w (0x00e91970).l,D0; btst #14,D0; bne`, PBU2_$ASSOC 0xE58D0E
/// (#13), PBU_$UNWIRE_USER_PAGES 0xE59D98 `move.w (0x00e91970).l,D1; btst
/// #12,D1; sne D1`, PBU_$SETUP 0xE59B1E (#11); no compiled `btst #k` on an
/// even absolute address outside device registers in either image.  Through
/// an address register those bits stay a `btst` of the byte (MST_$TOUCH
/// 0xE0DFE8 `btst #3,(-0x3f6,A4)`, bit 11 of the word whose bit 15 is
/// 0xE0DFE2's `tst.w`).
///
/// Not for a volatile field (its byte is referenced as written, 5.5) nor a
/// record that may lie at an odd address: a byte-aligned one (`aligned(0)`
/// on the record or on what holds it, [`byte_aligned`]) keeps the byte
/// unless a static or frame offset proves its address even ([`even`]:
/// through a pointer, a parameter passed by address or another unit's
/// object its type's alignment does not tell).
fn word_sign_field(p: &Program, e: &SExpr, addr: &Addr, field: BitField, ty: Scalar) -> BitField {
    let ExprKind::Field {
        base,
        bits: Some(b),
        ..
    } = &e.kind
    else {
        return field;
    };
    let even_byte = field.unit == Width::Byte && (b.bit / 8) % 2 == 0;
    let absolute = matches!(addr, Addr::Symbol { .. });
    if ty != Scalar::BOOLEAN
        || field.width != 1
        || !even_byte
        || !(field.shift == 7 || absolute)
        || matches!(addr, Addr::Volatile { .. })
        || (byte_aligned(p, base) && even(p, base) != Some(true))
    {
        return field;
    }
    BitField {
        unit: Width::Word,
        shift: field.shift + 8,
        width: 1,
    }
}

/// `e` as an element of a packed array of booleans (one bit each, ref
/// 3.11.4.2 Table 3-5 p. 3-47; [`Expr::BitElem`]): the array's address
/// and size, the index (a word) and the array's lower bound; `None` when `e` is no
/// element of a packed array of bits.  Refused: elements of 2 or 4 bits
/// and one-bit elements that are no `boolean` (their values and code are
/// not catalogued), a constant index (no site: whether it becomes a
/// `bset #k`/`btst #k` as a packed record's field does is not seen), an
/// index of another width than a word (every site's is a word: PCHIST,
/// ACL; codegen-patterns 5.2), and volatile arrays.
fn bit_element(p: &Program, e: &SExpr) -> Res<Option<(Addr, u32, Expr, i64)>> {
    let ExprKind::Index {
        base,
        index,
        lo,
        elem_bits: Some(bits),
        ..
    } = &e.kind
    else {
        return Ok(None);
    };
    if *bits != 1 || !matches!(p.types.kind(e.ty), TypeKind::Boolean) {
        return Err(unsupported(
            e.span,
            format!(
                "an element of a packed array of {bits}-bit elements of type {} (only booleans \
                 are catalogued, codegen-patterns 5.2) is",
                p.types.display(e.ty)
            ),
        ));
    }
    if volatile_access(p, e).is_some() {
        return Err(unsupported(
            e.span,
            "an element of a volatile packed array of booleans is",
        ));
    }
    if index.value.is_some() {
        return Err(unsupported(
            index.span,
            "an element of a packed array of booleans with a constant index (no site in the \
             image, codegen-patterns 5.2) is",
        ));
    }
    let ix = value_of(p, index)?;
    if ix.ty().width != Width::Word {
        return Err(unsupported(
            index.span,
            format!(
                "a {}-byte index into a packed array of booleans (only word indices are \
                 catalogued, codegen-patterns 5.2) is",
                ix.ty().bytes()
            ),
        ));
    }
    Ok(Some((addr_of(p, base)?, p.types.size(base.ty), ix, *lo)))
}

/// The value type of bit field `e` ([`bit_field`]): a boolean of one bit,
/// or an ordinal of non-negative values (an enumeration, a subrange with a
/// low bound of 0 or more), zero-extended.  Best effort (not
/// byte-identical; bead domain-pascal-eoc6): a subrange with negative
/// values (`-100..40`, ref Figure 3-17) is its signed type, its sign
/// extended by the selector (no site seen); a set (ref 3.10.7 Table 3-3,
/// p. 3-36: one bit per element of 0..hi, member k at bit k of the
/// field's value, as an unpacked set's bit string) is its unsigned set
/// value, the field's bits zero-extended.
fn bit_field_type(p: &Program, e: &SExpr) -> Res<Scalar> {
    let t = &p.types;
    if t.is_boolean(e.ty) {
        return Ok(Scalar::BOOLEAN);
    }
    if is_set(p, e.ty) && t.size(e.ty) <= 4 {
        best_effort(
            e.span,
            format!(
                "a packed field of type {} that is not a whole byte, word or longword is",
                t.display(e.ty)
            ),
        )?;
        return scalar_of(p, e);
    }
    match t.ordinal_range(e.ty) {
        Some((lo, _)) if lo >= 0 => scalar_of(p, e),
        Some(_) => {
            best_effort(
                e.span,
                "a packed field of a subrange with negative values (its sign extension is not \
                 catalogued) is",
            )?;
            scalar_of(p, e)
        }
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
        // into memory, `bset`/`bclr`).  For a device that read is not
        // "gratuitous" (ref 3.15.3 p. 3-57): the neighbouring fields must
        // be kept, and the sau5 image updates a device record's packed
        // field in place with the ordinary field form (sm_$command
        // 0xE6F1A2..0xE6F1AE `andi.b #-4,(0x31,A0); move.b D1,D3; andi.b
        // #3,D3; or.b D3,(0x31,A0)` into SM_$CB's 2-bit unit field;
        // codegen-patterns 5.5, bead domain-pascal-eaw7).  A write-only
        // device cannot be read (ref 3.15.3 p. 3-59): its fields' stores
        // are refused, what the original did not being known.  For a
        // volatile object that is not a device the field form writes the
        // unit twice (`andi` then `ori`), and whether the original did so
        // for one is not known.
        if a.device {
            if check_device_access(p, target, true).is_err() {
                return Err(unsupported(
                    s.span,
                    "storing into a packed field of a write-only device record (`device(write)`: \
                     the field form reads its unit, ref 3.15.3 p. 3-59) is",
                ));
            }
        } else {
            best_effort(
                s.span,
                "storing into a packed field of a volatile record (its unit read and written \
                 back, perhaps twice; not seen, ref 3.15.1) is",
            )?;
        }
    }
    if let Some(st) = store_bits_set_update(p, s, target, value, &addr, field, ty)? {
        return Ok(st);
    }
    let (v, mask) = if let Some(c) = &value.value {
        (const_of(c, ty, value.span)?, false)
    } else {
        let v = value_of(p, value)?;
        let v = if v.ty().bytes() < ty.bytes() {
            // A one-byte set into a set field of a word: zero-extended, as
            // an integer (its members at the same bits).
            if !p.types.is_integer(value.ty) && !is_set(p, value.ty) {
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

/// `f := f + c`, `f := f - c`, `f := f * c` for a set field `f` that is a
/// bit field ([`bit_field`]) and a constant set `c`, or `None` for any
/// other store.  Best effort (not byte-identical; bead
/// domain-pascal-eoc6; no site seen): one member added or removed is its
/// bit set or cleared as a one-bit boolean field's, `bset`/`bclr #b` on
/// the byte that holds it (the in-place form of a set in memory,
/// TTY_$K_SET_FLAG 0xE6745C `bset #4,(9,A2)`, 0xE6747C `bclr`); any other
/// is the field's value combined with the constant as an integer (`!`
/// for `+`, `&` of the complement for `-`, `&` for `*`) and merged back
/// under the field's mask.
fn store_bits_set_update(
    p: &Program,
    s: &SStmt,
    target: &SExpr,
    value: &SExpr,
    addr: &Addr,
    field: BitField,
    ty: Scalar,
) -> Res<Option<Stmt>> {
    let ExprKind::Binary { op, lhs, rhs } = &value.kind else {
        return Ok(None);
    };
    if !is_set(p, target.ty) || value.value.is_some() {
        return Ok(None);
    }
    let Some(ConstValue::Set(members)) = &rhs.value else {
        return Ok(None);
    };
    if !matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul)
        || bit_field(p, lhs)? != Some((addr.clone(), field))
    {
        return Ok(None);
    }
    let width = i64::from(field.width);
    let bits: Vec<i64> = members
        .iter()
        .copied()
        .filter(|k| (0..width).contains(k))
        .collect();
    let c = bits.iter().fold(0i64, |m, k| m | (1 << k));
    let all = if width >= 63 { -1 } else { (1i64 << width) - 1 };
    let stmt = |kind| Stmt { kind, span: s.span };
    if bits.len() == 1 && matches!(op, BinOp::Add | BinOp::Sub) {
        let u = field.shift + bits[0] as u32;
        let byte = field.unit.bytes() - 1 - u / 8;
        return Ok(Some(stmt(StmtKind::StoreBits {
            addr: addr.clone().offset_by(byte as i32),
            field: BitField {
                unit: Width::Byte,
                shift: u % 8,
                width: 1,
            },
            value: Expr::Const {
                value: if *op == BinOp::Add { -1 } else { 0 },
                ty: Scalar::BOOLEAN,
            },
            mask: false,
        })));
    }
    let (iop, k) = match op {
        BinOp::Add => (IrBinOp::Or, c),
        BinOp::Sub => (IrBinOp::And, all & !c),
        _ => (IrBinOp::And, c),
    };
    let old = Expr::Bits {
        addr: addr.clone(),
        field,
        ty,
    };
    Ok(Some(stmt(StmtKind::StoreBits {
        addr: addr.clone(),
        field,
        value: Expr::Binary {
            op: iop,
            lhs: Box::new(old),
            rhs: Box::new(Expr::Const { value: k, ty }),
            ty,
        },
        mask: true,
    })))
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
/// The variable `x` of a pointer `ptr` of the form `addr(x)`, through any
/// type transfers (`T(addr(x))`): its dereference `T(addr(x))^` is the
/// object `x` itself, viewed as the pointer's base type (SIO_$INIT_LINE
/// 0xE32B40 `lea (692,A2),A0` for `sio_$tty_handlers_ptr_t(addr(tty.
/// xmit_callback))^ := ...`, tty in A2; RIP_$FIND_NEXTHOP 0xE156B6 `lea
/// (A2),A1` for `rip_$nexthop_ptr_t(addr(addr_info))^`; bead
/// domain-pascal-yqf6).
fn viewed(ptr: &SExpr) -> Option<&SExpr> {
    match &ptr.kind {
        ExprKind::Transfer(x) => viewed(x),
        ExprKind::Builtin {
            builtin: Builtin::Addr,
            args,
        } => args.first(),
        _ => None,
    }
}

fn volatile_access(p: &Program, e: &SExpr) -> Option<Access> {
    fn inherited(p: &Program, e: &SExpr) -> (bool, bool) {
        let a = p.types.attrs(e.ty);
        let (mut v, mut d) = (a.is_volatile(), a.device.is_some());
        if let ExprKind::Field { base, .. } | ExprKind::Index { base, .. } = &e.kind {
            let (bv, bd) = inherited(p, base);
            v |= bv;
            d |= bd;
        }
        if let ExprKind::WithRecord(w) = &e.kind {
            let (bv, bd) = inherited(p, &with_binding(*w).0);
            v |= bv;
            d |= bd;
        }
        // `T(addr(x))^` is `x` ([`viewed`]): volatile when `x` is.
        if let ExprKind::Deref(ptr) = &e.kind {
            if let Some(x) = viewed(ptr) {
                let (bv, bd) = inherited(p, x);
                v |= bv;
                d |= bd;
            }
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
/// one up to a dereference.  A record or array is referenced whole by a
/// copy ([`copy`]), so a part of it that denies the access is an error
/// too: a record with a `device(write)` field and a `device(read)` one
/// inherits `device(read, write)` (ref 3.15.7 p. 3-77,
/// [`dpc_sema::types::TypeTable`]'s inheritance), but copying it reads
/// the one and writes the other.
fn check_device_access(p: &Program, e: &SExpr, read: bool) -> Res<()> {
    fn denied(t: &TypeTable, ty: TypeId, read: bool) -> bool {
        if let Some(d) = t.attrs(ty).device {
            if (read && !d.read) || (!read && !d.write) {
                return true;
            }
        }
        match t.kind(ty) {
            TypeKind::Record(r) => r.all_fields().iter().any(|f| denied(t, f.ty, read)),
            TypeKind::Array { elem, .. } => denied(t, *elem, read),
            _ => false,
        }
    }
    let parts = match p.types.kind(e.ty) {
        TypeKind::Record(r) => r.all_fields().iter().any(|f| denied(&p.types, f.ty, read)),
        TypeKind::Array { elem, .. } => denied(&p.types, *elem, read),
        _ => false,
    };
    if parts {
        return Err(Diagnostic::error(
            e.span,
            if read {
                "reading a record or array with a write-only device part (`device(write)`, ref \
                 3.15.3, p. 3-59)"
            } else {
                "writing a record or array with a read-only device part (`device(read)`, ref \
                 3.15.3, p. 3-59)"
            },
        ));
    }
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
            ExprKind::WithRecord(w) => return check_device_access(p, &with_binding(*w).0, read),
            ExprKind::Deref(ptr) if viewed(ptr).is_some() => x = viewed(ptr).unwrap(),
            _ => return Ok(()),
        }
    }
}

/// `addr`, the address of designator `e`, wrapped in [`Addr::Volatile`]
/// when `e` is volatile, or is a record or array with a volatile or
/// device part: a volatile field does not make its record volatile (ref
/// 3.15.7 p. 3-77), but a reference to the whole record (a copy,
/// [`copy`]; one of 1, 2 or 4 bytes moved as a scalar) references that
/// field too.  `device` when `e` or a part is a device (a device part
/// makes its record a device, ref 3.15.7 p. 3-77, so the type's own
/// attribute says).
fn wrap_volatile(p: &Program, e: &SExpr, addr: Addr) -> Addr {
    let whole = volatile_access(p, e);
    let parts = matches!(
        p.types.kind(e.ty),
        TypeKind::Record(_) | TypeKind::Array { .. }
    ) && p.types.has_volatile_part(e.ty);
    let access = match whole {
        Some(a) if parts => Access {
            device: a.device || p.types.attrs(e.ty).device.is_some(),
            ..a
        },
        Some(a) => a,
        None if parts => Access {
            device: p.types.attrs(e.ty).device.is_some(),
            atomic: false,
        },
        None => return addr,
    };
    Addr::Volatile {
        addr: Box::new(addr),
        access,
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

/// The parity of `e`'s address: `Some(true)` even, `Some(false)` odd,
/// `None` not known ([`check_even`]).  An object reached through a
/// pointer, a parameter passed by address or another unit's object whose
/// type is byte aligned by an attribute of its own (`aligned(0)`, ref
/// 3.15.6.3 p. 3-69; [`dpc_sema::types::TypeInfo::align_attr`]) is of
/// unknown parity: `TypeTable::attributed` gives such a record an
/// alignment of 2 (its size stays a multiple of 2), so its `align` does
/// not tell (bead domain-pascal-epjn: a word-unit bit field of one was
/// read at an odd address).  A record that only inherits a field's
/// `aligned(0)` (ref 3.15.6.2 pp. 3-67..3-68) is word aligned (ref
/// 3.10.5.6: "Domain Pascal always allocates a record on at least a word
/// boundary") when it is 4 bytes or more: `layout_fields` aligns a field
/// on its type's specification, inherited ones included, and moves a
/// byte-aligned field of 4 bytes or more to the next word (it would cross
/// two word boundaries at an odd offset, p. 3-26), but a 2-byte one stays
/// at an odd offset (`record c: char; w: w_t end`, `w_t = record f:
/// [aligned(0)] pb_t end`: w at 1), so that one is of unknown parity.
fn even(p: &Program, e: &SExpr) -> Option<bool> {
    // Aligned as the type says.
    let typed = |e: &SExpr| type_is_even(p, e.ty).then_some(true);
    match &e.kind {
        ExprKind::Field { base, offset, .. } => Some(even(p, base)? == (offset % 2 == 0)),
        // An element of a packed array of bits has no address.
        ExprKind::Index {
            elem_bits: Some(_), ..
        } => None,
        ExprKind::Index {
            base,
            index,
            lo,
            elem_size,
            elem_bits: None,
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
        ExprKind::Deref(_) => typed(e),
        ExprKind::WithRecord(w) => even(p, &with_binding(*w).0),
        // `T(v)` seen in place ([`transfer_view`]): `v`'s address.
        ExprKind::Transfer(x) => even(p, x),
        ExprKind::Var { var, .. } => match &p.var(*var).storage {
            Storage::Static { offset, .. } => Some(offset % 2 == 0),
            Storage::Frame { .. } => Some(true),
            // An argument passed by address, used in place (no local
            // copy): the caller's object, which the callee may assume
            // only as aligned as the parameter's type says (ref
            // 3.15.6.8 p. 3-76; owner ruling "5.1 Attribute lists on
            // parameter types", `[aligned(0)] error_$integer32`).  A
            // value slot or local copy is in the frame.
            Storage::Param { index } => {
                let owner = p.var(*var).owner.expect("a parameter has an owner routine");
                let prm = &p.routine(owner).params[*index];
                if prm.passing() == Passing::ByAddress && prm.copy_offset.is_none() {
                    typed(e)
                } else {
                    Some(true)
                }
            }
            // Another unit's object, or the caller's place for a hidden
            // result: aligned as its type says.
            Storage::Extern { .. } | Storage::HiddenResult { .. } => typed(e),
        },
        _ => None,
    }
}

/// Whether an object of type `ty` whose place only its type tells (a
/// pointer's target, an argument passed by address, another unit's object)
/// is at an even address ([`even`]): its alignment is 2 or more, it has no
/// `aligned(0)` of its own, and it does not inherit one at 2 bytes.
fn type_is_even(p: &Program, ty: TypeId) -> bool {
    let t = p.types.get(ty);
    let inherited_odd = t.attrs.align == Some(1) && t.size < 4;
    t.align >= 2 && t.align_attr != Some(1) && !inherited_odd
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
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } | ExprKind::Transfer(base) => {
            byte_aligned(p, base)
        }
        ExprKind::WithRecord(w) => byte_aligned(p, &with_binding(*w).0),
        _ => false,
    }
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
/// object (codegen-patterns 2) and every static one (codegen-patterns 11,
/// "Byte-sized module variables").  An object inside a byte-aligned array
/// or record counts too.  A target that reads and writes at odd addresses
/// ([`Program::unaligned_access`]: the MC68020, whose SAU9 kernel does so,
/// codegen-patterns 14, "Odd addresses"; bead domain-pascal-gkrx) needs no
/// refusal.
fn check_even(p: &Program, e: &SExpr) -> Res<()> {
    if p.unaligned_access {
        return Ok(());
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
                l => Ok(outer(a, *l as u32, p.types.size(p.var(*var).ty))),
            }
        }
        ExprKind::Field { base, offset, .. } => Ok(addr_of(p, base)?.offset_by(*offset as i32)),
        // The record of an enclosing `with` ([`Cx::with_stmt`]).
        ExprKind::WithRecord(w) => Ok(with_binding(*w).1),
        // `T(addr(x))^`, a view of `x` through a pointer type transfer:
        // the object `x` itself (SIO_$INIT_LINE 0xE32B40 `lea (692,A2),A0`
        // for `sio_$tty_handlers_ptr_t(addr(tty.xmit_callback))^ := ...`,
        // tty in A2; RIP_$FIND_NEXTHOP 0xE156B6 `lea (A2),A1` for
        // `rip_$nexthop_ptr_t(addr(addr_info))^`; bead domain-pascal-yqf6).
        // [`viewed`]: whether `x` is volatile or a device is asked of
        // the whole designator ([`volatile_access`],
        // [`check_device_access`] see through the view).
        ExprKind::Deref(ptr) => match viewed(ptr) {
            // A part of `x` volatile, not the whole: the view's type does
            // not say which bytes are.
            Some(x) if p.types.has_volatile_part(x.ty) && volatile_access(p, x).is_none() => {
                Err(unsupported(
                    e.span,
                    "a view through a pointer type transfer (`T(addr(x))^`) of an object with a \
                     volatile or device part is",
                ))
            }
            Some(x) => addr_of(p, x),
            None => Ok(Addr::Indirect {
                base: Box::new(value_of(p, ptr)?),
                offset: 0,
            }),
        },
        // `a[i]`: the element at `(i - lo) * size` bytes into the array
        // (codegen-patterns 5.2).  A constant index is folded into the
        // displacement; otherwise the lower bound is (`(-2,A5,D0.w)` for
        // a 1-based word array, AUDIT_$IS_PROCESS_AUDITED 0xE70DA8).  No
        // range check is emitted: the image has none (FIM_$FREE_ASID's C,
        // `fim/free_asid.c`, notes "the original performs no bounds check").
        // Its byte and bit are formed by the selector ([`Expr::BitElem`],
        // [`StmtKind::StoreBitElem`]); passing one by address or taking its
        // `addr` is not seen in the image.
        ExprKind::Index {
            elem_bits: Some(_), ..
        } => Err(unsupported(
            e.span,
            "the address of an element of a packed array of bits (ref 3.11.4.2 Table 3-5) is",
        )),
        ExprKind::Index {
            base,
            index,
            lo,
            elem_size,
            elem_bits: None,
        } => {
            let b = addr_of(p, base)?;
            let big = || unsupported(e.span, "an array offset beyond 32 bits is");
            if let Some(i) = index.value.as_ref().and_then(ConstValue::ordinal) {
                let d = i32::try_from((i - lo) * *elem_size as i64).map_err(|_| big())?;
                return Ok(b.offset_by(d));
            }
            // A constant part of the index, `x + k`, `k + x` or `x - k`,
            // goes into the displacement with the lower bound: `x` is
            // the index register (MMAP_$INIT 0xE31A4C..0xE31A50 `move.w
            // D0,D1; lsl.w #2,D1; clr.l (0,A0,D1.w)`, `table[... div 1024
            // + 1]` of a 1-based table; PEB_UNWIRED's module-local
            // 0xE31DD4 at 0xE31E2C..0xE31E36, `x + 1`; TTY_$K_PUT 0xE67C68..0xE67C6E
            // `clr.l D1; ...; move.w D3,D1; pea (0,A0,D1.l)`,
            // `buffer[chars_written + 1]` of a 1-based buffer;
            // codegen-patterns 5.2, "The index's constant part").  `x` is
            // then within `lo - k ..  hi - k`.
            // Not folded where the displacement, with the element's size,
            // would pass 16 bits (`a[k + 1000]` of 48-byte records: the
            // sum computed, as before), nor for a signed byte, which would
            // be indexed as a byte (no `ext.w` index in the image) where
            // the sum is a word.
            let (index, k) = match index_constant_part(index) {
                (x, k) if k != 0 => {
                    let d = -(lo - k) * *elem_size as i64;
                    let near =
                        i16::try_from(d).is_ok() && i16::try_from(d + *elem_size as i64).is_ok();
                    let signed_byte =
                        p.types.size(x.ty) == 1 && sign_of(&p.types, x.ty) != Sign::Unsigned;
                    if near && !signed_byte {
                        (x, k)
                    } else {
                        (&**index, 0)
                    }
                }
                other => other,
            };
            let lo = lo - k;
            let mut ix = value_of(p, index)?;
            let s = *elem_size as i64;
            let fits = |r: Option<(i64, i64)>| {
                r.is_some_and(|(lo, hi)| lo * s >= -0x8000 && hi * s <= 0x7fff)
            };
            // The range of the index register: the bounds less the constant
            // part.
            let core_range = match p.types.kind(base.ty) {
                TypeKind::Array { index: it, .. } => {
                    p.types.ordinal_range(*it).map(|(l, h)| (l - k, h - k))
                }
                _ => None,
            };
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
                    // Best effort for a signed integer byte, sign-extended
                    // (bead domain-pascal-5kgg); a boolean's true is 0xFF,
                    // which neither extension makes its ordinal 1.
                    if p.types.is_boolean(index.ty) {
                        // A boolean (0 or 0xFF, ref 3.5.3) made its
                        // ordinal, 0 or 1, by its top bit: `lsr.b #7`, the
                        // byte then zero-extended to the word index
                        // (SIO2681_$SET_LINE 0xE1D2F0..0xE1D2FC `smi D1;
                        // clr.w D5; lsr.b #7,D1; move.b D1,D5; add.w D5,D5`,
                        // `baud_masks[chip^.config1.w < 0]`).  Best effort
                        // (not byte-identical; bead domain-pascal-pxov):
                        // the zero-extension's form is the selector's.
                        best_effort(
                            index.span,
                            "an array index of boolean type (`lsr.b #7`, then the byte widened: \
                             SIO2681_$SET_LINE 0xE1D2F0, one site; codegen-patterns 5.2) is",
                        )?;
                        let ub = Scalar::unsigned(Width::Byte);
                        let v = match ix {
                            Expr::Load { addr, .. } => Expr::Load { addr, ty: ub },
                            x => Expr::Convert {
                                value: Box::new(x),
                                ty: ub,
                                in_place: false,
                            },
                        };
                        let bit = Expr::Binary {
                            op: IrBinOp::Shr,
                            lhs: Box::new(v),
                            rhs: Box::new(Expr::Const { value: 7, ty: ub }),
                            ty: ub,
                        };
                        ix = convert(bit, Scalar::unsigned(Width::Word));
                    } else {
                        best_effort(
                            index.span,
                            "an array index of a signed byte type (no `ext.w` index in the \
                             image, codegen-patterns 5.2) is",
                        )?;
                        ix = convert(ix, Scalar::signed(Width::Word));
                    }
                } else {
                    ix = convert(ix, Scalar::unsigned(Width::Word));
                }
            }
            if ix.ty().width == Width::Word {
                // A word index is scaled at the word's width and used as
                // `(d,An,Dn.w)`, which sign-extends it (MC68000 brief
                // extension word): right only while every index × size of
                // the array's bounds fits in -32768..32767.  Otherwise
                // ([`wide_index`]) a variable whose type's range does not
                // fit either is widened to a longword and used as a `.l`
                // index (codegen-patterns 5.2, "Word indices used as `.l`",
                // bead domain-pascal-wkg).
                if !fits(core_range) {
                    if k != 0 && matches!(ix, Expr::Load { .. }) {
                        // A variable with a constant part, taken at the
                        // index type: widened by the sign of its range
                        // there, zero-extended when it cannot be negative
                        // (AREA_'s module-local 0xE075CA at 0xE076C6..
                        // 0xE076D8 `clr.l D3; movea.l #sym,A3; move.w
                        // D2,D3; lsl.l #4,D3; ...`, `areas[id + 1]` of the
                        // 1-based area table next to `ext.l` for
                        // `areas[id]`; TTY_$K_PUT 0xE67C68, above).
                        let unsigned = ix.ty().sign == Sign::Unsigned
                            || core_range.is_some_and(|(l, _)| l >= 0);
                        if !unsigned {
                            // Best effort (not byte-identical): a register
                            // that may be negative is sign-extended; no
                            // site.
                            dpc_syntax::diag::best_effort(unsupported(
                                index.span,
                                "a computed word index (a variable with a constant part) that \
                                 may be negative, into an array whose bounds times the element \
                                 size do not fit in 16 bits (`ext.l`: no site; the one site, \
                                 AREA_'s module-local 0xE075CA at 0xE076C6, zero-extends one \
                                 that cannot be; codegen-patterns 5.2, \"The index's constant \
                                 part\") is",
                            ))?;
                        }
                        let sign = if unsigned {
                            Sign::Unsigned
                        } else {
                            Sign::Signed
                        };
                        ix = convert(ix, Scalar::integer(Width::Long, sign));
                    } else {
                        ix = wide_index(p, index, ix, fits(p.types.ordinal_range(index.ty)))?;
                    }
                }
            } else if !matches!(ix, Expr::Load { .. })
                && fits(core_range)
                && (*elem_size).is_power_of_two()
            {
                // A computed longword index into an array whose bounds
                // times the element size fit in 16 bits: its low word,
                // scaled at the word's width and used as a `.w` index
                // (codegen-patterns 5.2, "Computed indices"; bead
                // domain-pascal-69fe): MMAP_$INIT 0xE31A44..0xE31A50 (`div
                // 1024 + 1`, above), SAU5 MT_$GET_IOVA 0xE76DDA (`lsr.l
                // #8; lsr.l #2; lsl.w #2,D1; or.l (0x1a,A0,D1.w),D0`,
                // `rshft(...) + 1`, the 1 in the displacement), SAU5's
                // mt_$set_iomap 0xE75CBA and SM_C_$CINIT 0xE74FAA; a
                // longword variable stays `.l` (EC2_$REGISTER_EC1
                // 0xE429A4..0xE429AA `move.l (3736,A5),D0; lsl.l #2,D0`
                // into a 257-entry table; DIR's module-local 0xE533E6 at
                // 0xE5337E), with a constant part too (AUDIT's
                // audit_$log_cname_op 0xE4BF0E `lea (0,A6,D3.l),A0; lea
                // (-527,A0),A2`, `data.names[n1 + 1]` of a longword `n1`;
                // audit_$log_link_op 0xE4BD90).  Only for an element size that is a power of
                // two: the one product seen, HINT_$GET_HINTS 0xE4998E
                // (`and.w`, then `muls.w D3,D5` and `.l`), is not this
                // form.
                let sign = ix.ty().sign;
                if ix.ty().width == Width::Long {
                    // Computed at the word's width where that gives the
                    // same low word ([`narrow`]): AST_$TOUCH_AREA SAU4
                    // 0xF803D0C `add.w D6,D4` for `ast_pmaps[seg_index,
                    // pages_done + page]` of an `integer` and a `0..65535`
                    // word (../source finding 760; the SAU2's alike).
                    let wty = Scalar::integer(Width::Word, sign);
                    ix = narrow(&ix, wty).unwrap_or_else(|| convert(ix, wty));
                }
            }
            // An array that is a part of its variable (a record's field, an
            // element of an outer array at a constant index other than the
            // first) in an overlay section of more than 0x8000 bytes: the
            // base the image takes for such a section's arrays, its start or
            // 60000 bytes in, is seen only for whole variables (and the
            // first element of one), at offsets 0 and 0x7C00 and beyond
            // (codegen-patterns 5.2, "Overlay sections of more than 0x8000
            // bytes"), so whether it follows the variable's offset or the
            // array's is not known.
            if let (Addr::Symbol { offset: ao, .. }, Some((vo, size))) = (&b, overlay_var(p, base))
            {
                if size > 0x8000 && i64::from(*ao) != i64::from(vo) {
                    return Err(unsupported(
                        e.span,
                        "an element of an array that is a part of its variable (a field, or an \
                         element at a constant index) in an overlay section of more than 0x8000 \
                         bytes (its base, the section's start or 60000 bytes in, is seen only \
                         for whole variables, codegen-patterns 5.2) is",
                    ));
                }
            }
            let offset = i32::try_from(-lo * s).map_err(|_| big())?;
            Ok(Addr::Index {
                base: Box::new(b),
                index: Box::new(ix),
                scale: *elem_size,
                offset,
            })
        }
        // `T(v)` with a field or element selected from it (`T(v).f`,
        // `T(v)[i]`; [`transfer_view`]): `v`'s own storage seen as `T`.
        ExprKind::Transfer(x) => {
            transfer_view(p, e, x)?;
            addr_of(p, x)
        }
        ExprKind::Call(_) if hidden_call(p, e).is_some() => {
            Err(unsupported(e.span, HIDDEN_CALL_USE))
        }
        _ => Err(unsupported(e.span, "this designator is")),
    }
}

/// Whether `e` reaches memory in place: a designator ([`SExpr::
/// is_designator`]), or a field or element of a type transfer of one
/// (`T(v).f`, [`transfer_view`]), or such a transfer itself.
fn in_place(e: &SExpr) -> bool {
    match &e.kind {
        ExprKind::Transfer(x) => in_place(x),
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => in_place(base),
        _ => e.is_designator(),
    }
}

/// Check type transfer `e`, `T(x)`, as the base of a field or element
/// selection (`T(v).f`, `T(v)[i]`): `x`'s storage read in place as type
/// `T` ([`addr_of`]).
///
/// Ref "Type Transfer Functions", p. 4-197: "Domain Pascal type transfer
/// functions enable you to change the type of a variable or expression
/// within a statement"; "the type transfer function does not change any
/// bits in the argument.  Domain Pascal just 'sees' the argument as a
/// value of the new type" (also C.3.19, p. C-11).  The reference shows
/// no selector after a transfer, but a value of a record or array type
/// has its fields and elements, and with no bits changed they are the
/// bytes of `x` at the field's offset.  The image reads them there, with
/// no copy (codegen-patterns 5.2, "Fields of a type-transferred
/// variable"): PROC1_$CREATE_P 0xE15156 `move.w (0xe,A6),D2` and 0xE15162
/// `move.w (0xc,A6),-(SP)`, the two words of its longword value
/// parameter; FILE_$SET_ACL 0xE5E08A `and.w (-0x44,A6),D0`, the third
/// word of a UID copy at (-0x48,A6); the module-local 0xE09A6A's argument
/// at AREA_$COPY 0xE092DE `move.w (-0xa,A6),-(SP)`, the low word of the
/// integer32 local at (-0xc,A6).
///
/// The reference says nothing of a transfer as a variable (an assignment
/// target, a `var` argument, `addr`, a `with` record; open question 46 in
/// docs/reference-clarifications.md): `dpc-sema` keeps such a selection a
/// value ([`SExpr::is_designator`] is false), so it is refused there, and
/// passing one by address (in place, or a copy as for an expression:
/// codegen-patterns 1.5) is refused here, no site being known
/// ([`call`]).  Refused too: a transfer of a value that is not
/// in memory (a function result, an expression: its temporary is not
/// seen), of another size (the integer-subrange exception, p. 4-197: the
/// bytes kept are then not `x`'s own), and of a volatile or device
/// object (no site).
fn transfer_view(p: &Program, e: &SExpr, x: &SExpr) -> Res<()> {
    if !in_place(x) {
        return Err(unsupported(
            e.span,
            "a field or element of a type transfer of a value that is not a variable (a \
             function result, an expression) is",
        ));
    }
    if p.types.size(x.ty) != p.types.size(e.ty) {
        return Err(unsupported(
            e.span,
            "a field or element of a type transfer between integer types of different sizes is",
        ));
    }
    if volatile_access(p, x).is_some() {
        return Err(unsupported(
            e.span,
            "a field or element of a type transfer of a volatile or device object is",
        ));
    }
    Ok(())
}

/// Whether `v`, a longword value, is computed from words only, one of
/// them unsigned, and may lie outside -32768..32767 (a `pinteger`
/// variable's `n - 1` or `a + 1`, but not `n div 2`, whose low word is its
/// value; `for` bounds, bead domain-pascal-3iws): every variable it reads
/// is a word or a byte, it is an operation (not a variable read or a
/// constant), and the interval its operations give it (a range not
/// known counting as not fitting) leaves the signed word's.
fn unsigned_word_arith(v: &Expr) -> bool {
    fn leaves(e: &Expr, unsigned: &mut bool) -> bool {
        match e {
            Expr::Const { .. } => true,
            Expr::Load { ty, .. } => {
                if ty.width == Width::Word && !ty.is_signed() && !ty.pointer {
                    *unsigned = true;
                }
                ty.bytes() <= 2
            }
            // A function result or a packed field, by its type's
            // width (as a variable).
            Expr::Call { ty, .. } | Expr::Bits { ty, .. } => {
                if ty.width == Width::Word && !ty.is_signed() && !ty.pointer {
                    *unsigned = true;
                }
                ty.bytes() <= 2 && !ty.pointer
            }
            Expr::Convert { value, .. } => leaves(value, unsigned),
            Expr::Unary { operand, .. } => leaves(operand, unsigned),
            Expr::Binary { lhs, rhs, .. } => leaves(lhs, unsigned) && leaves(rhs, unsigned),
            _ => false,
        }
    }
    fn range(e: &Expr) -> Option<(i64, i64)> {
        match e {
            Expr::Const { value, .. } => Some((*value, *value)),
            Expr::Load { ty, .. } | Expr::Call { ty, .. } | Expr::Bits { ty, .. } => {
                let bits = ty.bytes() as u32 * 8;
                Some(if ty.is_signed() {
                    (-(1i64 << (bits - 1)), (1i64 << (bits - 1)) - 1)
                } else {
                    (0, (1i64 << bits) - 1)
                })
            }
            Expr::Convert { value, ty, .. } if ty.bytes() >= value.ty().bytes() => range(value),
            Expr::Binary { op, lhs, rhs, .. } => {
                let (a, b) = (range(lhs)?, range(rhs)?);
                // Checked: an overflow is a range not known.
                match op {
                    IrBinOp::Add => Some((a.0.checked_add(b.0)?, a.1.checked_add(b.1)?)),
                    IrBinOp::Sub => Some((a.0.checked_sub(b.1)?, a.1.checked_sub(b.0)?)),
                    IrBinOp::Mul => {
                        let p = [
                            a.0.checked_mul(b.0)?,
                            a.0.checked_mul(b.1)?,
                            a.1.checked_mul(b.0)?,
                            a.1.checked_mul(b.1)?,
                        ];
                        Some((*p.iter().min()?, *p.iter().max()?))
                    }
                    IrBinOp::Div if b.0 == b.1 && b.0 > 0 => Some((a.0 / b.0, a.1 / b.0)),
                    IrBinOp::And if b.0 == b.1 && b.0 >= 0 && a.0 >= 0 => Some((0, b.0.min(a.1))),
                    IrBinOp::Shr if b.0 == b.1 && (0..32).contains(&b.0) && a.0 >= 0 => {
                        Some((a.0 >> b.0, a.1 >> b.0))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    let mut unsigned = false;
    matches!(v, Expr::Binary { .. } | Expr::Unary { .. })
        && leaves(v, &mut unsigned)
        && unsigned
        && !range(v).is_some_and(|(lo, hi)| lo >= -0x8000 && hi <= 0x7fff)
}

/// Whether the selector folds the `for` bound `to = x - c` (`v`, before
/// narrowing) into its count exactly from the constant `from`
/// (`Selector::for_`: `x` an unsigned word variable, `c >= 1`, `from` 0 or
/// 1 or `from + c` within a word; then `subq.w #from + c; bcs`).
fn folds_exactly(v: &Expr, from: Option<i64>) -> bool {
    let Some(from) = from else { return false };
    let Expr::Binary {
        op: IrBinOp::Sub,
        lhs,
        rhs,
        ..
    } = v
    else {
        return false;
    };
    let x = match &**lhs {
        Expr::Convert { value, .. } => &**value,
        x => x,
    };
    let unsigned_var = matches!(x, Expr::Load { ty, .. }
        if ty.width == Width::Word && !ty.is_signed() && !ty.pointer);
    match &**rhs {
        // `c` within a word: the selector sees it narrowed, and a larger
        // one would wrap negative (no fold, a signed guard).
        Expr::Const { value: c, .. } => {
            unsigned_var
                && (1..=0x7fff).contains(c)
                && from >= 0
                && ((0..=1).contains(&from) || from + c <= 0x7fff)
        }
        _ => false,
    }
}

/// `index` split into its run-time part and the constant added to it
/// ([`addr_of`], `a[x + k]`): `x + k`, `k + x` and `x - k` of integers
/// (nested too), `(index, 0)` for any other expression.
fn index_constant_part(index: &SExpr) -> (&SExpr, i64) {
    let ExprKind::Binary { op, lhs, rhs } = &index.kind else {
        return (index, 0);
    };
    let constant = |x: &SExpr| x.value.as_ref().and_then(ConstValue::ordinal);
    let (x, k) = match op {
        BinOp::Add => match (constant(lhs), constant(rhs)) {
            (None, Some(k)) => (&**lhs, k),
            (Some(k), None) => (&**rhs, k),
            _ => return (index, 0),
        },
        BinOp::Sub => match constant(rhs) {
            Some(k) if constant(lhs).is_none() => (&**lhs, -k),
            _ => return (index, 0),
        },
        _ => return (index, 0),
    };
    let (x, j) = index_constant_part(x);
    (x, j + k)
}

/// The index `ix` (the value of `index`, a word) of an element of an
/// array whose bounds times the element size do not fit in 16 bits
/// ([`addr_of`]), as a `.l` index: widened to a longword, which the
/// selector scales at that width (codegen-patterns 5.2, "Word indices used
/// as `.l`"; bead domain-pascal-wkg).  `type_fits`: whether the range of
/// `index`'s own type times the element size fits.
///
/// The image chooses the form per index expression, not per array: the
/// PROC2 table at 0xEA551C (228-byte entries, about 70 of them) is indexed
/// `mulu.w #228` / `.w` by some values (PROC2_$SIGNAL 0xE3EFFC, an element
/// of the pid map; PROC2_$CREATE 0xE72AA0, an entry's `(-198)` link; 41
/// sites) and `muls.w #228` / `.l` by others (PROC2_$ACKNOWLEDGE 0xE3F378,
/// the same element held in a register; 90 sites, and 1 `mulu.w` / `.l`,
/// PROC2_$GET_DEBUGGER_PID 0xE41BAC), and one value goes `.w`
/// for a smaller element and `.l` for a larger one (SMD_$INQ_DISP_INFO
/// 0xE70172 ×96, 0xE70182 ×268; MMAP_$WS_SCAN `(-8190,A3)` ×20 at
/// 0xE0D436, ×128 at 0xE0D3DE).  Whether the array's bounds count too is
/// not settled; here, where they do not fit, a variable (one load) whose
/// type's range does not fit either is `.l` by both readings, widened by
/// its type's sign:
///
/// - an unsigned word (a `0..65535`-style subrange): zero-extended,
///   `clr.l Dn; move.w x,Dn` (AREA_$DELETE 0xE07C4A..0xE07C5E, `area_id:
///   pinteger` into `areas[1..16#540]` of 48 bytes, compared unsigned at
///   0xE07C3E `bls`; 271 such `.l` indices), or `mulu.w #c` (47); into
///   another unit's array the selector takes it from a data register only
///   (the base's place against a load from memory is not settled,
///   `dpc-m68k` `other_unit_index`);
/// - a signed one with a negative lower bound (`integer`): `ext.l` (AREA_'s
///   module-local 0xE075CA at 0xE076DE; 240), or `muls.w #c` (122).
///
/// Refused: a variable whose type's range fits (`.w` by the index's type,
/// `.l` by the array's bounds: no site separates them); a non-negative
/// `integer16` subrange (of the four values that go `.w` or `.l` by the
/// element size, three are zero-extended or `mulu.w`, AST_$PURIFY
/// 0xE05932, and one `muls.w`, SMD_$INQ_DISP_INFO 0xE7017E: the sign is
/// bead domain-pascal-4x2's question); a computed index (`.w` at the 39
/// sites into arrays that fit; zero-extended at the one site into an array
/// that does not, AREA_'s 0xE075CA at 0xE076C6, `areas[id + 1]` next to
/// `ext.l` for `areas[id]`, in a `for` loop's stepped pointers: a variable
/// with a constant part, which [`addr_of`] widens itself); a byte index
/// (no site).
fn wide_index(p: &Program, index: &SExpr, ix: Expr, type_fits: bool) -> Res<Expr> {
    let refuse = |what: &str| {
        Err(unsupported(
            index.span,
            format!(
                "{what} into an array whose bounds times the element size do not fit in 16 bits \
                 (codegen-patterns 5.2, \"Word indices used as `.l`\"; bead domain-pascal-wkg) is"
            ),
        ))
    };
    if p.types.size(index.ty) != 2 {
        return refuse("a byte index (no site)");
    }
    // A variable: its value is one load (a packed record's field at a
    // byte boundary too).
    if !matches!(ix, Expr::Load { .. }) {
        // Best effort (not byte-identical): the word computed, then
        // widened by its own sign (zero-extended as at AREA_'s 0xE076C6
        // for an unsigned one, `ext.l` for a signed one).
        if let Err(e) = refuse(
            "a computed word index (into arrays that fit, `.w` at the 39 sites whose \
             computation is visible; into one that does not, zero-extended at the one site \
             known, AREA_'s module-local 0xE075CA at 0xE076C6, `areas[id + 1]` next to `ext.l` \
             for `areas[id]`)",
        ) {
            dpc_syntax::diag::best_effort(e)?;
        }
        let sign = ix.ty().sign;
        return Ok(convert(ix, Scalar::integer(Width::Long, sign)));
    }
    if type_fits {
        return refuse(
            "a word index whose type's range times the element size fits in 16 bits (`.w` by \
             the index's type or `.l` by the array's: not settled)",
        );
    }
    let sign = sign_of(&p.types, index.ty);
    let negative = p
        .types
        .ordinal_range(index.ty)
        .is_some_and(|(lo, _)| lo < 0);
    if sign == Sign::Signed && !negative {
        return refuse(
            "a word index of a non-negative `integer16` subrange (of the four values the image \
             indexes `.w` or `.l` by the element size, three zero-extended or `mulu.w`, one \
             `muls.w`: its sign is bead domain-pascal-4x2's question)",
        );
    }
    Ok(convert(ix, Scalar::integer(Width::Long, sign)))
}

/// The offset of the variable designator `e` is a part of (through fields
/// and constant indices) and the size of its section, when that variable
/// is in an overlay section ([`addr_of`]).
fn overlay_var(p: &Program, e: &SExpr) -> Option<(u32, u32)> {
    match &e.kind {
        ExprKind::Var { var, .. } => match &p.var(*var).storage {
            Storage::Static {
                offset, section, ..
            } if is_overlay(p, section) => {
                let size = p
                    .data_sections
                    .iter()
                    .find(|(n, _)| n == section)
                    .map_or(0, |(_, s)| *s);
                Some((*offset, size))
            }
            _ => None,
        },
        ExprKind::Field { base, .. } | ExprKind::Transfer(base) => overlay_var(p, base),
        ExprKind::Index { base, index, .. } if index.value.is_some() => overlay_var(p, base),
        _ => None,
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
        // The result's place, through the hidden address in the first
        // argument slot, as an `out` parameter's object (ACL_$DEFAULT_ACL
        // 0xE478C8 `movea.l (0x8,A6),A2; move.l (A1)+,(A2)`).
        Storage::HiddenResult { slot } => Ok(Addr::Indirect {
            base: Box::new(Expr::Load {
                addr: Addr::Frame(*slot),
                ty: Scalar {
                    pointer: true,
                    ..Scalar::unsigned(Width::Long)
                },
            }),
            offset: 0,
        }),
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
                            ty: Scalar {
                                pointer: true,
                                ..Scalar::unsigned(width)
                            },
                        }),
                        offset: 0,
                    })
                }
            }
        }
        // The unit's data area (codegen-patterns 4): `(d,A5)` on m68k; of
        // another data section than the routine's, that section's block
        // (codegen-patterns 11, "Data per section").  A variable of an
        // overlay section is an offset into the section's symbol, as
        // another unit's object (codegen-patterns 11, "Overlay sections":
        // never through A5; PCHIST_$CNTL 0xE5CD1A `tst.w (0xe8604e).l`,
        // PCHIST_UNWIRED_D+0x936; 0xE5CE46 `movea.l #0xe85718,A2`, the
        // section's start, for fields at `(0x510,A2)`; `tools/sectsurvey.py
        // overlays`).
        Storage::Static {
            offset, section, ..
        } => {
            let offset = i32::try_from(*offset)
                .map_err(|_| unsupported(span, "a static variable at this offset is"))?;
            if is_overlay(p, section) {
                return Ok(Addr::Symbol {
                    name: crate::overlay_symbol(section),
                    offset,
                });
            }
            let own = DATA_SECTION.with(|d| d.borrow().as_ref().is_none_or(|s| s == section));
            Ok(if own {
                Addr::Static(offset)
            } else {
                Addr::OtherStatic {
                    section: section.clone(),
                    offset,
                }
            })
        }
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
///   `univ` parameter see `univ_const_type`), the address of a symbol
///   a cell holding it ([`PoolConst::address`], [`symbol_address`]), an
///   expression by address its frame temporary ([`Arg::Temp`]):
///   codegen-patterns 1.5 (TERM_$SEND_KBD_STRING 0xE1AB0C, TONE_$ENABLE
///   0xE1ACFC).  A variable of another size than its value or `in`
///   parameter (a converted copy) is not supported yet.
/// - The pad: the callee's `arg_pad` (codegen-patterns 1.4).  A
///   `variable` routine called with fewer arguments (ref 5.5.5) is not
///   supported yet: its pad and slots are not catalogued.
fn call(p: &Program, c: &SCall, span: Span) -> Res<Call> {
    refuse_unknown_result(p, c, span)?;
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
    for (i, (a, &(mode, univ, prm_ty))) in c.args.iter().zip(&params).enumerate() {
        let slot = a.conv.slot_size;
        args.push(match a.conv.passing {
            Passing::ByValue => {
                let size = p.types.size(prm_ty);
                let width = Width::of_size(size).ok_or_else(|| {
                    unsupported(a.expr.span, format!("a {size}-byte value argument is"))
                })?;
                let ty = scalar_of_type(&p.types, prm_ty, width);
                let value = match &a.expr.value {
                    // `nil` pushed: `clr.l -(SP)` at NET_IO_$PUT_IN_SOCK
                    // 0xE0E4CC and NET_IO_$PUT_IN_SOCK_INT 0xE0E506.  The
                    // 19 `move.l #0,-(SP)` (all EC_$WAIT arguments) are
                    // not `nil`: their immediates are in the binder's
                    // RELOC table, a symbol at 0 (the map's EC_$NIL_EC),
                    // pushed as TIME_$CLOCKH_EC is beside them at FLP's
                    // EXCS 0xE3E2B6; so are DXM_$HELPER_COMMON 0xE171F2's
                    // and PKT_$PING_SERVER 0xE12C14's `movea.l #0,A2`
                    // pushed with `pea (A2)` (`tools/nilsurvey.py push`,
                    // `areg`; bead domain-pascal-73sx).  So `nil` is the
                    // pointer 0, pushed as any longword 0 (`clr.l -(SP)`,
                    // codegen-patterns 1.5 "Zero").  At both sites it is
                    // the first argument and its one neighbour a
                    // parameter (`move.w (8,A6),-(SP)` pushed just
                    // before it, 0xE0E4C8); a `nil` next to another constant
                    // argument has no site, and the original might have
                    // treated the pair otherwise (CLAUDE.md 3.4, merged
                    // word constants), so it is refused.
                    Some(ConstValue::Nil) => {
                        let is_const = |b: &dpc_sema::program::Arg| {
                            matches!(b.conv.passing, Passing::ByValue) && b.expr.value.is_some()
                        };
                        if [i.checked_sub(1), Some(i + 1)]
                            .into_iter()
                            .flatten()
                            .filter_map(|j| c.args.get(j))
                            .any(is_const)
                        {
                            best_effort(
                                a.expr.span,
                                "`nil` passed by value next to another constant argument (no \
                                 site; NET_IO_$PUT_IN_SOCK 0xE0E4CC has none beside it) is",
                            )?;
                        }
                        stored_const(&ConstValue::Nil, ty, a.expr.span)?
                    }
                    Some(v) => const_of(v, ty, a.expr.span)?,
                    // A set expression takes the parameter's size (`[k]`
                    // pushed: DISK_$FORMAT 0xE3D4E2..0xE3D4E6 `clr.w D1;
                    // bset D3,D1; move.w D1,-(SP)`, codegen-patterns 5.1).
                    None if is_set(p, prm_ty) => set_value(p, &a.expr, ty)?,
                    // Another width: converted as in an assignment
                    // ([`assign_convert`]).  Widened: APP_$RECEIVE
                    // 0xE008E6..0xE008EC `clr.l D1; move.w (994,A3),D1;
                    // move.l D1,-(SP)` (a `0..65535` field to an integer32
                    // `val_param`); SOCK_$CLOSE 0xE16008..0xE1600E `clr.w
                    // D1; move.b (27,A2),D1; move.w D1,-(SP)` (a `[byte]
                    // 0..255` field to an integer).  Narrowed: DBUF_$GET_
                    // BLOCK 0xE3A5F2..0xE3A5F6 `rshft` of a longword, its
                    // low word pushed; LOG_$ADD 0xE1775E `move.w D5,-(SP)`.
                    None => {
                        let v = value_of(p, &a.expr)?;
                        let vb = v.ty().bytes();
                        let what = format!(
                            "passing a {vb}-byte value to a {}-byte parameter is",
                            width.bytes()
                        );
                        match assign_convert(p, &a.expr, v.clone(), ty) {
                            Some(c) => c,
                            None => narrow_best_effort(p, &a.expr, v, ty, a.expr.span, &what)
                                .unwrap_or_else(|| Err(unsupported(a.expr.span, &what)))?,
                        }
                    }
                };
                Arg::Value { value, slot }
            }
            Passing::ByAddress => {
                // A bit field of a packed record has no address (ref
                // 3.10.3): passed as an expression, its value in a frame
                // temporary (below).  Best effort (not byte-identical; bead
                // domain-pascal-eoc6): no site.
                let bits = bit_field(p, &a.expr)?.is_some();
                if a.needs_temp && a.expr.is_designator() && !bits {
                    let what = "passing a variable of another size to a value or `in` parameter \
                                (a converted copy, codegen-patterns 1.5) is";
                    // Best effort for integers (bead domain-pascal-5kgg):
                    // the value converted as in an assignment into a frame
                    // temporary of the parameter's size, whose address is
                    // passed, as for an expression.
                    let width = Width::of_size(p.types.size(prm_ty));
                    let (Some(width), true, true) = (
                        width,
                        p.types.is_integer(a.expr.ty),
                        p.types.is_integer(prm_ty),
                    ) else {
                        return Err(unsupported(a.expr.span, what));
                    };
                    best_effort(a.expr.span, what)?;
                    let ty = Scalar::integer(width, sign_of(&p.types, prm_ty));
                    let v = value_of(p, &a.expr)?;
                    let value = match assign_convert(p, &a.expr, v.clone(), ty) {
                        Some(c) => c,
                        None => Expr::Convert {
                            value: Box::new(v),
                            ty,
                            in_place: false,
                        },
                    };
                    args.push(Arg::Temp {
                        value,
                        offset: 0,
                        slot,
                    });
                    continue;
                }
                // A variable of another size than its `var`, `out` or `in
                // out` parameter: a record whose size an alignment
                // attribute changed (ref 3.15.6.2, Figure 3-21 p. 3-67:
                // `[aligned(2)]` rounds it up; Figure 3-26 p. 3-71), which
                // is compatible with the plain record (`[attrs] T` is `T`).
                // The callee would read or write the parameter's size at
                // the argument's address.
                if !univ
                    && a.expr.is_designator()
                    && a.expr.value.is_none()
                    && matches!(mode, ParamMode::Var | ParamMode::Out | ParamMode::InOut)
                    && p.types.size(a.expr.ty) != p.types.size(prm_ty)
                {
                    return Err(unsupported(
                        a.expr.span,
                        format!(
                            "passing a {}-byte variable to a {}-byte `var`, `out` or `in out` \
                             parameter (an alignment attribute changes a record's size, ref \
                             3.15.6.2) is",
                            p.types.size(a.expr.ty),
                            p.types.size(prm_ty)
                        ),
                    ));
                }
                if let Some(v) = &a.expr.value {
                    // A constant: its cell in the code section (codegen-
                    // patterns 1.5, "Constant by address"), holding the
                    // value as the parameter's type (a `univ` one: see
                    // `univ_const_type`).
                    let ty = if univ {
                        univ_const_type(p, v, a.expr.ty, prm_ty, a.expr.span)?
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
                // `T(v).f` by address: whether the original passed `v`'s
                // bytes in place or a copy, as for an expression, is not
                // seen ([`transfer_view`]).
                if !matches!(a.expr.kind, ExprKind::Transfer(_)) && a.expr.selects_transfer() {
                    // Best effort: in place (bead domain-pascal-5kgg).
                    best_effort(
                        a.expr.span,
                        "a field or element of a type transfer passed by address (in place or \
                         as a copy: no site known) is",
                    )?;
                }
                if let ExprKind::Call(ic) = &a.expr.kind {
                    refuse_unknown_result(p, ic, a.expr.span)?;
                }
                if let Some(inner) = hidden_call(p, &a.expr) {
                    // A function result through a hidden address, passed
                    // on by address: the inner call made among the
                    // pushes, its temporary's address the argument
                    // ([`Arg::Call`]; codegen-patterns 3, "Record results
                    // over 4 bytes").  Only value and `in` parameters take
                    // a call (`dpc-sema`).
                    if !univ && p.types.size(a.expr.ty) != p.types.size(prm_ty) {
                        return Err(unsupported(
                            a.expr.span,
                            "passing a record function result to a parameter of another size is",
                        ));
                    }
                    args.push(Arg::Call {
                        call: Box::new(call(p, inner, a.expr.span)?),
                        slot,
                    });
                    continue;
                }
                if !a.expr.is_designator() || bits {
                    // The address of a symbol (another unit's object, a
                    // routine), `addr(x)` or `integer32(addr(x))`, for a
                    // parameter of 4 bytes or more: a link-time constant,
                    // passed as a constant is, a 4-byte cell holding the
                    // address (codegen-patterns 1.5, "Address of a symbol
                    // by address").  The cell keeps the address's own 4
                    // bytes whatever the parameter's size, as constants'
                    // cells do (ref 5.3.3 p. 5-11 extends an expression to
                    // a `univ` parameter's size; the image does not): SAU5
                    // VME_$INT 0xE0ABBE `pea (0x80,PC)` for LOG_$ADD's
                    // 192-byte `univ` data, the cell 0xE0AC40 `00ff 7c00`
                    // (PBU_CTL), the next routine at 0xE0AC44 (bead
                    // domain-pascal-68tc); SAU2 PEB_$LOAD_WCS 0xE32216
                    // for MST_$WIRE_AREA's `univ_ptr`s, cells 0xE322E4,
                    // 0xE322E8.
                    // Only another unit's object (`extern`) or a routine: a
                    // variable of an overlay section is a symbol plus an
                    // offset too ([`var_addr`]), but the unit's own data
                    // stays a temporary (PCHIST_$CNTL 0xE5CF06 `lea
                    // (0x934,A2),A0; move.l A0,(-40,A6); pea (-40,A6)`).
                    if p.types.size(prm_ty) >= 4 && extern_address(p, &a.expr) {
                        let value = value_of(p, &a.expr)?;
                        if let Some((name, offset)) = symbol_address(&value) {
                            args.push(Arg::Address {
                                addr: Addr::Const(PoolConst::address(name, offset)),
                                slot,
                                writable: false,
                            });
                            continue;
                        }
                    }
                    // An expression: stored into a frame temporary, whose
                    // address is passed (codegen-patterns 1.5, TONE_$ENABLE
                    // 0xE1ACFC); the offset is assigned once the body is
                    // lowered (`assign_temps`).
                    // An integer expression to a larger `univ` parameter
                    // of no scalar size: a temporary of the expression's
                    // own size (NETLOG_$SEND_PAGE 0xE71D2E..0xE71D44
                    // `move.l D1,(-0x20,A6); pea (-0x20,A6)`, `lshft(...)`
                    // for NET_IO_$SEND's 16-byte `univ` data_pages).
                    let own = (univ
                        && p.types.is_integer(a.expr.ty)
                        && Width::of_size(p.types.size(prm_ty)).is_none()
                        && p.types.size(prm_ty) > p.types.size(a.expr.ty))
                    .then(|| Width::of_size(p.types.size(a.expr.ty)))
                    .flatten();
                    let width = own
                        .or_else(|| Width::of_size(p.types.size(prm_ty)))
                        .ok_or_else(|| {
                            unsupported(
                                a.expr.span,
                                format!(
                                    "a {}-byte expression passed by address is",
                                    p.types.size(prm_ty)
                                ),
                            )
                        })?;
                    let mut value = value_of(p, &a.expr)?;
                    // A wider integer expression to an integer parameter
                    // is narrowed as an assignment narrows it (owner's
                    // ruling on question 41: integer16 is a subrange of
                    // integer32, the low bytes kept) when it can be
                    // computed at the parameter's width ([`narrow`]):
                    // then it is the expression of that width the
                    // temporary already takes (an `lshft` of a word, a
                    // longword sum of words, bead domain-pascal-4rfc).
                    if value.ty().width.bytes() > width.bytes()
                        && p.types.is_integer(a.expr.ty)
                        && p.types.is_integer(prm_ty)
                        && !value.ty().pointer
                    {
                        let narrow_ty = Scalar::integer(width, sign_of(&p.types, prm_ty));
                        if let Some(n) = narrow(&value, narrow_ty) {
                            value = n;
                        }
                    }
                    // The temporary has the parameter's size.  An
                    // expression of another size (to a `univ` parameter:
                    // extended to its size, ref 5.3.3 p. 5-11; to a
                    // larger value or `in` one: assignment compatible,
                    // ref 5.3.2 p. 5-7) needs a conversion whose code is
                    // not catalogued (bead domain-pascal-4be).
                    if value.ty().width != width {
                        let what = format!(
                            "passing a {}-byte expression by address to a {}-byte parameter is",
                            value.ty().bytes(),
                            width.bytes()
                        );
                        // Best effort for integers (bead domain-pascal-5kgg):
                        // converted as in an assignment, widened by the
                        // value's sign or narrowed to its low bytes.  To a
                        // `univ` parameter too: ref 5.3.3 p. 5-11, "When you
                        // pass an expression argument (as opposed to a
                        // variable argument) to a univ parameter, Domain
                        // Pascal extends the expression to be the same size
                        // as the univ parameter" and reports it "converted
                        // to NEWTYPE" -- a conversion of the value, where a
                        // variable argument passes its own bytes.
                        if !(p.types.is_integer(a.expr.ty)
                            && p.types.is_integer(prm_ty)
                            && !value.ty().pointer)
                        {
                            return Err(unsupported(a.expr.span, what));
                        }
                        best_effort(a.expr.span, &what)?;
                        let t = Scalar::integer(width, sign_of(&p.types, prm_ty));
                        value = if value.ty().bytes() < width.bytes() {
                            convert(value, t)
                        } else {
                            Expr::Convert {
                                value: Box::new(value),
                                ty: t,
                                in_place: false,
                            }
                        };
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
    // A result through a hidden address (codegen-patterns 3, "Record
    // results over 4 bytes"): the address of a new frame temporary is the
    // first argument, pushed last (PROC2_$UPGID_TO_UID 0xE41034 `pea
    // (-0x14,A6)'); its slot is what the routine's arguments take beyond
    // its parameters' (`dpc-sema` adds it).
    if let SCallee::Routine(id) = &c.callee {
        let r = p.routine(*id);
        if is_hidden_result(r) && r.parent.is_some() {
            return Err(unsupported(span, HIDDEN_NESTED));
        }
        if let Some(res) = r.result.as_ref().filter(|res| res.reg == ResultReg::Hidden) {
            let slot = r.arg_bytes - r.params.iter().map(|q| q.conv.slot_size).sum::<u32>();
            args.insert(
                0,
                Arg::Address {
                    addr: new_result_temp(p.types.size(res.ty)),
                    slot,
                    writable: true,
                },
            );
        }
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

/// `for v := lo to hi do a[v] := 0` where `v` is a local variable used
/// for nothing but the index, `lo` and `hi` are constants and the element
/// is a scalar (`0` any constant stored as 0: `false`, `nil`, the first
/// enumeration value, `[]`): [`StmtKind::Fill`] of `hi - lo + 1` elements
/// from `a[lo]`.  Another constant stays a [`StmtKind::For`] (below).
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
        elem_bits: None,
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
    if is_set(p, target.ty) && p.types.size(target.ty) > 4 {
        // A set of more than a longword is no scalar: the loop stores it
        // as any set assignment ([`large_set_store`]).
        return Ok(None);
    }
    check_even(p, target)?;
    let ty = scalar_of(p, target)?;
    if ty.bytes() != *elem_size {
        return Ok(None);
    }
    let stored = stored_const(c, ty, value.span)?;
    if !matches!(stored, Expr::Const { value: 0, .. }) {
        // A constant other than 0 (bead domain-pascal-1owb): a `For`, whose
        // stepped pointer is the image's form for such loops, the store
        // `move.<s> #c,(d,Ap)` and `addq.l #s,Ap`, the pointer from `base +
        // from * size` and `-lo * size` (and the field offset) in the
        // displacement (codegen-patterns 7.4): MMAP_$INIT 0xE3199E..0xE319B2
        // `moveq #55,D0; lea (4,A2),A0; move.l #0xfff,(-4,A0); addq.l
        // #4,A0; dbf` (`for i := 1 to 56 do table[i] := 16#FFF`), RIP_$SEND
        // 0xE8720A..0xE87218 `moveq #2,D0; movea.l (8,A6),A0; move.w
        // #-1,(4,A0); addq.l #2,A0; dbf`, MAC_OS_$ARP 0xE0C15A..0xE0C164
        // (the constant from a register that holds it already).  No
        // compiled loop stores a constant with `(An)+` (only clears do,
        // 0xE00ED8).
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
            value: stored,
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
