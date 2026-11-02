//! Constants substituted for reaching definitions (`dpc
//! --substitute-constants`, `dpc_ir::constprop`; ref 6.4.24 pp.
//! 6-27..6-28; codegen-patterns 7.1, "Constants for reaching
//! definitions"; bead domain-pascal-pvy8).  SAU3 PBU_$SETUP 0xE594E0 is
//! the model: a local's only reaching definition a constant, its tests
//! folded, the parts they skip gone, the stores no read is left of gone.

use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

fn compile(src: &str, substitute: bool) -> Vec<String> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k::default())
        .map_err(render)
        .unwrap();
    let mut module = dpc_ir::lower(&program).map_err(render).unwrap();
    if substitute {
        let abnormal = program
            .routines
            .iter()
            .filter(|r| r.options.abnormal)
            .map(|r| r.symbol.clone())
            .collect();
        for f in &mut module.functions {
            dpc_ir::constprop::substitute_constants(f, &abnormal);
        }
    }
    let asm = emit(&module, "t.pas", &Options::default())
        .map_err(render)
        .unwrap();
    asm.lines()
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('.'))
        .collect()
}

const DECLS: &str = "module m;
type
  info_t = packed record bus: 0..7; rest: 0..31 end;
var
  n: extern integer;
  c: extern integer32;
  info: extern info_t;
  ok: extern boolean;
procedure work(i: integer); options(val_param, extern);
procedure byref(var i: integer); extern;
";

/// The body of `procedure t`, from after `link` to before `unlk`.
fn body(routine: &str, substitute: bool) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{routine}"), substitute);
    let start = lines.iter().position(|l| l == "T:").unwrap() + 2;
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.starts_with("unlk"))
            .unwrap();
    lines[start..end].to_vec()
}

fn has(b: &[String], s: &str) -> bool {
    b.iter().any(|l| l == s)
}

/// A test of a constant local folds; the part it skips goes, the part
/// it runs stays without a test (PBU_$SETUP 0xE595AA..0xE595D2).  The
/// local's store goes: nothing reads it any more.
#[test]
fn tests_of_a_constant_fold() {
    let src = "define t;
procedure t;
var bus: integer;
begin
  bus := 2;
  if bus = 1 then n := 5;
  if bus = 2 then n := 6;
end;";
    let off = body(src, false);
    assert!(has(&off, "cmpi.w\t#1,%d0") || off.iter().any(|l| l.starts_with("cmpi.w")));
    let on = body(src, true);
    assert_eq!(on, vec!["move.w\t#6,(N).l"], "{on:#?}");
}

/// `for` bounds of constant locals: the count and the first value as
/// constants (PBU_$SETUP 0xE595D8 `move.w #7`, 0xE595E0 `clr.w D4`),
/// `first` and `last` stored nowhere.
#[test]
fn for_bounds_of_constant_locals() {
    let src = "define t;
procedure t;
var first, last, i: integer;
begin
  first := 0;
  last := 7;
  for i := first to last do work(i);
end;";
    let on = body(src, true);
    assert!(on.iter().any(|l| l.starts_with("moveq\t#7,%d")), "{on:#?}");
    assert!(on.iter().any(|l| l.starts_with("clr.w\t%d")), "{on:#?}");
    assert!(
        on.iter()
            .all(|l| !l.contains("(-2,%fp)") && !l.contains("(-4,%fp)")),
        "{on:#?}"
    );
}

/// A packed field store's value is not substituted: it reads the local
/// (PBU_$SETUP 0xE597D4), and the local's store stays (0xE5957C).  A
/// plain store's is (0xE596E4).
#[test]
fn packed_field_store_reads_the_local() {
    let src = "define t;
procedure t;
var bus: 0..7;
begin
  bus := 2;
  n := bus;
  info.bus := bus;
end;";
    let on = body(src, true);
    assert!(has(&on, "move.w\t#2,(N).l"), "{on:#?}");
    assert!(on.iter().any(|l| l.starts_with("moveq\t#2,")), "{on:#?}");
    assert!(on.iter().any(|l| l.starts_with("or.b")), "{on:#?}");
}

/// Two different constants reaching a use: no substitution.
#[test]
fn different_definitions_at_a_join() {
    let src = "define t;
procedure t;
var k: integer;
begin
  if ok then k := 1 else k := 2;
  n := k;
end;";
    assert_eq!(body(src, true), body(src, false));
}

/// A local changed in a loop is not a constant at the loop's head.
#[test]
fn local_changed_in_a_loop() {
    let src = "define t;
procedure t;
var k: integer;
begin
  k := 1;
  while k < 10 do begin work(k); k := k + 1 end;
end;";
    assert_eq!(body(src, true), body(src, false));
}

/// A local passed by address is not substituted.
#[test]
fn local_passed_by_address() {
    let src = "define t;
procedure t;
var k: integer;
begin
  k := 1;
  byref(k);
  n := k;
end;";
    assert_eq!(body(src, true), body(src, false));
}

/// A `goto` carries its state to its label: a constant on every path
/// in survives the label, another value does not.
#[test]
fn goto_joins_its_label() {
    let src = "define t;
procedure t;
label 1;
var k: integer;
begin
  k := 3;
  if ok then goto 1;
  work(0);
1:
  n := k;
end;";
    let on = body(src, true);
    assert!(has(&on, "move.w\t#3,(N).l"), "{on:#?}");
    let src2 = "define t;
procedure t;
label 1;
var k: integer;
begin
  k := 3;
  if ok then begin k := 4; goto 1 end;
  work(0);
1:
  n := k;
end;";
    let on2 = body(src2, true);
    assert!(
        !has(&on2, "move.w\t#3,(N).l") && !has(&on2, "move.w\t#4,(N).l"),
        "{on2:#?}"
    );
}

/// `case` on a constant local keeps its table (PBU_$SETUP 0xE596F0
/// `moveq #2,D0; subq.w #1,D0; cmpi.w #4,D0`).
#[test]
fn case_keeps_its_table() {
    let src = "define t;
procedure t;
var k: integer;
begin
  k := 2;
  case k of
    1: n := 1;
    2: n := 2;
    3: n := 3;
    4: n := 4;
  end;
end;";
    let on = body(src, true);
    assert!(on.iter().any(|l| l.starts_with("moveq\t#2,")), "{on:#?}");
    assert!(on.iter().any(|l| l.starts_with("jmp")), "{on:#?}");
}

/// A routine that calls an `abnormal` routine is left alone (ref 5.5.6,
/// p. 5-19; SAU2 PKT_$COPY_TO_PA 0xE1260C reads `buf_count` from its slot
/// after FIM_$CLEANUP's second return).
#[test]
fn abnormal_call_leaves_the_routine_alone() {
    let src = "function cleanup(out r: integer32): integer32; options(abnormal, extern);
define t;
procedure t;
var k: integer; r: integer32;
begin
  k := 0;
  if cleanup(r) = 0 then n := 1 else n := k;
end;";
    assert_eq!(body(src, true), body(src, false));
}

/// A constant substituted on the left of a compare goes to the right:
/// `k := 0; if k < n` is `n > 0`.
#[test]
fn constant_goes_to_the_right() {
    let src = "define t;
procedure t;
var k: integer;
begin
  k := 0;
  if k < n then work(1);
end;";
    let on = body(src, true);
    assert!(on.iter().any(|l| l == "tst.w\t(N).l"), "{on:#?}");
    assert!(on.iter().any(|l| l.starts_with("ble")), "{on:#?}");
}
