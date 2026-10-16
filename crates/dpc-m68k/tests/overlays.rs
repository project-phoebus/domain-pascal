//! Overlay sections of more than 0x8000 bytes end to end from Pascal
//! source (codegen-patterns 5.2, "Overlay sections of more than 0x8000
//! bytes"; bead domain-pascal-7pcf): the base of their arrays, the
//! section's start or 60000 bytes in, and the shapes refused.
use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

/// Compile `src` to assembly lines without comments and blank lines,
/// spaces normalised, or the rendered diagnostics.
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
            let l = l.split('|').next().unwrap();
            l.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .filter(|l| !l.is_empty())
        .collect())
}

/// A unit with overlay section `sec` holding `pad` bytes, the array `a`
/// of 64 words from 1, then `more` bytes, and routine `t` with body
/// `body`.
fn unit(pad: u32, more: u32, body: &str) -> String {
    let chars = |name: &str, n: u32| {
        if n == 0 {
            String::new()
        } else {
            format!("  {name}: array[1..{n}] of char;\n")
        }
    };
    let (pad, tail) = (chars("pad", pad), chars("tail", more));
    format!(
        "module m;
var (sec)
{pad}  a: array[1..64] of integer;
{tail}define t;
procedure t(in n: integer; in l: integer32);
var i: integer;
begin
  {body}
end;
"
    )
}

#[test]
fn bases_of_a_large_section() {
    // At the section's start: the symbol itself (FILE_$CHECK_PROT 0xE5D1C8).
    let lines = compile(&unit(0, 0x8000, "a[n] := 0;")).unwrap();
    assert!(lines.contains(&"movea.l #SEC,%a0".to_string()), "{lines:?}");
    assert!(
        lines.contains(&"clr.w (-2,%a0,%d0.w)".to_string()),
        "{lines:?}"
    );
    // At 0x7C00 and beyond, stepped in a `for` loop: 60000 bytes in
    // (ACL_$INIT 0xE3112A); the element 1 at 0x7C00 = 60000 + 2 - 0x6E62.
    let lines = compile(&unit(0x7C00, 0x400, "for i := 1 to 64 do a[i] := n;")).unwrap();
    assert!(
        lines.contains(&"movea.l #SEC+60000,%a1".to_string()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"move.w (%a0),(-0x6e62,%a1)".to_string()),
        "{lines:?}"
    );
    // A whole element at a constant index: absolute (FILE_$LOCK_INIT
    // 0xE327AC), wherever it lies.
    let lines = compile(&unit(0x9000, 0, "a[3] := 0;")).unwrap();
    assert!(
        lines.contains(&"clr.w (SEC+0x9004).l".to_string()),
        "{lines:?}"
    );
}

#[test]
fn refused_in_a_large_section() {
    let err = |src: String| compile(&src).unwrap_err().join("\n");
    // From 60000 bytes in at a displacement beyond a byte: the `lea`'s
    // register is not settled (ACL_$IN_SUBSYS 0xE470B2).
    let e = err(unit(0x7C00, 0x400, "a[n] := 0;"));
    assert!(e.contains("reached from its base 60000 bytes in"), "{e}");
    // ... and within a byte (the array at 60000, `(-2,An,Dx)`): not seen.
    let e = err(unit(60000, 0, "a[n] := 0;"));
    assert!(e.contains("reached from its base 60000 bytes in"), "{e}");
    // Between the start and 0x7C00: not seen.
    let e = err(unit(0x100, 0x8000, "a[n] := 0;"));
    assert!(e.contains("between the start and offset 0x7C00"), "{e}");
    let e = err(unit(0x100, 0x8000, "for i := 1 to 64 do a[i] := n;"));
    assert!(e.contains("between the start and offset 0x7C00"), "{e}");
    // At 0x7C00 or beyond in a section of 0x8000 bytes or less: not seen.
    let e = err(unit(0x7C00, 0x100, "for i := 1 to 64 do a[i] := n;"));
    assert!(e.contains("0x8000 bytes or less"), "{e}");
}
