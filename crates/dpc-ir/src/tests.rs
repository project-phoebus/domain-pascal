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
            ty: Scalar::unsigned(Width::Word),
            in_place: false,
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

/// An integer constant transferred to a pointer type of another size
/// (owner's ruling on p. 4-197; bead domain-pascal-gqp) is a 4-byte
/// pointer constant: Apollo's `ecp := ec2_$ptr_t( ec2_$always_ready_ec )`.
#[test]
fn integer_constant_transferred_to_a_pointer() {
    let m = lower_src(
        "module m;\ntype p = ^integer32;\nconst one = 1;\nvar e: p;\n\
         procedure q;\nbegin\n  e := p(one);\nend;\n",
    )
    .unwrap();
    let StmtKind::Store { value, .. } = &m.functions[0].body[0].kind else {
        panic!("not a store: {:?}", m.functions[0].body[0].kind)
    };
    assert_eq!(
        *value,
        Expr::Const {
            value: 1,
            ty: Scalar::POINTER
        }
    );
}

/// `nil` stored into a pointer is the pointer constant 0, stored like
/// any longword 0 (`clr.l`, codegen-patterns 13; bead domain-pascal-j4ip):
/// a variable and a routine pointer.
#[test]
fn nil_stored_is_the_pointer_zero() {
    let m = lower_src(
        "module m;\ntype p = ^integer32;\nvar e: p; f: ^procedure;\n\
         procedure q;\nbegin\n  e := nil;\n  f := nil;\nend;\n",
    )
    .unwrap();
    for st in &m.functions[0].body {
        let StmtKind::Store { value, .. } = &st.kind else {
            panic!("not a store: {:?}", st.kind)
        };
        assert_eq!(
            *value,
            Expr::Const {
                value: 0,
                ty: Scalar::POINTER
            }
        );
    }
}

/// `nil` passed by value is the pointer constant 0, pushed like any
/// longword 0 (`clr.l -(SP)`, NET_IO_$PUT_IN_SOCK 0xE0E4CC; bead
/// domain-pascal-73sx), unless another constant argument is next to it
/// (no site).
#[test]
fn nil_passed_by_value_is_the_pointer_zero() {
    let decls = "module m;\ntype p = ^integer32;\n\
                 procedure byval(x: p; n: integer; y: p); options(val_param, extern);\n";
    let m = lower_src(&format!(
        "{decls}procedure q(k: integer; z: p);\nbegin\n  byval(nil, k, z);\n  byval(z, k, nil);\nend;\n"
    ))
    .unwrap();
    let body = &m.functions[0].body;
    for (st, i) in body.iter().zip([0, 2]) {
        let StmtKind::Call(c) = &st.kind else {
            panic!("not a call: {:?}", st.kind)
        };
        assert_eq!(
            c.args[i],
            Arg::Value {
                value: Expr::Const {
                    value: 0,
                    ty: Scalar::POINTER
                },
                slot: 4
            }
        );
    }
    // Two `nil`s with a non-constant between them: neither has a constant
    // neighbour.
    lower_src(&format!(
        "{decls}procedure q(k: integer);\nbegin\n  byval(nil, k, nil);\nend;\n"
    ))
    .unwrap();
    for call in ["byval(nil, 1, z)", "byval(z, 0, nil)"] {
        let e = errs(&format!(
            "{decls}procedure q(k: integer; z: p);\nbegin\n  {call};\nend;\n"
        ));
        assert_eq!(e.len(), 1, "{call}: {e:?}");
        assert!(
            e[0].contains("`nil` passed by value next to another constant argument"),
            "{e:?}"
        );
    }
}

