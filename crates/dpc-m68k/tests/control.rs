//! Control flow and the pop rule, end to end from Pascal source
//! (codegen-patterns 1.1, 7, 12.2): what is selected, and what is
//! reported as not supported yet.

use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

/// Compile `src` to assembly lines without comments and blank lines, or
/// the rendered diagnostics.
fn compile(src: &str) -> Result<Vec<String>, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k).map_err(render)?;
    let module = dpc_ir::lower(&program).map_err(render)?;
    let asm = emit(&module, "t.pas", &Options::default()).map_err(render)?;
    Ok(asm
        .lines()
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

const DECLS: &str = "module m;
type
  pinteger = 0..65535;
  ip = ^pinteger;
var
  w: extern integer;
  u: extern pinteger;
  b: extern boolean;
  r: extern integer;
procedure ext_$p(x: integer); options(val_param, extern);
";

/// The body lines of routine `t`, after its `link`, up to its `unlk`.
fn body(routine: &str) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{routine}")).unwrap();
    let start = lines.iter().position(|l| l == "T:").unwrap() + 2;
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.starts_with("unlk"))
            .unwrap();
    lines[start..end].to_vec()
}

fn error(routine: &str) -> String {
    let e = compile(&format!("{DECLS}{routine}")).unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    e[0].clone()
}

/// LOG_$READ's worker 0xE1778C..0xE1779E: entry loads, `tst.l`, a
/// `then` part ending in `return` branches to the epilogue, an unsigned
/// `<` skips with `bcc`.
#[test]
fn return_branches_to_the_epilogue() {
    let b = body(
        "var pad: array[1..5] of integer32; ptr: univ_ptr;
procedure t(offset: pinteger; max_len: pinteger; len: ip); internal;
begin
  if ptr = nil then begin
    len^ := 0;
    return;
  end;
  len^ := 16#400;
  if max_len < 16#400 then len^ := 1;
end;
",
    );
    // `offset' is not read, so not loaded.
    assert_eq!(
        b,
        [
            "move.w\t(0xa,%fp),%d0",
            "movea.l\t(0xc,%fp),%a0",
            "tst.l\t(0x14,%a5)",
            "bne.s\t1f",
            "clr.w\t(%a0)",
            "bra.s\t2f",
            "1:",
            "move.w\t#0x400,(%a0)",
            "cmpi.w\t#0x400,%d0",
            "bcc.s\t3f",
            "move.w\t#1,(%a0)",
            "3:",
            "2:",
        ]
    );
}

#[test]
fn repeat_head_is_aligned_and_branched_back_to() {
    let b = body(
        "procedure t; internal;
begin
  r := 0;
  repeat
    r := 1;
  until (w = 3) or else b;
end;
",
    );
    assert_eq!(
        b,
        [
            "clr.w\t(R).l",
            ".balignw\t4,0x2048",
            "1:",
            "move.w\t#1,(R).l",
            "cmpi.w\t#3,(W).l",
            "beq.s\t2f",
            "tst.b\t(B).l",
            "bpl.s\t1b",
            "2:",
        ]
    );
}

/// codegen-patterns 1.1: a call is popped when a conditional branch or a
/// push follows on some path to the epilogue, not otherwise.
#[test]
fn pop_rule_across_branches() {
    let b = body(
        "procedure t; internal;
begin
  if w = 0 then begin
    ext_$p(1);
    return;
  end;
  ext_$p(2);
  if w = 1 then r := 0;
end;
",
    );
    let pops: Vec<&String> = b.iter().filter(|l| l.starts_with("addq.w")).collect();
    assert_eq!(pops, ["addq.w\t#4,%sp"], "{b:#?}");
    // The first call is followed by the `bra` to the epilogue, unpopped.
    let i = b.iter().position(|l| l == "move.w\t#1,-(%sp)").unwrap();
    assert_eq!(b[i + 1], "jsr\t(EXT_$P).l");
    assert!(b[i + 2].starts_with("bra.s\t"), "{b:#?}");
}

