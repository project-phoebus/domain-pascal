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
    let e =
        errs("module m;\nprocedure p(var x: integer);\nbegin\n  case x of 1: x := 2; end;\nend;\n");
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
    assert_eq!(zp.frame_size, 2);
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
        })
    );
    assert_eq!(
        pz.body[1].kind,
        StmtKind::Call(Call {
            callee: Callee::Local("ZERO_PAGE".into()),
            args: vec![ppn()],
            pad: 0,
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
                    slot: 4
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
        })
    );
}

#[test]
fn unsupported_for_loops() {
    let pre = "module m;
var buf: extern array[0..15] of integer32; j: extern integer;
procedure p;
var i: integer;
begin
";
    let one = |stmt: &str| errs(&format!("{pre}  {stmt}\nend;\n"));
    let shape = "error: a `for` loop other than `for i := c1 to c2 do a[i] := c` (codegen-patterns 7) is not supported yet";
    for stmt in [
        "for i := 15 downto 0 do buf[i] := 0;",
        "for i := 0 to j do buf[i] := 0;",
        "for i := 0 to 15 do buf[i] := i;",
        "for i := 0 to 14 do buf[i + 1] := 0;",
        "for i := 0 to 15 do begin buf[i] := 0; buf[i] := 1 end;",
        "for j := 0 to 15 do buf[j] := 0;",
    ] {
        let e = one(stmt);
        assert_eq!(e.len(), 1, "{stmt}: {e:?}");
        assert!(e[0].ends_with(shape), "{stmt}: {e:?}");
    }
    let e = one("for i := 3 to 2 do buf[i] := 0;");
    assert!(
        e[0].ends_with("a `for` loop that never runs is not supported yet"),
        "{e:?}"
    );
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
procedure q(in v: integer32); extern;
procedure p;
begin
  q(3);
end;
",
    );
    assert!(
        e[0].ends_with("passing a constant or an expression by address (codegen-patterns 1.5) is not supported yet"),
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
    assert!(one("while w = 0 do w := 1;").contains("a `while` loop"));
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

/// A function's result assignment inside an `if` is not its last
/// statement (codegen-patterns 3).
#[test]
fn result_assignment_in_an_if_is_rejected() {
    let e = errs(&format!(
        "{CTL}function f: integer; internal;\nbegin\n  if b then f := 1;\nend;\n"
    ));
    assert!(
        e.iter()
            .any(|m| m.contains("not the routine's last statement")),
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
    let e = match m {
        Ok(m) => panic!("{m:#?}"),
        Err(e) => e,
    };
    // LOG_$OTHER calls only a global routine, which sets A5 itself.
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].starts_with("t.pas:16:1: error: an exported routine that does not use"));
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
type pinteger = 0..65535;
var u: extern pinteger;
function f: integer32; internal;
begin
  f := u div 3;
end;
",
    );
    assert!(e[0].contains("unsigned operand (ref 4.3.3)"), "{e:?}");
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
