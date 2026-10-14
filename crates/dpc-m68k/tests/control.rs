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
        // A by-address slot used once after a branch, whose lowest free
        // register an earlier statement left a value in (12.4 R4(b),
        // `check_late_slot_loads`).
        (
            "(var x, y: integer); internal;
begin
  y := 2;
  if w = 3 then x := 0;
end;",
            "whose lowest free register an earlier statement left a value in",
        ),
        // The rule restarted at a call, while a register written before
        // the call (A2, the entry load of `s') is dirty too.
        (
            "(var x, y, s: integer); internal;
begin
  s := 0;
  ext_$p(1);
  s := 1;
  x := 1;
  y := 2;
end;",
            "whose lowest free register an earlier statement left a value in",
        ),
        // A variable in memory against a parameter held in a register:
        // which one the original loaded is not known.
        (
            "(x: integer); internal;
begin
  r := x;
  if w < x then r := 0;
end;",
            "comparing a variable in memory with one held in a register",
        ),
        // A0 after a call, A1 written before (A0 or A1 in the image).
        (
            "(var x, y, z: integer); internal;
begin
  y := 2;
  z := 3;
  ext_$p(1);
  x := 0;
end;",
            "in A0 while A1 was written before",
        ),
    ];
    for (b, want) in with_param {
        let e = error(&format!("procedure t{b}\n"));
        assert!(e.contains(want), "{e}");
    }
}

/// codegen-patterns 12.4 R4(b) after a body's first branch or call: a
/// slot used once takes the lowest free register when no earlier
/// statement left a value in it on any path (a call clobbering it counts
/// as clean): TTY_$K_SET_INPUT_FLAG 0xE67684 `movea.l (0x10,A6),A1` after
/// the call, A0 holding the descriptor; survey `tools/regalloc.py slots`.
#[test]
fn slot_loads_after_branches_and_calls() {
    let b = body(
        "procedure t(var x: integer); internal;
begin
  r := 1;
  if w = 3 then x := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "move.w\t#1,(R).l",
            "cmpi.w\t#3,(W).l",
            "bne.s\t1f",
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(%a0)",
            "1:",
        ]
    );
    let b = body(
        "procedure t(var x: integer); internal;
begin
  ext_$p(1);
  r := 1;
  x := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "move.w\t#1,(R).l",
            "movea.l\t(8,%fp),%a0",
            "clr.w\t(%a0)",
        ]
    );
    // Two statements after a call in straight-line code: the second slot
    // passes over A0, which the first left (the rule restarted at the
    // call; PROC2_$PGUID_TO_UPGID 0xE410B2/0xE410B8).
    let b = body(
        "procedure t(var x, y: integer); internal;
begin
  ext_$p(1);
  x := 1;
  y := 2;
end;
",
    );
    assert_eq!(
        b,
        [
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "movea.l\t(8,%fp),%a0",
            "move.w\t#1,(%a0)",
            "movea.l\t(0xc,%fp),%a1",
            "move.w\t#2,(%a1)",
        ]
    );
    // Two slots in one statement after a call: A0 is live at the second
    // load, so A1, which is clean.
    let b = body(
        "procedure t(var x, y: integer); internal;
begin
  ext_$p(1);
  w := y + x;
end;
",
    );
    assert_eq!(
        b,
        [
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "movea.l\t(0xc,%fp),%a0",
            "movea.l\t(8,%fp),%a1",
            "move.w\t(%a0),%d0",
            "add.w\t(%a1),%d0",
            "move.w\t%d0,(W).l",
        ]
    );
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

/// Two variables in memory: the left one loaded, the right one the
/// operand (TTY_$K_SET_FLAG 0xE67462 `move.w (0x2cc,A2),D0; cmp.w
/// (0x2ca,A2),D0; beq.b`; codegen-patterns 7.1).  A signed and an
/// unsigned word: both widened, the scheduler interleaving the two
/// widenings (TTY_$K_PUT 0xE67C42..0xE67C4A).
#[test]
fn two_variables_in_memory() {
    let b = body(
        "procedure t; internal;
begin
  if w <> r then r := 0;
  if w < u then r := 1;
end;
",
    );
    assert_eq!(
        b,
        [
            "move.w\t(W).l,%d0",
            "cmp.w\t(R).l,%d0",
            "beq.s\t1f",
            "clr.w\t(R).l",
            "1:",
            "move.w\t(W).l,%d0",
            "clr.l\t%d1",
            "ext.l\t%d0",
            "move.w\t(U).l,%d1",
            "cmp.l\t%d1,%d0",
            "bge.s\t2f",
            "move.w\t#1,(R).l",
            "2:",
        ]
    );
}

/// Records of a size whose `cmpm` form the image does not show (12..28
/// bytes; codegen-patterns 7.1, "Records") are refused, not compared
/// with a guessed loop.
#[test]
fn record_compare_of_an_uncatalogued_size_is_refused() {
    let e = compile(
        "module m;
type
  t3 = record a, b, c: integer32; end;
var
  x, y: extern t3;
  r: extern integer;
procedure t; internal;
begin
  if x = y then r := 0;
end;
",
    )
    .unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("comparing records of 12 bytes"), "{e:?}");
}

/// The pop rule through a `goto` (codegen-patterns 1.1, 7.5): a call
/// whose continuation is a `goto` is not popped when the code from the
/// label to the epilogue is clean, as before a `bra` to a clean join;
/// popped when it is not (a call there), or when a cycle of `goto`s
/// passes a call.
#[test]
fn pop_rule_through_goto() {
    let clean = body(
        "procedure t; internal;
label l;
begin
  if b then begin ext_$p(1); goto l end;
  w := 2;
l:
  r := 3;
end;",
    );
    assert_eq!(
        clean,
        [
            "tst.b\t(B).l",
            "bpl.s\t1f",
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "bra.s\t2f",
            "1:",
            "move.w\t#2,(W).l",
            "2:",
            "move.w\t#3,(R).l",
        ]
    );
    let calls = body(
        "procedure t; internal;
label l;
begin
  if b then begin ext_$p(1); goto l end;
  w := 2;
l:
  ext_$p(3);
end;",
    );
    assert!(
        calls
            .windows(2)
            .any(|p| p[0] == "jsr\t(EXT_$P).l" && p[1] == "addq.w\t#4,%sp"),
        "{calls:?}"
    );
    // A backward `goto` whose loop calls: the label's continuation is
    // not clean, so the call before the `goto` is popped.
    let cycle = body(
        "procedure t; internal;
label l;
begin
l:
  ext_$p(1);
  if b then goto l;
end;",
    );
    assert!(
        cycle
            .windows(2)
            .any(|p| p[0] == "jsr\t(EXT_$P).l" && p[1] == "addq.w\t#4,%sp"),
        "{cycle:?}"
    );
}

/// Bead domain-pascal-1dn: the parameter-use walk took time exponential
/// in the loop nesting (each loop body was walked twice), so 30 nested
/// `repeat`s never finished.  Now each nesting costs a constant factor.
#[test]
fn deeply_nested_loops_compile_quickly() {
    let n = 40;
    for (head, tail) in [
        ("repeat ", " until x = 1"),
        ("while x = 1 do begin ", " end"),
    ] {
        let src = format!(
            "module m; var x: integer; procedure p(v: integer); options(val_param); \
             begin {}x := v{}; end;",
            head.repeat(n),
            tail.repeat(n)
        );
        let t = std::time::Instant::now();
        let _ = compile(&src);
        assert!(t.elapsed().as_secs() < 10, "{head}: {:?}", t.elapsed());
    }
}
