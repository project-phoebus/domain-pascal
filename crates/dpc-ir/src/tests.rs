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
    fn predeclared_cond_vars(&self) -> &'static [(&'static str, bool)] {
        &[]
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

/// Set operators on register-sized sets (codegen-patterns 5.1): the
/// constructor `[7]` (a `set of 0..7`, ref 3.9.3) takes the operation's
/// type, a constant is its mask (member k is bit k), `[k]` a
/// [`Expr::SetOf`] of the word member.
#[test]
fn set_operators() {
    let src = "module m;
type
  e = (e0, e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16);
  r = record s: set of e; w: set of 0..15; end;
  rp = ^r;
procedure t(p: rp; k: e); options(val_param);
begin
  p^.s := p^.s - [e1, e16];
  p^.w := [];
  p^.s := p^.s + [k];
end;
";
    let m = lower_src(src).unwrap();
    let f = &m.functions[0];
    let long = Scalar::unsigned(Width::Long);
    let s = || {
        Box::new(Expr::Load {
            addr: Addr::Indirect {
                base: Box::new(Expr::Load {
                    addr: Addr::Frame(8),
                    ty: Scalar::POINTER,
                }),
                offset: 0,
            },
            ty: long,
        })
    };
    let StmtKind::Store { value, .. } = &f.body[0].kind else {
        panic!("a store")
    };
    assert_eq!(
        *value,
        Expr::SetOp {
            op: SetOp::Diff,
            lhs: s(),
            rhs: Box::new(Expr::Const {
                value: 0x1_0002,
                ty: long
            }),
            ty: long,
        }
    );
    let StmtKind::Store { value, .. } = &f.body[1].kind else {
        panic!("a store")
    };
    assert_eq!(
        *value,
        Expr::Const {
            value: 0,
            ty: Scalar::unsigned(Width::Word)
        }
    );
    let StmtKind::Store { value, .. } = &f.body[2].kind else {
        panic!("a store")
    };
    assert_eq!(
        *value,
        Expr::SetOp {
            op: SetOp::Union,
            lhs: s(),
            rhs: Box::new(Expr::SetOf {
                elem: Box::new(Expr::Load {
                    addr: Addr::Frame(12),
                    ty: Scalar::unsigned(Width::Word)
                }),
                ty: long
            }),
            ty: long,
        }
    );
}

/// `c in s` for a `set of char` (32 bytes) in memory: the char widened
/// to a word; refused for an element type beyond the set's bits.
#[test]
fn membership_of_large_sets() {
    let src = "module m;
type r = record s: set of char; t: set of 'a'..'z'; end; rp = ^r;
var g: extern integer;
procedure t(p: rp; c: char); options(val_param);
begin
  g := 0;
  if c in p^.s then g := 1;
end;
";
    let m = lower_src(src).unwrap();
    let StmtKind::If { cond, .. } = &m.functions[0].body[1].kind else {
        panic!("an if")
    };
    let Cond::Test(Expr::SetMemberAt { elem, set, bytes }) = cond else {
        panic!("{cond:?}")
    };
    assert_eq!(*bytes, 32);
    assert_eq!(
        **elem,
        Expr::Convert {
            value: Box::new(Expr::Load {
                addr: Addr::Frame(12),
                ty: Scalar::unsigned(Width::Byte)
            }),
            ty: Scalar::unsigned(Width::Word)
        }
    );
    assert!(matches!(set, Addr::Indirect { offset: 0, .. }));
    // `set of 'a'..'z'` is 16 bytes (ref 3.9.3, p. 3-17): a char may lie
    // beyond its 128 bits.
    let e = errs(&src.replace("c in p^.s", "c in p^.t"));
    assert_eq!(e.len(), 1);
    assert!(e[0].contains("guarded test"), "{e:?}");
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

/// Value parameters under the default convention are passed by address
/// and copied by the callee (ref 5.2 Table 5-1, p. 5-5): the function
/// lists the copies in parameter order, and the body reads, writes and
/// passes on the copy, never the caller's object (bead domain-pascal-5aw.12;
/// DIR_$GET_ENTRYU 0xE4D500).
#[test]
fn value_parameter_copies() {
    let m = lower_src(
        "module m;
type uid_$t = record high, low: integer32; end;
procedure q(var u: uid_$t; var n: integer); extern;
procedure p(u: uid_$t; in k: integer; n: integer);
var l: integer;
begin
  n := n + 1;
  l := n;
  q(u, n);
end;
",
    )
    .expect("lowers");
    let f = &m.functions[0];
    let copies: Vec<(i32, i32, u32)> = f
        .param_copies
        .iter()
        .map(|c| (c.slot, c.copy, c.bytes))
        .collect();
    // `u` at (8,A6) copied to -8..-1, `n` at (0x10,A6) to -10; the `in`
    // parameter `k` is not copied; the local `l` comes after the copies.
    assert_eq!(copies, vec![(8, -8, 8), (16, -10, 2)]);
    assert_eq!(f.address_params, vec![8, 12, 16]);
    let w = Scalar::signed(Width::Word);
    let ld = |o| Expr::Load {
        addr: Addr::Frame(o),
        ty: w,
    };
    // The assignment goes to the copy, not through the slot.
    assert_eq!(
        f.body[0].kind,
        StmtKind::Store {
            addr: Addr::Frame(-10),
            value: Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(ld(-10)),
                rhs: Box::new(Expr::Const { value: 1, ty: w }),
                ty: w,
            },
        }
    );
    assert_eq!(
        f.body[1].kind,
        StmtKind::Store {
            addr: Addr::Frame(-12),
            value: ld(-10),
        }
    );
    // Passed on by address: the copy's address.
    let StmtKind::Call(c) = &f.body[2].kind else {
        panic!("a call")
    };
    let addrs: Vec<&Addr> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address { addr, .. } => addr,
            _ => panic!("by address"),
        })
        .collect();
    assert_eq!(addrs, vec![&Addr::Frame(-8), &Addr::Frame(-10)]);
}

