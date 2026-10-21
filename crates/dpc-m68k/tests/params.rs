//! Value parameters under the default convention (ref 5.2 Table 5-1,
//! p. 5-5): passed by address and copied by the callee on entry
//! (codegen-patterns 1.2, "Value parameters passed by address"; bead
//! domain-pascal-5aw.12), end to end from Pascal source.

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
        .map(|l| {
            let code = l.split('|').next().unwrap();
            code.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .filter(|l| !l.is_empty())
        .collect())
}

const DECLS: &str = "module m;
type
  uid_$t = record high, low: integer32; end;
  str3 = array[1..3] of char;
  opts_t = record a, b, c: integer32; d: integer; end;
procedure ext_$w(w: integer); options(val_param, extern);
procedure ext_$uid(in u: uid_$t); extern;
procedure ext_$opts(in w: integer; in o: opts_t); extern;
";

/// The lines of routine `t` from its `link` to its `rts`.
fn routine(src: &str) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{src}")).unwrap();
    let start = lines.iter().position(|l| l == "T:").unwrap() + 1;
    let end = start + lines[start..].iter().position(|l| l == "rts").unwrap();
    lines[start..=end].to_vec()
}

fn error(src: &str) -> String {
    let e = compile(&format!("{DECLS}{src}")).unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    e[0].clone()
}

/// The callee writes its copy: the assignment is a store to the frame,
/// and the slot `(8,A6)` (the caller's variable's address) is read once,
/// by the copy, and never written through (ref 5.2 p. 5-4).
#[test]
fn assignment_goes_to_the_copy() {
    let lines = routine(
        "procedure t(w: integer);
begin
  w := 5;
  ext_$w(w);
end;",
    );
    assert_eq!(
        lines,
        [
            "link.w %fp,#-4",
            "movea.l (8,%fp),%a0",
            "move.w (%a0),(-2,%fp)",
            "move.w #5,(-2,%fp)",
            "subq.l #2,%sp",
            "move.w (-2,%fp),-(%sp)",
            "jsr (EXT_$W).l",
            "unlk %fp",
            "rts",
        ]
    );
    let through_slot = lines.iter().filter(|l| l.contains("(8,%fp)")).count();
    assert_eq!(through_slot, 1);
}

/// An 8-byte copy, `(A0)+` twice (DIR_$GET_ENTRYU 0xE4D512..0xE4D51A),
/// passed on by address as the copy (0xE4D55E `pea (-8,A6)`).
#[test]
fn uid_copy_passed_on() {
    assert_eq!(
        routine(
            "procedure t(u: uid_$t);
begin
  ext_$uid(u);
end;",
        ),
        [
            "link.w %fp,#-8",
            "movea.l (8,%fp),%a0",
            "move.l (%a0)+,(-8,%fp)",
            "move.l (%a0)+,(-4,%fp)",
            "pea (-8,%fp)",
            "jsr (EXT_$UID).l",
            "unlk %fp",
            "rts",
        ]
    );
}

/// `in` parameters are not copied (ref 5.3.2, p. 5-7: the callee may not
/// assign them): read through the address.
#[test]
fn in_parameters_are_not_copied() {
    assert_eq!(
        routine(
            "procedure t(in w: integer);
begin
  ext_$w(w);
end;",
        ),
        [
            "link.w %fp,#0",
            "subq.l #2,%sp",
            "movea.l (8,%fp),%a0",
            "move.w (%a0),-(%sp)",
            "jsr (EXT_$W).l",
            "unlk %fp",
            "rts",
        ]
    );
}

#[test]
fn refused_copies() {
    let cases = [
        // Never used: whether the original copied it is not visible.
        (
            "procedure t(w: integer);
begin
  ext_$w(1);
end;",
            "a value parameter passed by address that the routine never uses",
        ),
        // An empty procedure is a bare `rts`; with a copy it would need a
        // frame.
        (
            "procedure t(w: integer);
begin
end;",
            "an empty procedure with a value parameter passed by address",
        ),
        // An array of char: no entry copy of one in the image
        // (codegen-patterns 5.2, "Arrays of char"; beads domain-pascal-40r2, -4lf1).
        (
            "procedure t(s: str3);
begin
  ext_$w(ord(s[1]));
end;",
            "copying a value parameter that is an array of char",
        ),
        // A block copy after a copy that left its register: only A0 to
        // A1 is seen (XPD_$SET_PTRACE_OPTS 0xE5AFB0).
        (
            "procedure t(w: integer; o: opts_t);
begin
  ext_$opts(w, o);
end;",
            "a block copy of a value parameter passed by address while an earlier copy",
        ),
        // In a nested routine (none catalogued).
        (
            "procedure t;
var x: uid_$t;
  procedure n(u: uid_$t);
  begin
    ext_$uid(u);
  end;
begin
  n(x);
end;",
            "to a nested routine",
        ),
    ];
    for (src, want) in cases {
        let e = error(src);
        assert!(e.contains(want), "{e}");
    }
}

