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
    lower_src_with(src, &DataBaseOptions::default())
}

/// [`lower_src`] with data-base options.
fn lower_src_with(src: &str, db: &DataBaseOptions) -> Result<Module, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &Options::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &M68kAbi).map_err(render)?;
    lower_with(&program, db).map_err(render)
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
    // Raw: the 1-byte result variable in a word of its own (frame objects
    // are word aligned, codegen-patterns 2).  The target rounds it.
    assert_eq!(f.frame_size, 2);
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

/// codegen-patterns 3, "Result variable": a result assignment anywhere
/// in the body is lowered to `SetResult`, in nested statements too.
#[test]
fn result_assignments_anywhere() {
    let m = lower_src(
        "module m;
function f(b: boolean): integer; options(val_param);
begin
  f := 1;
  if b then begin f := 2; return end;
  f := 3;
end;
",
    )
    .expect("lowers");
    let mut n = 0;
    crate::visit_stmts(&m.functions[0].body, &mut |s| {
        n += matches!(s.kind, StmtKind::SetResult(_)) as u32;
    });
    assert_eq!(n, 3);
}

#[test]
fn function_without_result_assignment() {
    let e = errs("module m;\nfunction f: integer;\nbegin\nend;\n");
    assert_eq!(e.len(), 1);
    assert!(
        e[0].starts_with("t.pas:2:1: error: a function with no assignment to its result")
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
    let e = errs(
        "module m;\ntype r = record a: integer; end;\nprocedure p(var x: r);\nbegin\n  with x do a := 2;\nend;\n",
    );
    assert_eq!(
        e,
        vec!["t.pas:5:3: error: this statement is not supported yet"]
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
    // A 6-byte set (sets of one word or one longword are tested in a
    // register).
    let e = errs(
        "module m;
type s16 = set of 0..40;
function f(k: integer; var s: s16): boolean; options(val_param);
begin
  f := k in s;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:5:8: error: `in` on a 6-byte set is not supported yet"]
    );
}

#[test]
fn unsupported_result_type_is_reported_once() {
    // A pointer result in A0 and D0 (`d0_return`, ref 5.5.10; no image
    // site, codegen-patterns 3) is not lowered; the assignment to it must
    // not add a second diagnostic.
    let e = errs(
        "module m;
type ip = ^integer;
function f(in p: ip): ip; options(d0_return);
begin
  f := p;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:3:1: error: a pointer result in A0 and D0 (`d0_return`, `c_param`) is not supported yet"]
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

/// Narrowing an integer32 value into an integer16 object is not
/// catalogued; widening is a conversion (0xE02C28..0xE02C2C `move.w
/// D2,D0; ext.l D0; move.l D0,(-32,A6)`).
#[test]
fn narrowing_store_is_rejected() {
    let e = errs(
        "module m;
procedure p(var x: integer; i: integer32); options(val_param);
begin
  x := i;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:4:8: error: assigning a 4-byte value to a 2-byte object is not supported yet"]
    );
    let m = lower_src(
        "module m;
procedure p(var x: integer32; i: integer); options(val_param);
begin
  x := i;
end;
",
    )
    .unwrap();
    let StmtKind::Store { value, .. } = &m.functions[0].body[0].kind else {
        panic!()
    };
    assert!(matches!(value, Expr::Convert { ty, .. } if *ty == Scalar::signed(Width::Long)));
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

/// codegen-patterns 4, used mode (the default): an exported routine that
/// does not use its unit's data does not load the data base
/// (PROC1_$TST_LOCK 0xE148CA, in a unit with data).
#[test]
fn exported_routine_not_using_unit_data_does_not_load() {
    let m = lower_src(&format!(
        "{LOADAV}procedure p(out x: integer32);\nbegin\n  x := 0;\nend;\n"
    ))
    .unwrap();
    assert!(!m.functions[0].loads_data_base);
    assert!(!m.functions[0].data_absolute);
}

/// A unit for the data-base options: data, an exported user, two
/// exported non-users, an internal user, an empty procedure.
const MODES: &str = "module m;
var
  x: integer;
procedure a_$uses;
begin
  x := 1;
end;
procedure b_$not(out y: integer);
begin
  y := 0;
end;
procedure worker; internal;
begin
  x := 0;
end;
procedure c_$empty;
begin
end;
procedure d_$not(out y: integer);
begin
  y := 1;
end;
";

fn loads(m: &Module) -> Vec<bool> {
    m.functions.iter().map(|f| f.loads_data_base).collect()
}

fn db(always_from: Option<&str>, always: &[&str], absolute: &[&str]) -> DataBaseOptions {
    DataBaseOptions {
        always_from: always_from.map(str::to_string),
        always: always.iter().map(|s| s.to_string()).collect(),
        absolute: absolute.iter().map(|s| s.to_string()).collect(),
    }
}

/// `--a5-always-from`: always mode from the named routine on, in
/// emission order, for exported routines with a frame (SMD 0xE1D750:
/// SMD_$PUTC a bare `rts`, SMD_$BUSY_WAIT 0xE1D8A4 loads A5).
#[test]
fn data_base_always_from() {
    let m = lower_src(MODES).unwrap();
    assert_eq!(loads(&m), [true, false, false, false, false]);
    let m = lower_src_with(MODES, &db(Some("B_$NOT"), &[], &[])).unwrap();
    // the internal worker inherits; the empty procedure is a bare `rts`
    assert_eq!(loads(&m), [true, true, false, false, true]);
    let m = lower_src_with(MODES, &db(Some("worker"), &[], &[])).unwrap();
    assert_eq!(loads(&m), [true, false, false, false, true]);
    let m = lower_src_with(MODES, &db(Some("a_$uses"), &[], &[])).unwrap();
    assert_eq!(loads(&m), [true, true, false, false, true]);
}

/// Emission order is body order (codegen-patterns 6, "Order": forward
/// `bsr`s such as AUDIT_$SHUTDOWN 0xE70D2E -> 0xE70D38): a routine
/// declared `forward` or `extern` (ref 7.3.4 p. 7-11) and defined later is
/// emitted at its body; one left `extern` is not emitted.  "From ROUTINE
/// on" in `--a5-always-from` follows the same order.
#[test]
fn functions_in_body_order() {
    let src = "module m;
var
  x: integer;
define m_$late;
procedure m_$late(out y: integer); extern;
procedure m_$ext; extern;
procedure m_$fwd(out y: integer); forward;
procedure m_$first(out y: integer);
begin
  m_$fwd(y);
  m_$late(y);
  m_$ext;
end;
procedure m_$late;
begin
  y := 2;
end;
procedure m_$fwd;
begin
  y := 1;
end;
";
    let m = lower_src(src).unwrap();
    let names: Vec<_> = m.functions.iter().map(|f| f.symbol.as_str()).collect();
    assert_eq!(names, ["M_$FIRST", "M_$LATE", "M_$FWD"]);
    let m = lower_src_with(src, &db(Some("m_$late"), &[], &[])).unwrap();
    assert_eq!(loads(&m), [false, true, true]);
}

/// `--a5-always`: per routine, `internal` ones too (0xE14B12, RING
/// 0xE76048); a call to a routine that loads A5 itself is not a use.
#[test]
fn data_base_always_list() {
    let m = lower_src_with(MODES, &db(None, &["d_$not", "WORKER"], &[])).unwrap();
    assert_eq!(loads(&m), [true, false, true, false, true]);
    let src = "module m;
var
  x: integer;
procedure worker; internal;
begin
  x := 0;
end;
procedure p_$caller;
begin
  worker;
end;
";
    let m = lower_src(src).unwrap();
    assert_eq!(loads(&m), [false, true]);
    let m = lower_src_with(src, &db(None, &["worker"], &[])).unwrap();
    assert_eq!(loads(&m), [true, false]);
}

/// Used mode through chains of `internal` routines (codegen-patterns 4:
/// 117 `S` routines use A5 only through a `bsr` callee, 25 through deeper
/// chains, LOG_$READ 0xE17800): a use reached through an `internal`
/// routine declared `forward` makes every caller up to the exported one
/// load A5; a recursive `internal` routine that uses nothing does not;
/// an `internal` routine that only calls an exported one (which loads A5
/// itself) is not a use.
#[test]
fn data_base_through_internal_chains() {
    let src = "module m;
var
  x: integer;
procedure leaf; forward; internal;
procedure mid; internal;
begin
  leaf;
end;
procedure leaf;
begin
  x := 1;
end;
procedure rec; internal;
begin
  rec;
end;
procedure m_$e2;
begin
  x := 2;
end;
procedure calls_e2; internal;
begin
  m_$e2;
end;
procedure m_$a;
begin
  mid;
end;
procedure m_$b;
begin
  rec;
end;
procedure m_$c;
begin
  calls_e2;
end;
";
    let m = lower_src(src).unwrap();
    let by_name = |n: &str| {
        m.functions
            .iter()
            .find(|f| f.symbol == n)
            .unwrap_or_else(|| panic!("{n}"))
            .loads_data_base
    };
    // Internal routines never load; the exported ones by what they reach.
    for (n, l) in [
        ("LEAF", false),
        ("MID", false),
        ("REC", false),
        ("CALLS_E2", false),
        ("M_$E2", true),
        ("M_$A", true),
        ("M_$B", false),
        ("M_$C", false),
    ] {
        assert_eq!(by_name(n), l, "{n}");
    }
    // An absolute routine may not reach the data through the chain either.
    let e = lower_src_with(src, &db(None, &[], &["m_$a"])).unwrap_err();
    assert!(
        e.iter()
            .any(|x| x.contains("calls an internal routine that uses the unit's data")),
        "{e:?}"
    );
}

/// `--a5-absolute`: no load, the data addressed absolutely
/// (REM_NAME_$REGISTER_SERVER 0xE4A4AE); it overrides always mode.
#[test]
fn data_base_absolute() {
    let m = lower_src_with(MODES, &db(Some("a_$uses"), &[], &["A_$USES"])).unwrap();
    assert_eq!(loads(&m), [false, true, false, false, true]);
    assert!(m.functions[0].data_absolute);
    assert!(!m.functions[1].data_absolute);
}

#[test]
fn data_base_option_errors() {
    let e = |o: DataBaseOptions| -> Vec<String> {
        match lower_src_with(MODES, &o) {
            Ok(m) => panic!("expected errors, got {m:#?}"),
            Err(e) => e,
        }
    };
    let has = |e: &[String], what: &str| assert!(e.iter().any(|x| x.contains(what)), "{e:?}");
    has(
        &e(db(Some("nope"), &[], &[])),
        "--a5-always-from: no routine `nope`",
    );
    has(
        &e(db(None, &["nope"], &[])),
        "--a5-always: no routine `nope`",
    );
    has(
        &e(db(None, &[], &["nope"])),
        "--a5-absolute: no routine `nope`",
    );
    has(
        &e(db(None, &["a_$uses"], &["a_$uses"])),
        "`A_$USES` is in both --a5-always and --a5-absolute",
    );
    has(
        &e(db(None, &["c_$empty"], &[])),
        "`C_$EMPTY` is an empty procedure",
    );
    has(
        &e(db(None, &[], &["c_$empty"])),
        "`C_$EMPTY` is an empty procedure",
    );
    has(
        &e(db(None, &[], &["worker"])),
        "`WORKER` is an internal routine",
    );
    has(
        &e(db(None, &[], &["b_$not"])),
        "--a5-absolute: `B_$NOT` does not address the unit's static data",
    );
    // A unit without data has no area to load (the 4-byte blocks with A5
    // at their end are not covered).
    let nodata = "module m;\nprocedure p(out y: integer);\nbegin\n  y := 0;\nend;\n";
    for o in [db(Some("p"), &[], &[]), db(None, &["p"], &[])] {
        let e = lower_src_with(nodata, &o).unwrap_err();
        has(&e, "the unit has no static data");
    }
    let e = lower_src_with(nodata, &db(None, &[], &["p"])).unwrap_err();
    has(&e, "does not address the unit's static data");
    // An absolute routine may not rely on an inheriting callee.
    let src = "module m;
var
  x: integer;
procedure worker; internal;
begin
  x := 0;
end;
procedure p_$caller;
begin
  x := 1;
  worker;
end;
";
    let e = lower_src_with(src, &db(None, &[], &["p_$caller"])).unwrap_err();
    has(&e, "calls an internal routine that uses the unit's data");
    // ... unless that callee loads the data base itself.
    let m = lower_src_with(src, &db(None, &["worker"], &["p_$caller"])).unwrap();
    assert_eq!(loads(&m), [true, false]);
}

/// codegen-patterns 2: an empty procedure (a bare `rts`) never loads the
/// data base.
#[test]
fn empty_procedure() {
    let m = lower_src(MODES).unwrap();
    let f = &m.functions[3];
    assert!(f.is_empty_procedure());
    assert!(!m.functions[0].is_empty_procedure());
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

const ZERO_PAGE: &str = "module ast_;
type page_t = array[0..255] of integer32;
var ast_$zero_buff: extern page_t;
procedure ml_$lock(resource_id: integer); options(val_param, extern);
procedure mmu_$install(ppn: integer32; va: univ_ptr; asid: integer;
                       prot: integer); options(val_param, extern);
procedure mmu_$remove(ppn: integer32); options(val_param, extern);
procedure zero_page(ppn: integer32); internal;
var i: integer;
begin
  mmu_$install(ppn, addr(ast_$zero_buff), 0, 16#16);
  for i := 0 to 255 do ast_$zero_buff[i] := 0;
  mmu_$remove(ppn);
end;
procedure ast_$page_zero(ppn: integer32); options(val_param);
begin
  ml_$lock(16#14);
  zero_page(ppn);
end;
";

/// Module-local 0xE00EB0 and AST_$PAGE_ZERO 0xE00EEC
/// (tests/conformance/ast_page_zero.pas).
#[test]
fn calls_and_fill() {
    let m = lower_src(ZERO_PAGE).unwrap();
    let long = Scalar::signed(Width::Long);
    let word = Scalar::signed(Width::Word);
    let ppn = || Arg::Value {
        value: Expr::Load {
            addr: Addr::Frame(8),
            ty: long,
        },
        slot: 4,
    };
    let buf = || Addr::Symbol {
        name: "AST_$ZERO_BUFF".into(),
        offset: 0,
    };
    let zp = &m.functions[0];
    assert_eq!(zp.symbol, "ZERO_PAGE");
    assert!(!zp.global);
    // The 4-byte slot of a routine that is not a global symbol, then `i`
    // (codegen-patterns 2, "Frame size").
    assert_eq!(zp.frame_size, 6);
    let kinds: Vec<&StmtKind> = zp.body.iter().map(|s| &s.kind).collect();
    assert_eq!(
        kinds,
        [
            &StmtKind::Call(Call {
                callee: Callee::External("MMU_$INSTALL".into()),
                args: vec![
                    ppn(),
                    Arg::Value {
                        value: Expr::AddrOf(buf()),
                        slot: 4
                    },
                    Arg::Value {
                        value: Expr::Const { value: 0, ty: word },
                        slot: 2
                    },
                    Arg::Value {
                        value: Expr::Const {
                            value: 0x16,
                            ty: word
                        },
                        slot: 2
                    },
                ],
                pad: 0,
                link: None,
            }),
            &StmtKind::Fill {
                start: buf(),
                count: 256,
                value: Expr::Const { value: 0, ty: long },
            },
            &StmtKind::Call(Call {
                callee: Callee::External("MMU_$REMOVE".into()),
                args: vec![ppn()],
                pad: 0,
                link: None,
            }),
        ]
    );
    let pz = &m.functions[1];
    assert_eq!(pz.symbol, "AST_$PAGE_ZERO");
    assert_eq!(
        pz.body[0].kind,
        StmtKind::Call(Call {
            callee: Callee::External("ML_$LOCK".into()),
            args: vec![Arg::Value {
                value: Expr::Const {
                    value: 0x14,
                    ty: word
                },
                slot: 2
            }],
            pad: 2,
            link: None,
        })
    );
    assert_eq!(
        pz.body[1].kind,
        StmtKind::Call(Call {
            callee: Callee::Local("ZERO_PAGE".into()),
            args: vec![ppn()],
            pad: 0,
            link: None,
        })
    );
}

/// By-address arguments: the designator's address; a byte value in a
/// 2-byte slot; the first element of a 1-based array.
#[test]
fn by_address_arguments_and_one_based_fill() {
    let m = lower_src(
        "module m;
type a_t = array[1..8] of integer;
var x: extern integer32; tab: extern a_t;
procedure q(var v: integer32; b: boolean); options(val_param, extern);
procedure p;
var i: integer;
begin
  for i := 1 to 8 do tab[i] := 0;
  q(x, true);
end;
",
    )
    .unwrap();
    let body = &m.functions[0].body;
    assert_eq!(
        body[0].kind,
        StmtKind::Fill {
            start: Addr::Symbol {
                name: "TAB".into(),
                offset: 0
            },
            count: 8,
            value: Expr::Const {
                value: 0,
                ty: Scalar::signed(Width::Word)
            },
        }
    );
    assert_eq!(
        body[1].kind,
        StmtKind::Call(Call {
            callee: Callee::External("Q".into()),
            args: vec![
                Arg::Address {
                    addr: Addr::Symbol {
                        name: "X".into(),
                        offset: 0
                    },
                    slot: 4,
                    writable: true
                },
                Arg::Value {
                    value: Expr::Const {
                        value: -1,
                        ty: Scalar::BOOLEAN
                    },
                    slot: 2
                },
            ],
            pad: 2,
            link: None,
        })
    );
}

#[test]
fn for_loops() {
    let pre = "module m;
var buf: extern array[0..15] of integer32; j: extern integer;
procedure p;
var i: integer; b: char;
begin
";
    let one = |stmt: &str| lower_src(&format!("{pre}  {stmt}\nend;\n"));
    // Not the `Fill` shape: a general `for` loop (codegen-patterns 7.4).
    for (stmt, down) in [
        ("for i := 15 downto 0 do buf[i] := 0;", true),
        ("for i := 0 to j do buf[i] := 0;", false),
        ("for i := 0 to 15 do buf[i] := i;", false),
        ("for i := 0 to 14 do buf[i + 1] := 0;", false),
        (
            "for i := 0 to 15 do begin buf[i] := 0; buf[i] := 1 end;",
            false,
        ),
    ] {
        let m = one(stmt).unwrap_or_else(|e| panic!("{stmt}: {e:?}"));
        let StmtKind::For(l) = &m.functions[0].body[0].kind else {
            panic!("{stmt}: {:?}", m.functions[0].body)
        };
        assert_eq!(l.down, down, "{stmt}");
        assert_eq!(l.index.ty, Scalar::signed(Width::Word), "{stmt}");
        assert!(!l.body.is_empty(), "{stmt}");
    }
    let e = one("for j := 0 to 15 do buf[j] := 0;").unwrap_err();
    assert!(
        e[0].ends_with(
            "a `for` index variable that is not a local of the routine (ref p. 4-89) is not \
             supported yet"
        ),
        "{e:?}"
    );
    let e = one("for b := 'a' to 'z' do j := 0;").unwrap_err();
    assert!(e[0].contains("that is not a word"), "{e:?}");
    // The index is the loop's register only (ref p. 4-89): no reads
    // outside its loops, no assignments or addresses inside them.
    for stmt in [
        "for i := 0 to 15 do j := 0; j := i;",
        "for i := 0 to 15 do i := 3;",
        "for i := 0 to 15 do for i := 1 to 2 do j := 0;",
    ] {
        let e = one(stmt).unwrap_err();
        assert!(
            e[0].contains("the index variable of a `for` loop read outside the loop"),
            "{stmt}: {e:?}"
        );
    }
    let e = one("for i := 3 to 2 do buf[i] := 0;").unwrap_err();
    assert!(
        e[0].ends_with("a `for` loop that never runs is not supported yet"),
        "{e:?}"
    );
}

/// `while`, `exit` and `next` (ref pp. 4-208, 4-77, 4-123).
#[test]
fn while_exit_next() {
    let b = ctl_body("while w = 0 do begin if b then exit; if b2 then next; w := 1 end;");
    let StmtKind::While { cond, body } = &b[0].kind else {
        panic!("{b:?}")
    };
    assert!(matches!(cond, Cond::Compare { op: Rel::Eq, .. }));
    assert_eq!(body.len(), 3);
    let StmtKind::If { then_, .. } = &body[0].kind else {
        panic!()
    };
    assert_eq!(then_[0].kind, StmtKind::Exit);
    let StmtKind::If { then_, .. } = &body[1].kind else {
        panic!()
    };
    assert_eq!(then_[0].kind, StmtKind::Next);
}

#[test]
fn unsupported_calls() {
    let e = errs(
        "module m;
procedure q(var v: integer); options(val_param, extern);
procedure p;
begin
  q(3);
end;
",
    );
    assert!(e[0].contains("a variable is needed"), "{e:?}");
    let e = errs(
        "module m;
type s = set of 0..15;
procedure q(in v: s); extern;
procedure p;
begin
  q([1]);
end;
",
    );
    assert!(
        e[0].ends_with(
            "a constant of 2 bytes or of this kind passed by address is not supported yet"
        ),
        "{e:?}"
    );
    let e = errs(
        "module m;
var w: extern integer;
procedure q(v: integer32); options(val_param, extern);
procedure p;
begin
  q(w);
end;
",
    );
    assert!(
        e[0].ends_with("passing a 2-byte value to a 4-byte parameter is not supported yet"),
        "{e:?}"
    );
    let e = errs(
        "module m;
procedure q(a, b: integer); options(val_param, variable, extern);
procedure p;
begin
  q(1);
end;
",
    );
    assert!(e[0].contains("fewer arguments than parameters"), "{e:?}");
}

// ---- control flow (codegen-patterns 7) -------------------------------

const CTL: &str = "module ctl;
type
  pinteger = 0..65535;
var
  w: extern integer;
  l: extern integer32;
  u: extern pinteger;
  p: extern ^integer;
  b, b2: extern boolean;
";

/// The statement part of `body` in procedure `t` of a unit with `CTL`'s
/// declarations, lowered.
fn ctl_body(body: &str) -> Vec<Stmt> {
    let m = lower_src(&format!(
        "{CTL}procedure t; internal;\nbegin\n{body}\nend;\n"
    ))
    .unwrap();
    m.functions[0].body.clone()
}

fn ctl_errs(body: &str) -> Vec<String> {
    errs(&format!(
        "{CTL}procedure t; internal;\nbegin\n{body}\nend;\n"
    ))
}

fn sym(name: &str, ty: Scalar) -> Expr {
    Expr::Load {
        addr: Addr::Symbol {
            name: name.into(),
            offset: 0,
        },
        ty,
    }
}

const W: Scalar = Scalar::signed(Width::Word);
const UW: Scalar = Scalar::unsigned(Width::Word);

/// `if` with and without `else`, nested; `begin ... end` is flattened.
#[test]
fn if_statements() {
    let b = ctl_body(
        "if w = 1 then w := 2 else begin w := 3; return end;
         if u < 16#8000 then begin begin w := 4 end end;",
    );
    assert_eq!(b.len(), 2);
    let StmtKind::If { cond, then_, else_ } = &b[0].kind else {
        panic!("{:?}", b[0].kind)
    };
    assert_eq!(
        *cond,
        Cond::Compare {
            op: Rel::Eq,
            lhs: sym("W", W),
            rhs: Expr::Const { value: 1, ty: W }
        }
    );
    assert_eq!(then_.len(), 1);
    assert_eq!(else_.len(), 2);
    assert_eq!(else_[1].kind, StmtKind::Return);
    let StmtKind::If { cond, then_, else_ } = &b[1].kind else {
        panic!()
    };
    // The constant takes the unsigned word type of the other operand
    // (0xE1779A `cmpi.w #0x400,D1`).
    assert_eq!(
        *cond,
        Cond::Compare {
            op: Rel::Lt,
            lhs: sym("U", UW),
            rhs: Expr::Const {
                value: 0x8000,
                ty: UW
            }
        }
    );
    assert_eq!(then_.len(), 1);
    assert!(else_.is_empty());
}

#[test]
fn conditions() {
    let cond = |c: &str| {
        let b = ctl_body(&format!("if {c} then w := 0;"));
        let StmtKind::If { cond, .. } = &b[0].kind else {
            panic!()
        };
        cond.clone()
    };
    let bool_b = sym("B", Scalar::BOOLEAN);
    let bool_b2 = sym("B2", Scalar::BOOLEAN);
    assert_eq!(cond("b"), Cond::Test(bool_b.clone()));
    assert_eq!(
        cond("not b"),
        Cond::Not(Box::new(Cond::Test(bool_b.clone())))
    );
    assert_eq!(
        cond("b and then b2"),
        Cond::AndThen(
            Box::new(Cond::Test(bool_b.clone())),
            Box::new(Cond::Test(bool_b2.clone()))
        )
    );
    assert_eq!(
        cond("b or else b2"),
        Cond::OrElse(Box::new(Cond::Test(bool_b)), Box::new(Cond::Test(bool_b2)))
    );
    // nil is the pointer 0.
    assert_eq!(
        cond("p = nil"),
        Cond::Compare {
            op: Rel::Eq,
            lhs: sym("P", Scalar::POINTER),
            rhs: Expr::Const {
                value: 0,
                ty: Scalar::POINTER
            }
        }
    );
    // A constant on the left takes the right operand's type.
    assert_eq!(
        cond("3 < w"),
        Cond::Compare {
            op: Rel::Lt,
            lhs: Expr::Const { value: 3, ty: W },
            rhs: sym("W", W)
        }
    );
    // Two variables of one type.
    assert_eq!(
        cond("l >= l"),
        Cond::Compare {
            op: Rel::Ge,
            lhs: sym("L", Scalar::signed(Width::Long)),
            rhs: sym("L", Scalar::signed(Width::Long))
        }
    );
}

#[test]
fn unsupported_conditions_and_loops() {
    let one = |body: &str| {
        let e = ctl_errs(body);
        assert_eq!(e.len(), 1, "{e:?}");
        e[0].clone()
    };
    assert!(one("if b and b2 then w := 0;").contains("`and`/`or` in a condition"));
    assert!(one("if w < l then w := 0;")
        .ends_with("comparing a 2-byte signed value with a 4-byte signed one (implicit widening) is not supported yet"));
    assert!(one("if w < u then w := 0;").contains("implicit widening"));
    assert!(one("if u < 70000 then w := 0;").ends_with(
        "comparing with a constant outside the other operand's type is not supported yet"
    ));
    assert!(one("if u > -1 then w := 0;").contains("outside the other operand's type"));
    assert!(one("while w = 0 do ;").contains("a `while` loop with an empty body"));
    assert!(one("if w = 0 then ;").contains("an `if` with an empty `then` part"));
    assert!(one("if true then w := 0;").contains("a constant condition"));
    assert!(one("if b = true then w := 0;").contains("comparing boolean values"));
    assert!(one("if b <> b2 then w := 0;").contains("comparing boolean values"));
}

#[test]
fn repeat_and_return() {
    let b = ctl_body("repeat w := 1; w := 2 until b; return;");
    assert_eq!(b.len(), 2);
    let StmtKind::Repeat { body, until } = &b[0].kind else {
        panic!()
    };
    assert_eq!(body.len(), 2);
    assert_eq!(*until, Cond::Test(sym("B", Scalar::BOOLEAN)));
    assert_eq!(b[1].kind, StmtKind::Return);
}

/// `case` (ref "Case", pp. 4-36..4-37): arms and labels in source order,
/// an empty arm kept with no statements, a `char` selector widened to an
/// unsigned word (codegen-patterns 7.3: `clr.w Dn; move.b x,Dn` at
/// module-local 0xE1CC6C).
#[test]
fn case_statements() {
    let b = ctl_body("case w of 3, 1: w := 0; 2: ; otherwise begin w := 1; w := 2 end end;");
    assert_eq!(b.len(), 1);
    let StmtKind::Case {
        selector,
        arms,
        otherwise,
    } = &b[0].kind
    else {
        panic!()
    };
    assert_eq!(*selector, sym("W", Scalar::signed(Width::Word)));
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].labels, vec![3, 1]);
    assert_eq!(arms[0].body.len(), 1);
    assert_eq!(arms[1].labels, vec![2]);
    assert!(arms[1].body.is_empty());
    assert_eq!(otherwise.as_ref().map(Vec::len), Some(2));

    let pre = "module m;\nvar c: extern char; w: extern integer;\nprocedure t; internal;\nbegin\n";
    let m = lower_src(&format!(
        "{pre}case c of 'a': w := 1; 'b': w := 2; end;\nend;\n"
    ))
    .unwrap();
    let StmtKind::Case { selector, arms, .. } = &m.functions[0].body[0].kind else {
        panic!()
    };
    assert_eq!(
        *selector,
        Expr::Convert {
            value: Box::new(sym("C", Scalar::unsigned(Width::Byte))),
            ty: Scalar::unsigned(Width::Word),
        }
    );
    assert_eq!(arms[0].labels, vec![97]);
}

#[test]
fn unsettled_case_statements_are_rejected() {
    let one = |body: &str| {
        let e = ctl_errs(body);
        assert_eq!(e.len(), 1, "{e:?}");
        e[0].clone()
    };
    assert!(one("case b of true: w := 1; false: w := 2; end;")
        .contains("a signed byte (boolean, `-128..127`) `case` selector"));
    assert!(one("case 3 of 1: w := 1; 3: w := 2; end;").contains("a constant `case` selector"));
    assert!(
        one("case w of 1: w := 1; 2: w := 2; otherwise end;").contains("an empty `otherwise` part")
    );
    assert!(one("case u of 1: w := 1; -1: w := 2; end;")
        .contains("case label -1 is outside the selector's range"));
}

/// An assignment to an enclosing function's result from a nested
/// routine is refused (the result variable is held in a register,
/// codegen-patterns 3).
#[test]
fn enclosing_result_assignment_is_rejected() {
    let e = errs(
        "module m;
function f: integer;
  procedure g;
  begin
    f := 1;
  end;
begin
  g;
  f := 2;
end;
",
    );
    assert!(
        e.iter()
            .any(|m| m.contains("assignment to an enclosing function's result")),
        "{e:?}"
    );
}

/// codegen-patterns 4: an exported routine loads the data base when it
/// uses the unit's data through an internal callee that inherits A5
/// (LOG_$READ 0xE17806, worker 0xE17778); a global callee loads its own.
#[test]
fn data_base_through_internal_callees() {
    let m = lower_src(
        "module log_;
var
  x: integer;
procedure worker; internal;
begin
  x := 0;
end;
procedure middle; internal;
begin
  worker;
end;
procedure log_$read;
begin
  if x = 1 then middle;
end;
procedure log_$other;
begin
  log_$read;
end;
",
    );
    // LOG_$OTHER calls only a global routine, which sets A5 itself: in
    // used mode it loads nothing.
    let m = m.unwrap();
    assert_eq!(loads(&m), [false, false, true, false]);
    let m = lower_src(
        "module log_;
var
  x: integer;
procedure worker; internal;
begin
  x := 0;
end;
procedure middle; internal;
begin
  worker;
end;
procedure log_$read;
begin
  middle;
end;
",
    )
    .unwrap();
    let loads: Vec<bool> = m.functions.iter().map(|f| f.loads_data_base).collect();
    assert_eq!(loads, [false, false, true]);
}

/// By-address parameter slots are listed for the selector
/// (codegen-patterns 12.2).
#[test]
fn address_params_are_listed() {
    let m = lower_src(
        "module m;
procedure p(var a: integer; b: integer; in c: integer32; out d: integer); options(val_param);
begin
  a := 0;
end;
",
    )
    .unwrap();
    let f = &m.functions[0];
    // `in c` of 4 bytes goes by value under val_param (ref 5.5.7, p. 5-19).
    assert_eq!(f.address_params, [8, 0x12]);
    let values: Vec<i32> = f.value_params.iter().map(|p| p.offset).collect();
    assert_eq!(values, [0xc, 0xe]);
}

// ---- expressions (codegen-patterns 5.2, 8; beads -5aw.13, -d7i) -------

fn result_expr(src: &str) -> Expr {
    let m = lower_src(src).unwrap();
    let StmtKind::SetResult(e) = &m.functions[0].body[0].kind else {
        panic!("not a result assignment")
    };
    e.clone()
}

/// `a[i]`: the lower bound folded into the offset, the field's offset
/// added; a constant index folded into the address.
#[test]
fn array_elements() {
    let src = "module m;
type r = record a: integer32; b: integer; end;
var rs: array[1..4] of r; i: extern integer;
function f: integer; internal;
begin
  f := rs[i].b;
end;
";
    let w = Scalar::signed(Width::Word);
    assert_eq!(
        result_expr(src),
        Expr::Load {
            addr: Addr::Index {
                base: Box::new(Addr::Static(0)),
                index: Box::new(Expr::Load {
                    addr: Addr::Symbol {
                        name: "I".into(),
                        offset: 0
                    },
                    ty: w
                }),
                scale: 6,
                offset: -6 + 4,
            },
            ty: w,
        }
    );
    let src = "module m;
var ws: array[1..4] of integer;
function f: integer; internal;
begin
  f := ws[3];
end;
";
    assert_eq!(
        result_expr(src),
        Expr::Load {
            addr: Addr::Static(4),
            ty: w
        }
    );
}

/// Integer16 operands of an integer32 operation are converted; an
/// integer32 expression assigned to a word is computed at word width
/// (0xE177C4); the sum of two unsigned words is unsigned (0xE177B8).
#[test]
fn widths_and_signs_of_operations() {
    let l = Scalar::signed(Width::Long);
    let src = "module m;
var a: extern integer; b: extern integer32;
function f: integer32; internal;
begin
  f := a + b;
end;
";
    let Expr::Binary { op, lhs, ty, .. } = result_expr(src) else {
        panic!()
    };
    assert_eq!((op, ty), (BinOp::Add, l));
    assert!(matches!(*lhs, Expr::Convert { ty, .. } if ty == l));
    let src = "module m;
type pinteger = 0..65535;
var u: extern pinteger;
function f: pinteger; internal;
begin
  f := 16#400 - u;
end;
";
    let Expr::Binary { op, lhs, rhs, ty } = result_expr(src) else {
        panic!()
    };
    assert_eq!((op, ty), (BinOp::Sub, Scalar::unsigned(Width::Word)));
    assert_eq!(
        *lhs,
        Expr::Const {
            value: 0x400,
            ty: Scalar::unsigned(Width::Word)
        }
    );
    assert!(matches!(*rhs, Expr::Load { ty, .. } if ty == Scalar::unsigned(Width::Word)));
    let src = "module m;
type pinteger = 0..65535;
var u, v: extern pinteger;
function f: boolean; internal;
begin
  f := u + v > 16#400;
end;
";
    let Expr::Test(c) = result_expr(src) else {
        panic!()
    };
    let Cond::Compare { lhs, .. } = *c else {
        panic!()
    };
    assert_eq!(lhs.ty(), Scalar::unsigned(Width::Long));
}

#[test]
fn unsupported_operations() {
    let e = errs(
        "module m;
var a: extern integer;
function f: integer; internal;
begin
  f := rshft(a, 2);
end;
",
    );
    assert!(e[0].contains("`rshft` of a signed operand"), "{e:?}");
    let e = errs(
        "module m;
var c: extern char; s: array[char] of integer;
function f: integer; internal;
begin
  f := s[c];
end;
",
    );
    assert!(e[0].contains("an array index of one byte"), "{e:?}");
}

/// Ref 4.3.3, pp. 4-6..4-7: `div` and `mod` are unsigned when both
/// operands are unsigned run-time values, or one is and the other a
/// constant in 0..2147483647; signed otherwise.
#[test]
fn div_mod_signedness() {
    let src = |e: &str| {
        format!(
            "module m;
type pinteger = 0..65535;
var u, v: extern pinteger; w: extern integer;
function f: integer32; internal;
begin
  f := {e};
end;
"
        )
    };
    let op_ty = |e: &str| match result_value(&src(e)) {
        Expr::Convert { value, .. } => value.ty(),
        v => v.ty(),
    };
    // A `0..65535` word has an integer32 host: the operation is a
    // longword one, its operands zero-extended words.
    assert_eq!(op_ty("u div 3"), Scalar::unsigned(Width::Long));
    assert_eq!(op_ty("u mod v"), Scalar::unsigned(Width::Long));
    assert_eq!(op_ty("u div w"), Scalar::signed(Width::Long));
    assert_eq!(op_ty("w mod 3"), Scalar::signed(Width::Word));
}

/// codegen-patterns 8: a word sum, difference or product as the dividend
/// of `div`/`mod` is divided in 32 bits (CAL_$WEEKDAY 0xE686AE, RIP_$SERVER
/// 0xE68AE2, TPAD_$DATA 0xE69396), the result narrowed back to a word.
#[test]
fn word_sum_dividend_is_divided_in_32_bits() {
    let e = result_value(
        "module m;
var w: extern integer;
function f: integer; internal;
begin
  f := (w + 9) mod 12;
end;
",
    );
    let Expr::Convert { value, ty } = e else {
        panic!("{e:?}")
    };
    assert_eq!(ty, Scalar::signed(Width::Word));
    let Expr::Binary { op, lhs, rhs, ty } = *value else {
        panic!()
    };
    assert_eq!((op, ty), (BinOp::Mod, Scalar::signed(Width::Long)));
    // The sum's operands are widened, not the word sum.
    assert!(
        matches!(*lhs, Expr::Binary { op: BinOp::Add, ty, .. } if ty == Scalar::signed(Width::Long))
    );
    assert_eq!(
        *rhs,
        Expr::Const {
            value: 12,
            ty: Scalar::signed(Width::Long)
        }
    );
    // A variable dividend stays a word division.
    let e = result_value(
        "module m;
var w: extern integer;
function f: integer; internal;
begin
  f := w div 12;
end;
",
    );
    assert!(
        matches!(e, Expr::Binary { op: BinOp::Div, ty, .. } if ty == Scalar::signed(Width::Word))
    );
}

/// codegen-patterns 8: a longword `div` narrowed to a word also as an
/// operand of a narrowed sum: module-local 0xE077DA at 0xE078CC..0xE078D6
/// `move.l A3,D0; sub.l #0xd94c00,D0; divu.w #48,D0; addq.w #1,D0`;
/// PROC2_$GET_CR_REC 0xE40186..0xE40190 (`divs.w #24`, `addq.w #1`).  The
/// quotient is right only when it fits in 16 bits, as at those sites.
#[test]
fn longword_div_narrowed_inside_a_sum() {
    let e = result_value(
        "module m;
var l: extern integer32;
function f: integer; internal;
begin
  f := (l div 48) + 1;
end;
",
    );
    let Expr::Binary { op, lhs, ty, .. } = e else {
        panic!("{e:?}")
    };
    assert_eq!((op, ty), (BinOp::Add, Scalar::signed(Width::Word)));
    assert!(
        matches!(&*lhs, Expr::Convert { value, ty } if *ty == Scalar::signed(Width::Word)
            && matches!(**value, Expr::Binary { op: BinOp::Div, .. })),
        "{lhs:?}"
    );
}

/// Appendix E (pp. E-3..E-5) and "Discard" (p. 4-62): `disable`/`enable`
/// statements, `set_sr` values, `discard` of a call or of `set_sr`
/// (codegen-patterns 10; bead domain-pascal-5aw.7).
#[test]
fn interrupt_builtins_lower() {
    let m = lower_src(
        "module m;
var w: extern integer; d: extern integer32;
function ext_$f(x: integer): integer; options(val_param, extern);
procedure p;
var old: integer;
begin
  disable;
  old := set_sr(16#F8FF);
  enable;
  discard(set_sr(old));
  discard(ext_$f(w));
end;
",
    )
    .unwrap();
    let f = &m.functions[0];
    let k: Vec<&StmtKind> = f.body.iter().map(|s| &s.kind).collect();
    assert_eq!(k[0], &StmtKind::Disable);
    let w = Scalar::signed(Width::Word);
    assert_eq!(
        k[1],
        &StmtKind::Store {
            addr: Addr::Frame(-2),
            value: Expr::SetSr(Box::new(Expr::Const {
                value: -0x701,
                ty: w
            })),
        }
    );
    assert_eq!(k[2], &StmtKind::Enable);
    assert_eq!(
        k[3],
        &StmtKind::Discard(Expr::SetSr(Box::new(Expr::Load {
            addr: Addr::Frame(-2),
            ty: w
        })))
    );
    assert!(matches!(k[4], StmtKind::Call(_)));
    assert_eq!(f.scalar_locals, vec![ValueParam { offset: -2, ty: w }]);
}

/// "Succ"/"Pred" (pp. 4-191, 4-147) of a run-time value: `x + 1` / `x -
/// 1` at x's type; `max`, `min`, `in_range` of run-time values and
/// `discard` of other expressions are refused (no image pattern).
#[test]
fn ordinal_builtins_lower() {
    let e = result_value(
        "module m; var w: extern integer;
function f: integer; begin f := succ(w) end;",
    );
    assert!(matches!(
        e,
        Expr::Binary {
            op: BinOp::Add,
            ty: Scalar {
                width: Width::Word,
                ..
            },
            ..
        }
    ));
    for (src, needle) in [
        ("w := max(w, 3)", "`max`, `min` or `in_range`"),
        ("b := in_range(s)", "`max`, `min` or `in_range`"),
        ("b := succ(b)", "this predeclared function is"),
        ("discard(w + 1)", "`discard` of an expression"),
    ] {
        let e = errs(&format!(
            "module m; type small = 1..5; var w: extern integer; b: extern boolean;
             s: extern small;
procedure p; begin {src} end;"
        ));
        assert!(e.iter().any(|m| m.contains(needle)), "{src}: {e:?}");
    }
}

// ---- nested routines (codegen-patterns 6) ----------------------------

/// The PMAP_$FLUSH shape (0xE1376C): two nested routines, the first
/// called by the parent and by its sibling (static link passed), the
/// second by the parent only (the caller's frame).
const NEST: &str = "module nst;
type
  rec_t = record
    a: integer32;
    seg: integer;
  end;
procedure ext_$p(var r: rec_t; v: integer32); options(val_param); extern;
procedure ext_$w(w: integer); options(val_param); extern;
procedure par(var r: rec_t; flags: integer; count: integer); options(val_param);
var
  n: integer;
  done: integer;

  procedure first(v: integer32);
  begin
    ext_$p(r, v);
    ext_$w(flags);
  end;

  procedure second;
  begin
    ext_$w(n);
    first(5);
    n := 0;
  end;

begin
  n := count;
  done := count;
  second;
  first(7);
  ext_$w(done);
end;
";

fn func<'a>(m: &'a Module, sym: &str) -> &'a Function {
    m.functions.iter().find(|f| f.symbol == sym).unwrap()
}

fn calls_of(f: &Function) -> Vec<(String, Option<LinkArg>)> {
    let mut out = Vec::new();
    for st in &f.body {
        if let StmtKind::Call(c) = &st.kind {
            let (Callee::Local(s) | Callee::External(s)) = &c.callee;
            out.push((s.clone(), c.link));
        }
    }
    out
}

#[test]
fn nested_routines_order_and_static_links() {
    let m = lower_src(NEST).unwrap();
    // Nested routines first, in source order, then the parent (0xE1359C,
    // 0xE1360C, PMAP_$FLUSH 0xE1376C).
    let order: Vec<&str> = m.functions.iter().map(|f| f.symbol.as_str()).collect();
    assert_eq!(order, ["FIRST", "SECOND", "PAR"]);
    let first = func(&m, "FIRST");
    let second = func(&m, "SECOND");
    assert!(!first.global && !second.global);
    // Called by its sibling too: the link is passed (0xE1359C, A1).
    assert_eq!(
        first.nesting,
        Some(Nesting {
            parent: "PAR".into(),
            link: StaticLink::Passed
        })
    );
    // Called by the parent only: the caller's frame (0xE1360C, `(A6)`).
    assert_eq!(
        second.nesting.as_ref().unwrap().link,
        StaticLink::CallerFrame
    );
    assert_eq!(func(&m, "PAR").nesting, None);
    // The parent passes its frame, the sibling its own link (0xE13928,
    // 0xE13718); a call of a routine whose link is not passed has none.
    assert_eq!(
        calls_of(func(&m, "PAR")),
        [
            ("SECOND".to_string(), None),
            ("FIRST".to_string(), Some(LinkArg::OwnFrame)),
            ("EXT_$W".to_string(), None)
        ]
    );
    assert_eq!(
        calls_of(second),
        [
            ("EXT_$W".to_string(), None),
            ("FIRST".to_string(), Some(LinkArg::OwnLink))
        ]
    );
}

#[test]
fn uplevel_references() {
    let m = lower_src(NEST).unwrap();
    let first = func(&m, "FIRST");
    // `r`, the parent's by-address parameter, passed on: its slot through
    // the static link (0xE135B8 `move.l (8,A2),-(SP)`); `flags`, a value
    // parameter, read in the enclosing frame (0xE135AA).
    let StmtKind::Call(c) = &first.body[0].kind else {
        panic!()
    };
    assert_eq!(
        c.args[0],
        Arg::Address {
            addr: Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Enclosing(8),
                    ty: Scalar::unsigned(Width::Long)
                }),
                offset: 0
            },
            slot: 4,
            writable: true
        }
    );
    let StmtKind::Call(c) = &first.body[1].kind else {
        panic!()
    };
    assert!(matches!(
        &c.args[0],
        Arg::Value {
            value: Expr::Load {
                addr: Addr::Enclosing(12),
                ..
            },
            ..
        }
    ));
    // The parent's local `n` read and written by `second` (0xE1362E).
    let second = func(&m, "SECOND");
    assert!(matches!(
        &second.body[2].kind,
        StmtKind::Store {
            addr: Addr::Enclosing(-2),
            ..
        }
    ));
    // The parent lists what its nested routines reach; `n` is not a
    // register candidate, `done` is (bead domain-pascal-thc).
    let par = func(&m, "PAR");
    assert_eq!(par.uplevel_refs, [(-2, 2), (8, 4), (12, 2)]);
    let locals: Vec<i32> = par.scalar_locals.iter().map(|l| l.offset).collect();
    assert_eq!(locals, [-4]);
}

#[test]
fn nested_frames_reserve_the_link_slot() {
    // codegen-patterns 6: a nested routine's locals start below a 4-byte
    // slot at -4 (0xE1359C `link #-4`, no locals).
    let m = lower_src(
        "module m;
procedure p; options(val_param);
var a: integer;
  procedure q;
  var b: integer;
  begin
    b := 1;
    a := b;
  end;
  procedure e;
  begin
    a := 0;
  end;
begin
  q;
  e;
end;
",
    )
    .unwrap();
    assert_eq!(func(&m, "Q").frame_size, 6);
    assert_eq!(func(&m, "Q").scalar_locals[0].offset, -6);
    assert_eq!(func(&m, "E").frame_size, 4);
    assert_eq!(func(&m, "P").frame_size, 2);
}

#[test]
fn recursive_nested_routine_passes_its_link() {
    // 0xE397D0, 0xE39BC2, 0xE4F3BA, 0xE50D5E call themselves with
    // `movea.l An,A1`, An copied from A1 at entry.
    let m = lower_src(
        "module m;
procedure p; options(val_param);
var k: integer;
  procedure r;
  begin
    k := k - 1;
    if k > 0 then r;
  end;
begin
  k := 3;
  r;
end;
",
    )
    .unwrap();
    assert_eq!(
        func(&m, "R").nesting.as_ref().unwrap().link,
        StaticLink::Passed
    );
    assert_eq!(
        calls_of(func(&m, "P")),
        [("R".to_string(), Some(LinkArg::OwnFrame))]
    );
    let StmtKind::If { then_, .. } = &func(&m, "R").body[1].kind else {
        panic!()
    };
    assert!(matches!(&then_[0].kind, StmtKind::Call(c) if c.link == Some(LinkArg::OwnLink)));
}

#[test]
fn link_passed_only_to_routines_that_need_it() {
    // A nested routine that reaches no enclosing frame gets no link even
    // when a sibling or itself calls it: ROUTE_$SERVICE's nested 0xE69E40,
    // called by its sibling 0xE69EC2 at 0xE69F3C with no A1 load.  `mid'
    // needs its link only to pass it on to `leaf' (transitively).
    let m = lower_src(
        "module m;
var g: integer;
procedure p; options(val_param);
var k: integer;
  procedure free(x: integer);
  begin
    g := x;
    if x > 0 then free(x - 1);
  end;
  procedure leaf;
  begin
    k := 0;
  end;
  procedure mid;
  begin
    free(1);
    leaf;
  end;
  procedure top;
  begin
    mid;
  end;
begin
  k := 3;
  free(2);
  leaf;
  mid;
  top;
end;
",
    )
    .unwrap();
    let link = |s: &str| func(&m, s).nesting.as_ref().unwrap().link;
    assert_eq!(link("FREE"), StaticLink::CallerFrame);
    assert_eq!(link("LEAF"), StaticLink::Passed);
    assert_eq!(link("MID"), StaticLink::Passed);
    assert_eq!(link("TOP"), StaticLink::CallerFrame);
    assert_eq!(
        calls_of(func(&m, "P")),
        [
            ("FREE".to_string(), None),
            ("LEAF".to_string(), Some(LinkArg::OwnFrame)),
            ("MID".to_string(), Some(LinkArg::OwnFrame)),
            ("TOP".to_string(), None),
        ]
    );
    assert_eq!(
        calls_of(func(&m, "MID")),
        [
            ("FREE".to_string(), None),
            ("LEAF".to_string(), Some(LinkArg::OwnLink))
        ]
    );
    assert_eq!(
        calls_of(func(&m, "TOP")),
        [("MID".to_string(), Some(LinkArg::OwnLink))]
    );
}

#[test]
fn unsupported_nesting() {
    // Two levels: outer frames through a chain of links (DIR 0xE4F034).
    let e = errs(
        "module m;
procedure p;
  procedure q;
    procedure r;
    begin
    end;
  begin
  end;
begin
end;
",
    );
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(
        e[0].starts_with("t.pas:4:5: error: a routine nested in a nested routine"),
        "{e:?}"
    );
    // A nested routine declared `forward`.
    let e = errs(
        "module m;
procedure p;
  procedure q; forward;
  procedure q;
  begin
  end;
begin
  q;
end;
",
    );
    assert!(
        e[0].contains("a nested routine declared `forward`"),
        "{e:?}"
    );
    // Two nested routines of one name: the local symbols collide.
    let e = errs(
        "module m;
procedure p;
  procedure h;
  begin
  end;
begin
  h;
end;
procedure q;
  procedure h;
  begin
  end;
begin
  h;
end;
",
    );
    assert!(e[0].contains("two routines named `H` in one unit"), "{e:?}");
    // A `for` index a nested routine reaches (RINGLOG_$CNTL 0xE72296: kept
    // in the frame, not implemented).
    let e = errs(
        "module m;
var w: extern integer;
procedure p;
var i: integer;
  procedure q;
  begin
    w := i;
  end;
begin
  for i := 1 to 5 do q;
end;
",
    );
    assert!(
        e[0].starts_with("t.pas:10:3: error: a `for` loop whose index a nested routine reaches"),
        "{e:?}"
    );
    // A nested routine's loop over the enclosing routine's variable
    // (0xE721F0, through the static link).
    let e = errs(
        "module m;
var w: extern integer;
procedure p;
var i: integer;
  procedure q;
  begin
    for i := 1 to 5 do w := i;
  end;
begin
  q;
end;
",
    );
    assert!(
        e[0].contains("a `for` index variable that is not a local"),
        "{e:?}"
    );
}

/// Constants passed by address (codegen-patterns 1.5, bead
/// domain-pascal-5aw.9): a cell of the parameter's type and size, a byte
/// in the high byte of a word, the type's identity as the key.
#[test]
fn constants_by_address_are_pool_cells() {
    let m = lower_src(
        "module m;
type e = (e0, e1, e2);
procedure q(a: integer; in b: integer32; c: boolean; d: e; f: char); extern;
procedure p;
begin
  q(0, -2, true, e2, 'A');
end;
",
    )
    .unwrap();
    let StmtKind::Call(c) = &func(&m, "P").body[0].kind else {
        panic!()
    };
    let cells: Vec<Vec<u8>> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address {
                addr: Addr::Const(k),
                slot: 4,
                writable: false,
            } => k.bytes.clone(),
            a => panic!("{a:?}"),
        })
        .collect();
    assert_eq!(
        cells,
        [
            vec![0, 0],
            vec![0xff, 0xff, 0xff, 0xfe],
            vec![0xff, 0],
            vec![0, 2],
            vec![0x41, 0]
        ]
    );
    // The key is the type: two parameter types, two keys.
    let keys: Vec<u32> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address {
                addr: Addr::Const(k),
                ..
            } => k.key,
            _ => unreachable!(),
        })
        .collect();
    assert_ne!(keys[0], keys[1]);
}

