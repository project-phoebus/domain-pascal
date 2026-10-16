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
/// with `(A0)+` (XPD_$SET_PTRACE_OPTS 0xE5AFA4..0xE5AFBE).
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
            "move.l (%a0)+,(-8,%fp)",
            "move.l (%a0)+,(-4,%fp)",
            "movea.l (0xc,%fp),%a0",
            "lea (-0x18,%fp),%a1",
            "move.l (%a0)+,(%a1)+",
            "move.l (%a0)+,(%a1)+",
            "move.l (%a0)+,(%a1)+",
        ]
    );
}
