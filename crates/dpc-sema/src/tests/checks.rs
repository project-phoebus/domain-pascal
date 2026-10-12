//! Name resolution, type checking, storage and diagnostics.

use super::*;
use crate::consts::ConstValue;
use crate::program::{ExprKind, InitStore, StmtKind, Storage};

// ---------------------------------------------------------------------
// Diagnostics carry positions
// ---------------------------------------------------------------------

#[test]
fn undefined_identifier_has_position() {
    let e = errs("module m;\nprocedure p;\nbegin\n  x := 1;\nend;\n");
    assert_eq!(e, vec!["t.pas:4:3: error: undefined identifier `x`"]);
}

#[test]
fn undefined_type_has_position() {
    let e = errs("module m;\nvar v: nosuch;\n");
    assert_eq!(e, vec!["t.pas:2:8: error: undefined type `nosuch`"]);
}

#[test]
fn several_errors_reported() {
    let e = errs("module m;\nprocedure p;\nbegin\n  a := 1;\n  b := 2;\nend;\n");
    assert_eq!(e.len(), 2, "{e:#?}");
}

#[test]
fn type_mismatch_in_assignment() {
    assert_err(
        "module m; var b: boolean; procedure p; begin b := 1 end;",
        "not compatible with the assignment variable",
    );
    assert_err(
        "module m; var i: integer; procedure p; begin i := 'x' end;",
        "not compatible",
    );
}

#[test]
fn constant_out_of_range() {
    assert_err(
        "module m; type t = 0..9; var v: t; procedure p; begin v := 10 end;",
        "out of range",
    );
    assert_err(
        "module m; var v: integer; procedure p; begin v := 40000 end;",
        "out of range",
    );
}

#[test]
fn duplicate_declaration() {
    assert_err("module m; var a: integer; a: char;", "already declared");
}

// ---------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------

#[test]
fn constant_folding() {
    let p = module(
        "const a = 16#10; b = a * 2 + 1; c = -b; d = 8#7 ! 16#30;
               big = 70000; neg = 16#FFFFFFFF; ch = chr(65); s = 'abc';
         var v1: integer := b; v2: integer := c; v3: integer := d;
             v4: integer32 := big; v5: integer32 := neg; v6: char := ch;
             v7: array[1..4] of char := s;",
    );
    let init = |n: &str| var(&p, n).init.clone().unwrap();
    let v = |n: &str| init(n)[0].value.clone();
    assert_eq!(v("v1"), ConstValue::Int(33));
    assert_eq!(v("v2"), ConstValue::Int(-33));
    assert_eq!(v("v3"), ConstValue::Int(0x37));
    assert_eq!(v("v4"), ConstValue::Int(70000));
    // ref 3.4 NOTE p. 3-10: "the compiler treats them as negative numbers
    // (for example, 16#FFFFFFFF)".
    assert_eq!(v("v5"), ConstValue::Int(-1));
    assert_eq!(v("v6"), ConstValue::Char(b'A'));
    assert_eq!(v("v7"), ConstValue::Str(b"abc".to_vec()));
    // maxint (ref 3.2.3, p. 3-5).
    let p = module("const m = maxint; var x: integer := m;");
    assert_eq!(
        var(&p, "x").init.as_ref().unwrap()[0].value,
        ConstValue::Int(32767)
    );
}

#[test]
fn enumeration_constants() {
    let p = module(
        "type pets = (cats, dogs, dolphins);
         var x: pets := dolphins;
             s: set of pets := [cats, dolphins];",
    );
    let pets = ty(&p, "pets");
    assert_eq!(
        var(&p, "x").init.as_ref().unwrap()[0].value,
        ConstValue::Enum { ty: pets, ord: 2 }
    );
    assert_eq!(
        var(&p, "s").init.as_ref().unwrap()[0].value,
        ConstValue::Set(vec![0, 2])
    );
}