/// codegen-patterns 12.2 (bead domain-pascal-b1v): a by-address slot used
/// more than once is loaded at entry, in parameter order with the by-value
/// parameters (LOG_$READ's worker 0xE17780..0xE17788); a slot used once
/// (`buffer`) is not.
#[test]
fn multi_use_address_slot_is_loaded_at_entry() {
    let b = body(
        "procedure t(var buffer: integer; offset: pinteger; max_len: pinteger;
            var len: pinteger); internal;
begin
  len := 0;
  if max_len < u then len := offset;
end;
",
    );
    assert_eq!(
        b,
        [
            "move.w\t(0xc,%fp),%d0",
            "move.w\t(0xe,%fp),%d1",
            "movea.l\t(0x10,%fp),%a0",
            "clr.w\t(%a0)",
            "cmp.w\t(U).l,%d1",
            "bcc.s\t1f",
            "move.w\t%d0,(%a0)",
            "1:",
        ]
    );
}

#[test]
fn unsupported_control_flow() {
    let cases: &[(&str, &str)] = &[
        // Two parts ending in calls of the same size, a pop needed.
        (
            "begin
  if w = 0 then ext_$p(1) else ext_$p(2);
  if w = 1 then r := 0;
end;",
            "shares the pop at the join",
        ),
        (
            "begin
  r := 1;
  if w = 0 then begin r := 0; return end;
end;",
            "a `return` at the end of an `if` part that ends the routine",
        ),
        (
            "begin
  return;
  r := 1;
end;",
            "a statement after `return`",
        ),
        (
            "begin
  if 3 < w then r := 0;
  r := 1;
end;",
            "left operand is not a variable",
        ),
        (
            "begin
  if w = r then r := 0;
  r := 1;
end;",
            "the left one is not held in a data register",
        ),
        (
            "begin
  if u > 0 then r := 0;
  r := 1;
end;",
            "an unsigned ordering comparison with 0",
        ),
    ];
    for (b, want) in cases {
        let e = error(&format!("procedure t; internal;\n{b}\n"));
        assert!(e.contains(want), "{e}");
    }
    let with_param: &[(&str, &str)] = &[
        // The first test right after the entry load of its register.
        (
            "(x: integer); internal;
begin
  if x = 0 then r := 0;
  r := 1;
end;",
            "testing a register the previous instruction loaded",
        ),
        // A body of one `if` that reads a by-value parameter.
        (
            "(x: integer); internal;
begin
  if x = 3 then r := 0;
end;",
            "a body of one `if`, `case` or loop statement",
        ),
        // A by-address slot used once in a body of several statements.
        (
            "(var x: integer); internal;
begin
  r := 1;
  if w = 3 then x := 0;
end;",
            "used once (loaded where needed",
        ),
        // The same in a body with a call (12.4 R4(b) is known only for
        // bodies without branches and calls).
        (
            "(var x: integer); internal;
begin
  x := 0;
  ext_$p(1);
end;",
            "with branches or calls, a pointer parameter or by-address parameter that is used once",
        ),
    ];
    for (b, want) in with_param {
        let e = error(&format!("procedure t{b}\n"));
        assert!(e.contains(want), "{e}");
    }
}

