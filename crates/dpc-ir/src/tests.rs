//! IR lowering tests.

use dpc_syntax::preprocess::{preprocess, Options};
use dpc_syntax::SourceMap;
use dpc_target::Target;

use crate::*;

/// The m68k numbers (mirrors `dpc_m68k::M68k`, which this crate cannot
/// depend on: dpc-m68k depends on us).
struct M68kAbi;

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
}

/// Lower `src`; errors are rendered `t.pas:line:col: error: msg`.
fn lower_src(src: &str) -> Result<Module, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &Options::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &M68kAbi).map_err(render)?;
    lower(&program).map_err(render)
}

fn errs(src: &str) -> Vec<String> {
    match lower_src(src) {
        Ok(m) => panic!("expected errors, got {m:#?}"),
        Err(e) => e,
    }
}

const PCB: &str = "module proc1_tst;
type
  proc1_$pcb_t = record
    pad: array[1..16] of integer32;
    resource_locks_held: set of 0..31;
  end;
var
  proc1_$current_pcb: extern ^proc1_$pcb_t;
";

/// PROC1_$TST_LOCK 0xE148CA (CLAUDE.md 6.3).
#[test]
fn tst_lock() {
    let src = format!(
        "{PCB}function proc1_$tst_lock(lock_id: integer): boolean; options(val_param);
begin
  proc1_$tst_lock := lock_id in proc1_$current_pcb^.resource_locks_held;
end;
"
    );
    let m = lower_src(&src).unwrap();
    assert_eq!(m.name, "proc1_tst");
    assert_eq!(m.functions.len(), 1);
    let f = &m.functions[0];
    assert_eq!(f.symbol, "PROC1_$TST_LOCK");
    assert!(f.global);
    assert_eq!(f.code_section, ".text");
    // Raw: the 1-byte result variable.  The target rounds it.
    assert_eq!(f.frame_size, 1);
    assert_eq!(
        f.result,
        Some(ResultSpec {
            class: ResultClass::Data,
            ty: Scalar::BOOLEAN
        })
    );
    assert_eq!(f.body.len(), 1);
    let StmtKind::SetResult(e) = &f.body[0].kind else {
        panic!("not a result assignment")
    };
    assert_eq!(
        *e,
        Expr::SetMember {
            elem: Box::new(Expr::Load {
                addr: Addr::Frame(8),
                ty: Scalar::signed(Width::Word)
            }),
            set: Box::new(Expr::Load {
                addr: Addr::Indirect {
                    base: Box::new(Expr::Load {
                        addr: Addr::Symbol {
                            name: "PROC1_$CURRENT_PCB".into(),
                            offset: 0
                        },
                        ty: Scalar::POINTER
                    }),
                    offset: 0x40
                },
                ty: Scalar::unsigned(Width::Long)
            }),
            set_width: Width::Long,
        }
    );
}

#[test]
fn by_address_parameter_is_indirect() {
    let m = lower_src(
        "module m;
function f(var x: integer32): integer32;
begin
  f := x;
end;
",
    )
    .unwrap();
    let StmtKind::SetResult(e) = &m.functions[0].body[0].kind else {
        panic!("not a result assignment")
    };
    assert_eq!(
        *e,
        Expr::Load {
            addr: Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Frame(8),
                    ty: Scalar::POINTER
                }),
                offset: 0
            },
            ty: Scalar::signed(Width::Long)
        }
    );
}

#[test]
fn constants_lower() {
    let m = lower_src("module m;\nfunction f: integer;\nbegin\n  f := 7;\nend;\n").unwrap();
    let StmtKind::SetResult(e) = &m.functions[0].body[0].kind else {
        panic!("not a result assignment")
    };
    assert_eq!(
        *e,
        Expr::Const {
            value: 7,
            ty: Scalar::signed(Width::Word)
        }
    );
}

#[test]
fn empty_procedure_and_extern_routines() {
    let m = lower_src("module m;\nprocedure e(x: integer); extern;\nprocedure p;\nbegin\nend;\n")
        .unwrap();
    assert_eq!(m.functions.len(), 1);
    assert_eq!(m.functions[0].symbol, "P");
    assert!(m.functions[0].body.is_empty());
    assert_eq!(m.functions[0].result, None);
}

#[test]
fn result_assignment_must_be_last() {
    let e = errs(
        "module m;
function f: integer;
begin
  f := 1;
  f := 2;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:4:3: error: a function result assignment that is not the routine's last statement is not supported yet"]
    );
}

