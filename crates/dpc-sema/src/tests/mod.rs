//! Semantic-analysis tests (CLAUDE.md 6.1).
//!
//! - `layout`: sizes and offsets from the reference's own examples and
//!   from `_Static_assert`s in `../source/domain_os`;
//! - `table5_1`: the argument-passing conventions, row by row;
//! - `checks`: name resolution, type checking, diagnostics;
//! - `tst_lock`: the PROC1_$TST_LOCK module end to end.

mod attrs;
mod checks;
mod layout;
mod packed;
mod table5_1;
mod tst_lock;

use dpc_syntax::preprocess::{preprocess, Options};
use dpc_syntax::SourceMap;
use dpc_target::Target;

use crate::program::{Program, Routine, Var};
use crate::types::{Field, TypeId, TypeKind};

/// The m68k numbers (mirrors `dpc_m68k::M68k`, which this crate cannot
/// depend on: dpc-m68k depends on dpc-ir, which depends on us).  The
/// driver test `crates/dpc/tests/driver.rs` runs the real one.
pub(crate) struct M68kAbi;

impl Target for M68kAbi {
    fn name(&self) -> &'static str {
        "m68k"
    }
    fn pointer_size(&self) -> u32 {
        4
    }
    fn arg_slot_size(&self) -> u32 {
        2
    }
    fn arg_area_align(&self) -> u32 {
        4
    }
    fn small_arg_offset_in_slot(&self, _size: u32) -> u32 {
        0
    }
    fn first_arg_offset(&self) -> i32 {
        8
    }
    fn val_param_max_size(&self) -> u32 {
        4
    }
    fn predeclared_cond_vars(&self) -> &'static [(&'static str, bool)] {
        &[]
    }
}

/// Analyse `src`; errors are rendered `t.pas:line:col: error: msg`.
pub(crate) fn analyze_src(src: &str) -> Result<Program, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &Options::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).unwrap_or_else(|ds| {
        panic!(
            "parse errors: {:#?}",
            ds.iter().map(|d| d.render(&sm, "dpc")).collect::<Vec<_>>()
        )
    });
    crate::analyze(&unit, &M68kAbi).map_err(|ds| ds.iter().map(|d| d.render(&sm, "dpc")).collect())
}

/// Analyse `src`, which must be free of errors.
pub(crate) fn ok(src: &str) -> Program {
    analyze_src(src).unwrap_or_else(|e| panic!("unexpected errors: {e:#?}"))
}

/// Analyse `src`, which must have errors; return them.
pub(crate) fn errs(src: &str) -> Vec<String> {
    match analyze_src(src) {
        Ok(_) => panic!("expected errors"),
        Err(e) => e,
    }
}

/// Assert that `src` fails with an error containing `needle`.
pub(crate) fn assert_err(src: &str, needle: &str) {
    let e = errs(src);
    assert!(
        e.iter().any(|m| m.contains(needle)),
        "no error containing {needle:?} in {e:#?}"
    );
}

/// `module m; <body>`, analysed.
pub(crate) fn module(body: &str) -> Program {
    ok(&format!("module m;\n{body}"))
}

pub(crate) fn ty(p: &Program, name: &str) -> TypeId {
    p.types
        .find_named(name)
        .unwrap_or_else(|| panic!("no type {name}"))
}

pub(crate) fn size(p: &Program, name: &str) -> u32 {
    p.types.size(ty(p, name))
}

pub(crate) fn field<'p>(p: &'p Program, rec: &str, f: &str) -> &'p Field {
    match p.types.kind(ty(p, rec)) {
        TypeKind::Record(r) => r.field(f).unwrap_or_else(|| panic!("no field {f}")),
        k => panic!("{rec} is not a record: {k:?}"),
    }
}

pub(crate) fn offset(p: &Program, rec: &str, f: &str) -> u32 {
    field(p, rec, f).offset
}

pub(crate) fn var<'p>(p: &'p Program, name: &str) -> &'p Var {
    p.var(p.find_var(name).unwrap_or_else(|| panic!("no var {name}")))
}

pub(crate) fn routine<'p>(p: &'p Program, name: &str) -> &'p Routine {
    p.routine(
        p.find_routine(name)
            .unwrap_or_else(|| panic!("no routine {name}")),
    )
}