/// `nil` passed by address is refused (no site; codegen-patterns 13,
/// "`nil`"); as a function result it is the pointer 0, whose form the
/// selector decides (codegen-patterns 3, "`nil` results"; bead
/// domain-pascal-bdi0).
#[test]
fn nil_passed_by_address_is_refused_nil_result_lowered() {
    let decls = "module m;\ntype p = ^integer32;\n\
                 procedure byref(in x: p); extern;\n";
    let e = errs(&format!(
        "{decls}procedure q;\nbegin\n  byref(nil);\nend;\n"
    ));
    assert!(e[0].contains("`nil` passed by address"), "{e:?}");
    let m = lower_src(&format!("{decls}function f: p;\nbegin\n  f := nil;\nend;\n")).unwrap();
    assert_eq!(
        m.functions[0].body[0].kind,
        StmtKind::SetResult(Expr::Const {
            value: 0,
            ty: Scalar::POINTER
        })
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
        "module m;\ntype r = record a: integer; end;\nprocedure p(var x: r);\nbegin\n  if x.a = 2 then ;\nend;\n",
    );
    assert_eq!(
        e,
        vec!["t.pas:5:3: error: an `if` with an empty `then` part is not supported yet"]
    );
    // A 16-byte set tested with a char element, which may lie beyond its
    // 128 bits (the guarded form, codegen-patterns 5.1; an `integer`
    // element is tested unguarded, MSG_$FORK 0xE73F2E).
    let e = errs(
        "module m;
type s16 = set of 'a'..'z';
function f(k: char; var s: s16): boolean; options(val_param);
begin
  f := k in s;
end;
",
    );
    assert_eq!(
        e,
        vec![
            "t.pas:5:8: error: `in` on a 16-byte set with an element whose type reaches beyond \
             the set's 128 bits (the guarded test, NAME_$INIT 0xE3183E, codegen-patterns 5.1) is \
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

#[test]
fn unknown_result_conventions_are_refused_when_used() {
    // codegen-patterns 3: how sets, reals and records other than 4-byte
    // ones are returned is not known.  `dpc-sema` accepts the
    // declarations (Apollo's inserts have them, bead domain-pascal-qia); a
    // definition and a call are refused here, and an unused extern
    // declaration is not.
    lower_src("module m; type s = record a: integer; end; function f: s; extern; function g: real; extern;")
        .unwrap();
    let e = errs(
        "module m;
type s = record a: integer; end;
function f: s;
var r: s;
begin
  r.a := 1;
  f := r;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:3:1: error: functions returning s are not supported yet (codegen-patterns 3: result convention unknown)"]
    );
    let e = errs(
        "module m;
type s = record a: integer; end;
function f: s; extern;
procedure p;
var x: s;
begin
  x := f;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:7:8: error: functions returning s are not supported yet (codegen-patterns 3: result convention unknown)"]
    );
    // ... and through a routine pointer.
    let e = errs(
        "module m;
type s = record a: integer; end;
  fp = ^function: s;
procedure p(q: fp);
var x: s;
begin
  x := q^;
end;
",
    );
    assert_eq!(
        e,
        vec!["t.pas:7:8: error: functions returning s are not supported yet (codegen-patterns 3: result convention unknown)"]
    );
    // A discarded result is refused too: a bare call could be wrong code
    // if such results need caller-provided storage.
    for (decl, call) in [
        ("function f: s; extern;", "f"),
        ("function f: real; extern;", "f"),
        ("var q: ^function: s;", "q^"),
    ] {
        let e = errs(&format!(
            "module m;\ntype s = record a: integer; end;\n{decl}\nprocedure p;\nbegin\n  discard({call});\nend;\n"
        ));
        assert_eq!(e.len(), 1, "{decl}: {e:?}");
        assert!(
            e[0].starts_with("t.pas:6:11: error: functions returning ")
                && e[0].contains("result convention unknown"),
            "{decl}: {e:?}"
        );
    }
}

const STATUS: &str = "module m;
type status_$t = packed record case integer of
    1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
    2: (all: integer32);
  end;
  cleanup_rec_t = record pad: array[1..6] of integer32; end;
function fim_$cleanup(out rec: cleanup_rec_t): status_$t; options(abnormal, extern);
procedure use(in s: status_$t); extern;
";

/// codegen-patterns 3, "Record results": a 4-byte record comes back in
/// D0.  Callers read it like a longword data result; the callee copies
/// the record into its result variable's frame slot and loads that.
#[test]
fn four_byte_record_results() {
    // The caller: `move.l D0,<dst>` (MAC_OS_$SEND 0xE0B604).
    let m = lower_src(&format!(
        "{STATUS}procedure p;
var r: cleanup_rec_t; cs: status_$t;
begin
  cs := fim_$cleanup(r);
end;
"
    ))
    .unwrap();
    let StmtKind::Store {
        addr: Addr::Frame(_),
        value: Expr::Call { ty, reg, .. },
    } = &m.functions[0].body[0].kind
    else {
        panic!("{:#?}", m.functions[0].body)
    };
    assert_eq!(*ty, Scalar::unsigned(Width::Long));
    assert_eq!(*reg, ResultClass::Data);

    // The callee: a store into the result slot, then the slot as the
    // result (PEB_$GET_STATUS 0xE5AE52).
    let m = lower_src(&format!(
        "{STATUS}function g(in x: integer32): status_$t;
var st: status_$t;
begin
  st.all := x;
  g := st;
end;
"
    ))
    .unwrap();
    let f = &m.functions[0];
    assert_eq!(
        f.result,
        Some(ResultSpec {
            class: ResultClass::Data,
            ty: Scalar::unsigned(Width::Long),
        })
    );
    let slot = f.result_slot.expect("a result slot");
    let n = f.body.len();
    assert!(matches!(
        &f.body[n - 2].kind,
        StmtKind::Store { addr: Addr::Frame(o), value: Expr::Load { addr: Addr::Frame(_), .. } }
            if *o == slot
    ));
    assert!(matches!(
        &f.body[n - 1].kind,
        StmtKind::SetResult(Expr::Load { addr: Addr::Frame(o), .. }) if *o == slot
    ));

    // Refused: other values, other places of the assignment, records of
    // other sizes, `a0_return`, calls through routine pointers.
    let refused = [
        (
            "function g: status_$t; begin g := fim_$cleanup(r); end;",
            "a record function result assigned other than a record variable",
        ),
        (
            "function g(var s: status_$t): status_$t; begin g := s; g := s; end;",
            "a record function result assigned other than once",
        ),
        (
            "function g(var s: status_$t): status_$t; begin if s.all = 0 then g := s else g := s; end;",
            "a record function result assigned other than once",
        ),
        (
            "function g(var s: status_$t): status_$t; begin if s.all = 0 then return; g := s; end;",
            "a record function result assigned other than once",
        ),
        (
            "function g(s: status_$t): status_$t; options(val_param); begin g := s; end;",
            "a record function result assigned from a parameter passed by value",
        ),
        (
            "function g(var s: status_$t): status_$t; options(a0_return); begin g := s; end;",
            "functions returning status_$t are not supported yet (codegen-patterns 3: result \
             convention unknown)",
        ),
        (
            "type two = record a: integer; end;
function g(var s: two): two; begin g := s; end;",
            "functions returning two are not supported yet",
        ),
        (
            "procedure q; type fp = ^function: status_$t; var f: fp; s: status_$t;
begin s := f^; end;",
            "a record result of a call through a routine pointer",
        ),
    ];
    for (src, want) in refused {
        let src = format!("{STATUS}var r: cleanup_rec_t;\n{src}\n");
        let e = errs(&src);
        assert_eq!(e.len(), 1, "{src}: {e:?}");
        assert!(e[0].contains(want), "{src}: {e:?}");
    }
}

/// codegen-patterns 3, "Record results over 4 bytes": a larger record goes
/// through a hidden first argument, the address of a place for it.  The
/// callee copies into the object that slot points to and has no result
/// register; the caller passes a frame temporary's address last and copies
/// the record out of it, or passes the temporary on by address.
#[test]
fn hidden_address_record_results() {
    const UID: &str = "module m;
type uid_$t = record high, low: integer32; end;
var root: uid_$t;
";
    // The callee (ACL_$DEFAULT_ACL 0xE4787E, NAME_$GET_ROOT_UID 0xE589BC).
    let m = lower_src(&format!(
        "{UID}function g(in k: integer): uid_$t;
begin
  if k = 0 then g := root;
end;
"
    ))
    .unwrap();
    let f = &m.functions[0];
    assert_eq!(f.result, None);
    assert_eq!(f.result_slot, None);
    assert_eq!(f.address_params, [8, 0xc]);
    assert_eq!(f.frame_size, 0);
    let StmtKind::If { then_, .. } = &f.body[0].kind else {
        panic!("{:#?}", f.body)
    };
    let StmtKind::Copy {
        dst: Addr::Indirect { base, offset: 0 },
        src: Addr::Static(0),
        bytes: 8,
        chars: false,
    } = &then_[0].kind
    else {
        panic!("{then_:#?}")
    };
    assert!(matches!(
        &**base,
        Expr::Load {
            addr: Addr::Frame(8),
            ..
        }
    ));

    // The caller: `pea temp; bsr; pop; copy' (PROC2_$UPGID_TO_UID
    // 0xE41030..0xE41046), the temporary below the locals at a multiple
    // of 4, no pad for a word argument.
    let m = lower_src(&format!(
        "{UID}function h(w: integer): uid_$t; options(val_param, extern);
procedure p(in x: integer);
var u: uid_$t; v: integer;
begin
  v := x;
  u := h(v);
end;
"
    ))
    .unwrap();
    let f = &m.functions[0];
    assert_eq!(f.frame_size, 20);
    let StmtKind::Call(c) = &f.body[1].kind else {
        panic!("{:#?}", f.body)
    };
    assert_eq!(c.pad, 0);
    assert_eq!(c.bytes(), 6);
    assert!(matches!(
        &c.args[0],
        Arg::Address {
            addr: Addr::Frame(-20),
            slot: 4,
            writable: true
        }
    ));
    assert!(matches!(
        &f.body[2].kind,
        StmtKind::Copy {
            dst: Addr::Frame(-8),
            src: Addr::Frame(-20),
            bytes: 8,
            chars: false
        }
    ));

    // Passed on by address: the inner call is an argument, its temporary
    // its first argument (TIME_$SET_ITIMER 0xE58E9E..0xE58ECA: two at -8
    // and -0x10).
    let m = lower_src(&format!(
        "{UID}function h(in w: integer): uid_$t; extern;
procedure q(in a: uid_$t; in b: uid_$t); extern;
procedure p;
begin
  q(h(1), h(2));
end;
"
    ))
    .unwrap();
    let f = &m.functions[0];
    let StmtKind::Call(c) = &f.body[0].kind else {
        panic!("{:#?}", f.body)
    };
    let temp = |a: &Arg| match a {
        Arg::Call { call, slot: 4 } => match &call.args[0] {
            Arg::Address {
                addr: Addr::Frame(o),
                ..
            } => *o,
            a => panic!("{a:?}"),
        },
        a => panic!("{a:?}"),
    };
    // Pushed right to left: b's first.
    assert_eq!((temp(&c.args[1]), temp(&c.args[0])), (-8, -16));
    assert_eq!(f.frame_size, 16);

    // Refused: the forms the image does not show.
    let refused = [
        (
            "function g: uid_$t; extern; function k: uid_$t; begin k := g; end;",
            "a record function result assigned other than a record variable",
        ),
        (
            "function g: uid_$t; extern; procedure p; begin discard(g); end;",
            "a call of a function whose record result goes through a hidden address",
        ),
        (
            "function g: uid_$t; extern; procedure p; begin if g = root then root.low := 0; end;",
            "a call of a function whose record result goes through a hidden address",
        ),
        (
            "function g: uid_$t; extern; function k: integer; extern;
var t: array[1..2] of uid_$t;
procedure p; begin t[k] := g; end;",
            "a record function result assigned to a designator whose address needs a call",
        ),
        (
            "function g: uid_$t; options(extern, a0_return); procedure p; begin root := g; end;",
            "functions returning uid_$t are not supported yet",
        ),
        (
            "type fp = ^function: uid_$t; procedure q(in u: uid_$t); extern;
procedure p(f: fp); begin q(f^); end;",
            "functions returning uid_$t are not supported yet",
        ),
    ];
    // A nested one: the definition and the calls refused, assigned or
    // passed on (the routine's first error is reported).
    for call in ["g := f", "q(f)"] {
        let e = errs(&format!(
            "{UID}procedure q(in u: uid_$t); extern;
procedure p;
var g: uid_$t;
  function f: uid_$t;
  begin
    f := root;
  end;
begin
  {call};
end;
"
        ));
        assert_eq!(e.len(), 2, "{e:?}");
        assert!(
            e.iter().all(|m| m
                .contains("a nested function whose record result goes through a hidden address")),
            "{e:?}"
        );
    }
    for (src, want) in refused {
        let src = format!("{UID}{src}\n");
        let e = errs(&src);
        assert_eq!(e.len(), 1, "{src}: {e:?}");
        assert!(e[0].contains(want), "{src}: {e:?}");
    }
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
        // Needs 32 bits: 4 bytes, unsigned when non-negative (ref 3.4
        // p. 3-9, 4.3.3 p. 4-7; `linteger` of base.ins.pas), signed
        // otherwise.
        ("0..65536", Scalar::unsigned(Width::Long)),
        ("0..2147483647", Scalar::unsigned(Width::Long)),
        ("-1..65536", Scalar::signed(Width::Long)),
        ("boolean", Scalar::BOOLEAN),
        ("char", Scalar::unsigned(Width::Byte)),
        ("color", Scalar::unsigned(Width::Word)),
        // A size attribute (ref 3.15.5 p. 3-62): one-byte integers,
        // unsigned when non-negative ("[byte] 0..255 corresponds to the
        // unsigned char data type in Domain/C").
        ("[byte] 0..255", Scalar::unsigned(Width::Byte)),
        ("[byte] 0..15", Scalar::unsigned(Width::Byte)),
        ("[byte] -128..127", Scalar::signed(Width::Byte)),
        ("[word] 0..255", Scalar::signed(Width::Word)),
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

/// Narrowing a value into a smaller object keeps its low bytes (bead
/// domain-pascal-8td; `assign_convert`): a variable or parameter is read
/// at the narrow width, a [`Expr::Convert`] of its load (APP_$DEMUX
/// 0xE00AF6).  In an assignment (bead domain-pascal-d59a; `store_narrow`)
/// also an element, a pointer's target and a function's result one step
/// narrower, and a longword variable into a byte (DISK_IO 0xE3D630); a
/// call inside a computed value, a volatile object, a longword element or
/// result into a byte and the select `if a < b then a := b` are refused.
/// Widening is a conversion (0xE02C28..0xE02C2C `move.w D2,D0; ext.l D0;
/// move.l D0,(-32,A6)`).
#[test]
fn narrowing_store() {
    let m = lower_src(
        "module m;
procedure p(var x: integer; i: integer32); options(val_param);
begin
  x := i;
end;
",
    )
    .unwrap();
    let StmtKind::Store { value, .. } = &m.functions[0].body[0].kind else {
        panic!()
    };
    assert!(
        matches!(value, Expr::Convert { value: v, ty, .. }
            if *ty == Scalar::signed(Width::Word)
                && matches!(**v, Expr::Load { ty: t, .. } if t == Scalar::signed(Width::Long))),
        "{value:?}"
    );
    // A right shift: computed in 32 bits, the low word used.
    let m = lower_src(
        "module m;
procedure p(var x: integer; i: integer32); options(val_param);
begin
  x := rshft(i, 5);
end;
",
    )
    .unwrap();
    let StmtKind::Store { value, .. } = &m.functions[0].body[0].kind else {
        panic!()
    };
    assert!(
        matches!(value, Expr::Convert { value: v, .. }
            if matches!(**v, Expr::Binary { op: BinOp::Shr, .. })),
        "{value:?}"
    );
    // An element (at a constant or a run-time index), a pointer's target:
    // a [`Expr::Convert`] of the whole object's load marked `in_place`,
    // the target choosing the low bytes' address (MMAP 0xE0C602, 0xE0C664;
    // bead domain-pascal-dkik).
    let store_value = |decl: &str, stmt: &str| {
        let m = lower_src(&format!("module m;\n{decl}\nbegin\n  {stmt};\nend;\n")).unwrap();
        match &m.functions[0].body.last().unwrap().kind {
            StmtKind::Store { value, .. } => value.clone(),
            k => panic!("{k:?}"),
        }
    };
    let w = Scalar::signed(Width::Word);
    for (decl, stmt) in [
        (
            "type a_t = array[1..2] of integer32;\nprocedure p(var x: integer; in a: a_t);",
            "x := a[1]",
        ),
        (
            "type p_t = ^integer32;\nprocedure p(var x: integer; q: p_t); options(val_param);",
            "x := q^",
        ),
    ] {
        let v = store_value(decl, stmt);
        assert!(
            matches!(&v, Expr::Convert { value: l, ty, in_place: true }
                if *ty == w && matches!(**l, Expr::Load { ty: t, .. } if t == Scalar::signed(Width::Long))),
            "{v:?}"
        );
    }
    let v = store_value(
        "type a_t = array[1..8] of integer32;\nprocedure p(var x: integer; i: integer; in a: a_t); \
         options(val_param);",
        "x := a[i]",
    );
    assert!(
        matches!(&v, Expr::Convert { value: l, ty, in_place: true } if *ty == w
            && matches!(&**l, Expr::Load { addr: Addr::Index { offset: -4, .. }, ty: t }
                if *t == Scalar::signed(Width::Long))),
        "{v:?}"
    );
    // A function's result: its register at the narrow width.
    let v = store_value(
        "function f: integer32; extern;\nprocedure p(var x: integer);",
        "x := f",
    );
    assert!(
        matches!(&v, Expr::Convert { value: c, ty, in_place: false } if *ty == w
            && matches!(**c, Expr::Call { .. })),
        "{v:?}"
    );
    // A longword value parameter into a byte, in place (DISK_IO 0xE3D630).
    let v = store_value(
        "type b_t = [byte] 0..255;\nprocedure p(var x: b_t; i: integer32); options(val_param);",
        "x := i",
    );
    assert!(
        matches!(&v, Expr::Convert { value: l, ty, in_place: true } if ty.width == Width::Byte
            && matches!(**l, Expr::Load { addr: Addr::Frame(_), .. })),
        "{v:?}"
    );
    for (decl, stmt, msg) in [
        (
            "function f: integer32; extern;\nprocedure p(var x: integer);",
            "x := f + 1",
            "assigning a 4-byte value to a 2-byte object",
        ),
        (
            "type b_t = [byte] 0..255;\na_t = array[1..2] of integer32;\n\
             procedure p(var x: b_t; in a: a_t);",
            "x := a[2]",
            "narrowing a longword element, pointer's target or `with` record to a byte (not \
             catalogued; a variable's is DISK_IO 0xE3D630, codegen-patterns 8)",
        ),
        (
            "type b_t = [byte] 0..255;\nvar g: integer32;\nprocedure p(var x: b_t);",
            "x := g",
            "narrowing a longword local, unit or enclosing routine's variable to a byte (not \
             catalogued; the one site is a value parameter's slot, DISK_IO 0xE3D630, \
             codegen-patterns 8)",
        ),
        (
            "type b_t = [byte] 0..255;\nprocedure p(var x: b_t; i: integer32); options(val_param);\n\
             var l: integer32;",
            "l := i; x := l",
            "narrowing a longword local, unit or enclosing routine's variable to a byte (not \
             catalogued; the one site is a value parameter's slot, DISK_IO 0xE3D630, \
             codegen-patterns 8)",
        ),
        (
            "type b_t = [byte] 0..255;\nprocedure p(var x: b_t; in i: integer32);",
            "x := i",
            "narrowing a longword parameter passed by address (or a field of one) to a byte (not \
             catalogued; a value parameter's slot is DISK_IO 0xE3D630, codegen-patterns 8)",
        ),
        (
            "type p_t = ^integer32;\nprocedure p(var x: integer; q: p_t); options(val_param);",
            "if q^ > x then begin x := q^; end",
            "a select `if a < b then a := b` that narrows `b` into `a` (the compare and the \
             store at the wider width in `a`'s register, XNS_ERROR_$SEND 0xE17A04 `move.l \
             (A3),D5`; codegen-patterns 8, bead domain-pascal-dzad)",
        ),
        (
            "type b_t = [byte] 0..255;\nfunction f: integer32; extern;\nprocedure p(var x: b_t);",
            "x := f",
            "narrowing a longword function result to a byte (not catalogued, codegen-patterns 8)",
        ),
        (
            "var d: extern integer32;\nprocedure p(var x: integer);",
            "x := d",
            "",
        ),
        (
            // Passed by value: an element's longword is loaded whole at
            // AST_$TOUCH 0xE0350E (refused, codegen-patterns 8).
            "type a_t = array[1..2] of integer32;\n\
             procedure q(w: integer); options(val_param, extern);\n\
             procedure p(in a: a_t);",
            "q(a[1])",
            "passing a 4-byte value to a 2-byte parameter",
        ),
        (
            "var a: extern [volatile] array[1..2] of integer32;\n\
             procedure p(var x: integer; i: integer); options(val_param);",
            "x := a[i]",
            "narrowing a volatile or device object (4 bytes into 2: an access of another size, \
             ref 3.15.1-3.15.3)",
        ),
        (
            "type p_t = ^integer32;\nprocedure p(var x: integer; q: p_t); options(val_param);",
            "if x < q^ then x := q^",
            "a select `if a < b then a := b` that narrows `b` into `a` (the compare and the \
             store at the wider width in `a`'s register, XNS_ERROR_$SEND 0xE17A04 `move.l \
             (A3),D5`; codegen-patterns 8, bead domain-pascal-dzad)",
        ),
    ] {
        if msg.is_empty() {
            // (a variable of another unit: [`assign_convert`]'s.)
            assert!(lower_src(&format!("module m;\n{decl}\nbegin\n  {stmt};\nend;\n")).is_ok());
            continue;
        }
        let e = errs(&format!("module m;\n{decl}\nbegin\n  {stmt};\nend;\n"));
        assert!(
            e[0].ends_with(&format!("{msg} is not supported yet")),
            "{e:?}"
        );
    }
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
            bytes: 12,
            chars: false
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
            bytes: 6,
            chars: false
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
procedure q; extern;
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
    // An index of another scope (ref p. 4-89): kept in memory, its home
    // the unit's variable (bead domain-pascal-1mtr).
    let m = one("for j := 0 to 15 do buf[j] := 0;").unwrap();
    let StmtKind::For(l) = &m.functions[0].body[0].kind else {
        panic!("{:?}", m.functions[0].body)
    };
    assert!(matches!(l.home, Some(Addr::Symbol { .. })), "{:?}", l.home);
    // A byte index (bead domain-pascal-1mtr).
    let m = one("for b := 'a' to 'z' do j := 0;").unwrap();
    let StmtKind::For(l) = &m.functions[0].body[0].kind else {
        panic!("{:?}", m.functions[0].body)
    };
    assert_eq!(l.index.ty.width, Width::Byte);
    assert_eq!(l.home, None);
    // The index is kept in a register (codegen-patterns 7.4): read after a
    // loop of constant bounds in the statements that follow it; passed by
    // address to a value or `in` parameter inside it; never assigned in its
    // loop (ref p. 4-89) nor anywhere else.
    // A loop with a run-time bound too: the target sets the index before
    // its guard (bead domain-pascal-trdz).
    let homes = |m: &crate::Module| {
        let mut h = Vec::new();
        crate::visit_stmts(&m.functions[0].body, &mut |s| {
            if let StmtKind::For(l) = &s.kind {
                h.push(l.home.clone());
            }
        });
        h
    };
    for stmt in [
        "for i := 0 to 15 do j := 0; j := i;",
        "for i := 0 to 15 do if buf[i] = 0 then exit; if i > 15 then j := i;",
        "for i := 0 to 15 do j := 0; for i := 1 to 2 do j := i; j := i;",
        "for i := 0 to 15 do j := 0; while j = 0 do j := i;",
        "for i := 0 to j do j := 0; j := i;",
    ] {
        let m = one(stmt).unwrap_or_else(|e| panic!("{stmt}: {e:?}"));
        assert!(homes(&m).iter().all(Option::is_none), "{stmt}");
    }
    // An index used outside its loops other than so: refused under
    // `--strict`; else every loop over it keeps it in memory, at its own
    // slot (best effort, bead domain-pascal-trdz).
    let rehomed = [
        (
            "j := i; for i := 0 to 15 do j := 0;",
            "read before its loop",
        ),
        (
            "if j = 0 then for i := 0 to 15 do j := 0; j := i;",
            "read before its loop",
        ),
        (
            "for i := 0 to 15 do j := 0; if j = 1 then for i := 1 to 2 do j := 0; j := i;",
            "another loop over it",
        ),
        ("for i := 0 to 15 do j := 0; i := 2;", "outside its loops"),
        (
            "for i := 1 to 15 do if buf[i] = 0 then exit; for i := 1 to i do j := 0;",
            "bound reads its own index",
        ),
        (
            "for i := 1 to 15 do if buf[i] = 0 then exit; for i := i to 15 do q;",
            "bound reads its own index",
        ),
    ];
    for (stmt, msg) in [
        ("for i := 0 to 15 do i := 3;", "in its loop"),
        (
            "for i := 0 to 15 do for i := 1 to 2 do j := 0;",
            "of an enclosing `for` loop",
        ),
    ]
    .into_iter()
    .chain(rehomed)
    {
        let e = one(stmt).unwrap_err();
        assert!(e[0].contains(msg), "{stmt}: {e:?}");
    }
    // A label after the loop that a `goto` from before it reaches.
    let lab = |stmt: &str| {
        lower_src(&format!(
            "module m;\nvar j: extern integer;\nprocedure p;\nlabel 1;\nvar i: integer;\nbegin\n  \
             {stmt}\nend;\n"
        ))
    };
    let m = lab("for i := 0 to 15 do if j = i then goto 1; j := 0; 1: j := i;").unwrap();
    assert_eq!(homes(&m), vec![None]);
    let past = "if j = 0 then goto 1; for i := 0 to 15 do j := 0; 1: j := i;";
    let e = lab(past).unwrap_err();
    assert!(e[0].contains("past a label"), "{e:?}");
    let was = dpc_syntax::diag::set_strict(false);
    for (stmt, _) in rehomed {
        let m = one(stmt).unwrap_or_else(|e| panic!("{stmt}: {e:?}"));
        let h = homes(&m);
        assert!(
            !h.is_empty() && h.iter().all(|h| matches!(h, Some(Addr::Frame(_)))),
            "{stmt}: {h:?}"
        );
    }
    let m = lab(past).unwrap();
    assert!(matches!(homes(&m)[..], [Some(Addr::Frame(_))]));
    for (stmt, msg) in [
        ("for i := 0 to 15 do i := 3;", "in its loop"),
        (
            "for i := 0 to 15 do for i := 1 to 2 do j := 0;",
            "of an enclosing `for` loop",
        ),
    ] {
        let e = one(stmt).unwrap_err();
        assert!(e[0].contains(msg), "{stmt}: {e:?}");
    }
    let _ = dpc_syntax::diag::take_best_effort();
    dpc_syntax::diag::set_strict(was);
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
type s = set of 0..63;
procedure q(in v: s); extern;
procedure p;
begin
  q([1]);
end;
",
    );
    assert!(
        e[0].ends_with(
            "a 8-byte set constant passed by address (no site known) is not supported yet"
        ),
        "{e:?}"
    );
    let e = errs(
        "module m;
type s = [byte] set of 0..7;
procedure q(in v: s); extern;
procedure p;
begin
  q([1]);
end;
",
    );
    assert!(
        e[0].ends_with(
            "a 1-byte set constant passed by address (no site known) is not supported yet"
        ),
        "{e:?}"
    );
    let e = errs(
        "module m;
procedure q(v: integer); options(val_param, extern);
function f: integer32; extern;
procedure p;
begin
  q(f);
end;
",
    );
    assert!(
        e[0].ends_with("passing a 4-byte value to a 2-byte parameter is not supported yet"),
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
        in_place: false,
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
    // Arrays (Table 4-1, "All types"): compared as records are, best
    // effort below 32 bytes of longwords (the unit of the image's array
    // loops is not settled, 0xE6C278 `cmpm.b`; bead domain-pascal-yqf6).
    let e = errs(&src("if v1 = v2 then w := 0;"));
    assert!(e[0].contains("comparing arrays other than"), "{e:?}");
    let strict = dpc_syntax::diag::set_strict(false);
    let m = lower_src(&src("if v1 = v2 then w := 0;"));
    dpc_syntax::diag::set_strict(strict);
    let w = dpc_syntax::diag::take_best_effort();
    assert!(
        w.iter().any(|d| d.message.contains("comparing arrays other than")),
        "{w:?}"
    );
    let StmtKind::If { cond, .. } = &m.unwrap().functions[0].body[0].kind else {
        panic!()
    };
    assert_eq!(
        *cond,
        Cond::Block {
            op: Rel::Eq,
            lhs: at("V1"),
            rhs: at("V2"),
            size: 8
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
    assert!(one("if u < 70000 then w := 0;").ends_with(
        "comparing with a constant outside the other operand's type is not supported yet"
    ));
    assert!(one("if u > -1 then w := 0;").contains("outside the other operand's type"));
    assert!(one("while w = 0 do ;").contains("a `while` loop with an empty body"));
    assert!(one("if w = 0 then ;").contains("an `if` with an empty `then` part"));
    assert!(one("if true then w := 0;").contains("an `if` with a constant condition"));
    assert!(one("if false then w := 0 else w := 1;").contains("an `if` with a constant condition"));
    assert!(one("while true do w := 0;").contains("a `while` loop with a constant condition"));
    assert!(one("while false do w := 0;").contains("a `while` loop with a constant condition"));
    assert!(one("repeat w := 0 until true;").contains("a constant `true` condition"));
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
    assert_eq!(*until, Some(Cond::Test(sym("B", Scalar::BOOLEAN))));
    assert_eq!(b[1].kind, StmtKind::Return);
}

/// `repeat ... until false` (the reference's examples, e.g. p. 4-42): a
/// loop with no test (codegen-patterns 7.2), whatever constant expression
/// gives the `false` (constant expressions are computed at every level,
/// ref 6.4.24, p. 6-27).
#[test]
fn repeat_until_false() {
    for src in [
        "repeat w := 1; if b then exit until false;",
        "repeat w := 1; if b then exit until not true;",
        "repeat w := 1; if b then exit until (1 = 2);",
    ] {
        let b = ctl_body(src);
        assert_eq!(b.len(), 1, "{src}");
        let StmtKind::Repeat { body, until } = &b[0].kind else {
            panic!("{src}")
        };
        assert_eq!(body.len(), 2, "{src}");
        assert_eq!(*until, None, "{src}");
    }
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
            in_place: false,
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
var a: extern 0..65535;
function f: integer; internal;
begin
  f := arshft(a, 2);
end;
",
    );
    assert!(e[0].contains("an `arshft` of an operand"), "{e:?}");
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
        e[0].contains("an array index of boolean type"),
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
            ty: Scalar::unsigned(Width::Word),
            in_place: false,
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
    let Expr::Convert { value, ty, .. } = e else {
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
        matches!(&*lhs, Expr::Convert { value, ty, .. } if *ty == Scalar::signed(Width::Word)
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
/// 1` at x's type; `max`/`min` of non-integers, `in_range` of anything
/// but an enumeration or a non-negative word subrange and `discard` of
/// other expressions are refused (no image pattern).  `in_range` of an
/// enumeration is the unsigned `x <= last` (codegen-patterns 10.1, bead
/// domain-pascal-k1sv).
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
        (
            "c := max(c, 'a')",
            "`max` or `min` of values other than integers",
        ),
        ("w := min(y, w)", "`max` or `min` of a one-byte integer"),
        (
            "b := in_range(w)",
            "`in_range` of a value other than an enumeration or an integer subrange",
        ),
        ("b := in_range(n)", "with a negative lower bound"),
        ("b := succ(b)", "this predeclared function is"),
        ("discard(w + 1)", "`discard` of an expression"),
    ] {
        let e = errs(&format!(
            "module m; type small = 1..5; neg = -7..7; byte_t = [byte] 0..255;
             var w: extern integer; b: extern boolean;
             s: extern small; n: extern neg; c: extern char; y: extern byte_t;
procedure p; begin {src} end;"
        ));
        assert!(e.iter().any(|m| m.contains(needle)), "{src}: {e:?}");
    }
    let e = result_value(
        "module m; type c_t = (c0, c1, c2, c3); var c: extern c_t;
function f: boolean; begin f := in_range(c) end;",
    );
    assert!(
        matches!(
            &e,
            Expr::Test(c) if matches!(&**c, Cond::Compare {
                op: Rel::Le,
                rhs: Expr::Const { value: 3, .. },
                ..
            })
        ),
        "{e:?}"
    );
}

/// `min`/`max` of run-time integers and `in_range` of a word subrange
/// (bead domain-pascal-bzkf; codegen-patterns 10.1): the comparison's
/// width and sign (ref "Min" p. 4-113, "Max" p. 4-111, 4.3.3 pp.
/// 4-6..4-7), word arithmetic widened (NETBUF_$ADD_PAGES 0xE0E95A), the
/// result narrowed back; `in_range` as one unsigned compare (SIO_$K_SET_
/// PARAM 0xE68252, 0xE68278).
#[test]
fn min_max_and_in_range_lower() {
    let src = |decl: &str, value: &str, result: &str| {
        format!(
            "module m; type pinteger = 0..65535; linteger = 0..2147483647;
             two = 0..3; three = 1..3;
             var i, j: extern integer; l: extern integer32; p, q: extern pinteger;
             u: extern linteger; t: extern two; h: extern three; {decl}
function f: {result}; begin f := {value} end;"
        )
    };
    let min_ty = |e: &Expr| match e {
        Expr::Binary {
            op: BinOp::Min | BinOp::Max,
            ty,
            ..
        } => Some(*ty),
        Expr::Convert { value, .. } => match &**value {
            Expr::Binary {
                op: BinOp::Min | BinOp::Max,
                ty,
                ..
            } => Some(*ty),
            _ => None,
        },
        _ => None,
    };
    let sw = Scalar::signed(Width::Word);
    let uw = Scalar::unsigned(Width::Word);
    let sl = Scalar::signed(Width::Long);
    let ul = Scalar::unsigned(Width::Long);
    for (value, result, ty) in [
        // Two signed words; a constant and a signed word.
        ("min(i, j)", "integer", sw),
        ("max(i, 3)", "integer", sw),
        // Two unsigned words (PKT_$BRK_INTERNET_HDR 0xE1246A, `bls`), a
        // non-negative constant and one (NET_IO_$COPY_PACKET 0xE0E612).
        ("min(p, q)", "pinteger", uw),
        ("min(1024, p)", "pinteger", uw),
        // A word and an unsigned longword (MMAP_$ALLOC_FREE 0xE0D8AC).
        ("min(p, u)", "linteger", ul),
        ("min(100, u)", "linteger", ul),
        // A signed and an unsigned operand: signed; two words of other
        // signs widened, as a comparison is.
        ("min(i, u)", "integer32", sl),
        ("min(i, p)", "integer32", sl),
        // A negative constant and an unsigned word: signed, widened.
        ("max(-1, p)", "integer32", sl),
        // A word difference: in 32 bits (NETBUF_$ADD_PAGES 0xE0E95A).
        ("min(i, 176 - j)", "integer", sl),
        ("min(i, l - 1)", "integer32", sl),
    ] {
        let e = result_value(&src("", value, result));
        assert_eq!(min_ty(&e), Some(ty), "{value}: {e:?}");
    }
    // The word difference computed at 32 bits, its operands widened, the
    // result narrowed back to the word.
    let e = result_value(&src("", "min(i, 176 - j)", "integer"));
    let Expr::Convert { value, ty, .. } = &e else {
        panic!("{e:?}")
    };
    assert_eq!(*ty, sw);
    let Expr::Binary { lhs, rhs, .. } = &**value else {
        panic!("{e:?}")
    };
    assert!(
        matches!(&**lhs, Expr::Convert { ty, .. } if *ty == sl),
        "{e:?}"
    );
    assert!(
        matches!(&**rhs, Expr::Binary { op: BinOp::Sub, lhs, ty, .. }
            if *ty == sl && matches!(&**lhs, Expr::Const { value: 176, .. })),
        "{e:?}"
    );
    // `in_range` of 0..3: the word compared unsigned with 3.
    let e = result_value(&src("", "in_range(t)", "boolean"));
    assert!(
        matches!(&e, Expr::Test(c) if matches!(&**c, Cond::Compare {
            op: Rel::Le,
            lhs: Expr::Load { ty, .. },
            rhs: Expr::Const { value: 3, .. },
        } if *ty == uw)),
        "{e:?}"
    );
    // `in_range` of 1..3: `x - 1 <= 2` in 32 bits, unsigned.
    let e = result_value(&src("", "in_range(h)", "boolean"));
    assert!(
        matches!(&e, Expr::Test(c) if matches!(&**c, Cond::Compare {
            op: Rel::Le,
            lhs: Expr::Binary { op: BinOp::Sub, ty, .. },
            rhs: Expr::Const { value: 2, .. },
        } if *ty == ul)),
        "{e:?}"
    );
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
                    ty: Scalar::POINTER
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
    // Two levels and more are lowered (bead domain-pascal-ogf8; the
    // chain of links, `tests/exec/ogf8_nested.pas`).
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
    // A `for` index a nested routine reaches, and a nested routine's loop
    // over the enclosing routine's variable: kept in memory, in the frame
    // (RINGLOG_$CNTL 0xE72296) and through the static link (0xE721F0);
    // bead domain-pascal-1mtr.
    for (src, home) in [
        (
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
            Addr::Frame(-2),
        ),
        (
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
            Addr::Enclosing(-2),
        ),
    ] {
        let m = lower_src(src).unwrap_or_else(|e| panic!("{e:?}"));
        let mut found = None;
        for f in &m.functions {
            crate::visit_stmts(&f.body, &mut |s| {
                if let StmtKind::For(l) = &s.kind {
                    found = Some((l.home.clone(), l.index.offset));
                }
            });
        }
        assert_eq!(found, Some((Some(home), 0)), "{src}");
    }
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
            Arg::Value { .. } | Arg::Temp { .. } | Arg::Call { .. } => panic!("{a:?}"),
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
            Arg::Value { .. } | Arg::Temp { .. } | Arg::Call { .. } => panic!("{a:?}"),
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

/// An integer constant for a 4-byte non-integer `univ` parameter (bead
/// domain-pascal-45yc; ref 5.3.3 p. 5-11, 3.2.3 p. 3-5): an integer32
/// cell, the value sign-extended, big-endian, keyed as integer32 (SAU5
/// MBUS_$UNDEF_INT 0xE0AA10, cell 0xE0AA20 `0000 002a` for
/// CRASH_SYSTEM's status).
#[test]
fn univ_small_constants_for_a_4_byte_record() {
    let m = lower_src(
        "module m;
type r = record a, b: integer; end;
  u = 0..65535;
const c = 16#2A;
procedure q(in a: univ r; in b: univ r; in d: univ r; in e: univ integer32);
  extern;
procedure p;
begin
  q(c, -1, lastof(u), 42);
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
            &[0, 0, 0, 0x2a][..],
            &[0xff, 0xff, 0xff, 0xff],
            &[0, 0, 0xff, 0xff],
            &[0, 0, 0, 0x2a]
        ]
    );
    // All keyed as integer32: 16#2A for the record and for the integer32
    // parameter share one cell.
    assert!(cells.iter().all(|(_, k)| *k == cells[0].1));

    // A real formal: a nonzero integer constant stays refused (converted
    // or its bits: not settled); 0 is the same bytes either way.
    let err = lower_src(
        "module m;
procedure q(in x: univ real); extern;
procedure p;
begin
  q(7);
end;
",
    )
    .unwrap_err();
    assert!(err.iter().any(|e| e.contains("real `univ` parameter")), "{err:?}");
    assert!(lower_src(
        "module m;
procedure q(in x: univ real); extern;
procedure p;
begin
  q(0);
end;
",
    )
    .is_ok());
}

/// `addr(x)` of another unit's object passed by address, to a 192-byte
/// `univ` parameter and to a `univ_ptr`, is a cell holding the address
/// (bead domain-pascal-68tc; SAU5 VME_$INT 0xE0ABBE, cell 0xE0AC40).
#[test]
fn symbol_addresses_by_address_are_cells() {
    let m = lower_src(
        "module m;
type big = array[1..96] of integer;
  r = record a, b: integer; end;
var x: extern r;
procedure q(in a: univ big; in b: univ_ptr; in c: integer32); extern;
procedure p;
begin
  q(addr(x), addr(x.b), integer32(addr(x)));
end;
",
    )
    .unwrap();
    let StmtKind::Call(c) = &func(&m, "P").body[0].kind else {
        panic!()
    };
    let cells: Vec<PoolConst> = c
        .args
        .iter()
        .map(|a| match a {
            Arg::Address {
                addr: Addr::Const(k),
                slot: 4,
                writable: false,
            } => k.clone(),
            a => panic!("{a:?}"),
        })
        .collect();
    assert_eq!(
        cells,
        [
            PoolConst::address("X".into(), 0),
            PoolConst::address("X".into(), 2),
            PoolConst::address("X".into(), 0)
        ]
    );

    // A variable of the unit's own overlay section, a symbol plus an
    // offset too, stays a frame temporary (SAU2 PCHIST_$CNTL 0xE5CF06).
    let m = lower_src(
        "module m;
var (ov_d) pages: array[1..3] of integer32; count: integer;
procedure q(in a: univ_ptr); extern;
procedure p;
begin
  q(addr(count));
end;
",
    )
    .unwrap();
    let StmtKind::Call(c) = &func(&m, "P").body[0].kind else {
        panic!()
    };
    assert!(
        matches!(&c.args[..], [Arg::Temp { slot: 4, .. }]),
        "{:?}",
        c.args
    );
}

/// What `univ` extension is not supported yet (beads domain-pascal-4be,
/// -9286): an integer constant larger than the parameter, a boolean or
/// enumeration constant of another size, an integer constant for a
/// non-integer parameter of less than 4 bytes, and an expression
/// temporary of another size than the parameter.
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
            "in a: univ c3",
            "0",
            "2-byte constant of this type to a 3-byte `univ` parameter",
        ),
        (
            "in a: univ c3",
            "red",
            "2-byte constant of this type to a 3-byte `univ` parameter",
        ),
        (
            "in a: univ char",
            "0",
            "2-byte constant of this type to a 1-byte `univ` parameter",
        ),
        (
            "in a: univ integer32",
            "'2'",
            "1-byte constant of this type to a 4-byte `univ` parameter",
        ),
        (
            "in a: univ buf",
            "-1",
            "a nonzero integer constant for a 1024-byte non-integer `univ` parameter",
        ),
        (
            "in a: univ buf",
            "16#12345",
            "a nonzero integer constant for a 1024-byte non-integer `univ` parameter",
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
  c3 = array[1..3] of char;
  color = (red, green);
  buf = array[1..1024] of char;
var w: extern integer;
procedure q({param}); extern;
procedure p;
begin
  q({arg});
end;
"
        ));
        assert!(e[0].starts_with("t.pas:10:5: error: "), "{e:?}");
        assert!(e[0].contains(msg), "{e:?}");
        assert!(e[0].ends_with("not supported yet"), "{e:?}");
    }
}

/// A constant for a `univ` parameter of another size keeps a cell of its
/// own size (beads domain-pascal-9286, -c5u; codegen-patterns 1.5, "To a
/// `univ` parameter"): a string its characters, an odd count padded with
/// a zero byte (0xE2FFD4), whatever the parameter's size (0xE30020, 36
/// characters for a 4-byte parameter); an integer constant for a
/// non-integer parameter of 4 bytes or more an integer32 cell (0xE12BB4,
/// 0xE7508C); a character a word with it in the high byte (0xE333DA).
#[test]
fn univ_constants_keep_their_own_size() {
    let m = lower_src(
        "module m;
type buf = array[1..1024] of char;
  st = record all: integer32; end;
  arg = [aligned(0)] integer32;
procedure q(in a: univ buf; in b: univ integer32; in c: univ st; in d: univ buf;
  in e: univ buf; in f: univ arg);
  extern;
procedure p;
begin
  q('abc', 'abcd', 0, '2', 0, 0);
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
            &b"abc\0"[..],
            b"abcd",
            &[0, 0, 0, 0],
            &[b'2', 0],
            &[0, 0, 0, 0],
            &[0, 0, 0, 0],
        ]
    );
    // The 0s for the record, the buffer and the `[aligned(0)] integer32`
    // are integer32 cells, one key (0xE2FFFC: VFMT_$WRITE2's argument and
    // LOG_$ADD's `univ` data share it).
    assert_eq!(cells[2].1, cells[4].1);
    assert_eq!(cells[2].1, cells[5].1);
    assert_ne!(cells[0].1, cells[1].1);
}

/// A field or element of a type transfer of a variable (`T(v).f`,
/// `T(v)[i]`; ref "Type Transfer Functions", p. 4-197, no bits changed)
/// is a load of `v`'s own bytes at the field's offset, no copy
/// (PROC1_$CREATE_P 0xE15156 `move.w (0xe,A6),D2`; FILE_$SET_ACL 0xE5E08A
/// `and.w (-0x44,A6),D0`; bead domain-pascal-w0rj).
#[test]
fn fields_of_type_transfers() {
    let src = "module m;
type
  uid_t = record high, low: integer32; end;
  view_t = record
    case integer of
      1: (uid: uid_t);
      2: (w0, w1: integer; w2: 0..65535; w3: integer);
      3: (b0, b1: char);
  end;
  args_t = record stack_size, proc_type: integer; end;
  arr_t = array[0..1] of integer;
  tbl_t = array[1..4] of uid_t;
var
  w: integer;
  c: char;
  l: integer32;
  tbl: tbl_t;
procedure s(a: integer32; u: uid_t; var r: uid_t; i: integer);
begin
  w := args_t(a).proc_type;
  w := view_t(u).w2;
  c := view_t(r).b1;
  w := arr_t(l)[1];
  w := view_t(tbl[3]).w3;
  w := view_t(tbl[i]).w1;
  w := arr_t(view_t(u).uid.low)[0];
end;";
    let m = lower_src(src).unwrap();
    let f = &m.functions[0];
    let load = |i: usize| match &f.body[i].kind {
        StmtKind::Store {
            value: Expr::Load { addr, ty },
            ..
        } => (addr.clone(), *ty),
        k => panic!("not a load stored: {k:?}"),
    };
    let word = Scalar::signed(Width::Word);
    // The callee's copy of the longword value parameter (at -4): its
    // second word.
    assert_eq!(load(0), (Addr::Frame(-2), word));
    // The UID's copy (at -12): its third word, a `0..65535`.
    assert_eq!(load(1), (Addr::Frame(-8), Scalar::unsigned(Width::Word)));
    // Through a `var` parameter's slot, at offset 1.
    assert!(matches!(load(2), (Addr::Indirect { offset: 1, .. }, ty)
        if ty == Scalar::unsigned(Width::Byte)));
    // An element of the unit's longword (at 4): its second word.
    assert_eq!(load(3), (Addr::Static(6), word));
    // A constant element of the table (at 8), its last word: 8 + 2 * 8 + 6.
    assert_eq!(load(4), (Addr::Static(30), word));
    // A run-time element: indexed, the field folded into the offset.
    assert!(matches!(
        load(5).0,
        Addr::Index {
            scale: 8,
            offset: -6,
            ..
        }
    ));
    // A transfer of a field of a transfer: the copy's `low` (at -8).
    assert_eq!(load(6), (Addr::Frame(-8), word));
    // Not designators (ref p. 4-197 does not say a transfer is a
    // variable): refused where one is needed; and a transfer of a value
    // not in memory (its temporary is not seen).
    let refused = |stmt: &str| {
        errs(&src.replace("begin\n  w :=", &format!("begin\n  {stmt}\n  w :="))).join("\n")
    };
    // Open question 46: assignment, `var` argument, `addr`, `with`.
    assert!(refused("args_t(l).stack_size := 1;")
        .contains("assigning to a type transfer of a variable"));
    assert!(refused("view_t(u).uid := u;").contains("open question 46"));
    assert!(refused("w := ord(addr(args_t(l).proc_type) = nil);")
        .contains("`addr` of a type transfer of a variable"));
    assert!(refused("with args_t(a) do w := proc_type;")
        .contains("`with` over a type transfer of a variable"));
    // Not of a variable: still the plain errors.
    assert!(
        refused("with args_t(l + 1) do w := proc_type;").contains("with needs a record variable")
    );
    assert!(refused("w := args_t(l + 1).proc_type;")
        .contains("type transfer of a value that is not a variable"));
    assert!(refused("s(args_t(l).proc_type, u, r, i);").contains("type transfer passed by address"));
    assert!(refused("s(a, u, view_t(u).uid, i);")
        .contains("passed to a `var`, `out` or `in out` parameter"));
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
    let Expr::Convert { value: sum, ty, .. } = value(0) else {
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
    assert!(matches!(*rhs, Expr::Convert { ref value, ty, .. }
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

/// A routine reaches its own data section's block through the data base
/// ([`Addr::Static`]) and another data section's block as
/// [`Addr::OtherStatic`] (codegen-patterns 11: PROC1_$INIT 0xE2F95C
/// `movea.l #0xE254E8,A0`, PROC1_'s WIRED_DATA block from OS_INIT_PROC),
/// which does not make the routine load its data base.
#[test]
fn static_of_another_data_section() {
    let m = lower_src(
        "module s, wired_proc, wired_data;
var
  hits: integer32;
[section(init_proc, init_data)]
procedure s_$init;
var
  n: static integer;
begin
  hits := 0;
  n := 1;
end;
",
    )
    .unwrap();
    let f = &m.functions[0];
    assert_eq!(f.data_section, "init_data");
    assert!(f.loads_data_base);
    let addrs: Vec<&Addr> = f
        .body
        .iter()
        .map(|st| match &st.kind {
            StmtKind::Store { addr, .. } => addr,
            k => panic!("{k:?}"),
        })
        .collect();
    assert_eq!(
        addrs,
        [
            &Addr::OtherStatic {
                section: "wired_data".into(),
                offset: 0
            },
            &Addr::Static(0),
        ]
    );
    // Only another section's data: no data base.
    let m = lower_src(
        "module s, wired_proc, wired_data;
var
  hits: integer32;
[section(init_proc, init_data)]
procedure s_$init;
begin
  hits := 0;
end;
",
    )
    .unwrap();
    assert!(!m.functions[0].loads_data_base);
    // A nested routine has its top-level routine's data base.
    let m = lower_src(
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
    )
    .unwrap();
    let q = m.functions.iter().find(|f| f.symbol != "S_$P").unwrap();
    assert!(matches!(
        &q.body[0].kind,
        StmtKind::Store {
            addr: Addr::OtherStatic { section, offset: 0 },
            ..
        } if section == "other_data"
    ));
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

/// Overlay sections, `var (name)` (ref 3.14, p. 3-52; 7.2.4, p. 7-7;
/// codegen-patterns 11, "Overlay sections"): allocated in order, a later
/// part of the same name continuing the first; their variables are
/// offsets into the section's symbol, never the unit's data area.
#[test]
fn overlay_sections() {
    let src = "module m, , wired_data;
var
  a5var: integer;
var (pchist_unwired_d)
  proc_data: array[1..64] of array[1..5] of integer32;
  wire_pages: array[1..3] of integer32;
var (other_sec)
  o: integer;
var (pchist_unwired_d)
  wired_count: integer;
  flag: boolean;
var (wired_data)
  w: integer;
define p;
procedure p;
var s: static integer32;
var (other_sec) t: integer32;
begin
  wired_count := 1;
  wire_pages[2] := 0;
  o := 2;
  t := 3;
  w := 4;
end;
";
    let m = lower_src(src).unwrap();
    let at = |n: &str, offset| Addr::Symbol {
        name: n.into(),
        offset,
    };
    let stores: Vec<&Addr> = m.functions[0]
        .body
        .iter()
        .map(|st| match &st.kind {
            StmtKind::Store { addr, .. } => addr,
            k => panic!("{k:?}"),
        })
        .collect();
    assert_eq!(
        stores,
        [
            &at("PCHIST_UNWIRED_D", 0x50C),
            &at("PCHIST_UNWIRED_D", 0x504),
            &at("OTHER_SEC", 0),
            &at("OTHER_SEC", 4),
            // `var (wired_data)`: the heading's data section, the block.
            &Addr::Static(2),
        ]
    );
    let ov: Vec<(&str, u32)> = m
        .overlays
        .iter()
        .map(|d| (d.section.as_str(), d.size))
        .collect();
    assert_eq!(ov, [("pchist_unwired_d", 0x50F), ("other_sec", 8)]);
    assert!(m
        .overlays
        .iter()
        .all(|d| d.init.is_empty() && d.globals.is_empty()));
    let blocks: Vec<(&str, u32)> = m
        .data
        .iter()
        .map(|d| (d.section.as_str(), d.size))
        .collect();
    assert_eq!(blocks, [("wired_data", 8)]);

    // Initial values; `addr` of an overlay variable as one.
    let m = lower_src(
        "module m;
type ip = ^integer;
var (sec) a: integer := 5; b: integer32;
var p: ip := addr(b);
",
    )
    .unwrap();
    assert_eq!(
        m.overlays[0].init,
        [DataInit {
            offset: 0,
            value: InitValue::Int { size: 2, value: 5 }
        }]
    );
    assert_eq!(
        m.data[0].init,
        [DataInit {
            offset: 0,
            value: InitValue::Address(InitAddr::Symbol {
                name: "SEC".into(),
                offset: 4
            })
        }]
    );

    // `define`d variables (ref 3.14: a `define` of an `extern` variable
    // of a `var (name)` part allocates it there; bead domain-pascal-m15q):
    // the area's globals, at their offsets; addressed through the
    // section's symbol like the others.
    let m = lower_src(
        "module m;
var (sec) a: integer; b: define integer32; c: extern integer;
define c;
procedure p;
begin
  c := 1;
end;
",
    )
    .unwrap();
    assert_eq!(
        m.overlays[0].globals,
        [("B".to_string(), 4), ("C".to_string(), 8)]
    );
    assert_eq!(m.overlays[0].size, 10);
    match &m.functions[0].body[0].kind {
        StmtKind::Store { addr, .. } => assert_eq!(addr, &at("SEC", 8)),
        k => panic!("{k:?}"),
    }
    // the section named like a `define`d variable of the unit
    let e = errs("module m;\nvar (v) v: define integer;\n");
    assert!(
        e[0].contains("named like a routine or variable symbol of the unit (`V`)"),
        "{e:?}"
    );

    // Refused: names that are no assembler symbol, or the unit's own
    // symbols; two spellings of one symbol; sections beyond 0x8000 bytes.
    let e = errs("module m;\nvar ('a b') v: integer;\n");
    assert!(e[0].contains("overlay section name `a b`"), "{e:?}");
    let e = errs("module m;\nvar ('.sec') v: integer;\n");
    assert!(e[0].contains("overlay section name `.sec`"), "{e:?}");
    let e = errs("module m;\nvar (p) v: integer;\ndefine p;\nprocedure p; begin end;\n");
    assert!(
        e[0].contains("named like a routine or variable symbol of the unit (`P`)"),
        "{e:?}"
    );
    let e = errs("module m;\nvar (sec) v: integer;\nvar ('SEC') w: integer;\n");
    assert!(e[0].contains("one symbol `SEC`"), "{e:?}");
    // the unit's own data section spelt another way (question 44)
    let e = errs("module m, , wired_data;\nvar ('WIRED_DATA') v: integer;\n");
    assert!(
        e[0].contains("the unit's data section `wired_data` spelt another way"),
        "{e:?}"
    );

    // A section beyond 0x8000 bytes (ACL_$DATA, 0xAD98; codegen-patterns
    // 5.2, "Overlay sections of more than 0x8000 bytes"): its variables
    // offsets into its symbol as in any other; the base of its arrays is
    // the target's.  An array that is a part of its variable there is
    // refused (the image's are whole variables, or their first element).
    let big = "module m;
type r = record h: integer; a: array[1..8] of integer; end;
var (sec) c: array[0..30] of array[1..512] of integer;
  s: array[1..64] of r;
  q: r;
procedure p(in i: integer);
var x: integer;
begin
  x := s[2].h;
  x := s[i].a[3];
  x := c[0][i];
  BODY
end;
";
    let m = lower_src(&big.replace("BODY", "")).unwrap();
    assert_eq!(m.overlays[0].size, 0x7C00 + 64 * 18 + 18);
    let loads: Vec<&Addr> = m.functions[0]
        .body
        .iter()
        .map(|st| match &st.kind {
            StmtKind::Store {
                value: Expr::Load { addr, .. },
                ..
            } => addr,
            k => panic!("{k:?}"),
        })
        .collect();
    assert_eq!(loads[0], &at("SEC", 0x7C00 + 18));
    // `c[0][i]`: the first element's array, at the variable's offset.
    assert!(
        matches!(loads[2], Addr::Index { base, .. } if **base == at("SEC", 0)),
        "{:?}",
        loads[2]
    );
    for body in ["x := q.a[i];", "x := c[1][i];"] {
        let e = errs(&big.replace("BODY", body));
        assert!(
            e[0].contains("a part of its variable") && e[0].contains("more than 0x8000 bytes"),
            "{body}: {e:?}"
        );
    }
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
fn packed_signed_field_sign_extended() {
    // Best effort (bead domain-pascal-eoc6; refused under `--strict`,
    // above): a field of negative values, its bits zero-extended, then
    // `lshft`/`arshft` by 16 - 4.
    let was = dpc_syntax::diag::set_strict(false);
    let m = lower_src(
        "module m;
         type r = packed record a: boolean; s: -8..7 end;
         var g: extern integer;
         procedure t(var q: r); begin g := q.s end;",
    );
    dpc_syntax::diag::set_strict(was);
    let w = dpc_syntax::diag::take_best_effort();
    assert!(
        w.iter().any(|d| d.message.contains("negative values")),
        "{w:?}"
    );
    let m = m.unwrap();
    let StmtKind::Store { value, .. } = &m.functions[0].body[0].kind else {
        panic!("a store")
    };
    let Expr::Binary {
        op: BinOp::Ashr,
        lhs,
        rhs,
        ..
    } = value
    else {
        panic!("{value:?}")
    };
    assert!(matches!(**rhs, Expr::Const { value: 12, .. }), "{rhs:?}");
    let Expr::Binary {
        op: BinOp::Shl,
        lhs: bits,
        ..
    } = &**lhs
    else {
        panic!("{lhs:?}")
    };
    assert!(
        matches!(
            **bits,
            Expr::Bits {
                field: BitField {
                    unit: Width::Byte,
                    shift: 3,
                    width: 4
                },
                ..
            }
        ),
        "{bits:?}"
    );
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
            // byte variables one to a word (codegen-patterns 11)
            int(8, 1, 0xFF),
            int(10, 1, 0),
            int(12, 1, 0x41),
            int(14, 2, 2),
            // members 0, 3, 4, 5 in the last byte, 31 in the first
            bytes(16, &[0x80, 0, 0, 0x39]),
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

#[test]
fn packed_bit_field_initial_values() {
    // ref 3.10.7 p. 3-35; KBD's kbd_$state_entry_t (WIRED_DATA
    // 0xE2DDE4+0xC0 `FB 20'): the char a byte, the 4-bit fields bits
    // 0..3 and 4..7 of the next; a boolean one bit, set for true; a
    // negative subrange its two's complement in its 8 bits.
    use crate::{DataInit, InitValue};
    let m = lower_src(
        "module m;
type
  e = packed record key: char; action: 0..15; next: 0..15 end;
  n = packed record f: boolean; s: -100..40 end;
var
  t: e := [key := chr(16#FB), action := 2, next := 0];
  x: n := [true, -100];
procedure z;
begin
end;
",
    )
    .unwrap();
    let bits = |offset, bit, width, value| DataInit {
        offset,
        value: InitValue::Bits { bit, width, value },
    };
    assert_eq!(
        m.data[0].init,
        vec![
            DataInit {
                offset: 0,
                value: InitValue::Int {
                    size: 1,
                    value: 0xFB
                },
            },
            bits(1, 0, 4, 2),
            bits(1, 4, 4, 0),
            bits(2, 0, 1, 1),
            bits(2, 1, 8, 0x9C),
        ]
    );
}

/// Owner's ruling on 3.13.4, p. 3-51 (docs/reference-clarifications.md
/// "3.13.4 addr(...) as an initial value"): `addr(...)` initial values
/// are addresses fixed at link time: a routine's or another unit's
/// symbol, or the unit's data area (a `static` variable has no symbol),
/// plus an offset.
#[test]
fn address_initial_values() {
    use crate::{DataInit, InitAddr, InitValue};
    let m = lower_src(
        "module m;
type pr = ^procedure(in x: integer32);
     epv = record init: pr; mon: pr; data: ^integer32 end;
procedure m_$ext(in x: integer32); extern;
procedure m_$init(in x: integer32); forward;
var
  c: integer32;
  t: array[1..3] of integer32;
  o: extern integer32;
  v: epv := [init := addr(m_$init), mon := nil, data := addr(c)];
  pt: ^integer32 := addr(t[2]);
  po: ^integer32 := addr(o);
  pr2: pr := addr(m_$ext);
procedure m_$init;
var s: static ^integer32 := addr(t[3]);
begin
  c := x;
end;
",
    )
    .unwrap();
    let sym = |offset, name: &str, off| DataInit {
        offset,
        value: InitValue::Address(InitAddr::Symbol {
            name: name.into(),
            offset: off,
        }),
    };
    let stat = |offset, off| DataInit {
        offset,
        value: InitValue::Address(InitAddr::Static {
            section: ".data".into(),
            offset: off,
        }),
    };
    assert_eq!(
        m.data[0].init,
        vec![
            sym(16, "M_$INIT", 0),
            DataInit {
                offset: 20,
                value: InitValue::Int { size: 4, value: 0 },
            },
            stat(24, 0),
            stat(28, 8),
            sym(32, "O", 0),
            sym(36, "M_$EXT", 0),
            stat(40, 12),
        ]
    );
}

/// A routine's `static` variable in its own data section may hold the
/// address of a variable of another data section: a relocation, not an
/// access through the routine's data base (which reaches only its own
/// section's block, codegen-patterns 4, 11).
#[test]
fn address_initial_value_across_data_sections() {
    use crate::{InitAddr, InitValue};
    let m = lower_src(
        "module m, mcode, mdata;
var c: integer32; d: integer32;
[section(pc, pdata)]
procedure m_$p;
var s: static integer32; ps: static univ_ptr := addr(d);
begin
  s := 1;
end;
",
    )
    .unwrap();
    let pdata = m.data.iter().find(|d| d.section == "pdata").unwrap();
    assert_eq!(
        pdata.init[0].value,
        InitValue::Address(InitAddr::Static {
            section: "mdata".into(),
            offset: 4,
        })
    );
}

/// A later initial value over part of an address (a variant's field)
/// is refused: its bytes would be the address's.
#[test]
fn initial_value_over_part_of_an_address_is_refused() {
    let e = lower_src(
        "module m;
type r = record case integer of 1: (p: ^integer32); 2: (w: integer) end;
var c: integer32; v: r := [p := addr(c), w := 1];
procedure z; begin end;
",
    )
    .unwrap_err();
    assert!(
        e.iter()
            .any(|d| d.contains("an initial value over part of an address initial value")),
        "{e:#?}"
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
    // A view through a pointer type transfer, `T(addr(x))^`, is `x`:
    // volatile when `x` is (bead domain-pascal-yqf6).
    let view = "type ip = ^integer; rp = ^regs; var d: extern [device] integer; dr: extern regs;";
    assert_eq!(
        vol(&format!("{view} procedure p; begin ip(addr(d))^ := 0 end;")),
        Some(Access {
            device: true,
            atomic: false
        })
    );
    assert_eq!(
        vol(&format!("{view} procedure p; begin rp(addr(dr))^.csr := 'a' end;")),
        Some(Access {
            device: true,
            atomic: false
        })
    );
    assert_eq!(
        vol(&format!("{view} procedure p; begin ip(addr(q.key))^ := 0 end;")),
        Some(dflt)
    );
    assert_eq!(
        vol(&format!("{view} procedure p; begin ip(addr(i))^ := 0 end;")),
        None
    );
    // Of a record with a volatile part: refused (the view's type does not
    // say which bytes are volatile).
    let e = errs(&format!("{types}{view} procedure p; begin ip(addr(q))^ := 0 end;"));
    assert!(e[0].contains("volatile or device part"), "{e:?}");
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
/// `device(write)` ones not read; a device record's packed field is stored
/// with the field form unless write-only.  A record with volatile
/// parts is block-copied, its side wrapped in [`Addr::Volatile`]
/// (codegen-patterns 5.5; bead domain-pascal-e07e).
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
    // A device record's packed field is stored with the field form, its
    // unit's address wrapped (bead domain-pascal-eaw7); a write-only one
    // is refused (the form reads the unit), a read-only one is an error.
    let m = lower_src(&format!("{types}procedure x; begin p.a := 1 end;")).unwrap();
    let StmtKind::StoreBits {
        addr: Addr::Volatile { access, .. },
        ..
    } = &m.functions[0].body[0].kind
    else {
        panic!("{:#?}", m.functions[0].body)
    };
    assert!(access.device);
    let wo_rec = "module m;
type
  pw = [device(write)] packed record a: 0..15; b: 0..15 end;
  pr = [device(read)] packed record a: 0..15; b: 0..15 end;
var
  w: extern pw;
  r: extern pr;
";
    let e = errs(&format!("{wo_rec}procedure x; begin w.a := 1 end;"));
    assert!(e[0].contains("packed field of a write-only device record"), "{e:?}");
    let e = errs(&format!("{wo_rec}procedure x; begin r.a := 1 end;"));
    assert!(e[0].contains("writing a read-only device object"), "{e:?}");
    let m = lower_src(&format!("{types}procedure x; begin v1 := v2 end;")).unwrap();
    let StmtKind::Copy {
        dst: Addr::Volatile { access: da, .. },
        src: Addr::Volatile { access: sa, .. },
        bytes: 12,
        chars: false,
    } = &m.functions[0].body[0].kind
    else {
        panic!("{:#?}", m.functions[0].body)
    };
    assert!(!da.device && !sa.device, "{da:?} {sa:?}");
}

/// A record or array copy with a volatile or device side (ref
/// 3.15.1-3.15.3; bead domain-pascal-e07e): that side's address wrapped in
/// [`Addr::Volatile`], `device` for a device or a record with a device
/// part, the other side plain (the sau5 WIN_ RESTORE 0xE19A1E copies the
/// window's descriptor into a plain local).  A part that denies the
/// access is an error: a record with a `device(write)` field is not read
/// whole, one with a `device(read)` field not written whole (a record
/// with both inherits `device(read, write)`, ref 3.15.7, so its own
/// attribute does not say); a `device(read)` record is read, a
/// `device(write)` one written.
#[test]
fn device_record_copies() {
    let types = "module m;
type
  desc = record cyl: integer; hs: integer; phys: integer32; page: integer; drive: integer end;
  regs = [device] record go: integer; desc: array[1..2] of desc end;
  ro = [device(read)] integer;
  wo = [device(write)] integer;
  wpart = record a: wo; b: integer; c: integer32 end;
  rpart = record a: ro; b: integer; c: integer32 end;
  plain = record a, b, c: integer32 end;
  ror = [device(read)] plain;
  wor = [device(write)] plain;
var
  rp: extern ^regs;
  d: extern desc;
  wp1, wp2: extern wpart;
  rp1, rp2: extern rpart;
  mr: extern ror;
  mw: extern wor;
  pl: extern plain;
";
    for (stmt, dst_dev, src_dev) in [
        ("d := rp^.desc[1]", None, Some(true)),
        ("rp^.desc[2] := d", Some(true), None),
        ("rp^.desc[2] := rp^.desc[1]", Some(true), Some(true)),
        ("pl := mr", None, Some(true)),
        ("mw := pl", Some(true), None),
    ] {
        let m = lower_src(&format!("{types}procedure x; begin {stmt} end;"))
            .unwrap_or_else(|e| panic!("{stmt}: {e:?}"));
        let StmtKind::Copy { dst, src, .. } = &m.functions[0].body[0].kind else {
            panic!("{stmt}: {:#?}", m.functions[0].body)
        };
        let dev = |a: &Addr| a.unwrap_volatile().1.map(|a| a.device);
        assert_eq!((dev(dst), dev(src)), (dst_dev, src_dev), "{stmt}");
    }
    for (stmt, what) in [
        ("wp1 := wp2", "reading a record or array with a write-only device part"),
        ("rp1 := rp2", "writing a record or array with a read-only device part"),
        ("mr := pl", "writing a read-only device object"),
        ("pl := mw", "reading a write-only device object"),
    ] {
        let e = errs(&format!("{types}procedure x; begin {stmt} end;"));
        assert!(e[0].contains(what), "{stmt}: {e:?}");
    }
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
/// but may lie at an odd address all the same (`f1` after a char at 1 in
/// a record): refused when it is not known to be even, accepted when it
/// is.  A static variable is always at an even offset (codegen-patterns
/// 11, "Byte-sized module variables"), so `x` after a char is at 2.
#[test]
fn elements_of_byte_aligned_arrays_at_odd_addresses_are_refused() {
    let types = "module m;
type
  oa = [aligned(0)] array[1..2] of integer16;
  oa1 = [aligned(0)] array[1..1] of integer16;
  r1 = record c: char; f: oa end;
  r2 = record c: char; f1: oa1 end;
var
  c0: char;
  x: oa;
  rr: r1;
  r2v: r2;
  i: integer;
procedure v(n: integer); val_param; extern;
";
    let e = errs(&format!("{types}procedure p; begin i := r2v.f1[1] end;"));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    let e = errs(&format!("{types}procedure p; begin v(r2v.f1[1]) end;"));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    lower_src(&format!(
        "{types}procedure p; begin i := x[1]; v(x[2]) end;"
    ))
    .unwrap();
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
var x: extern oa; y: oa;
procedure p; begin x := y end;",
    );
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
}

/// A parameter whose type carries an alignment attribute (owner ruling,
/// docs/reference-clarifications.md "5.1 Attribute lists on parameter
/// types"; ref 3.15.6.8 p. 3-76): an argument passed by address may be
/// only as aligned as the type says.  `[aligned(0)] integer32` (Apollo's
/// error.ins.pas) may be at an odd address, so a longword reference
/// through it is refused, as is the entry copy of a value parameter of
/// that type; `[aligned(1)]` (rws.ins.pas) is word aligned, which the
/// 68010 needs and no more.  A by-value one (`val_param`) is in the frame.
#[test]
fn byte_aligned_parameters_are_refused_where_referenced() {
    let types = "module m;
type
  status_$t = record all: integer32 end;
  b32 = [aligned(0)] integer32;
var g: integer32;
";
    let e = errs(&format!(
        "{types}procedure p(in a: univ [aligned(0)] integer32); begin g := a end;"
    ));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    let e = errs(&format!(
        "{types}procedure p(out a: b32); begin a := 0 end;"
    ));
    assert!(e[0].contains("byte-aligned object"), "{e:?}");
    let e = errs(&format!(
        "{types}procedure p(a: [aligned(0)] integer32); begin g := a end;"
    ));
    assert!(e[0].contains("byte-aligned type"), "{e:?}");
    // Passed on by address without a reference: accepted.
    lower_src(&format!(
        "{types}procedure q(in a: univ b32); extern;
procedure p(in a: univ [aligned(0)] integer32); begin q(a) end;"
    ))
    .unwrap();
    lower_src(&format!(
        "{types}procedure p(out status: [aligned(1)] status_$t);
begin status.all := 0 end;
procedure v(a: [aligned(0)] integer32); val_param; begin g := a end;"
    ))
    .unwrap();
}

/// Every word or longword path through a byte-aligned object is refused
/// unless its address is known to be even (bead domain-pascal-lmw review):
/// a record comparison (`cmpm`) and a packed field's word or longword
/// unit, for an element of a byte-aligned array reached through a
/// parameter (ref 3.15.6.8 p. 3-76).  A record with `[aligned(0)]` is
/// still word aligned (ref 3.10.5.5 p. 3-26), so its own fields and its
/// entry copy are accepted.
#[test]
fn byte_aligned_parameters_refuse_every_word_path() {
    let types = "module m;
type
  uid_t = record hi, lo: integer32 end;
  pr = packed record a: 0..15; b: 0..4095; c: integer16 end;
  bau = [aligned(0)] array[1..2] of uid_t;
  bap = [aligned(0)] array[1..2] of pr;
  buid = [aligned(0)] uid_t;
var g: integer32; gw: integer16; gu: uid_t;
";
    for body in [
        "procedure p(in a: bau); begin if a[1] = gu then g := 1 end;",
        "procedure p(in a: bau); begin if gu = a[2] then g := 1 end;",
        "procedure p(in a: bap); begin gw := a[1].b end;",
        "procedure p(var a: bap); begin a[1].b := 3 end;",
        "procedure p(in a: bau); begin with a[1] do g := hi end;",
    ] {
        let e = errs(&format!("{types}{body}"));
        assert!(e[0].contains("byte-aligned object"), "{body}: {e:?}");
    }
    // A byte unit is never odd-trapped.
    lower_src(&format!(
        "{types}procedure p(in a: bap); begin gw := a[1].a end;"
    ))
    .unwrap();
    // `[aligned(0)]` on a record: still word aligned.
    lower_src(&format!(
        "{types}procedure p(in a: buid); begin g := a.hi; if a = gu then g := 1 end;
procedure v(a: buid); begin g := a.lo end;"
    ))
    .unwrap();
}

/// An alignment attribute changes a record's size (ref 3.15.6.2, Figure
/// 3-21 p. 3-67), but `[aligned(2)] T` is `T` for compatibility: a
/// variable of the plain type passed to a `var`/`out` parameter of the
/// attributed one would be written beyond its end (here into the saved
/// frame pointer); it is refused, as is comparing records of the two
/// sizes.  `univ` turns the check off (ref 5.3.3).
#[test]
fn records_resized_by_alignment_are_not_passed_by_address() {
    let types = "module m;
type
  rec2 = record a: integer end;
  r4 = [aligned(2)] rec2;
  rec6 = record a, b, c: integer end;
var t: r4; g: integer32; x6: rec6;
procedure q(out r: [aligned(2)] rec2); begin r := t end;
";
    let e = errs(&format!("{types}procedure z; var x: rec2; begin q(x) end;"));
    assert!(e[0].contains("2-byte variable to a 4-byte"), "{e:?}");
    let e = errs(&format!(
        "{types}procedure z(in r: [aligned(2)] rec6); begin if x6 = r then g := 1 end;"
    ));
    assert!(e[0].contains("records of different sizes"), "{e:?}");
    lower_src(&format!(
        "{types}procedure u(out r: univ r4); extern;
procedure z; var x: r4; y: rec2; begin q(x); u(y) end;"
    ))
    .unwrap();
}

/// Volatile and device attributes on a parameter's type flow to the
/// references through it (ref 3.15.1-3.15.3; the parameter's type is the
/// one `= [attrs] T` declares), by address and in a value slot.
#[test]
fn volatile_parameter_types_are_volatile_references() {
    let types = "module m;
var g: integer;
";
    match only_stmt(&format!(
        "{types}procedure p(var x: [volatile] integer); begin g := x end;"
    )) {
        StmtKind::Store {
            value: Expr::Load { addr, .. },
            ..
        } => assert_eq!(addr.unwrap_volatile().1, Some(Access::default())),
        s => panic!("{s:?}"),
    }
    match only_stmt(&format!(
        "{types}procedure p(var x: [device] integer); begin x := 1 end;"
    )) {
        StmtKind::Store { addr, .. } => assert_eq!(
            addr.unwrap_volatile().1,
            Some(Access {
                device: true,
                atomic: false
            })
        ),
        s => panic!("{s:?}"),
    }
    let e = errs(&format!(
        "{types}procedure p(var x: [device(read)] integer); begin x := 1 end;"
    ));
    assert!(e[0].contains("read-only device object"), "{e:?}");
    // A value slot (`val_param`) too: every reference stays in the slot.
    match only_stmt(&format!(
        "{types}procedure p(x: [volatile] integer); val_param; begin g := x end;"
    )) {
        StmtKind::Store {
            value: Expr::Load { addr, .. },
            ..
        } => assert!(matches!(addr, Addr::Volatile { .. }), "{addr:?}"),
        s => panic!("{s:?}"),
    }
}

/// A function result of a volatile type is refused: the result variable
/// may be held in a register (bead domain-pascal-lmw review).
#[test]
fn volatile_function_results_are_refused() {
    let e = errs("module m; function f: [volatile] integer32; begin f := 1 end;");
    assert!(e[0].contains("function result of a volatile"), "{e:?}");
}

/// A value parameter that is a record with device or volatile parts is
/// not copied on entry (no entry copy of one is seen; an assignment of
/// one is block-copied, `device_record_copies`); a volatile scalar one
/// is.
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

/// Plain `and`/`or` (ref "And", p. 4-18; "Or", p. 4-134): both operands
/// kept, as [`Cond::And`]/[`Cond::Or`], in conditions and as values;
/// `odd(x)` (ref p. 4-128) as [`Cond::Odd`].
#[test]
fn plain_and_or_and_odd() {
    let b = ctl_body("if b and (w = 1) then w := 0;");
    let StmtKind::If { cond, .. } = &b[0].kind else {
        panic!("{b:?}")
    };
    assert!(
        matches!(cond, Cond::And(x, y) if matches!(**x, Cond::Test(_)) && matches!(**y, Cond::Compare { .. })),
        "{cond:?}"
    );
    let b = ctl_body("b := b or b2 or (l > 2);");
    let StmtKind::Store {
        value: Expr::Test(c),
        ..
    } = &b[0].kind
    else {
        panic!("{b:?}")
    };
    // Left to right: `(b or b2) or (l > 2)`.
    assert!(
        matches!(&**c, Cond::Or(x, _) if matches!(**x, Cond::Or(..))),
        "{c:?}"
    );
    let b = ctl_body("if odd(l) then w := 0;");
    let StmtKind::If { cond, .. } = &b[0].kind else {
        panic!("{b:?}")
    };
    assert_eq!(cond, &Cond::Odd(sym("L", Scalar::signed(Width::Long))));
    // A constant argument is folded by the checker.
    let b = ctl_body("b := odd(3);");
    assert!(
        matches!(
            &b[0].kind,
            StmtKind::Store {
                value: Expr::Const { value: -1, .. },
                ..
            }
        ),
        "{b:?}"
    );
}

/// `with` (ref "With", pp. 4-211..4-214): a record whose address has no
/// run-time part is named in place; one through a pointer the body may
/// change, or an element, gets a temporary assigned once at the start
/// ([`Function::with_temps`]), the constant displacements in each
/// reference (codegen-patterns 5.2, "`with`"; NETLOG_$LOG_IT 0xE71BCA).
#[test]
fn with_statements() {
    let src = |body: &str| {
        format!(
            "module w;\ntype r = record a: integer; b: integer32; end;\n  rp = ^r;\n\
             var g: r; p: rp; arr: array[1..4] of r; i: integer;\n\
             procedure t(q: rp; var v: r); options(val_param);\nvar lp: rp;\nbegin\n{body}\nend;\n"
        )
    };
    let f = |body: &str| lower_src(&src(body)).unwrap().functions[0].clone();
    // A variable: no code, the references as written; no frame slot
    // (the frame is `lp`'s 4 bytes).
    let a = f("with g do begin a := 1; b := 2 end;");
    assert!(a.with_temps.is_empty());
    assert_eq!(a.frame_size, 4);
    assert!(matches!(
        &a.body[0].kind,
        StmtKind::Store {
            addr: Addr::Static(0),
            ..
        }
    ));
    assert!(matches!(
        &a.body[1].kind,
        StmtKind::Store {
            addr: Addr::Static(2),
            ..
        }
    ));
    // A `var` parameter, the alias form: through its slot.
    let a = f("with v : x do x.b := 2;");
    assert!(a.with_temps.is_empty());
    assert_eq!(a.body.len(), 1);
    // A pointer parameter the body does not change: as written.
    let a = f("with q^ do begin a := 1; b := 2 end;");
    assert!(a.with_temps.is_empty());
    // A unit pointer: a temporary, its references at their displacements.
    let a = f("with p^ do begin a := 1; b := 2 end;");
    assert_eq!(a.with_temps, vec![crate::lower::WITH_TEMP_BASE]);
    // No frame slot for a pointer's temporary (HINT_$ADD_NET 0xE49C76
    // `link #0`; Uncertain, bead domain-pascal-njy0): `lp`'s 4 bytes.
    assert_eq!(a.frame_size, 4);
    let tmp = Addr::Frame(crate::lower::WITH_TEMP_BASE);
    assert!(
        matches!(&a.body[0].kind, StmtKind::Store { addr, value: Expr::Load { addr: Addr::Static(_), .. } } if *addr == tmp)
    );
    let StmtKind::Store {
        addr: Addr::Indirect { base, offset: 2 },
        ..
    } = &a.body[2].kind
    else {
        panic!("{:?}", a.body[2])
    };
    assert!(matches!(&**base, Expr::Load { addr, .. } if *addr == tmp));
    // A local pointer the body assigns: a temporary, so the record stays
    // the one of the start (p. 4-212, "changing p does not affect access").
    let a = f("lp := p;\nwith lp^ do begin a := 1; lp := p; b := 2 end;");
    assert_eq!(a.with_temps.len(), 1);
    // An element: its address without the lower bound, which goes into
    // the references.
    let a = f("with arr[i] do b := 2;");
    assert_eq!(a.with_temps.len(), 1);
    // Its 4-byte slot below the locals, never addressed (bead
    // domain-pascal-s9g0).
    assert_eq!(a.frame_size, 8);
    let StmtKind::Store {
        value:
            Expr::AddrOf(Addr::Index {
                offset: 0,
                scale: 6,
                ..
            }),
        ..
    } = &a.body[0].kind
    else {
        panic!("{:?}", a.body[0])
    };
    assert!(matches!(
        &a.body[1].kind,
        StmtKind::Store {
            addr: Addr::Indirect { offset: -4, .. },
            ..
        }
    ));
    // Nested: `with g, ...` resolves the second record in the first.
    let a = lower_src(
        "module w;\ntype s = record x: integer; end;\n  r = record a: integer; sub: s; end;\n\
         var g: r;\nprocedure t;\nbegin\nwith g, sub do x := 3;\nend;\n",
    )
    .unwrap();
    assert!(matches!(
        &a.functions[0].body[0].kind,
        StmtKind::Store {
            addr: Addr::Static(2),
            ..
        }
    ));
    // A record inside a `with` record reached at run time: through the
    // same temporary.
    let a = f("with p^ : x do with x do b := 2;");
    assert_eq!(a.with_temps.len(), 1);
    assert_eq!(a.body.len(), 2);
    assert_eq!(a.frame_size, 4);
}

/// Each `with` temporary of an array element keeps a longword-aligned
/// 4-byte frame slot below the locals (codegen-patterns 2, "Frame size";
/// bead domain-pascal-s9g0): MMAP_$AVAIL 0xE0CC64 `link #-4` with no
/// locals, XPD_$FIND_INDEX 0xE5AF38 `link #-8` with a word local,
/// NETLOG_$LOG_IT 0xE71B38 `link #-24` with 18 bytes of locals,
/// TERM_$SET_REAL_LINE_DISCIPLINE two.  A pointer's temporary keeps none
/// (HINT_$ADD_NET 0xE49C76 `link #0`; Uncertain, bead domain-pascal-njy0).
#[test]
fn with_temporaries_keep_frame_slots() {
    use crate::lower::with_slots;
    assert_eq!(with_slots(6, 0), 6);
    assert_eq!(with_slots(0, 1), 4);
    assert_eq!(with_slots(2, 1), 8);
    assert_eq!(with_slots(18, 1), 24);
    assert_eq!(with_slots(8, 2), 16);
    // The frame and the slots of the temporaries passed by address of
    // `with <w> do begin q(b + 1); a := 1 end` with a word local `k`.
    let frame = |w: &str| {
        let m = lower_src(&format!(
            "module w;
type r = record a: integer; b: integer32; end;
  rp = ^r;
         var p: rp; arr: array[1..4] of r; i: integer;
procedure q(in x: integer32); extern;
         procedure t;
var k: integer;
begin
with {w} do begin q(b + 1); a := 1 end;
end;
"
        ))
        .unwrap();
        let f = &m.functions[0];
        assert_eq!(f.with_temps.len(), 1);
        let temps: Vec<i32> = f
            .body
            .iter()
            .filter_map(|st| match &st.kind {
                StmtKind::Call(c) => Some(c),
                _ => None,
            })
            .flat_map(|c| c.args.iter())
            .filter_map(|a| match a {
                Arg::Temp { offset, .. } => Some(*offset),
                _ => None,
            })
            .collect();
        (f.frame_size, temps)
    };
    // An element's temporary: above the call temporaries' slots
    // (Uncertain: the slot is never addressed): `k` at -2, the `with`
    // slot at -8, the temporary passed by address under it at -12.
    assert_eq!(frame("arr[i]"), (12, vec![-12]));
    // A pointer's temporary: no slot, the temporary passed by address
    // right under `k` (HINT_$ADD_NET 0xE49C76 `link #0`; bead
    // domain-pascal-njy0).
    assert_eq!(frame("p^"), (8, vec![-8]));
}

/// One-byte integers (`[byte]` subranges, ref 3.15.5): widened to a word
/// by their own sign where they meet an integer operation or comparison
/// (ref 4.3.1, p. 4-5); TPAD_$DATA 0xE691D8 `clr.w D0; move.b (10,A1),D0;
/// cmpi.w #0xdf,D0`, 0xE69228..0xE6922E `move.b (12,A1),D1; ext.w D1`.
#[test]
fn byte_integers_widen_to_words() {
    let m = lower_src(
        "module m;
type ub = [byte] 0..255; sb = [byte] -128..127;
  pkt = record dev: ub; flags: ub; b0: ub; b1: ub end;
var x: integer; y: integer32; u: ub;
procedure p(in out k: pkt);
begin
  if k.dev = 16#DF then
    x := sb(k.b0) * x;
  x := lshft(k.b1, 6) + k.b0;
  u := k.flags;
  k.b1 := 200;
  if lshft(k.b1, 8) > 5 then x := 1;
end;
",
    )
    .unwrap();
    let body = &m.functions[0].body;
    let byte = |sign| Scalar {
        width: Width::Byte,
        sign,
        pointer: false,
    };
    let StmtKind::If {
        cond, then_: then, ..
    } = &body[0].kind
    else {
        panic!("{:?}", body[0].kind)
    };
    let Cond::Compare { lhs, rhs, .. } = cond else {
        panic!("{cond:?}")
    };
    assert_eq!(lhs.ty(), Scalar::signed(Width::Word));
    let Expr::Convert { value, .. } = lhs else {
        panic!("{lhs:?}")
    };
    assert_eq!(value.ty(), byte(Sign::Unsigned));
    assert_eq!(
        *rhs,
        Expr::Const {
            value: 0xdf,
            ty: Scalar::signed(Width::Word)
        }
    );
    // sb(k.b0): the byte retyped (a type transfer changes no bits,
    // p. 4-197), then sign-extended for the word product.
    let StmtKind::Store { value, .. } = &then[0].kind else {
        panic!("{:?}", then[0].kind)
    };
    let Expr::Binary { lhs, .. } = value else {
        panic!("{value:?}")
    };
    let Expr::Convert { value: b0, ty, .. } = &**lhs else {
        panic!("{lhs:?}")
    };
    assert_eq!(*ty, Scalar::signed(Width::Word));
    assert_eq!(b0.ty(), byte(Sign::Signed));
    // lshft(k.b1, 6): the byte zero-extended and shifted as a word.
    let StmtKind::Store { value, .. } = &body[1].kind else {
        panic!()
    };
    let Expr::Binary { lhs, .. } = value else {
        panic!("{value:?}")
    };
    // (An unsigned integer32 narrowed back to the word for the word sum:
    // the shift of an unsigned operand is unsigned, owner's ruling,
    // reference-clarifications "Lshft of a 16-bit operand"; bead
    // domain-pascal-4rfc.)
    assert_eq!(lhs.ty(), Scalar::unsigned(Width::Word));
    let lhs = match &**lhs {
        Expr::Convert { value, .. } if value.ty().width == Width::Word => value,
        _ => lhs,
    };
    let Expr::Binary {
        op: BinOp::Shl,
        lhs: shifted,
        ..
    } = &**lhs
    else {
        panic!("{lhs:?}")
    };
    assert_eq!(shifted.ty().width, Width::Word);
    assert!(
        matches!(&**shifted, Expr::Convert { value, .. } if value.ty() == byte(Sign::Unsigned))
    );
    // Byte to byte, and a constant into a byte: byte stores.
    for s in &body[2..4] {
        let StmtKind::Store { value, .. } = &s.kind else {
            panic!()
        };
        assert_eq!(value.ty(), byte(Sign::Unsigned));
    }
    // lshft(k.b1, 8) > 5: an unsigned integer32 (owner's ruling,
    // reference-clarifications "Lshft of a 16-bit operand"; bead
    // domain-pascal-4rfc): 200 shifted is 51200, compared unsigned, the
    // byte shifted as a word and the word widened, which the selector
    // refuses (RING_$GET_ID 0xE2FA66 widens it through a second register
    // cleared early, not implemented).
    let StmtKind::If { cond, .. } = &body[4].kind else {
        panic!("{:?}", body[4].kind)
    };
    let Cond::Compare { lhs, .. } = cond else {
        panic!("{cond:?}")
    };
    assert_eq!(lhs.ty(), Scalar::unsigned(Width::Long));
    assert!(
        matches!(lhs, Expr::Convert { value, .. }
            if matches!(&**value, Expr::Binary { op: BinOp::Shl, ty, .. }
                if *ty == Scalar::unsigned(Width::Word))),
        "{lhs:?}"
    );
}

/// Bead domain-pascal-mbtx: `addr` of an `in` parameter is lowered like
/// that of a `var` one (ref "Addr", p. 4-13; Table 5-1, p. 5-5): passed by
/// address, the address its slot holds; under `val_param`, an `in`
/// integer32 passed by value, its slot's address; a field, at its offset.
#[test]
fn addr_of_in_parameters() {
    let m = lower_src(
        "module m;
type r = record a, b: integer32; end;
var g, h, k: univ_ptr;
procedure p(in x: r);
begin g := addr(x); h := addr(x.b); end;
procedure q(in y: integer32); options(val_param);
begin k := addr(y); end;",
    )
    .unwrap();
    let slot = |o| Expr::Load {
        addr: Addr::Frame(o),
        ty: Scalar {
            pointer: true,
            ..Scalar::unsigned(Width::Long)
        },
    };
    let stored = |f: &Function, i: usize| match &f.body[i].kind {
        StmtKind::Store { value, .. } => value.clone(),
        k => panic!("{k:?}"),
    };
    assert_eq!(
        stored(&m.functions[0], 0),
        Expr::AddrOf(Addr::Indirect {
            base: Box::new(slot(8)),
            offset: 0
        })
    );
    assert_eq!(
        stored(&m.functions[0], 1),
        Expr::AddrOf(Addr::Indirect {
            base: Box::new(slot(8)),
            offset: 4
        })
    );
    assert_eq!(m.functions[0].address_params, [8]);
    assert_eq!(stored(&m.functions[1], 0), Expr::AddrOf(Addr::Frame(8)));
}

/// Elements of a packed array of booleans (ref 3.11.4.2 Table 3-5;
/// codegen-patterns 5.2, "Packed arrays of booleans"): tested and set
/// through [`Expr::BitElem`] / [`StmtKind::StoreBitElem`] with the array's
/// address, its size, the word index and the lower bound; a `for` loop of
/// constant stores is not a byte fill.
#[test]
fn packed_boolean_elements() {
    let m = lower_src(
        "module m;
         var pad: integer; bm: packed array[1..64] of boolean; g: integer;
         procedure t;
         var i: integer;
         begin
           if bm[g] then bm[g] := false;
           for i := 1 to 64 do bm[i] := true;
         end;",
    )
    .unwrap();
    let f = &m.functions[0];
    let StmtKind::If { cond, then_, .. } = &f.body[0].kind else {
        panic!("{:?}", f.body[0].kind)
    };
    let Cond::Test(Expr::BitElem {
        array,
        bytes,
        index,
        lo,
    }) = cond
    else {
        panic!("{cond:?}")
    };
    assert_eq!((array, *bytes, *lo), (&Addr::Static(2), 8, 1));
    assert_eq!(
        **index,
        Expr::Load {
            addr: Addr::Static(10),
            ty: Scalar::signed(Width::Word)
        }
    );
    assert!(matches!(
        &then_[0].kind,
        StmtKind::StoreBitElem {
            array: Addr::Static(2),
            bytes: 8,
            lo: 1,
            value: false,
            ..
        }
    ));
    let StmtKind::For(l) = &f.body[1].kind else {
        panic!("{:?}", f.body[1].kind)
    };
    assert!(matches!(
        &l.body[0].kind,
        StmtKind::StoreBitElem { value: true, .. }
    ));
}