#[test]
fn function_without_result_assignment() {
    let e = errs("module m;\nfunction f: integer;\nbegin\nend;\n");
    assert_eq!(e.len(), 1);
    assert!(
        e[0].starts_with("t.pas:2:1: error: a function whose last statement")
            && e[0].ends_with("not supported yet"),
        "{e:?}"
    );
}

#[test]
fn unsupported_constructs_have_positions() {
    // A callee-copied value parameter (default convention).
    let e = errs("module m;\nprocedure p(x: integer);\nbegin\nend;\n");
    assert_eq!(
        e,
        vec!["t.pas:2:13: error: value parameters passed by address (callee-made local copy) are not supported yet"]
    );
    // An ordinary statement.
    let e = errs("module m;\nprocedure p(var x: integer);\nbegin\n  if x = 1 then x := 2;\nend;\n");
    assert_eq!(
        e,
        vec!["t.pas:4:3: error: this statement is not supported yet"]
    );
    // Module data in a named data section, or with an initial value.
    let e = errs("module m, code, mydata;\nvar v: integer;\n");
    assert_eq!(
        e,
        vec!["t.pas:2:5: error: variables in a named data section (ref 3.14, 7.1.1) are not supported yet"]
    );
    let e = errs("module m;\nvar (sect) v: integer;\n");
    assert_eq!(
        e,
        vec!["t.pas:2:12: error: variables in a named data section (ref 3.14, 7.1.1) are not supported yet"]
    );
    let e = errs("module m;\nvar v: integer := 3;\n");
    assert_eq!(
        e,
        vec!["t.pas:2:5: error: initialised static variables are not supported yet"]
    );
    // A 2-byte set.
    let e = errs(
        "module m;
type s16 = set of 0..15;
function f(k: integer; var s: s16): boolean; options(val_param);
begin
  f := k in s;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:5:8: error: `in` on a 2-byte set is not supported yet"]
    );
}

#[test]
fn unsupported_result_type_is_reported_once() {
    // A pointer result (A0, codegen-patterns 3) is not lowered yet; the
    // assignment to it must not add a second diagnostic.
    let e = errs(
        "module m;
type ip = ^integer;
function f(in p: ip): ip;
begin
  f := p;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:3:1: error: pointer (address-register) results are not supported yet"]
    );
}

/// The first `SetResult` value of the module's only function.
fn result_value(src: &str) -> Expr {
    let m = lower_src(src).unwrap();
    let StmtKind::SetResult(e) = &m.functions[0].body[0].kind else {
        panic!("not a result assignment")
    };
    e.clone()
}

/// codegen-patterns 5 / `lower.rs` `sign_of`: the IR type of a value
/// parameter of each kind of source type, read by value (val_param).
#[test]
fn signedness_from_source_types() {
    let cases: &[(&str, Scalar)] = &[
        ("integer", Scalar::signed(Width::Word)),
        ("integer16", Scalar::signed(Width::Word)),
        ("integer32", Scalar::signed(Width::Long)),
        // 2-byte storage with an integer32 host: unsigned.
        ("0..65535", Scalar::unsigned(Width::Word)),
        ("0..40000", Scalar::unsigned(Width::Word)),
        // Fits integer16: the host's sign.
        ("0..100", Scalar::signed(Width::Word)),
        ("-5..5", Scalar::signed(Width::Word)),
        // Needs 32 bits: integer32 host, 4 bytes, signed.
        ("0..65536", Scalar::signed(Width::Long)),
        ("boolean", Scalar::BOOLEAN),
        ("char", Scalar::unsigned(Width::Byte)),
        ("color", Scalar::unsigned(Width::Word)),
        // Pointers: `signedness_through_designators` (pointer results are
        // not lowered yet).
    ];
    for (ty, want) in cases {
        let src = format!(
            "module m;
type color = (red, green, blue); ip = ^integer; t = {ty};
function f(x: t): t; options(val_param);
begin
  f := x;
end;
"
        );
        let m = lower_src(&src).unwrap_or_else(|e| panic!("{ty}: {e:?}"));
        let f = &m.functions[0];
        assert_eq!(f.result.map(|r| r.ty), Some(*want), "{ty} result");
        let StmtKind::SetResult(e) = &f.body[0].kind else {
            panic!("not a result assignment")
        };
        assert_eq!(e.ty(), *want, "{ty} value");
    }
}