#[test]
fn initialisers_flatten_to_stores() {
    let p = module(
        "type r = record a: char; b: integer32 end;
         var arr: array[1..4] of integer := [1, 2 of 7, 4 := 9];
             rec: r := [b := 5, a := 'x'];
             flags: set of 0..15 := [0, 3..5];",
    );
    let i16 = p.types.builtins.integer16;
    assert_eq!(
        var(&p, "arr").init.as_ref().unwrap(),
        &vec![
            InitStore {
                offset: 0,
                ty: i16,
                value: ConstValue::Int(1)
            },
            InitStore {
                offset: 2,
                ty: i16,
                value: ConstValue::Int(7)
            },
            InitStore {
                offset: 4,
                ty: i16,
                value: ConstValue::Int(7)
            },
            InitStore {
                offset: 6,
                ty: i16,
                value: ConstValue::Int(9)
            },
        ]
    );
    let rec = var(&p, "rec").init.clone().unwrap();
    assert_eq!(
        (rec[0].offset, rec[0].value.clone()),
        (2, ConstValue::Int(5))
    );
    assert_eq!(
        (rec[1].offset, rec[1].value.clone()),
        (0, ConstValue::Char(b'x'))
    );
    assert_eq!(
        var(&p, "flags").init.as_ref().unwrap()[0].value,
        ConstValue::Set(vec![0, 3, 4, 5])
    );
}

#[test]
fn routine_local_initialiser_needs_static() {
    // ref 3.2.2 p. 3-4.
    assert_err(
        "module m; procedure p; var x: integer := 0; begin end;",
        "only if it is static",
    );
    let p = module("procedure p; var x: static integer := 0; begin end;");
    assert!(matches!(
        var(&p, "x").storage,
        Storage::Static { global: false, .. }
    ));
}

// ---------------------------------------------------------------------
// Storage (ref 7.2)
// ---------------------------------------------------------------------

#[test]
fn unit_variable_storage() {
    let p = module(
        "var a: integer;
             b: static integer32;
             c: define char;
             d: extern ^integer;
         var (my_sec) e: integer; f: integer32;",
    );
    let st = |n: &str| var(&p, n).storage.clone();
    let stat = |sym: &str, global: bool, section: &str, offset: u32| Storage::Static {
        symbol: sym.into(),
        global,
        section: section.into(),
        offset,
    };
    assert_eq!(st("a"), stat("A", false, ".data", 0));
    assert_eq!(st("b"), stat("B", false, ".data", 4));
    assert_eq!(st("c"), stat("C", true, ".data", 8));
    assert_eq!(st("d"), Storage::Extern { symbol: "D".into() });
    assert_eq!(st("e"), stat("E", false, "my_sec", 0));
    assert_eq!(st("f"), stat("F", false, "my_sec", 4));
    assert_eq!(
        p.data_sections,
        vec![(".data".to_string(), 9), ("my_sec".to_string(), 8)]
    );
}

#[test]
fn define_part_methods() {
    // Method 2 (ref 7.2.2 p. 7-4): extern, then define with a value.
    let p = module("var x: extern integer; define x := 8;");
    assert!(matches!(
        var(&p, "x").storage,
        Storage::Static { global: true, .. }
    ));
    assert_eq!(
        var(&p, "x").init.as_ref().unwrap()[0].value,
        ConstValue::Int(8)
    );
    // Method 3 (ref 7.2.3 p. 7-6): define first.
    let p = module("define x; var x: extern integer := 8;");
    assert!(matches!(
        var(&p, "x").storage,
        Storage::Static { global: true, .. }
    ));
}

