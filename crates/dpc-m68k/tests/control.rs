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
        // An ordered test right after the entry load of its register
        // (only `beq`/`bne`/`bmi`/`bpl` on a load's flags are seen,
        // codegen-patterns 12.2).
        (
            "(x: integer); internal;
begin
  if x > 0 then r := 0;
  r := 1;
end;",
            "an ordered test of a parameter right after its entry load",
        ),
        // A function of one `if` that assigns a by-value parameter to its
        // result (bead domain-pascal-0zo8).
        (
            "(x: integer): integer; internal;
begin
  if x = 3 then t := 0 else t := x;
end;",
            "a function body of one `if`, `case` or loop statement",
        ),
        // A by-address slot used once after a branch, whose lowest free
        // register an earlier statement left a value in (12.4 R4(b),
        // `check_late_slot_loads`): past a dead slot load in A0, A1 a
        // call wrote (A0 or A1 in the image; bead domain-pascal-i24w).
        (
            "(var x, y: integer); internal;
begin
  ext_$p(1);
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
        let kind = if b.contains("): ") {
            "function"
        } else {
            "procedure"
        };
        let e = error(&format!("{kind} t{b}\n"));
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

/// A slot loaded after a branch passes over the free registers that hold
/// a dead slot load on every path to the first one never written
/// (codegen-patterns 12.4 R4(b), bead domain-pascal-i24w:
/// TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E8 `movea.l (0xc,A6),A3` past A2, the
/// status; PBU_$ADVANCE_EC_INT 0xE88432 A3 past A1).
#[test]
fn slot_loads_pass_over_dead_slot_loads() {
    let b = body(
        "procedure t(var a, c: integer; var x: integer); internal;
begin
  a := 1;
  c := 2;
  if w = 3 then x := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "pea\t(%a2)",
            "movea.l\t(8,%fp),%a0",
            "move.w\t#1,(%a0)",
            "movea.l\t(0xc,%fp),%a1",
            "move.w\t#2,(%a1)",
            "cmpi.w\t#3,(W).l",
            "bne.s\t1f",
            "movea.l\t(0x10,%fp),%a2",
            "clr.w\t(%a2)",
            "1:",
            "movea.l\t(-8,%fp),%a2",
        ]
    );
    // One slot before the branch: A1, never written (0xE58C28, 0xE5971C).
    let b = body(
        "procedure t(var x, y: integer); internal;
begin
  y := 2;
  if w = 3 then x := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "movea.l\t(0xc,%fp),%a0",
            "move.w\t#2,(%a0)",
            "cmpi.w\t#3,(W).l",
            "bne.s\t1f",
            "movea.l\t(8,%fp),%a1",
            "clr.w\t(%a1)",
            "1:",
        ]
    );
    // Past a dead slot load to a register a call wrote: A1 at
    // TTY_$K_SET_FLAG 0xE67454, A0 reused at VTOCE_$LOOKUP_FM 0xE39A9C;
    // refused (bead domain-pascal-quio).
    let e = error(
        "procedure t(var a: integer; var x: integer); internal;
begin
  ext_$p(1);
  a := 1;
  if w = 3 then x := 0;
end;
",
    );
    assert!(e.contains("12.4 R4(b)"), "{e}");
    // Every free register holds a dead slot load (round-robin-like in
    // the image, MSG_$SEND_HW 0xE59B6E..0xE59B8C; bead
    // domain-pascal-aerg): refused.
    let e = error(
        "procedure t(var a, c, d, f, g: integer; var x: integer); internal;
begin
  a := 1;
  c := 2;
  d := 3;
  f := 4;
  g := 5;
  if w = 3 then x := 0;
end;
",
    );
    assert!(e.contains("12.4 R4(b)"), "{e}");
    // A slot read from the frame at each use (its address escapes):
    // after the branch A1 holds its own dead load and A0 another slot's.
    // The image reuses the register past a dead load of the same slot (8
    // of 8: VTOCE_$TRUNCATE 0xE39E78, IO_$ACQUIRE 0xE1A6AC); not passed
    // over, refused (review of bead domain-pascal-i24w).
    let e = error(
        "procedure upd(var p: ip); options(extern);
procedure t(p: ip; var a: integer); options(val_param);
begin
  upd(p);
  a := 1;
  p^ := 1;
  if w = 3 then p^ := 2;
end;
",
    );
    assert!(e.contains("12.4 R4(b)"), "{e}");
}