/// Signedness follows the type through fields, pointers and by-address
/// parameters; the pointer itself is unsigned.
#[test]
fn signedness_through_designators() {
    let e = result_value(
        "module m;
type r = record a: integer32; w: 0..65535; end; rp = ^r;
function f(var p: rp): 0..65535;
begin
  f := p^.w;
end;
",
    );
    let Expr::Load { addr, ty } = e else {
        panic!("{e:?}")
    };
    assert_eq!(ty, Scalar::unsigned(Width::Word));
    let Addr::Indirect { base, offset: 4 } = addr else {
        panic!("{addr:?}")
    };
    // p^: the pointer read through the by-address slot.
    let Expr::Load { addr, ty } = *base else {
        panic!()
    };
    assert_eq!(ty, Scalar::POINTER);
    let Addr::Indirect { base, offset: 0 } = addr else {
        panic!("{addr:?}")
    };
    assert_eq!(
        *base,
        Expr::Load {
            addr: Addr::Frame(8),
            ty: Scalar::POINTER
        }
    );
}

#[test]
fn constant_signedness_follows_its_type() {
    let e = result_value("module m;\nfunction f: char;\nbegin\n  f := 'a';\nend;\n");
    assert_eq!(
        e,
        Expr::Const {
            value: 97,
            ty: Scalar::unsigned(Width::Byte)
        }
    );
    let e = result_value("module m;\nfunction f: integer32;\nbegin\n  f := -100000;\nend;\n");
    assert_eq!(
        e,
        Expr::Const {
            value: -100000,
            ty: Scalar::signed(Width::Long)
        }
    );
}

/// A `var` (by-address) parameter of a `0..65535` type: the slot holds a
/// pointer (unsigned), the value read through it is unsigned too.
#[test]
fn signedness_of_a_by_address_unsigned_word() {
    let e = result_value(
        "module m;\ntype w = 0..65535;\nfunction f(var x: w): w;\nbegin\n  f := x;\nend;\n",
    );
    assert_eq!(
        e,
        Expr::Load {
            addr: Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Frame(8),
                    ty: Scalar::POINTER
                }),
                offset: 0
            },
            ty: Scalar::unsigned(Width::Word)
        }
    );
}

/// TIME_$Q_INIT_QUEUE 0xE16C5E: stores through a by-value pointer, the
/// by-value parameters listed in parameter order.
#[test]
fn q_init_queue_stores() {
    let m = lower_src(
        "module time_q;
type
  q = record head: integer32; lock: integer32; is_vt: boolean; id: integer; end;
  qp = ^q;
procedure init(is_vt: boolean; id: integer; queue: qp); options(val_param);
begin
  queue^.head := 0;
  queue^.is_vt := is_vt;
  queue^.id := id;
end;
",
    )
    .unwrap();
    let f = &m.functions[0];
    assert_eq!(
        f.value_params,
        vec![
            ValueParam {
                offset: 8,
                ty: Scalar::BOOLEAN
            },
            ValueParam {
                offset: 10,
                ty: Scalar::signed(Width::Word)
            },
            ValueParam {
                offset: 12,
                ty: Scalar::POINTER
            },
        ]
    );
    let queue = |offset| Addr::Indirect {
        base: Box::new(Expr::Load {
            addr: Addr::Frame(12),
            ty: Scalar::POINTER,
        }),
        offset,
    };
    let kinds: Vec<StmtKind> = f.body.iter().map(|s| s.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            // The literal 0 takes the field's type.
            StmtKind::Store {
                addr: queue(0),
                value: Expr::Const {
                    value: 0,
                    ty: Scalar::signed(Width::Long)
                }
            },
            StmtKind::Store {
                addr: queue(8),
                value: Expr::Load {
                    addr: Addr::Frame(8),
                    ty: Scalar::BOOLEAN
                }
            },
            StmtKind::Store {
                addr: queue(10),
                value: Expr::Load {
                    addr: Addr::Frame(10),
                    ty: Scalar::signed(Width::Word)
                }
            },
        ]
    );
}