#[test]
fn module_heading_and_section_attribute() {
    let p = ok("module kitchen, wired_proc, wired_data;
                var v: integer;
                procedure a; begin end;
                [section(other_proc)] procedure b; begin end;
                [section(, other_data)] procedure c; var s: static integer; begin end;");
    assert_eq!(
        (p.code_section.as_str(), p.data_section.as_str()),
        ("wired_proc", "wired_data")
    );
    let sec = |n: &str| {
        let r = routine(&p, n);
        (r.code_section.clone(), r.data_section.clone())
    };
    assert_eq!(sec("a"), ("wired_proc".into(), "wired_data".into()));
    assert_eq!(sec("b"), ("other_proc".into(), "wired_data".into()));
    assert_eq!(sec("c"), ("wired_proc".into(), "other_data".into()));
    assert!(
        matches!(&var(&p, "s").storage, Storage::Static { section, .. } if section == "other_data")
    );
    assert!(
        matches!(&var(&p, "v").storage, Storage::Static { section, .. } if section == "wired_data")
    );
}

#[test]
fn linkage_and_symbols() {
    let p = module(
        "procedure ast_$page_zero; begin end;
         procedure worker; internal; begin end;
         procedure ml_$lock(lock: integer); val_param; extern;",
    );
    let r = routine(&p, "ast_$page_zero");
    assert_eq!(r.symbol, "AST_$PAGE_ZERO");
    assert!(r.global && !r.external);
    assert!(!routine(&p, "worker").global);
    let l = routine(&p, "ml_$lock");
    assert!(l.external && l.body.is_none());
}

#[test]
fn forward_and_body() {
    let p = module(
        "function f(x: integer): integer; forward;
         procedure g; var y: integer; begin y := f(1) end;
         function f; begin f := x end;",
    );
    let f = routine(&p, "f");
    assert!(f.body.is_some());
    assert_eq!(f.params.len(), 1);
    assert_eq!(p.routines.iter().filter(|r| r.name == "f").count(), 1);
    assert_err(
        "module m; procedure f; forward;",
        "declared forward but never defined",
    );
}

// ---------------------------------------------------------------------
// Expressions and statements
// ---------------------------------------------------------------------

#[test]
fn integer_widths() {
    let p = module(
        "var a: integer; b: integer32; c: boolean;
         procedure p; begin a := a + 1; b := a * b; c := a < b end;",
    );
    let r = routine(&p, "p");
    let tys: Vec<_> = r
        .body
        .as_ref()
        .unwrap()
        .stmts
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Assign { value, .. } => p.types.display(value.ty),
            k => panic!("{k:?}"),
        })
        .collect();
    assert_eq!(tys, vec!["integer16", "integer32", "boolean"]);
}

#[test]
fn function_result_assignment_and_uplevel() {
    let p = module(
        "function f(x: integer): integer;
           var l: integer;
           procedure inner;
           begin f := l; l := x end;
         begin inner; f := 0 end;",
    );
    let inner = routine(&p, "inner");
    let stmts = &inner.body.as_ref().unwrap().stmts;
    match &stmts[0].kind {
        StmtKind::Assign { target, value } => {
            assert!(matches!(target.kind, ExprKind::Result { uplevel: 1, .. }));
            assert!(matches!(value.kind, ExprKind::Var { uplevel: 1, .. }));
        }
        k => panic!("{k:?}"),
    }
    assert_err(
        "module m; function f: integer; begin end; procedure p; begin f := 1 end;",
        "cannot assign to `f`",
    );
}

#[test]
fn assignment_to_in_parameter() {
    // ref 5.3.2 p. 5-7.
    assert_err(
        "module m; procedure p(in x: integer); begin x := 1 end;",
        "Assignment to IN argument",
    );
}

#[test]
fn var_parameter_needs_variable_of_same_type() {
    assert_err(
        "module m; procedure q(var x: integer); extern; procedure p; begin q(1) end;",
        "a variable is needed",
    );
    assert_err(
        "module m; var c: char; procedure q(var x: integer); extern; procedure p; begin q(c) end;",
        "does not match",
    );
    // univ turns the check off (ref 5.3.3 p. 5-10).
    module("var c: char; procedure q(var x: univ integer); extern; procedure p; begin q(c) end;");
}