#[test]
fn unsupported_constructs_have_positions() {
    // An ordinary statement.
    let e = errs(
        "module m;\ntype r = record a: integer; end;\nprocedure p(var x: r);\nbegin\n  with x do a := 2;\nend;\n",
    );
    assert_eq!(
        e,
        vec!["t.pas:5:3: error: this statement is not supported yet"]
    );
    // Module data in an overlay section.
    let e = errs("module m;\nvar (sect) v: integer;\n");
    assert_eq!(
        e,
        vec!["t.pas:2:12: error: variables in an overlay section (`var (name)`, ref 3.14 p. 3-52) are not supported yet"]
    );

    // A 6-byte set tested with an integer element, which may lie beyond
    // its 48 bits (the guarded form, codegen-patterns 5.1).
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
        vec![
            "t.pas:5:8: error: `in` on a 6-byte set with an element whose type reaches beyond \
             the set's 48 bits (the guarded test, NAME_$INIT 0xE3183E, codegen-patterns 5.1) is \
             not supported yet"
        ]
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
        [DataArea {
            section: ".data".into(),
            size: 12,
            globals: vec![],
            init: vec![]
        }]
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
        [DataArea {
            section: ".data".into(),
            size: 10,
            globals: vec![("B".into(), 4)],
            init: vec![]
        }]
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
        always_from: always_from.map(str::to_string).into_iter().collect(),
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
    // A signed and an unsigned word: both widened, compared as signed
    // longwords (TTY_$K_PUT 0xE67C42; ref 4.3.1 p. 4-5), so that -1 <
    // 40000 holds.
    let long = Scalar::signed(Width::Long);
    let widened = |name: &str, ty: Scalar| Expr::Convert {
        value: Box::new(sym(name, ty)),
        ty: long,
    };
    assert_eq!(
        cond("w < u"),
        Cond::Compare {
            op: Rel::Lt,
            lhs: widened("W", W),
            rhs: widened("U", UW)
        }
    );
    assert_eq!(
        cond("u > w"),
        Cond::Compare {
            op: Rel::Gt,
            lhs: widened("U", UW),
            rhs: widened("W", W)
        }
    );
    // A word against a longword: the word widened by its sign.
    assert_eq!(
        cond("w < l"),
        Cond::Compare {
            op: Rel::Lt,
            lhs: widened("W", W),
            rhs: sym("L", long)
        }
    );
    assert_eq!(
        cond("l = u"),
        Cond::Compare {
            op: Rel::Eq,
            lhs: sym("L", long),
            rhs: widened("U", UW)
        }
    );
}