/// Ref 3.5.3, p. 3-11: true is the byte 0xFF (-1 as a signed byte),
/// false 0.
#[test]
fn boolean_constants_are_their_bytes() {
    let m = lower_src(
        "module m;
var b: extern boolean;
procedure p;
begin
  b := true;
  b := false;
end;
",
    )
    .unwrap();
    let values: Vec<Expr> = m.functions[0]
        .body
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Store { value, .. } => value.clone(),
            k => panic!("{k:?}"),
        })
        .collect();
    let c = |value| Expr::Const {
        value,
        ty: Scalar::BOOLEAN,
    };
    assert_eq!(values, vec![c(-1), c(0)]);
    assert_eq!(
        result_value("module m;\nfunction f: boolean;\nbegin\n  f := true;\nend;\n"),
        c(-1)
    );
}

#[test]
fn widening_store_is_rejected() {
    let e = errs(
        "module m;
procedure p(var x: integer32; i: integer); options(val_param);
begin
  x := i;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:4:8: error: assigning a 2-byte value to a 4-byte object is not supported yet"]
    );
}

// ---- module data and block copies (codegen-patterns 4, 5.2) ----------

const LOADAV: &str = "module proc1_;
type
  proc1_$loadav_t = array[1..3] of integer32;
var
  loadav: proc1_$loadav_t;
";

/// PROC1_$GET_LOADAV 0xE14BBA: an exported routine that uses the unit's
/// data loads the data base; the record assignment is a block copy from
/// the data area into the object the by-address parameter points to.
#[test]
fn get_loadav() {
    let m = lower_src(&format!(
        "{LOADAV}procedure proc1_$get_loadav(out l: proc1_$loadav_t);
begin
  l := loadav;
end;
"
    ))
    .unwrap();
    assert_eq!(
        m.data,
        Some(DataArea {
            size: 12,
            globals: vec![]
        })
    );
    let f = &m.functions[0];
    assert!(f.loads_data_base);
    assert_eq!(
        f.body[0].kind,
        StmtKind::Copy {
            dst: Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Frame(8),
                    ty: Scalar::POINTER
                }),
                offset: 0
            },
            src: Addr::Static(0),
            bytes: 12
        }
    );
}

#[test]
fn static_scalars_and_defined_variables() {
    let m = lower_src(
        "module m;
var
  a: integer;
  b: integer32;
  c: integer;
define b;
procedure p; options(val_param);
begin
  c := 1;
end;
procedure q; internal;
begin
  b := 0;
end;
",
    )
    .unwrap();
    // a at 0, b at 4, c at 8: dpc-sema's layout of statics (declaration
    // order, freestanding alignment; Uncertain against the image).
    assert_eq!(
        m.data,
        Some(DataArea {
            size: 10,
            globals: vec![("B".into(), 4)]
        })
    );
    let p = &m.functions[0];
    assert!(p.loads_data_base);
    assert_eq!(
        p.body[0].kind,
        StmtKind::Store {
            addr: Addr::Static(8),
            value: Expr::Const {
                value: 1,
                ty: Scalar::signed(Width::Word)
            }
        }
    );
    // An internal routine inherits the data base (0xE17778).
    assert!(!m.functions[1].loads_data_base);
}

/// The open case of codegen-patterns 4.
#[test]
fn exported_routine_not_using_unit_data_is_rejected() {
    let e = errs(&format!(
        "{LOADAV}procedure p(out x: integer32);\nbegin\n  x := 0;\nend;\n"
    ));
    assert_eq!(e.len(), 1);
    assert!(
        e[0].starts_with(
            "t.pas:6:1: error: an exported routine that does not use its unit's static data"
        ),
        "{e:?}"
    );
    // Without unit data it is fine.
    lower_src("module m;\nprocedure p(out x: integer32);\nbegin\n  x := 0;\nend;\n").unwrap();
}

/// Records and arrays of a scalar size are moved as scalars; others are
/// block copies whatever their size (the selector decides the form).
#[test]
fn block_assignments() {
    let m = lower_src(
        "module m;
type
  r6 = record a: integer32; b: integer; end;
  r4 = record a: integer; b: integer; end;
var
  x6, y6: r6;
  x4, y4: r4;
procedure p; options(val_param);
begin
  x6 := y6;
  x4 := y4;
end;
",
    )
    .unwrap();
    let f = &m.functions[0];
    assert_eq!(
        f.body[0].kind,
        StmtKind::Copy {
            dst: Addr::Static(0),
            src: Addr::Static(8),
            bytes: 6
        }
    );
    assert_eq!(
        f.body[1].kind,
        StmtKind::Store {
            addr: Addr::Static(14),
            value: Expr::Load {
                addr: Addr::Static(18),
                ty: Scalar::unsigned(Width::Long)
            }
        }
    );
}