#[test]
fn constants_passed_by_address_need_temps() {
    let p = module(
        "procedure ml_$lock(lock: integer); val_param; extern;
         procedure q(x: integer); extern;
         procedure p; begin ml_$lock(16#14); q(5) end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let arg = |i: usize| match &stmts[i].kind {
        StmtKind::Call(c) => c.args[0].clone(),
        k => panic!("{k:?}"),
    };
    assert!(!arg(0).needs_temp);
    assert!(arg(1).needs_temp);
}

#[test]
fn argument_count() {
    assert_err(
        "module m; procedure q(a, b: integer); extern; procedure p; begin q(1) end;",
        "wrong number of arguments",
    );
    // `variable` routines may get fewer (ref 5.5.5 p. 5-17).
    module("procedure q(a, b: integer); options(extern, variable); procedure p; begin q(1) end;");
}

#[test]
fn sets_and_in() {
    let p = module(
        "type s = set of 0..31;
         var v: s; i: integer; b: boolean;
         procedure p; begin
           v := v + [i]; v := [1, 3..5]; b := i in v; b := 3 in [1..5];
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    match &stmts[3].kind {
        StmtKind::Assign { value, .. } => assert_eq!(value.value, Some(ConstValue::Bool(true))),
        k => panic!("{k:?}"),
    }
    assert_err(
        "module m; type e = (x, y); var v: set of 0..7; b: boolean; procedure p; begin b := x in v end;",
        "does not match the set",
    );
}

#[test]
fn with_statement_binds_fields() {
    let p = module(
        "type r = record a: integer; b: char end;
         var v: r; w: array[1..2] of r;
         procedure p; begin
           with v do a := 1;
           with w[2] : q do q.b := 'x';
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    match &stmts[0].kind {
        StmtKind::With { items, body } => {
            assert_eq!(items.len(), 1);
            match &body.kind {
                StmtKind::Assign { target, .. } => match &target.kind {
                    ExprKind::Field { base, offset, .. } => {
                        assert_eq!(*offset, 0);
                        assert!(matches!(base.kind, ExprKind::WithRecord(id) if id == items[0].0));
                    }
                    k => panic!("{k:?}"),
                },
                k => panic!("{k:?}"),
            }
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn case_statement() {
    let p = module(
        "type k = (ka, kb, kc);
         var x: k; i: integer;
         procedure p; begin
           case x of ka, kb: i := 1; kc: i := 2; otherwise i := 3; end;
         end;",
    );
    match &routine(&p, "p").body.as_ref().unwrap().stmts[0].kind {
        StmtKind::Case {
            arms, otherwise, ..
        } => {
            assert_eq!(arms[0].labels, vec![0, 1]);
            assert_eq!(arms[1].labels, vec![2]);
            assert!(otherwise.is_some());
        }
        k => panic!("{k:?}"),
    }
    assert_err(
        "module m; var i: integer; procedure p; begin case i of 1: ; 1: ; end end;",
        "duplicate case label",
    );
}

#[test]
fn loops_exit_next() {
    module(
        "var i: integer;
         procedure p; begin
           for i := 1 to 10 do begin if i = 3 then next; if i = 5 then exit end;
           while i > 0 do i := i - 1;
           repeat i := i + 1 until i >= 4;
         end;",
    );
    assert_err("module m; procedure p; begin exit end;", "outside a loop");
    assert_err(
        "module m; var c: char; procedure p; begin for c := 1 to 2 do ; end;",
        "not compatible with the index variable",
    );
}

#[test]
fn goto_and_labels() {
    module("procedure p; label 10, done; begin goto done; 10: ; done: end;");
    assert_err("module m; procedure p; begin goto 9 end;", "not declared");
}

#[test]
fn pointers_nil_univ_ptr() {
    module(
        "type pi = ^integer; pc = ^char;
         var a, b: pi; c: pc; u: univ_ptr; x: integer;
         procedure p; begin
           a := nil; a := b; u := a; a := u; x := a^;
           if a = nil then a := b;
         end;",
    );
    assert_err(
        "module m; type pi = ^integer; pc = ^char; var a: pi; c: pc; procedure p; begin a := c end;",
        "not compatible",
    );
    assert_err(
        "module m; var u: univ_ptr; x: integer; procedure p; begin x := u^ end;",
        "univ_ptr cannot be dereferenced",
    );
}

#[test]
fn forward_pointer_types() {
    let p = module("type a = ^b; b = record next: a; v: integer end;");
    match p.types.kind(ty(&p, "a")) {
        crate::types::TypeKind::Pointer { target } => assert_eq!(*target, ty(&p, "b")),
        k => panic!("{k:?}"),
    }
    assert_err("module m; type a = ^nosuch;", "undefined type `nosuch`");
}

#[test]
fn char_arrays_and_strings() {
    // ref "Array Operations", p. 4-26.
    module(
        "var greeting: array[1..12] of char; a, b: array[1..20] of char;
         procedure p; begin greeting := 'aloha'; a := b end;",
    );
    assert_err(
        "module m; var f: array[1..12] of char; procedure p; begin f := 'auf wiedersehen' end;",
        "not compatible",
    );
}

#[test]
fn type_transfer() {
    // ref p. 4-197.
    let p = module(
        "type pi = ^integer;
         var a: pi; l: integer32; i: integer;
         procedure p; begin l := integer32(a); i := integer(l) end;",
    );
    let _ = p;
    assert_err(
        "module m; var c: char; l: integer32; procedure p; begin l := integer32(c) end;",
        "equal sizes",
    );
}

#[test]
fn builtins() {
    let p = module(
        "var i: integer; l: integer32; c: char; b: boolean;
         procedure p; begin
           disable;
           i := lshft(i, 1) ! 1; l := rshft(l, 3); i := ord(c); c := chr(i);
           b := odd(i); i := xor(i, 5); i := abs(i);
           enable;
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    assert!(matches!(
        stmts[0].kind,
        StmtKind::Builtin {
            builtin: crate::program::Builtin::Disable,
            ..
        }
    ));
}

#[test]
fn parameter_types_must_be_named() {
    // ref 5.1 p. 5-2: "Procedure range(small_range : 0..10); {WRONG!}".
    assert_err(
        "module m; procedure range(small_range: 0..10); extern;",
        "must be a type name",
    );
}

#[test]
fn kitchen_sink_module() {
    // The parser's whole-module test, with status_$t declared before use
    // (a parameter type must already be declared).
    let p = ok("{ A module touching most constructs. }
module kitchen_sink, wired_proc, wired_data;

const
  max_q = 16#20;
  msg = 'done'(13, 10);

type
  q_kind_t = (k_a, k_b);
  q_elem_t = record
    next: ^q_elem_t;
    key: integer32;
    case kind: q_kind_t of
      k_a: (a: integer);
      k_b: (b: packed array[0..3] of char);
  end;
  status_$t = integer32;
  handler_t = ^procedure (in elem: q_elem_t; out status: status_$t);

routine_option
  default_routine_options = val_param;

var (wired_data)
  queue: array[1..max_q] of q_elem_t;
  count: static integer := 0;
  flags: set of 0..15 := [0, 3..5];
  hook: extern handler_t;

define
  count;

procedure helper(var e: q_elem_t); internal;
var i: integer;
begin
  for i := 1 to max_q do begin
    if queue[i].key = 0 then next;
    with queue[i] : q do
      case q.kind of
        k_a: q.a := lshft(q.a, 1) ! 1;
        k_b: q.b[0] := 'x';
      end;
    if i >= max_q - 1 then exit;
  end;
end;

[section(wired_proc, wired_data)]
function q_$count(in which: integer): integer32;
  function inner: integer32;
  begin
    inner := count * 2;
  end;
begin
  disable;
  q_$count := inner;
  enable;
  if (which < 0) or else not (which in [1..max_q]) then
    return;
  repeat
    count := count - 1
  until count <= 0;
end;
");
    // default_routine_options = val_param applies to q_$count.
    let q = routine(&p, "q_$count");
    assert!(q.options.val_param);
    assert_eq!(q.params[0].conv.passing, crate::conv::Passing::ByValue);
    assert_eq!(q.nested.len(), 1);
    assert!(matches!(
        var(&p, "count").storage,
        Storage::Static { global: true, .. }
    ));
    assert_eq!(size(&p, "q_elem_t"), 14);
    assert_eq!(p.types.size(var(&p, "queue").ty), 14 * 32);
}

/// Ref 3.15.1-3.15.3 (pp. 3-56..3-57): `volatile`, `atomic` and `device`
/// change the code (no `clr` on a device register: WIN_ 0xE19112,
/// codegen-patterns 13), so they are rejected rather than dropped.
#[test]
fn optimisation_attributes_are_rejected() {
    for a in ["volatile", "atomic", "device"] {
        let e = errs(&format!(
            "module m;\nvar x: extern [{a}] integer;\nprocedure p;\nbegin\n  x := 0;\nend;\n"
        ));
        assert_eq!(
            e,
            vec![format!(
                "t.pas:2:16: error: the `{a}` attribute (ref 3.15) is not supported yet"
            )],
        );
    }
}