/// Compiled code has no branch to a `bra` (codegen-patterns 7.1), so the
/// shapes that would make one are rejected rather than guessed at.
#[test]
fn branches_to_a_bra_are_rejected() {
    let cases = [
        // The inner `then` part's `bra` to its join, then the outer `bra`.
        "begin
  if w = 1 then
    if u = 2 then r := 1 else r := 2
  else r := 3;
end;",
        // The inner test's `b<cc>` past its `then` part, then the outer `bra`.
        "begin
  if w = 1 then begin
    if u = 2 then r := 1;
  end else r := 3;
end;",
        // The inner test's `b<cc>`, then the `bra` of the `return`.
        "begin
  if w = 1 then begin
    if u = 2 then r := 1;
    return;
  end;
  r := 3;
end;",
    ];
    for b in cases {
        let e = error(&format!("procedure t; internal;\n{b}\n"));
        assert!(e.contains("a branch to a `bra`"), "{e}");
    }
    // The same nesting without an `else` (or a `return`) after the inner
    // `if` has no such branch: the joins coincide (0xE177B8..0xE177CC).
    let b = body(
        "procedure t; internal;
begin
  if w = 1 then
    if u = 2 then r := 1 else r := 2;
end;
",
    );
    assert_eq!(
        b,
        [
            "cmpi.w\t#1,(W).l",
            "bne.s\t1f",
            "cmpi.w\t#2,(U).l",
            "bne.s\t2f",
            "move.w\t#1,(R).l",
            "bra.s\t3f",
            "2:",
            "move.w\t#2,(R).l",
            "3:",
            "1:",
        ]
    );
}

/// codegen-patterns 9: `if c then return` is one branch on `c` to the
/// epilogue (TERM_$CONTROL 0xE66B3A `tst.l (A3); bne.w 0xE66D78`, the
/// register restore); with `or else`, each operand branches there
/// (compiled code has no conditional branch over a `bra`).  A call before
/// it is popped: a conditional branch follows (codegen-patterns 1.1).
#[test]
fn if_then_return_branches_to_the_epilogue() {
    let b = body(
        "procedure t; internal;
begin
  ext_$p(1);
  if w <> 0 then return;
  if b or else (w = 2) then return else r := 2;
  r := 3;
end;
",
    );
    assert_eq!(
        b,
        [
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "addq.w\t#4,%sp",
            "tst.w\t(W).l",
            "bne.s\t1f",
            "tst.b\t(B).l",
            "bmi.s\t1f",
            "cmpi.w\t#2,(W).l",
            "beq.s\t1f",
            "move.w\t#2,(R).l",
            "move.w\t#3,(R).l",
            "1:",
        ]
    );
    // At the end of the routine it would branch to the next instruction
    // (or be left out): not known.
    let e = error(
        "procedure t; internal;
begin
  r := 1;
  if w <> 0 then return;
end;
",
    );
    assert!(e.contains("a `return` at the end of an `if` part"), "{e}");
}

/// A stepped pointer (codegen-patterns 7.4) formed once from a pointer
/// local is right only while nothing assigns the local in the body: a
/// nested routine called in the body may (ref 2.4), so a local it reaches
/// is refused there; without the call the loop compiles.
#[test]
fn stepped_pointer_local_a_nested_routine_assigns() {
    let src = |call: &str| {
        format!(
            "type ls = array[1..8] of integer32; lp = ^ls;
var other: lp;
procedure t(n: integer32); options(val_param);
var i: integer; la: lp;
  procedure inner;
  begin
    la := other;
  end;
begin
  la := other;
  inner;
  for i := 1 to 4 do begin
    la^[i] := n;
    {call}
  end;
end;
"
        )
    };
    let e = error(&src("inner;"));
    assert!(
        e.contains("whose pointer is not a variable the body leaves unchanged"),
        "{e}"
    );
    assert!(body(&src("")).iter().any(|l| l == "lea\t(4,%a0),%a1"));
}

/// The same with the pointer a field of a record the body assigns as a
/// whole (a block copy over it).
#[test]
fn stepped_pointer_in_a_record_the_body_copies() {
    let e = error(
        "type ls = array[1..8] of integer32; lp = ^ls;
  hr = record a, b: integer32; p: lp; end;
var h, h2: hr;
procedure t;
var i: integer;
begin
  h.a := 0;
  for i := 1 to 4 do begin
    h.p^[i] := 0;
    h := h2;
  end;
end;
",
    );
    assert!(
        e.contains("whose pointer is not a variable the body leaves unchanged"),
        "{e}"
    );
}