/// The copies are made on entry, before the body: a `var` parameter and
/// a value parameter may name the same caller variable (`t(x, x)`), and
/// the value parameter keeps the value it had at the call after the body
/// writes the `var` one (ref 5.2 p. 5-4; 5.3.1 p. 5-6).
#[test]
fn copy_precedes_writes_through_other_parameters() {
    let lines = routine(
        "procedure t(var v: integer; w: integer);
begin
  v := 5;
  ext_$w(w);
end;",
    );
    let copy = lines
        .iter()
        .position(|l| l.starts_with("move.w (%a") && l.ends_with("(-2,%fp)"))
        .expect("the copy");
    let store = lines
        .iter()
        .position(|l| l.starts_with("move.w #5,"))
        .expect("the store through `v`");
    assert!(copy < store, "{lines:?}");
    assert!(
        lines.contains(&"move.w (-2,%fp),-(%sp)".to_string()),
        "{lines:?}"
    );
}

/// A block copy from A0 into A1 after an 8-byte copy that used A0 up
/// with `(A0)+` (XPD_$SET_PTRACE_OPTS 0xE5AFA4..0xE5AFBE), and the
/// copies' places there: the 14-byte one at -0x10, the UID's below it at
/// -0x18 (the larger first, codegen-patterns 2, "Where the compiler's
/// objects go"; both at multiples of 8).
#[test]
fn block_copy_after_postincrement_copy() {
    assert_eq!(
        routine(
            "procedure t(u: uid_$t; o: opts_t);
begin
  ext_$uid(u);
  ext_$opts(1, o);
end;",
        )[..9],
        [
            "link.w %fp,#-0x18",
            "movea.l (8,%fp),%a0",
            "move.l (%a0)+,(-0x18,%fp)",
            "move.l (%a0)+,(-0x14,%fp)",
            "movea.l (0xc,%fp),%a0",
            "lea (-0x10,%fp),%a1",
            "move.l (%a0)+,(%a1)+",
            "move.l (%a0)+,(%a1)+",
            "move.l (%a0)+,(%a1)+",
        ]
    );
}