/// `=` and `<>` of records (ref Table 4-1, p. 4-2): [`Cond::Block`] over
/// the record's bytes; records with padding are refused.
#[test]
fn record_comparisons() {
    let src = |body: &str| {
        format!(
            "module m;
type
  uid_t = record high, low: integer32; end;
  pad_t = record c: char; w: integer; end;
  arr_t = array[1..4] of integer;
var
  a, b: extern uid_t;
  x, y: extern pad_t;
  v1, v2: extern arr_t;
  w: extern integer;
procedure t; internal;
begin
{body}
end;
"
        )
    };
    let m = lower_src(&src("if a <> b then w := 0;")).unwrap();
    let StmtKind::If { cond, .. } = &m.functions[0].body[0].kind else {
        panic!()
    };
    let at = |n: &str| Addr::Symbol {
        name: n.into(),
        offset: 0,
    };
    assert_eq!(
        *cond,
        Cond::Block {
            op: Rel::Ne,
            lhs: at("A"),
            rhs: at("B"),
            size: 8
        }
    );
    let e = errs(&src("if x = y then w := 0;"));
    assert!(e[0].contains("bytes no field covers"), "{e:?}");
    let e = errs(&src("if a < b then w := 0;"));
    assert!(e[0].contains("ordering records"), "{e:?}");
    // Arrays: accepted by the checker (Table 4-1, "All types"), refused
    // here (the unit of the image's array loops is not settled, 0xE6C278
    // `cmpm.b`).
    let e = errs(&src("if v1 = v2 then w := 0;"));
    assert!(
        e[0].contains("comparing values other than ordinals and pointers"),
        "{e:?}"
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
var c: extern boolean; s: array[boolean] of integer;
function f: integer; internal;
begin
  f := s[c];
end;
",
    );
    assert!(
        e[0].contains("an array index of a signed byte type"),
        "{e:?}"
    );
    // A `char` index is zero-extended to a word (codegen-patterns 5.2,
    // "Byte indices").
    let m = lower_src(
        "module m;
var c: extern char; s: extern array[char] of integer; w: extern integer;
procedure t; internal;
begin
  w := s[c];
end;
",
    )
    .unwrap();
    let StmtKind::Store { value, .. } = &m.functions[0].body[0].kind else {
        panic!()
    };
    let Expr::Load {
        addr: Addr::Index { index, scale, .. },
        ..
    } = value
    else {
        panic!("{value:?}")
    };
    assert_eq!(*scale, 2);
    assert_eq!(
        **index,
        Expr::Convert {
            value: Box::new(sym("C", Scalar::unsigned(Width::Byte))),
            ty: Scalar::unsigned(Width::Word)
        }
    );
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
            let (Callee::Local(s) | Callee::External(s)) = &c.callee else {
                panic!("a call through a routine pointer")
            };
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

/// Calls through routine pointers (bead domain-pascal-5aw.8;
/// codegen-patterns 1.6): the callee is the pointer's value, the
/// conventions and the pad come from the pointer type (`val_param` on the
/// type, ref p. 4-146), and `writable` from its parameter modes (bead
/// domain-pascal-5aw.16).
#[test]
fn calls_through_routine_pointers() {
    let m = lower_src(
        "module m;
type t = ^procedure (var a: integer; out b: integer; in out c: integer; in d: integer;
  e: integer);
  v = ^procedure (in w: integer; l: integer32); val_param;
  f = ^function (b: boolean): integer32; val_param;
  pf = ^function: univ_ptr; val_param;
var x: extern integer; pt: t; pv: v; fv: f; r: integer32; pp: pf; u: univ_ptr;
procedure p;
begin
  pt^(x, x, x, x, x);
  pv^(3, r);
  r := fv^(true);
end;
procedure q;
begin
  u := pp^;
end;
",
    );
    let errs = m.as_ref().map(|_| Vec::new()).unwrap_or_else(|e| e.clone());
    // A pointer result: no compiled call through a pointer reads A0.
    assert!(
        errs.iter()
            .any(|e| e.contains("a pointer result of a call through a routine pointer")),
        "{errs:?}"
    );
    let m = lower_src(
        "module m;
type t = ^procedure (var a: integer; out b: integer; in out c: integer; in d: integer;
  e: integer);
  v = ^procedure (in w: integer; l: integer32); val_param;
  f = ^function (b: boolean): integer32; val_param;
var x: extern integer; pt: t; pv: v; fv: f; r: integer32;
procedure p;
begin
  pt^(x, x, x, x, x);
  pv^(3, r);
  r := fv^(true);
end;
",
    )
    .unwrap();
    let body = &func(&m, "P").body;
    let StmtKind::Call(c) = &body[0].kind else {
        panic!()
    };
    assert!(matches!(
        c.pointer(),
        Some(Expr::Load {
            addr: Addr::Static(0),
            ..
        })
    ));
    let w: Vec<bool> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address { writable, .. } => *writable,
            Arg::Value { .. } | Arg::Temp { .. } => panic!("{a:?}"),
        })
        .collect();
    assert_eq!(w, [true, true, true, false, false]);
    assert_eq!(c.pad, 0);
    // val_param: both by value, 2 + 4 bytes and a pad.
    let StmtKind::Call(c) = &body[1].kind else {
        panic!()
    };
    assert!(
        c.args.iter().all(|a| matches!(a, Arg::Value { .. })),
        "{c:?}"
    );
    assert_eq!(c.pad, 2);
    let StmtKind::Store {
        value: Expr::Call { call, reg, .. },
        ..
    } = &body[2].kind
    else {
        panic!("{:?}", body[2])
    };
    assert_eq!(*reg, ResultClass::Data);
    assert!(matches!(call.callee, Callee::Pointer(_)));
    // The address of a routine is its symbol's.
    let m = lower_src(
        "module m;
type v = ^procedure (in w: integer); val_param;
var pv: v;
procedure h (in w: integer); val_param; extern;
procedure p;
begin
  pv := addr(h);
end;
",
    )
    .unwrap();
    let StmtKind::Store { value, .. } = &func(&m, "P").body[0].kind else {
        panic!()
    };
    assert_eq!(
        *value,
        Expr::AddrOf(Addr::Symbol {
            name: "H".into(),
            offset: 0
        })
    );
    // A function call inside the pointer's address would run after the
    // pushes (not seen in compiled code): refused.
    for stmt in ["tbl[h]^(5)", "gp^.f^(5)"] {
        let e = lower_src(&format!(
            "module m;
type v = ^procedure (in w: integer32); val_param;
     rec = record f: v; end;
     rp = ^rec;
var tbl: array[1..4] of v;
function h: integer32; extern;
function gp: rp; extern;
procedure p;
begin
  {stmt};
end;
"
        ))
        .unwrap_err();
        assert!(
            e.iter().any(|m| m.contains(
                "a function call in the routine pointer of a call through a pointer is not \
                 supported yet"
            )),
            "{e:?}"
        );
    }
}

/// A constant passed by address to a `univ` parameter (bead
/// domain-pascal-4be; ref 5.3.3 p. 5-11: an expression argument is
/// extended to the size of the `univ` parameter): an integer constant for
/// a larger integer parameter gets a cell of the parameter's type and
/// size, holding the constant's value; equal sizes keep the constant's
/// own type.
#[test]
fn univ_constants_take_the_parameter_size() {
    let m = lower_src(
        "module m;
type r = record a, b: integer; end;
  u = 0..65535;
procedure q(in a: univ integer32; in b: univ integer; in c: univ integer32; in d: univ r;
  in e: univ integer32);
  extern;
procedure p;
begin
  q(0, 7, -2, 65536, lastof(u));
end;
",
    )
    .unwrap();
    let StmtKind::Call(c) = &func(&m, "P").body[0].kind else {
        panic!()
    };
    let cells: Vec<(Vec<u8>, u32)> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address {
                addr: Addr::Const(k),
                slot: 4,
                writable: false,
            } => (k.bytes.clone(), k.key),
            a => panic!("{a:?}"),
        })
        .collect();
    let bytes: Vec<&[u8]> = cells.iter().map(|(b, _)| b.as_slice()).collect();
    assert_eq!(
        bytes,
        [
            &[0, 0, 0, 0][..],
            &[0, 7],
            &[0xff, 0xff, 0xff, 0xfe],
            &[0, 1, 0, 0],
            &[0, 0, 0xff, 0xff]
        ]
    );
    // 0 and -2 are both converted to integer32: one key, not integer16's.
    assert_eq!(cells[0].1, cells[2].1);
    assert_ne!(cells[0].1, cells[1].1);
    // 65536 is an integer32 constant for a 4-byte record: its own type.
    assert_eq!(cells[3].1, cells[0].1);
    // The conversion keeps the value: `lastof` of a 2-byte 0..65535 is
    // 65535, extended as `0000 ffff`, not sign-extended from `ffff`.
    assert_eq!(cells[4].1, cells[0].1);
}