/// Compiled code has no branch to a `bra` (codegen-patterns 7.1, "Branch
/// threading"): a branch to a label that a `bra` follows goes to the
/// `bra`'s target (TTY_$K_SET_FLAG 0xE6746A `beq.b 0xE67482` past the
/// `bra.b 0xE67482` at 0xE6747A), and a `bra` no longer reached is
/// dropped (TTY_$K_GET 0xE1C448).
#[test]
fn branches_to_a_bra_are_threaded() {
    let cases = [
        // The inner `then` part's `bra` to its join, then the outer `bra`.
        "begin
  if w = 1 then
    if u = 2 then r := 1 else r := 2
  else r := 3;
end;",
        // The inner test's `b<cc>` past its `then` part, then the outer `bra`
        // (TTY_$K_SET_FLAG 0xE6745A..0xE67482).
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
        // A `return` ending the inner `else` part: the outer `bra` after
        // it is no longer reached and is dropped.
        "begin
  if w = 1 then
    if u = 2 then r := 1 else begin r := 2; return end
  else r := 3;
  ext_$p(1);
end;",
        // An `else` part that starts with a `while`: the outer test goes
        // to the loop's test, the opening `bra` is dropped (TTY_$K_GET
        // 0xE1C44A `bpl.w 0xE1C4FC`, the head 0xE1C4D6).
        "begin
  if w = 1 then r := 1
  else while u <> 0 do u := u - 1;
  ext_$p(1);
end;",
    ];
    let want: [&[&str]; 5] = [
        &[
            "cmpi.w\t#1,(W).l",
            "bne.s\t1f",
            "cmpi.w\t#2,(U).l",
            "bne.s\t2f",
            "move.w\t#1,(R).l",
            "bra.s\t4f",
            "2:",
            "move.w\t#2,(R).l",
            "bra.s\t4f",
            "1:",
            "move.w\t#3,(R).l",
            "4:",
        ],
        &[
            "cmpi.w\t#1,(W).l",
            "bne.s\t1f",
            "cmpi.w\t#2,(U).l",
            "bne.s\t3f",
            "move.w\t#1,(R).l",
            "bra.s\t3f",
            "1:",
            "move.w\t#3,(R).l",
            "3:",
        ],
        &[
            "cmpi.w\t#1,(W).l",
            "bne.s\t1f",
            "cmpi.w\t#2,(U).l",
            "bne.s\t3f",
            "move.w\t#1,(R).l",
            "bra.s\t3f",
            "1:",
            "move.w\t#3,(R).l",
            "3:",
        ],
        &[
            "cmpi.w\t#1,(W).l",
            "bne.s\t1f",
            "cmpi.w\t#2,(U).l",
            "bne.s\t2f",
            "move.w\t#1,(R).l",
            "bra.s\t5f",
            "2:",
            "move.w\t#2,(R).l",
            "bra.s\t4f",
            "1:",
            "move.w\t#3,(R).l",
            "5:",
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "4:",
        ],
        // The head still aligned (the module-local 0xE4AFA8: `bra.s` at
        // 0xE4B0F4, the filler, the head 0xE4B0F8).
        &[
            "cmpi.w\t#1,(W).l",
            "bne.s\t4f",
            "move.w\t#1,(R).l",
            "bra.s\t2f",
            ".balignw\t4,0x2048",
            "3:",
            "subq.w\t#1,(U).l",
            "4:",
            "tst.w\t(U).l",
            "bne.s\t3b",
            "2:",
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
        ],
    ];
    for (b, want) in cases.iter().zip(want) {
        assert_eq!(body(&format!("procedure t; internal;\n{b}\n")), want, "{b}");
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

/// Threading on loop paths (codegen-patterns 7.1, "Branch threading"):
/// a `next` in an inner `else` part is reached by the inner test's
/// branch alone, its own `bra` and the outer one after it dropped; a
/// `while` whose body is a `while` has its outer back edge go to the
/// inner test and the inner opening `bra` dropped, one alignment kept.
#[test]
fn loop_jumps_and_nested_whiles_are_threaded() {
    let b = body(
        "procedure t; internal;
begin
  while w > 0 do begin
    w := w - 1;
    if u = 1 then begin if r = 1 then r := 2 else next end else r := 3;
    ext_$p(r);
  end;
end;
",
    );
    assert_eq!(
        b,
        [
            "bra.s\t2f",
            ".balignw\t4,0x2048",
            "1:",
            "subq.w\t#1,(W).l",
            "cmpi.w\t#1,(U).l",
            "bne.s\t4f",
            "cmpi.w\t#1,(R).l",
            "bne.s\t2f",
            "move.w\t#2,(R).l",
            "bra.s\t7f",
            "4:",
            "move.w\t#3,(R).l",
            "7:",
            "subq.l\t#2,%sp",
            "move.w\t(R).l,-(%sp)",
            "jsr\t(EXT_$P).l",
            "addq.w\t#4,%sp",
            "2:",
            "tst.w\t(W).l",
            "bgt.s\t1b",
        ]
    );
    let b = body(
        "procedure t; internal;
begin
  while r > 0 do while w > 0 do w := w - 1;
end;
",
    );
    assert_eq!(
        b,
        [
            "bra.s\t2f",
            ".balignw\t4,0x2048",
            "4:",
            "subq.w\t#1,(W).l",
            "5:",
            "tst.w\t(W).l",
            "bgt.s\t4b",
            "2:",
            "tst.w\t(R).l",
            "bgt.s\t5b",
        ]
    );
    // A `repeat` (or a backward `goto`'s label) right before a `while`:
    // its head would fall into the opening `bra`, no longer a branch
    // target; aligned or not is not seen (bead domain-pascal-tfae).
    for b in [
        "begin repeat while w > 0 do w := w - 1; r := r - 1 until r = 0 end;",
        "label l; begin l: while w > 0 do w := w - 1; if r > 0 then begin r := r - 1; goto l end end;",
    ] {
        let e = error(&format!("procedure t; internal;\n{b}\n"));
        assert!(e.contains("right before a `while`"), "{e}");
    }
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

/// Bead domain-pascal-s4a5 (codegen-patterns 12.2, "One-statement
/// bodies"): a body of one `if` loads the by-value parameter it reads at
/// entry, and a test of the parameter loaded last branches on the load's
/// flags, in a body of one statement or of several (tty_$i_set_raw_mode
/// 0xE1BF82 `move.b (0xc,A6),D0; bpl.w`; AREA_$DELETE 0xE07C34).
#[test]
fn entry_load_flags_are_tested() {
    let b = body(
        "procedure t(x: integer; y: boolean); internal;
begin
  if y then r := x;
end;
",
    );
    assert_eq!(
        b[..3],
        ["move.w\t(8,%fp),%d0", "move.b\t(0xa,%fp),%d1", "bpl.s\t1f"],
        "{b:?}"
    );
    let b = body(
        "procedure t(x: integer); internal;
begin
  if x = 0 then r := 0;
  r := 1;
end;
",
    );
    assert_eq!(b[..2], ["move.w\t(8,%fp),%d0", "bne.s\t1f"], "{b:?}");
}

/// A computed value against 0 (bead domain-pascal-qlzu; codegen-patterns
/// 7.1, "A computed value against 0"; `tools/cmpsurvey.py opflags`).
const ZERO_DECLS: &str = "var l: extern integer32;
function ext_$f(x: integer): integer; options(val_param, extern);
";

/// `=`, `<>`, `<`, `>=` branch on the flags of the add, sub, and, or that
/// formed the value (SOCK_$ALLOCATE 0xE15EFA `or.w D4,D0; beq.s`;
/// tty_$i_put_chars 0xE1B046 `subq.w #1,D1; bpl.s`; SIO6509_$INQ_LINE
/// 0xE7267C `andi.l #0x180,D1; beq.s`); `>`, `<=` test it first
/// (SMD_$SIGNAL 0xE6F25C `sub.l D0,D1; tst.l D1; bgt.s`).
#[test]
fn computed_value_against_zero_on_the_operation_flags() {
    let b = body(&format!(
        "{ZERO_DECLS}procedure t; internal;
begin
  if (w ! r) = 0 then r := 0;
  if (w - r) < 0 then r := 1;
  if (w - r) > 0 then r := 2;
  if (l & 16#180) <> 0 then r := 3;
end;
"
    ));
    assert_eq!(
        b,
        [
            "move.w\t(W).l,%d0",
            "or.w\t(R).l,%d0",
            "bne.s\t1f",
            "clr.w\t(R).l",
            "1:",
            "move.w\t(W).l,%d0",
            "sub.w\t(R).l,%d0",
            "bpl.s\t2f",
            "move.w\t#1,(R).l",
            "2:",
            "move.w\t(W).l,%d0",
            "sub.w\t(R).l,%d0",
            "tst.w\t%d0",
            "ble.s\t3f",
            "move.w\t#2,(R).l",
            "3:",
            "move.l\t#0x180,%d0",
            "and.l\t(L).l,%d0",
            "beq.s\t4f",
            "move.w\t#3,(R).l",
            "4:",
        ]
    );
}

/// `tst` where the flags are not the value's: a function's result
/// (SOCK_$PUT_INT 0xE161EA `tst.w D0`), a quotient (0xE11BF4 `divs.w
/// #60,D1; tst.w D1; beq.s`), a remainder (`swap`, NETWORK_$PAGE_SERVER
/// 0xE11822), a byte widened to a word (SOCK_$CLOSE 0xE15FB8 `clr.w D0;
/// move.b (21,A2),D0; tst.w D0; beq.s`).
#[test]
fn computed_value_against_zero_tested() {
    let b = body(&format!(
        "{ZERO_DECLS}procedure t; internal;
begin
  if ext_$f(w) = 0 then r := 0;
  if (w div 60) = 0 then r := 1;
  if (w mod 60) <> 0 then r := 2;
end;
"
    ));
    assert_eq!(
        b,
        [
            "subq.l\t#2,%sp",
            "move.w\t(W).l,-(%sp)",
            "jsr\t(EXT_$F).l",
            "addq.w\t#4,%sp",
            "tst.w\t%d0",
            "bne.s\t1f",
            "clr.w\t(R).l",
            "1:",
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "divs.w\t#0x3c,%d0",
            "tst.w\t%d0",
            "bne.s\t2f",
            "move.w\t#1,(R).l",
            "2:",
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "divs.w\t#0x3c,%d0",
            "swap\t%d0",
            "tst.w\t%d0",
            "beq.s\t3f",
            "move.w\t#2,(R).l",
            "3:",
        ]
    );
    let b = body(
        "type bt = [byte] 0..255; br = record a: integer; q: bt; end; brp = ^br;
procedure t(p: brp); val_param;
begin
  if p^.q = 0 then r := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "movea.l\t(8,%fp),%a0",
            "clr.w\t%d0",
            "move.b\t(2,%a0),%d0",
            "tst.w\t%d0",
            "bne.s\t1f",
            "clr.w\t(R).l",
            "1:",
        ]
    );
}

/// `(x & 2^k) <> 0` of a register-held `x`: `btst #k` (pacct_$compress
/// 0xE5A9DE `btst #2,D0; sne D2`; no one-bit `and` mask before a branch in
/// compiled code).
#[test]
fn one_bit_mask_is_btst() {
    let b = body(
        "procedure t(x: integer32); val_param;
begin
  if (x & 16#4000) = 0 then r := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "move.l\t(8,%fp),%d0",
            "btst\t#14,%d0",
            "bne.s\t1f",
            "clr.w\t(R).l",
            "1:",
        ]
    );
    // The mask on the left: the same `btst` (the source order cannot be
    // seen; an `and` with a one-bit mask before a branch is not).
    let b = body(
        "procedure t(x: integer32); val_param;
begin
  if (16#4000 & x) <> 0 then r := 0;
end;
",
    );
    assert_eq!(
        b,
        [
            "move.l\t(8,%fp),%d0",
            "btst\t#14,%d0",
            "beq.s\t1f",
            "clr.w\t(R).l",
            "1:",
        ]
    );
}

/// `(x & 2^k) <> 0` of a word or longword variable in memory: the variable
/// loaded whole into a data register, then `btst #k` (codegen-patterns 5.1
/// and 7.1, "One bit"; bead domain-pascal-3xwx: tty_$i_put_chars 0xE1B01E
/// `move.w (8,A0),D1; btst #5,D1; beq.s`, 0xE1B0E6 `move.l (12,A0),D1;
/// btst #3,D1`; AST 0xE013BA `move.w (190,A2),D0; btst #4,D0`; FILE_
/// 0xE5F7B8 on an indexed word).  A value an operation computed: `btst`
/// on its register (DIR_$SET_PROTECTION 0xE522D4 `lsr.w #4,D1; btst
/// #4,D1; bne`).
#[test]
fn one_bit_of_memory_is_a_whole_load() {
    let b = body(&format!(
        "{ZERO_DECLS}type st = set of 0..15; rp = ^rec; rec = record pad: integer; s: st; f: integer32; end;
procedure t(p: rp); val_param;
begin
  if (integer(p^.s) & 16#20) <> 0 then r := 0;
  if (p^.f & 8) = 0 then r := 1;
  if (16#100000 & l) <> 0 then r := 2;
  if (w & 16#4000) <> 0 then r := 3;
  if (rshft(l, 4) & 16) <> 0 then r := 5;
end;
"
    ));
    assert_eq!(
        b,
        [
            "movea.l\t(8,%fp),%a0",
            "move.w\t(2,%a0),%d0",
            "btst\t#5,%d0",
            "beq.s\t1f",
            "clr.w\t(R).l",
            "1:",
            "move.l\t(4,%a0),%d0",
            "btst\t#3,%d0",
            "bne.s\t2f",
            "move.w\t#1,(R).l",
            "2:",
            "move.l\t(L).l,%d0",
            "btst\t#20,%d0",
            "beq.s\t3f",
            "move.w\t#2,(R).l",
            "3:",
            "move.w\t(W).l,%d0",
            "btst\t#14,%d0",
            "beq.s\t4f",
            "move.w\t#3,(R).l",
            "4:",
            "move.l\t(L).l,%d0",
            "lsr.l\t#4,%d0",
            "btst\t#4,%d0",
            "beq.s\t5f",
            "move.w\t#5,(R).l",
            "5:",
        ]
    );
}

/// `odd(x)` of a word or longword variable in memory: loaded whole, then
/// `lsr.b #1` and the carry (all 9 `lsr.b #1` of compiled code:
/// tty_$i_put_chars 0xE1B0F6 `move.l (12,A0),D1; lsr.b #1,D1; bcc.s`;
/// AREA_$COPY 0xE0905A `move.w (46,A1),D0; lsr.b #1,D0; bcc`); in a
/// register, `btst #0` as before (FIM_$BUILD_DF 0xE0A6F6).
#[test]
fn odd_of_memory_is_lsr() {
    let b = body(&format!(
        "{ZERO_DECLS}procedure t(x: integer); val_param;
begin
  if odd(l) then r := 0;
  if not odd(w) then r := 1;
  if odd(x) then r := 2;
end;
"
    ));
    assert_eq!(
        b,
        [
            "move.w\t(8,%fp),%d0",
            "move.l\t(L).l,%d1",
            "lsr.b\t#1,%d1",
            "bcc.s\t1f",
            "clr.w\t(R).l",
            "1:",
            "move.w\t(W).l,%d1",
            "lsr.b\t#1,%d1",
            "bcs.s\t2f",
            "move.w\t#1,(R).l",
            "2:",
            "btst\t#0,%d0",
            "beq.s\t3f",
            "move.w\t#2,(R).l",
            "3:",
        ]
    );
}

/// What stays refused of the one-bit tests (bead domain-pascal-3xwx):
/// bit 7, 15 or 31 (no `btst #7`/`#15`/`#31` in compiled code), a byte
/// variable in memory, a variable in memory as a boolean value (`&` or
/// `odd`), a call's result, a widened value.
#[test]
fn one_bit_test_refusals() {
    for (routine, why) in [
        (
            "if (l & 16#80) <> 0 then r := 0;",
            "testing bit 7, 15 or 31 alone",
        ),
        (
            "if (w & 16#80) <> 0 then r := 0;",
            "testing bit 7, 15 or 31 alone",
        ),
        (
            "if (l & 16#8000) = 0 then r := 0;",
            "testing bit 7, 15 or 31 alone",
        ),
        (
            "if (l & 16#80000000) = 0 then r := 0;",
            "testing bit 7, 15 or 31 alone",
        ),
        (
            "if (c & 4) <> 0 then r := 0;",
            "one bit of a byte widened for the `&`",
        ),
        (
            "if (u & 4) <> 0 then r := 0;",
            "one bit of a word widened to a longword",
        ),
        ("if odd(c) then r := 0;", "`odd` of a byte value"),
        (
            "b := (w & 4) <> 0;",
            "one bit of a variable in memory as a boolean value",
        ),
        (
            "b := odd(l);",
            "`odd` of a variable in memory as a boolean value",
        ),
        (
            "if (ext_$f(1) & 4) <> 0 then r := 0;",
            "testing one bit of a function's result",
        ),
        (
            "if (w & (-32768)) <> 0 then r := 0;",
            "testing bit 7, 15 or 31 alone",
        ),
    ] {
        let e = error(&format!(
            "{ZERO_DECLS}var c: extern [byte] 0..255;\nprocedure t; internal;\nbegin\n  {routine}\nend;\n"
        ));
        assert!(e.contains(why), "{routine}: {e}");
    }
}

/// A register-held variable an assignment just formed: the same rule
/// (MAC_OS_$SEND's nested routine, module-local at 0xE0B522, at 0xE0B57C
/// `sub.w D5,D3; bne.b`, `remaining := remaining - chunk; if remaining =
/// 0`; TTY_$K_PUT 0xE67C1A `subq.w #1,D2; bpl.s`;
/// LOG_$ADD 0xE1766A `addq.w #4,D2; tst.w D2; ble.w`).
#[test]
fn assigned_register_against_zero_on_the_operation_flags() {
    let b = body(&format!(
        "{ZERO_DECLS}procedure t; internal;
var x: integer;
begin
  x := ext_$f(1);
  x := x - w;
  if x = 0 then r := 0;
  x := x - w;
  if x < 0 then r := 1;
  x := x + w;
  if x <= 0 then r := 2;
  r := x;
end;
"
    ));
    assert_eq!(
        b[4..],
        [
            "move.w\t%d0,%d1",
            "sub.w\t(W).l,%d1",
            "bne.s\t1f",
            "clr.w\t(R).l",
            "1:",
            "sub.w\t(W).l,%d1",
            "bpl.s\t2f",
            "move.w\t#1,(R).l",
            "2:",
            "add.w\t(W).l,%d1",
            "tst.w\t%d1",
            "bgt.s\t3f",
            "move.w\t#2,(R).l",
            "3:",
            "move.w\t%d1,(R).l",
        ]
    );
}

/// Refused (no site either way): any test after an eor or a `neg`, a word
/// product tested as a word; `>`, `<=` after an and, or, shift; unsigned
/// orderings; one bit of a value in memory; a word widened to a longword.
#[test]
fn computed_value_against_zero_refusals() {
    let formed = "right after the operation that formed it";
    for (routine, why) in [
        ("if xor(w, r) = 0 then r := 0;", formed),
        ("if -w = 0 then r := 0;", formed),
        // A word product tested as a word: `muls.w` sets the flags from
        // 32 bits; `tst.w` first is not seen (0xE69626 reads the 32-bit
        // flags for `abs` of a word product).
        ("if (w * r) < 0 then r := 0;", formed),
        ("if (w & r) > 0 then r := 0;", formed),
        ("if arshft(w, 2) <= 0 then r := 0;", formed),
        (
            "if (u ! 1) > 0 then r := 0;",
            "an unsigned ordering comparison with 0",
        ),
        // A word widened to a longword, `clr.l; move.w; tst.l`: not seen
        // (only a byte widened to a word is tested, 83 sites).
        (
            "if integer32(u) = 0 then r := 0;",
            "testing a register the previous instruction loaded",
        ),
    ] {
        let e = error(&format!(
            "{ZERO_DECLS}procedure t; internal;\nbegin\n  {routine}\nend;\n"
        ));
        assert!(e.contains(why), "{routine}: {e}");
    }
}

/// Loops with no test (`repeat ... until false`, codegen-patterns 7.2;
/// bead domain-pascal-pkle): the bottom `bra` back to the head; an
/// `exit` nested at the end of the body takes the inverted branch back
/// (PKT_$SAR_INTERNET 0xE72152) or, unconditional, falls out with no `bra`
/// (MSG_$SARI 0xE59F76..0xE59F9E, the last `case` arm), the dispatch's
/// default threaded to the head (0xE59F06 `bra.s 0xE59EC0`).
#[test]
fn loops_with_no_test() {
    let b = body(
        "procedure t; internal;
begin
  repeat
    ext_$p(1);
    if b then begin
      r := 5;
      if w = 3 then exit;
    end;
  until false;
  r := 1;
end;
",
    );
    assert_eq!(
        b,
        [
            ".balignw\t4,0x2048",
            "1:",
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "addq.w\t#4,%sp",
            "tst.b\t(B).l",
            "bpl.s\t1b",
            "move.w\t#5,(R).l",
            "cmpi.w\t#3,(W).l",
            "bne.s\t1b",
            "move.w\t#1,(R).l",
        ]
    );
    let b = body(
        "procedure t; internal;
begin
  repeat
    ext_$p(1);
    case w of
      1: begin r := 1; exit end;
      3: r := 3;
      5: begin r := 5; exit end;
    end;
  until false;
  r := 1;
end;
",
    );
    assert_eq!(
        b,
        [
            ".balignw\t4,0x2048",
            "1:",
            "subq.l\t#2,%sp",
            "move.w\t#1,-(%sp)",
            "jsr\t(EXT_$P).l",
            "addq.w\t#4,%sp",
            "move.w\t(W).l,%d0",
            "cmpi.w\t#1,%d0",
            "beq.s\t3f",
            "cmpi.w\t#3,%d0",
            "beq.s\t4f",
            "cmpi.w\t#5,%d0",
            "beq.s\t5f",
            "bra.s\t1b",
            "3:",
            "move.w\t#1,(R).l",
            "bra.s\t6f",
            "4:",
            "move.w\t#3,(R).l",
            "bra.s\t1b",
            "5:",
            "move.w\t#5,(R).l",
            "6:",
            "move.w\t#1,(R).l",
        ]
    );
}

/// Code after a loop with no test that no `exit` leaves is not reached
/// (dead code, eliminated at `-opt 1` and above, ref 6.4.24, p. 6-27;
/// the image's 6 such loops end their routines, codegen-patterns 7.2):
/// refused, as a statement after `return` is; an `exit` of a nested loop
/// does not leave the outer one.  A label after the loop, `goto`'s
/// target, is reached.
#[test]
fn after_a_loop_with_no_way_out() {
    let why = "a statement after a `repeat ... until false` loop that no `exit` leaves";
    for routine in [
        "repeat ext_$p(1) until false; r := 1;",
        "repeat ext_$p(1); if b then return; until false; r := 1;",
        "repeat repeat if b then exit; until false; until false; r := 1;",
        "repeat while w = 0 do begin if b then exit; w := 1 end; until false; r := 1;",
    ] {
        let e = error(&format!(
            "procedure t; internal;\nbegin\n  {routine}\nend;\n"
        ));
        assert!(e.contains(why), "{routine}: {e}");
    }
    let b = body(
        "procedure t; internal;
label 9;
begin
  repeat
    ext_$p(1);
    if b then goto 9;
  until false;
9:
  r := 1;
end;
",
    );
    assert_eq!(b.last().map(String::as_str), Some("move.w\t#1,(R).l"));
}