/// Assignments to by-value parameters (codegen-patterns 12.2, "Assigned
/// parameters"; bead domain-pascal-jyba): loaded at entry, the parameter
/// is assigned in its register (mst_$set_mstes 0xE43E36 `ori.w #2,D0`),
/// a computation from itself made in it (area_$internal_create 0xE077FE);
/// the shapes no site settles are refused.
#[test]
fn assigned_by_value_parameters() {
    // A pointer assigned: kept in its slot (DISK_$ADD_QUE 0xE3C790
    // `move.l A0,(0x12,A6)`; bead domain-pascal-ogf8).
    let lines = routine(
        "type ip = ^integer;
procedure t(p: ip); options(val_param);
begin
  ext_$w(p^);
  p := nil;
  ext_$w(p^);
end;",
    );
    assert!(lines.contains(&"clr.l (8,%fp)".to_string()), "{lines:?}");
    assert!(!lines.iter().any(|l| l.starts_with("movea.l (8,%fp),%a2")), "{lines:?}");
    let lines = routine(
        "procedure t(w: integer); options(val_param);
begin
  w := w ! 2;
  ext_$w(w);
  w := (w + 16#7FFF) & 16#FF;
  ext_$w(w);
end;",
    );
    assert_eq!(lines[1..3], ["move.l %d2,-(%sp)", "move.w (8,%fp),%d2"]);
    assert!(
        lines.contains(&"ori.w #2,%d2".to_string())
            && lines.contains(&"addi.w #0x7fff,%d2".to_string())
            && lines.contains(&"andi.w #0xff,%d2".to_string()),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|l| l.contains(",(8,%fp)")), "{lines:?}");
    let cases = [
        (
            "procedure t(w: integer); options(val_param);
begin
  w := 1;
end;",
            "assigning to a by-value parameter in a body of one simple statement",
        ),
        (
            "procedure t(w: integer); options(val_param);
begin
  w := 1;
  ext_$w(2);
end;",
            "assigning to a by-value parameter the routine never reads",
        ),
        (
            "procedure ext_$v(var w: integer); extern;
procedure t(w: integer); options(val_param);
begin
  ext_$v(w);
  w := w + 1;
  ext_$w(w);
end;",
            "assigning to a by-value parameter whose slot is passed by address",
        ),
        (
            "procedure t(w: integer); options(val_param);
  procedure n;
  begin
    w := 0;
  end;
begin
  n;
  ext_$w(w);
end;",
            "assigning to an enclosing routine's parameter",
        ),
        // The original keeps these in their slots where the entry-load
        // rules would load them (bead domain-pascal-r0ag): no assignment
        // reads the parameter or clamps it.  Like area_$alloc_resources
        // 0xE07600 (`move.w D1,(8,A6)`, read back at 0xE07604, 0xE0772E).
        (
            "type pinteger = 0..65535;
var gw, maxe, nfree: pinteger;
procedure ext_$b; extern;
function t(count: pinteger): boolean; options(val_param);
var first, last: pinteger;
begin
  first := gw;
  if first >= maxe then
    t := false
  else begin
    t := true;
    if count + first > maxe then
      count := maxe - first;
    last := count + first;
    ext_$w(last);
    ext_$b;
    nfree := nfree + count;
  end;
end;",
            "none of whose assignments reads it or clamps it",
        ),
        // Like AST_$TRUNCATE 0xE05C7A (`clr.l (0xc,A6)`, read from the
        // slot at 0xE05C7E, 0xE05C98).
        (
            "var g: integer32;
procedure ext_$b; extern;
procedure t(loc: integer32; flags: integer); options(val_param);
var full: boolean;
begin
  full := (flags & 1) <> 0;
  if full then
    loc := 0;
  g := loc;
  ext_$b;
  g := loc;
end;",
            "none of whose assignments reads it or clamps it",
        ),
        // Like AREA_$INVALIDATE 0xE08EEA (`move.w D2,(0xe,A6)`, read from
        // the slot after).
        (
            "type pinteger = 0..65535;
var g: integer32;
procedure ext_$b; extern;
procedure t(seg: pinteger; off: pinteger; mx: integer32); options(val_param);
var start: integer32;
begin
  ext_$b;
  ext_$b;
  start := seg * 32 + off;
  if start > mx then
    off := mx & 16#1F;
  if off <> 0 then
    g := start;
  g := off;
end;",
            "none of whose assignments reads it or clamps it",
        ),
    ];
    for (src, want) in cases {
        let e = error(src);
        assert!(e.contains(want), "{src}\n{e}");
    }
}

/// A copy only read, not every read pushed, is held in a data register
/// loaded through the slot where the copy would be made; its frame slot
/// stays reserved (`link #-4`): FILE_$SET_REFCNT 0xE5E40A `movea.l
/// (0xc,A6),A0; move.l (A0),D0`, then `cmpi.l #-1,D0` (codegen-patterns
/// 1.2, "Held in a register instead"; bead domain-pascal-gfnj).  Copied as
/// before when every read pushes it, when it is assigned, or when a read
/// lies in a loop.
#[test]
fn copies_held_in_a_register() {
    let lines = routine(
        "procedure t(l: integer32; out w: integer);
begin
  if l = -1 then
    w := 0
  else
    w := l;
end;",
    );
    assert_eq!(lines[0], "link.w %fp,#-4", "{lines:?}");
    // A1: the `out` slot's entry load took A0 (0xE5E402 takes A2 and A3
    // for the slots used twice, then A0 for the count's)
    assert!(lines.iter().any(|l| l == "move.l (%a1),%d0"), "{lines:?}");
    assert!(lines.iter().any(|l| l == "cmpi.l #-1,%d0"), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("(-4,%fp)")), "{lines:?}");
    for src in [
        "procedure t(w: integer);
begin
  ext_$w(w);
end;",
        "procedure t(w: integer);
begin
  w := w + 1;
  ext_$w(w + 1);
end;",
        "var g: integer;
procedure t(w: integer);
var i: integer;
begin
  i := 0;
  repeat i := i + 1 until i = w;
  g := i;
end;",
    ] {
        let lines = routine(src);
        assert!(
            lines.iter().any(|l| l.starts_with("move.w (%a0),(-")),
            "{src}\n{lines:?}"
        );
    }
}
