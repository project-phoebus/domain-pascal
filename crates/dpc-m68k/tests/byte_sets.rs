//! One-byte sets, `[byte] set of 0..7` (ref 3.15.5 p. 3-61 and the
//! owner's ruling, reference-clarifications "A size attribute on a set
//! type"; codegen-patterns 5.1, "One-byte sets"; bead domain-pascal-tq8w):
//! the instruction forms the ACL_ and APP_ routines of the image show,
//! and the refusals of what they do not.

use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

fn compile(src: &str) -> Result<Vec<String>, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.message.clone()).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k).map_err(render)?;
    let module = dpc_ir::lower(&program).map_err(render)?;
    let asm = emit(&module, "t.pas", &Options::default()).map_err(render)?;
    Ok(asm
        .lines()
        .map(|l| {
            let l = l.split('|').next().unwrap();
            l.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .filter(|l| !l.is_empty() && !l.starts_with('.'))
        .collect())
}

/// A record with one-byte sets at 0..2 and a word set at 4 (`p^`), a
/// unit array of them, a word-set function.
const DECLS: &str = "module m;
type
  rights_t = set of 0..7;
  b_t = [byte] set of 0..7;
  r = record
    a: b_t;               { 0 }
    b: b_t;               { 1 }
    c: [byte] set of 0..7;  { 2 }
    w: set of 0..15;      { 4 }
  end;
  rp = ^r;
var
  arr: extern array[1..4] of b_t;
  gw: extern set of 0..15;
  gn: extern integer;
function wf: integer; extern;
procedure takes_in(in s: rights_t); extern;
";

fn body(routine: &str) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{routine}")).unwrap_or_else(|e| panic!("{e:?}"));
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

#[test]
fn constant_members_on_the_byte() {
    // ACL_$MIN_RIGHTS 0xE46922 `btst.b #4,(-0x14,A6)`; member 7 the
    // byte's sign (the word-set rule, 0xE1B63E `tst.b; bmi`).
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin if 4 in p^.b then gn := 1 end;"),
        [
            "movea.l (8,%fp),%a0",
            "btst #4,(1,%a0)",
            "beq.s 1f",
            "move.w #1,(GN).l",
            "1:"
        ]
    );
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin if 7 in p^.c then gn := 1 end;"),
        [
            "movea.l (8,%fp),%a0",
            "tst.b (2,%a0)",
            "bpl.s 1f",
            "move.w #1,(GN).l",
            "1:"
        ]
    );
}

#[test]
fn stores_and_updates() {
    // acl_$convert_image 0xE44EDC `clr.b`, acl_$def_acldata 0xE47916
    // `move.b #0x10`, acl_$convert_image 0xE44FBE `bset #2`,
    // acl_$image_internal 0xE47D40 `bclr #6`.
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin p^.a := []; p^.c := [4]; p^.b := p^.b + [2]; p^.a := p^.a - [6] end;"
        ),
        [
            "movea.l (8,%fp),%a0",
            "clr.b (%a0)",
            "move.b #0x10,(2,%a0)",
            "bset #2,(1,%a0)",
            "bclr #6,(%a0)"
        ]
    );
}

#[test]
fn widened_to_a_word() {
    // Into a word set (ACL_$MIN_RIGHTS 0xE4691C), into a word value
    // argument (acl_$prim_create_internal 0xE45360), compared with `[]`
    // (0xE45270).
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin gw := p^.b end;"),
        [
            "movea.l (8,%fp),%a0",
            "clr.w %d0",
            "move.b (1,%a0),%d0",
            "move.w %d0,(GW).l"
        ]
    );
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin if p^.b <> [] then gn := 1 end;"
        ),
        [
            "movea.l (8,%fp),%a0",
            "clr.w %d0",
            "move.b (1,%a0),%d0",
            "tst.w %d0",
            "beq.s 1f",
            "move.w #1,(GN).l",
            "1:"
        ]
    );
}

#[test]
fn narrowed_from_a_word() {
    // A function result (acl_$expand_default_acl 0xE45A50 `move.b D0`), a
    // word in memory, its low byte in place (acl_$find_acl_slot 0xE45F24).
    assert_eq!(
        body(
            "procedure t(p: rp); options(val_param);
             begin arr[1] := rights_t(wf); p^.a := p^.w end;"
        ),
        [
            "pea (%a2)",
            "movea.l (8,%fp),%a2",
            "jsr (WF).l",
            "move.b %d0,(ARR).l",
            "move.b (5,%a2),(%a2)",
            "movea.l (-4,%fp),%a2"
        ]
    );
}