/// What `univ` extension is not supported yet (bead domain-pascal-4be):
/// a constant larger than the parameter, a non-integer constant or a
/// non-integer parameter of another size, and an expression temporary of
/// another size than the parameter.
#[test]
fn univ_size_mismatches_are_refused() {
    let cases = [
        (
            "in a: univ integer",
            "100000",
            "4-byte constant of this type to a 2-byte `univ` parameter",
        ),
        (
            "in a: univ integer32",
            "true",
            "1-byte constant of this type to a 4-byte `univ` parameter",
        ),
        (
            "in a: univ r",
            "0",
            "2-byte constant of this type to a 4-byte `univ` parameter",
        ),
        (
            "in a: univ integer32",
            "w + 1",
            "passing a 2-byte expression by address to a 4-byte parameter",
        ),
    ];
    for (param, arg, msg) in cases {
        let e = errs(&format!(
            "module m;
type r = record a, b: integer; end;
var w: extern integer;
procedure q({param}); extern;
procedure p;
begin
  q({arg});
end;
"
        ));
        assert!(e[0].starts_with("t.pas:7:5: error: "), "{e:?}");
        assert!(e[0].contains(msg), "{e:?}");
        assert!(e[0].ends_with("not supported yet"), "{e:?}");
    }
}

/// Type transfers (ref "Type Transfer Functions", pp. 4-197..4-199): no
/// code between types of one size (a load or a constant retyped, anything
/// else in a same-width [`Expr::Convert`]); between integers a conversion
/// as in an assignment (widening by the operand's sign; narrowing only
/// where the value can be computed at the narrower width).
#[test]
fn type_transfers() {
    let src = "module m;
type
  pint = 0..65535;
  colour = (red, green, blue);
  wptr = ^integer;
var
  p: wptr;
  q: univ_ptr;
  l: integer32;
  w, x: integer;
  u: pint;
  c: colour;
procedure s(offset: pint); internal;
begin
  q := univ_ptr(integer32(p) + offset);
  l := integer32(p);
  p := wptr(l);
  l := integer32(u);
  w := integer(u);
  c := colour(w);
  w := integer16(u + x);
  p := univ_ptr(16#10000);
end;";
    let m = lower_src(src).unwrap();
    let body = &m.functions[0].body;
    let value = |i: usize| match &body[i].kind {
        StmtKind::Store { value, .. } => value.clone(),
        k => panic!("not a store: {k:?}"),
    };
    let long = Scalar::signed(Width::Long);
    // integer32(p) + offset: the pointer's load retyped, the offset
    // zero-extended, the sum seen as a pointer through a same-width
    // conversion (no code).
    let Expr::Convert { value: sum, ty } = value(0) else {
        panic!("not a retyped sum")
    };
    assert_eq!(ty, Scalar::POINTER);
    let Expr::Binary {
        op: BinOp::Add,
        lhs,
        rhs,
        ty,
    } = *sum
    else {
        panic!("not a sum")
    };
    assert_eq!(ty, long);
    assert!(matches!(*lhs, Expr::Load { ty, .. } if ty == long));
    assert!(matches!(*rhs, Expr::Convert { ref value, ty }
        if ty == long && value.ty() == Scalar::unsigned(Width::Word)));
    assert!(matches!(value(1), Expr::Load { ty, .. } if ty == long));
    assert!(matches!(value(2), Expr::Load { ty, .. } if ty == Scalar::POINTER));
    assert!(matches!(value(3), Expr::Convert { ty, .. } if ty == long));
    assert!(matches!(value(4), Expr::Load { ty, .. } if ty == Scalar::signed(Width::Word)));
    assert!(matches!(value(5), Expr::Load { ty, .. } if ty == Scalar::unsigned(Width::Word)));
    // integer16(u + x), a longword sum (u is a `0..65535` word): computed
    // at the word's width (p. 4-198).
    assert!(matches!(value(6), Expr::Binary { ty, .. } if ty == Scalar::signed(Width::Word)));
    assert!(matches!(value(7), Expr::Const { value: 0x10000, ty } if ty == Scalar::POINTER));
    // A constant transferred keeps its bits, read as the new type (sema's
    // fold, as the code computes it at run time): `0..65535(-1)` is 65535,
    // `integer16(0..65535(65535))` -1, `integer16(16#18000)` truncated.
    let m = lower_src(
        "module m; type pint = 0..65535; var l: integer32;
         procedure s; begin
           l := integer32(pint(-1));
           l := integer32(integer16(pint(65535)));
           l := integer32(integer16(16#18000));
           l := integer32(integer16(-1));
         end;",
    )
    .unwrap();
    let consts: Vec<i64> = m.functions[0]
        .body
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Store {
                value: Expr::Const { value, .. },
                ..
            } => *value,
            k => panic!("not a constant store: {k:?}"),
        })
        .collect();
    assert_eq!(consts, [0xffff, -1, -0x8000, -1]);
    // Narrowing a plain longword load is not catalogued, as in an
    // assignment.
    let e = errs(
        "module m; var l: integer32; w: integer;
         procedure s; begin w := integer(l) end;",
    );
    assert!(
        e[0].contains("type transfer of a 4-byte integer to a 2-byte one"),
        "{e:?}"
    );
}

// ---- named sections (codegen-patterns 11; ref 5.7.1, 7.1.1) ----

const SECTIONS: &str = "module s, wired_proc, wired_data;
var
  hits: integer32;
procedure s_$count; internal;
begin
  hits := 1;
end;
function s_$hits: integer32; internal;
begin
  s_$hits := hits;
end;
procedure s_$tick;
begin
  s_$count;
end;
[section(init_proc, init_data)]
procedure s_$init;
var
  boots: static integer32;
begin
  s_$count;
  boots := s_$hits;
end;
procedure s_$late;
begin
  hits := 2;
end;
";

fn callees(f: &Function) -> Vec<Callee> {
    let mut out = Vec::new();
    for s in &f.body {
        let mut calls = Vec::new();
        crate::lower::stmt_calls(s, &mut calls);
        out.extend(calls.into_iter().map(|c| c.callee.clone()));
    }
    out
}

/// One data area per data section, each routine with its code and data
/// section; calls into another code section, from a statement and from
/// an expression, are absolute (bead domain-pascal-86o); the internal
/// routine called from another data section loads its own base (0xE14B12
/// from PROC1_$INIT 0xE2F9EC), the caller of the same section that only
/// calls it does not.
#[test]
fn named_sections() {
    let m = lower_src(SECTIONS).unwrap();
    assert_eq!(
        m.data,
        [
            DataArea {
                section: "wired_data".into(),
                size: 4,
                globals: vec![],
                init: vec![]
            },
            DataArea {
                section: "init_data".into(),
                size: 4,
                globals: vec![],
                init: vec![]
            }
        ]
    );
    let secs: Vec<(&str, &str, &str)> = m
        .functions
        .iter()
        .map(|f| {
            (
                f.symbol.as_str(),
                f.code_section.as_str(),
                f.data_section.as_str(),
            )
        })
        .collect();
    assert_eq!(
        secs,
        [
            ("S_$COUNT", "wired_proc", "wired_data"),
            ("S_$HITS", "wired_proc", "wired_data"),
            ("S_$TICK", "wired_proc", "wired_data"),
            ("S_$INIT", "init_proc", "init_data"),
            ("S_$LATE", "wired_proc", "wired_data"),
        ]
    );
    assert_eq!(callees(&m.functions[2]), [Callee::Local("S_$COUNT".into())]);
    assert_eq!(
        callees(&m.functions[3]),
        [
            Callee::Section("S_$COUNT".into()),
            Callee::Section("S_$HITS".into())
        ]
    );
    assert_eq!(loads(&m), [true, true, false, true, true]);
    assert_eq!(m.functions[3].body.len(), 2);
}

/// A routine reaches only its own data section's block through the data
/// base; another section's data is addressed absolutely in the image
/// (PROC1_$INIT 0xE2F95C `movea.l #0xE254E8,A0`), not supported.
#[test]
fn static_of_another_data_section() {
    let e = errs(
        "module s, wired_proc, wired_data;
var
  hits: integer32;
[section(init_proc, init_data)]
procedure s_$init;
begin
  hits := 0;
end;
",
    );
    assert_eq!(
        e,
        vec![
            "t.pas:7:3: error: a static variable of data section `wired_data` in a routine whose \
              data section is `init_data` is not supported yet"
        ]
    );
    // A nested routine has its top-level routine's data base.
    let e = errs(
        "module s;
var
  hits: integer32;
procedure s_$p;
  [section(, other_data)] procedure q;
  var
    n: static integer;
  begin
    n := 1;
  end;
begin
  q;
end;
",
    );
    assert_eq!(
        e,
        vec![
            "t.pas:9:5: error: a static variable of data section `other_data` in a routine \
              whose data section is `.data` is not supported yet"
        ]
    );
}

#[test]
fn nested_routine_in_another_code_section() {
    let e = errs(
        "module s;
procedure s_$p;
  [section(other_proc)] procedure q;
  begin
  end;
begin
  q;
end;
",
    );
    assert_eq!(
        e,
        vec![
            "t.pas:3:3: error: a nested routine in another code section than its enclosing \
              routine is not supported yet"
        ]
    );
}

/// `--a5-always-from` switches each named routine's code section, from
/// it on in the unit's order within the section (one switch per map
/// segment); two names in one section are an error.
#[test]
fn data_base_always_from_per_section() {
    let src = "module s, wired_proc, wired_data;
var
  x: integer;
procedure s_$a(out y: integer);
begin
  y := 0;
end;
[section(init_proc, init_data)]
procedure s_$b(out y: integer);
var
  z: static integer;
begin
  y := 0;
end;
procedure s_$c(out y: integer);
begin
  y := 0;
end;
";
    let o = |names: &[&str]| DataBaseOptions {
        always_from: names.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    let m = lower_src_with(src, &o(&["s_$c"])).unwrap();
    assert_eq!(loads(&m), [false, false, true]);
    let m = lower_src_with(src, &o(&["s_$a"])).unwrap();
    assert_eq!(loads(&m), [true, false, true]);
    let m = lower_src_with(src, &o(&["s_$c", "s_$b"])).unwrap();
    assert_eq!(loads(&m), [false, true, true]);
    let e = lower_src_with(src, &o(&["s_$a", "s_$c"])).unwrap_err();
    assert!(
        e[0].contains("two routines of code section `wired_proc`"),
        "{e:?}"
    );
}

/// A quoted overlay section name (ref 3.14, p. 3-52) is refused twice
/// over: as an overlay and for its characters.
#[test]
fn overlay_section_name() {
    let e = errs("module m;\nvar ('a b') v: integer;\n");
    assert!(e.iter().any(|x| x.contains("overlay section")), "{e:?}");
    assert!(e.iter().any(|x| x.contains("section name `a b`")), "{e:?}");
}

/// Bit fields of packed records (ref 3.10.7): the unit that holds each
/// (`lower.rs`, `bit_field`; codegen-patterns 5.3), and fields that are
/// whole objects read as plain loads.
#[test]
fn packed_field_units() {
    let m = lower_src(
        "module m;
         type
           status_$t = packed record case integer of
             1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
             2: (all: integer32);
           end;
           r = packed record
             x: 0..255; b: boolean; depth: 0..255; low: 0..32767;
           end;
         var g: extern integer;
         procedure t(var s: status_$t; var q: r);
         begin
           if s.fail then g := s.code;
           g := q.depth;
           g := s.subsys;
         end;",
    )
    .unwrap();
    let f = &m.functions[0];
    let StmtKind::If { cond, then_, .. } = &f.body[0].kind else {
        panic!("{:?}", f.body[0].kind)
    };
    // `fail`: bit 7 of the first byte.
    assert!(matches!(
        cond,
        Cond::Test(Expr::Bits {
            field: BitField {
                unit: Width::Byte,
                shift: 7,
                width: 1
            },
            ty: Scalar::BOOLEAN,
            ..
        })
    ));
    // `code`: a whole word at 2, a plain load.
    let StmtKind::Store { value, .. } = &then_[0].kind else {
        panic!()
    };
    assert!(matches!(
        value,
        Expr::Load {
            addr: Addr::Indirect { offset: 2, .. },
            ..
        }
    ));
    // `depth`, bits 9..16: the longword at 0, bits 22..15 (APP_$RECEIVE
    // 0xE0086E `andi.l #-0x7f8001,(38,A2)`).
    let StmtKind::Store { value, .. } = &f.body[1].kind else {
        panic!()
    };
    assert!(matches!(
        value,
        Expr::Bits {
            addr: Addr::Indirect { offset: 0, .. },
            field: BitField {
                unit: Width::Long,
                shift: 15,
                width: 8
            },
            ..
        }
    ));
    // `subsys`, bits 1..7: the first byte, bits 6..0.
    let StmtKind::Store { value, .. } = &f.body[2].kind else {
        panic!()
    };
    assert!(matches!(
        value,
        Expr::Bits {
            field: BitField {
                unit: Width::Byte,
                shift: 0,
                width: 7
            },
            ..
        }
    ));
}

#[test]
fn packed_fields_refused() {
    let e = errs(
        "module m;
         type r = packed record a: boolean; s: -8..7 end;
         var g: extern integer;
         procedure t(var q: r); begin g := q.s end;",
    );
    assert!(e[0].contains("negative values"), "{e:?}");
}

#[test]
fn initial_values_of_module_data() {
    // ref 3.2.2, 3.5.1, 3.6.2, 3.7, 3.9.2, 3.10.4, 3.11.1, 3.13.4: each
    // store of the flattened initialiser, at its offset in the block.
    use crate::{DataInit, InitValue};
    let m = lower_src(
        "module m;
type
  colour = (red, green, blue);
  st = record case integer of 1: (all: integer32); 2: (w: integer) end;
  rec = record c: char; n: integer; end;
var
  i: integer := -2;
  l: integer32 := 16#120010;
  b: boolean := true;
  f: boolean := false;
  c: char := 'A';
  e: colour := blue;
  s: set of 0..31 := [0, 3..5, 31];
  p: ^integer := nil;
  a: array[1..4] of integer := [2 of 7, * of 7];
  t: array[1..6] of char := 'abc';
  r: rec := [n := 5, c := 'z'];
  v: array[0..1] of st := [[all := 16#B000E], [w := 1]];
procedure z;
begin
  i := 0;
end;
",
    )
    .unwrap();
    let int = |offset, size, value| DataInit {
        offset,
        value: InitValue::Int { size, value },
    };
    let bytes = |offset, b: &[u8]| DataInit {
        offset,
        value: InitValue::Bytes(b.to_vec()),
    };
    assert_eq!(m.data.len(), 1);
    assert_eq!(
        m.data[0].init,
        vec![
            int(0, 2, -2),
            // longword aligned (dpc-sema's freestanding layout)
            int(4, 4, 0x120010),
            int(8, 1, 0xFF),
            int(9, 1, 0),
            int(10, 1, 0x41),
            int(12, 2, 2),
            // members 0, 3, 4, 5 in the last byte, 31 in the first
            bytes(14, &[0x80, 0, 0, 0x39]),
            int(20, 4, 0),
            int(24, 2, 7),
            int(26, 2, 7),
            int(28, 2, 7),
            int(30, 2, 7),
            // the first three elements only, not blank-padded
            bytes(32, b"abc"),
            int(40, 2, 5),
            int(38, 1, 0x7A),
            // variant fields by name
            int(44, 4, 0xB000E),
            int(48, 2, 1),
        ]
    );
}

#[test]
fn one_character_initialises_a_char_array() {
    // ref 3.11.1.1 p. 3-39: `blank_line : STRING := chr(10)' initialises
    // element 1 only; a char literal likewise.
    use crate::{DataInit, InitValue};
    let m = lower_src(
        "module m;
var
  nl: array[1..80] of char := chr(10);
  x: array[1..3] of char := 'x';
procedure z;
begin
end;
",
    )
    .unwrap();
    assert_eq!(
        m.data[0].init,
        vec![
            DataInit {
                offset: 0,
                value: InitValue::Bytes(vec![10]),
            },
            DataInit {
                offset: 80,
                value: InitValue::Bytes(vec![b'x']),
            },
        ]
    );
}

/// The single statement of the single routine of `src`.
fn only_stmt(src: &str) -> StmtKind {
    let m = lower_src(src).unwrap_or_else(|e| panic!("{e:#?}"));
    m.functions[0].body[0].kind.clone()
}

/// References to volatile objects are wrapped in `Addr::Volatile`
/// (ref 3.15.1-3.15.3), with the attribute inherited from records and
/// arrays and through dereferences (3.15.7); addresses only taken are not.
#[test]
fn volatile_references_are_wrapped() {
    let types = "module m;
type
  lock = [volatile] integer;
  queue = record key: lock; users: integer end;
  volque = [volatile] queue;
  regs = [device] record csr: char; dcr: char end;
  vint = [atomic] integer;
var
  q: queue;
  vq: volque;
  r: extern ^regs;
  a: vint;
  i: integer;
procedure x(var z: integer); extern;
";
    let vol = |src: &str| match only_stmt(&format!("{types}{src}")) {
        StmtKind::Store { addr, .. } => addr.unwrap_volatile().1,
        s => panic!("{s:?}"),
    };
    let dflt = Access::default();
    // A volatile field; not its neighbour (p. 3-77: "references to
    // wait.key are volatile ... references to wait.users are not").
    assert_eq!(vol("procedure p; begin q.key := 1 end;"), Some(dflt));
    assert_eq!(vol("procedure p; begin q.users := 1 end;"), None);
    // `volque = [VOLATILE] queue`: every field.
    assert_eq!(vol("procedure p; begin vq.users := 1 end;"), Some(dflt));
    // Through a pointer to a device record.
    assert_eq!(
        vol("procedure p; begin r^.dcr := 'a' end;"),
        Some(Access {
            device: true,
            atomic: false
        })
    );
    assert_eq!(
        vol("procedure p; begin a := 1 end;"),
        Some(Access {
            device: false,
            atomic: true
        })
    );
    // A load as well.
    match only_stmt(&format!("{types}procedure p; begin i := q.key end;")) {
        StmtKind::Store {
            value: Expr::Load { addr, .. },
            ..
        } => assert!(matches!(addr, Addr::Volatile { .. })),
        s => panic!("{s:?}"),
    }
    // The address of a volatile object passed by address: not a reference.
    match only_stmt(&format!("{types}procedure p; begin x(q.key) end;")) {
        StmtKind::Call(c) => {
            assert!(matches!(c.args[0], Arg::Address { ref addr, .. }
                if !matches!(addr, Addr::Volatile { .. })))
        }
        s => panic!("{s:?}"),
    }
}

/// ref 3.15.3 p. 3-59: `device(read)` objects may not be written,
/// `device(write)` ones not read; a device record's packed field is not
/// stored into (a read-modify-write of its unit); a record with volatile
/// parts is not block-copied.
#[test]
fn device_access_restrictions() {
    let types = "module m;
type
  ro = [device(read)] char;
  wo = [device(write)] char;
  pr = [device] packed record a: 0..15; b: 0..15 end;
  vr = record a: [volatile] integer32; b: integer32; c: integer32 end;
var
  r: extern ro;
  w: extern wo;
  c: char;
  p: extern pr;
  v1, v2: vr;
";
    let e = errs(&format!("{types}procedure x; begin r := 'a' end;"));
    assert!(e[0].contains("writing a read-only device object"), "{e:?}");
    let e = errs(&format!("{types}procedure x; begin c := w end;"));
    assert!(e[0].contains("reading a write-only device object"), "{e:?}");
    lower_src(&format!("{types}procedure x; begin c := r; w := c end;")).unwrap();
    let e = errs(&format!("{types}procedure x; begin p.a := 1 end;"));
    assert!(e[0].contains("packed field of a device record"), "{e:?}");
    let e = errs(&format!("{types}procedure x; begin v1 := v2 end;"));
    assert!(e[0].contains("volatile or device parts"), "{e:?}");
}

/// Only locals of an ordinal, set or pointer type are register
/// candidates; a record or an array of 4 bytes or less is not
/// (codegen-patterns 12.5: the image keeps `status_$t` locals in the
/// frame, MST_$ALLOC_ASID 0xE42DB2).
#[test]
fn record_and_array_locals_are_not_scalar_locals() {
    let m = lower_src(
        "module m;
type st = record all: integer32; end;
  colour = (red, green);
procedure p;
var s: st; a: array[1..2] of integer; f: set of 0..15; c: char;
  k: colour; n: 0..65535; q: ^integer; b: boolean;
begin
  s.all := 0; a[1] := 0; f := []; c := 'x'; k := red; n := 1;
  b := true;
end;",
    )
    .unwrap();
    let f = &m.functions[0];
    let mut offs: Vec<i32> = f.scalar_locals.iter().map(|l| l.offset).collect();
    offs.sort();
    // s at -4, a at -8: left out; the six others are candidates.
    assert_eq!(offs.len(), 6, "{offs:?}");
    assert!(!offs.contains(&-4) && !offs.contains(&-8), "{offs:?}");
}

/// A volatile local is not offered to the selector as a register
/// candidate (codegen-patterns 12.5); ref 3.15.1's example `i: [volatile]
/// integer`.
#[test]
fn volatile_locals_stay_in_the_frame() {
    let m = lower_src(
        "module m;
procedure p;
var i: [volatile] integer; j: integer32; k: integer;
begin
  i := 0; j := 1; k := i;
end;",
    )
    .unwrap();
    let f = &m.functions[0];
    let offs: Vec<i32> = f.scalar_locals.iter().map(|l| l.offset).collect();
    assert_eq!(offs.len(), 2, "{offs:?}");
    assert!(!offs.contains(&-2), "{offs:?}");
}

/// A byte-aligned word at an odd offset (ref 3.15.6.3 Figure 3-24: `b,
/// c: [ALIGNED(0)] integer16` after a char, at 1 and 3) is not
/// referenced with a word instruction (the 68010 traps); at an even
/// offset it is.
#[test]
fn byte_aligned_words_at_odd_addresses_are_refused() {
    let types = "module m;
type
  s2 = record a: char; b, c: [aligned(0)] integer16 end;
  s3 = record a, z: char; b: [aligned(0)] integer16 end;
var
  r: s2;
  q: s3;
  i: integer;
";
    let e = errs(&format!("{types}procedure x; begin i := r.b end;"));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    let e = errs(&format!("{types}procedure x; begin r.c := 1 end;"));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    lower_src(&format!(
        "{types}procedure x; begin q.b := 1; i := q.b end;"
    ))
    .unwrap();
}

/// An element of a byte-aligned array has a type without the attribute,
/// but may lie at an odd address all the same (`x` after a char at 1):
/// refused when it is not known to be even, accepted when it is.
#[test]
fn elements_of_byte_aligned_arrays_at_odd_addresses_are_refused() {
    let types = "module m;
type
  oa = [aligned(0)] array[1..2] of integer16;
  r1 = record c: char; f: oa end;
var
  c0: char;
  x: oa;
  rr: r1;
  i: integer;
procedure v(n: integer); val_param; extern;
";
    let e = errs(&format!("{types}procedure p; begin i := x[1] end;"));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    let e = errs(&format!("{types}procedure p; begin v(x[2]) end;"));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    // `f` would cross two word boundaries at 1, so it is at 2 (p. 3-26).
    lower_src(&format!("{types}procedure p; begin i := rr.f[2] end;")).unwrap();
    // Through a pointer to a byte-aligned word (ref 3.15.6.7: the base
    // type gives the target's alignment): not known to be even.
    let e = errs(
        "module m;
type bw = [aligned(0)] integer16;
var bp: extern ^bw; i: integer;
procedure p; begin i := bp^ end;",
    );
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    // Nor is a `for` fill (`clr.w (A0)+`) of another unit's byte-aligned
    // array.
    let e = errs(
        "module m;
type oa = [aligned(0)] array[1..4] of integer16;
var x: extern oa;
procedure p; var k: integer; begin for k := 1 to 4 do x[k] := 0 end;",
    );
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    // Nor a block copy (`move.l (A0)+,(A1)+`) from or to one at an odd
    // address.
    let e = errs(
        "module m;
type oa = [aligned(0)] array[1..8] of integer16;
var c0: char; x: oa; y: oa;
procedure p; begin x := y end;",
    );
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
}

/// A value parameter that is a record with device or volatile parts is
/// not copied on entry (a block copy merges device references, ref 3.15.3
/// p. 3-57), as its assignment is not; a volatile scalar one is.
#[test]
fn volatile_record_value_parameters_are_not_copied() {
    let types = "module m;
type
  dr = [device] record a, b, c, d: integer end;
  vr = record a: [volatile] integer; b: integer end;
  vi = [volatile] integer;
var h: extern integer;
";
    for t in ["dr", "vr"] {
        let e = errs(&format!("{types}procedure p(r: {t}); begin h := r.b end;"));
        assert!(e[0].contains("volatile or device parts"), "{t}: {e:?}");
    }
    lower_src(&format!("{types}procedure p(x: vi); begin h := x end;")).unwrap();
}

/// A packed field of a volatile record is not stored into (the field
/// form reads and writes its unit back, `andi` then `ori`).
#[test]
fn volatile_packed_fields_are_not_stored() {
    let e = errs(
        "module m;
type pr = [volatile] packed record a: 0..15; b: 0..15 end;
var p: extern pr; t: extern integer;
procedure x; begin p.a := 3 end;",
    );
    assert!(e[0].contains("packed field of a volatile record"), "{e:?}");
    lower_src(
        "module m;
type pr = [volatile] packed record a: 0..15; b: 0..15 end;
var p: extern pr; t: extern integer;
procedure x; begin t := p.b end;",
    )
    .unwrap();
}

/// The statement part of `body` in procedure `t` with label declaration
/// `labels`, lowered, or its errors.
fn goto_body(labels: &str, body: &str) -> Result<Vec<Stmt>, Vec<String>> {
    let src = format!("{CTL}procedure t; internal;\nlabel {labels};\nbegin\n{body}\nend;\n");
    match lower_src(&src) {
        Ok(m) => Ok(m.functions[0].body.clone()),
        Err(_) => Err(errs(&src)),
    }
}

/// `goto` and labels (ref 2.2.2.1 p. 2-11; "Goto", pp. 4-95..4-97): `L:
/// s` is the label's point, then `s`, in the same sequence; labels are
/// numbered in source order.
#[test]
fn goto_and_labels() {
    let b = goto_body(
        "10, done",
        "if b then goto done;\n10: begin w := 1; if b2 then goto 10 end;\ndone:",
    )
    .unwrap();
    assert_eq!(b.len(), 5, "{b:?}");
    let StmtKind::If { then_, .. } = &b[0].kind else {
        panic!("{b:?}")
    };
    assert_eq!(then_[0].kind, StmtKind::Goto(1));
    assert_eq!(b[1].kind, StmtKind::Label(0));
    assert!(matches!(b[2].kind, StmtKind::Store { .. }));
    let StmtKind::If { then_, .. } = &b[3].kind else {
        panic!("{b:?}")
    };
    assert_eq!(then_[0].kind, StmtKind::Goto(0));
    assert_eq!(b[4].kind, StmtKind::Label(1));
}

/// Where a `goto` may jump (ref p. 4-95; p. 6-23; Appendix D p. D-4).
#[test]
fn goto_targets() {
    // Out of loops, between the arms of a `case`, into an `if` part.
    for (labels, body) in [
        (
            "l",
            "while b do begin if b2 then goto l; w := 1 end;\nl: w := 2;",
        ),
        ("l", "repeat goto l until b;\nl: w := 2;"),
        ("l", "case w of 1: goto l; 2: l: w := 3 end;"),
        (
            "l",
            "case w of 1: if b then w := 4 else goto l; 2: w := 2 otherwise l: w := 3 end;",
        ),
        (
            "l",
            "if b then goto l;\nif b2 then begin w := 1; l: w := 2 end;",
        ),
        (
            "l",
            "if b then w := 1 else begin l: w := 2 end;\nif b2 then goto l;",
        ),
    ] {
        assert!(
            goto_body(labels, body).is_ok(),
            "{body}: {:?}",
            goto_body(labels, body)
        );
    }
    // Into a loop or a `case` from outside (open question 30).
    for body in [
        "goto l;\nwhile b do begin l: w := 1 end;",
        "goto l;\nrepeat l: w := 1 until b;",
        "if b then goto l;\ncase w of 1: l: w := 1; 2: w := 2 end;",
        "case w of 1: goto l; 2: w := 2 end;\nwhile b do l: w := 1;",
    ] {
        let e = goto_body("l", body).unwrap_err();
        assert!(
            e[0].contains("a `goto` into a `for`, `while`, `repeat`, `with` or `case`")
                && e[0].contains("open question 30"),
            "{body}: {e:?}"
        );
    }
    // A non-local `goto` (ref p. 4-97).
    let e = errs(&format!(
        "{CTL}procedure t;\nlabel l;\n  procedure n;\n  begin\n    goto l\n  end;\nbegin\n  n;\nl: w := 1\nend;\n"
    ));
    assert!(
        e[0].contains("t.pas:14:5:") && e[0].contains("non-local `goto`"),
        "{e:?}"
    );
}
