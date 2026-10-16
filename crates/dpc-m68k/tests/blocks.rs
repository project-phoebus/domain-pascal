//! Block copies and record comparisons end to end from Pascal source
//! (codegen-patterns 5.2, "Block addresses"; beads domain-pascal-21xu,
//! -76uw, -40r2): what is refused at the address forms of bead 21xu.
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
  rec12 = record a: integer; b: array[0..3] of char; c: integer32; d: integer; end;
  chars24 = array[1..24] of char;
var
  mine: rec12;
  name: chars24;
  other_name: extern chars24;
";

fn error(routine: &str) -> String {
    compile(&format!("{DECLS}{routine}"))
        .err()
        .expect("refused")
        .join("\n")
}

/// The pointer held for a copy's source dies at its `lea (A0),Am`; the
/// destination's `lea (A5)` would take A0, which the image never does
/// (CAL_$WRITE_TIMEZONE 0xE3E628 `lea (A5),A3`; bead domain-pascal-76uw).
#[test]
fn second_address_in_a_dead_held_pointer() {
    let e = error(
        "procedure t(in r: rec12; out n: integer);
begin
  n := 0;
  if r.a = 0 then
    n := 1;
  mine := r;
end;
",
    );
    assert!(e.contains("domain-pascal-76uw"), "{e}");
}

/// Arrays of char at the addresses of bead 21xu are refused (the image
/// copies them by bytes, ACL_ 0xE45130; bead domain-pascal-40r2); at the
/// addresses copied before (the data area, a pointer slot) they are
/// copied by longwords as before.
#[test]
fn char_arrays() {
    let e = error(
        "procedure t(out c: chars24);
begin
  c := other_name;
end;
",
    );
    assert!(e.contains("domain-pascal-40r2"), "{e}");
    let e = error(
        "procedure t(in c: chars24; out n: integer);
begin
  n := 0;
  if c[1] = 'a' then
    n := 1;
  name := c;
  if c[2] = 'b' then
    n := 2;
end;
",
    );
    assert!(e.contains("domain-pascal-40r2"), "{e}");
    let lines = compile(&format!(
        "{DECLS}procedure t(out c: chars24);
begin
  c := name;
end;
"
    ))
    .unwrap();
    assert!(
        lines.contains(&"move.l\t(%a0)+,(%a1)+".to_string()),
        "{lines:?}"
    );
}