/// Expressions passed by address (codegen-patterns 1.5, TONE_$ENABLE
/// 0xE1ACE8): a frame temporary below the locals, per call in push order
/// (the last argument's highest), reused by the next call.
#[test]
fn expressions_by_address_get_frame_temporaries() {
    let m = lower_src(
        "module m;
var x: integer32; w: integer;
procedure q(a: univ_ptr; b: integer; c: univ_ptr); extern;
procedure p;
var s: integer;
begin
  q(addr(x), w + 1, addr(w));
  q(addr(x), 2, addr(w));
end;
",
    )
    .unwrap();
    let f = func(&m, "P");
    let temps = |k: usize| -> Vec<i32> {
        let StmtKind::Call(c) = &f.body[k].kind else {
            panic!()
        };
        c.args
            .iter()
            .filter_map(|a| match a {
                Arg::Temp { offset, .. } => Some(*offset),
                _ => None,
            })
            .collect()
    };
    // s at -2 (map-named routine: no reserved slot); c's temporary first,
    // longword aligned, then b's word, then a's.
    assert_eq!(temps(0), [-16, -10, -8]);
    assert_eq!(temps(1), [-12, -8]);
    assert_eq!(f.frame_size, 16);
}

/// `Arg::Address::writable` (bead domain-pascal-pe9): set for `var`,
/// `out` and `in out` parameters (ref 5.3.1 p. 5-6, 5.3.2 p. 5-7), clear
/// for value and `in` parameters passed by address (ref 5.2 p. 5-4).
#[test]
fn by_address_arguments_writable_by_mode() {
    let m = lower_src(
        "module m;
var x: extern integer;
procedure q(var a: integer; out b: integer; in out c: integer; in d: integer; e: integer);
  extern;
procedure p;
begin
  q(x, x, x, x, x);
end;
",
    )
    .unwrap();
    let StmtKind::Call(c) = &func(&m, "P").body[0].kind else {
        panic!()
    };
    let w: Vec<bool> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address { writable, .. } => *writable,
            Arg::Value { .. } | Arg::Temp { .. } => panic!("{a:?}"),
        })
        .collect();
    assert_eq!(w, [true, true, true, false, false]);
}