#[test]
fn compared_with_sets_best_effort() {
    // Best effort (bead domain-pascal-61fd; no site): a one-byte set
    // against a set other than `[]` is widened to a word as against `[]`
    // (acl_$prim_create_internal 0xE45270 `clr.w D1; move.b (27,A4),D1`),
    // then compared as one.
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin if p^.a = p^.b then gn := 1 end;"),
        [
            "movea.l (8,%fp),%a0",
            "clr.w %d0",
            "clr.w %d1",
            "move.b (%a0),%d0",
            "move.b (1,%a0),%d1",
            "cmp.w %d1,%d0",
            "bne.s 1f",
            "move.w #1,(GN).l",
            "1:"
        ]
    );
    // ... against a word set, and against a constant.
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin if p^.a <> p^.w then gn := 1 end;"),
        [
            "movea.l (8,%fp),%a0",
            "clr.w %d0",
            "move.b (%a0),%d0",
            "cmp.w (4,%a0),%d0",
            "beq.s 1f",
            "move.w #1,(GN).l",
            "1:"
        ]
    );
    assert_eq!(
        body("procedure t(p: rp); options(val_param); begin if p^.a = [1] then gn := 1 end;"),
        [
            "movea.l (8,%fp),%a0",
            "clr.w %d0",
            "move.b (%a0),%d0",
            "cmpi.w #2,%d0",
            "bne.s 1f",
            "move.w #1,(GN).l",
            "1:"
        ]
    );
}

#[test]
fn run_time_member() {
    // AREA's area_$free_slot_segments 0xE08328..0xE0832E `clr.w D6; and.w
    // D4,D5; move.b (A2),D6; btst D5,D6; beq.w` (bead domain-pascal-eoc6).
    // Best effort (refused under `--strict`, below; bead
    // domain-pascal-5kgg).
    let was = dpc_syntax::diag::set_strict(false);
    let b = std::panic::catch_unwind(|| {
        body("procedure t(p: rp; k: integer); options(val_param); begin if k in p^.a then gn := 1 end;")
    });
    dpc_syntax::diag::set_strict(was);
    let w = dpc_syntax::diag::take_best_effort();
    assert!(
        w.iter().any(|d| d.message.contains("run-time element")),
        "{w:?}"
    );
    assert_eq!(
        b.unwrap_or_else(|e| std::panic::resume_unwind(e)),
        [
            "movea.l (8,%fp),%a0",
            "move.w (0xc,%fp),%d0",
            "clr.w %d1",
            "move.b (%a0),%d1",
            "btst %d0,%d1",
            "beq.s 1f",
            "move.w #1,(GN).l",
            "1:",
        ]
    );
}

#[test]
fn refusals() {
    let r = |src: &str, needle: &str| {
        let e = error(src);
        assert!(e.contains(needle), "{e}");
    };
    r(
        "procedure t(p: rp; k: integer); options(val_param); begin if k in p^.a then gn := 1 end;",
        "`in` on a one-byte set with a run-time element",
    );
    r(
        "procedure t(i: integer); options(val_param); begin if 2 in arr[i] then gn := 1 end;",
        "`in` on a one-byte set held in a register, indexed or volatile",
    );
    r(
        "procedure t(p: rp; k: integer); options(val_param); begin p^.a := p^.a + [k] end;",
        "updating a one-byte set other than by one constant member",
    );
    r(
        "procedure t(p: rp); options(val_param); begin p^.a := p^.a + [1, 2] end;",
        "updating a one-byte set other than by one constant member",
    );
    r(
        "procedure t(p: rp); options(val_param); begin p^.a := p^.a + p^.b end;",
        "updating a one-byte set other than by one constant member",
    );
    r(
        "procedure t(p: rp); options(val_param); begin arr[2] := p^.a * p^.b end;",
        "a set operation on one-byte sets as a value",
    );
    r(
        "procedure t(p: rp); options(val_param); begin p^.a := p^.w + [3] end;",
        "a set operation on sets of 2 and 1 bytes",
    );
    r(
        "procedure t(p: rp); options(val_param); begin p^.a := rights_t(gn + 1) end;",
        "assigning a one-word set that is not a function result or a word in memory",
    );
    // By address to a word-set `in` parameter: a converted copy (1.5).
    r(
        "procedure t(p: rp); options(val_param); begin takes_in(p^.a) end;",
        "passing a variable of another size to a value or `in` parameter",
    );
    // A one-byte set held in a register, less another (`move.b; not.b;
    // and.b`: no site).
    r(
        "procedure t(p: rp); options(val_param); var s: b_t; i: integer;
         begin s := p^.a; for i := 1 to 10 do s := s - p^.b; p^.c := s end;",
        "a set operation on one-byte sets as a value",
    );
    // A word-set local narrowed into a byte set (held or in the frame: no
    // site; only a function result and a word in memory have one).
    r(
        "procedure t(p: rp); options(val_param); var s: set of 0..15; i: integer;
         begin s := p^.w; for i := 1 to 10 do s := s - [i]; p^.a := s end;",
        "assigning a one-word set local or value parameter to a one-byte set",
    );
}
