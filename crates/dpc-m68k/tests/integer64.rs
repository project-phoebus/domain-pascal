//! dpc's `integer64` (not in the reference; owner, 2026-11-12;
//! docs/aarch64.md): checked on every target, refused by the m68k
//! backend when it emits code, with a diagnostic rather than a panic.

use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

fn compile(src: &str) -> Result<String, Vec<String>> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> Vec<String> {
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k::default()).map_err(render)?;
    let module = dpc_ir::lower(&program).map_err(render)?;
    emit(&module, "t.pas", &Options::default())
        .map_err(|ds| ds.iter().map(|d| d.message.clone()).collect())
}

#[test]
fn refused_when_emitted() {
    for body in [
        "var a: integer64; procedure p; begin a := 5000000000 end;",
        "var a: integer64; l: integer32; procedure p; begin l := a end;",
        "function f(a: integer64): integer64; begin f := a + 1 end;",
        "procedure p; var a: integer64; begin a := 1 end;",
    ] {
        let e = compile(&format!("module m;\n{body}")).unwrap_err();
        assert!(
            e.iter().any(|m| m.contains("`integer64`") && m.contains("not supported on m68k")),
            "{body}: {e:?}"
        );
    }
}

#[test]
fn data_alone_is_emitted() {
    // A variable with no code that computes with it: its 8 bytes.
    let asm = compile("module m;\nvar a: integer64 := 16#123456789; b: integer;\n\
                       procedure p; begin b := 1 end;")
        .unwrap();
    assert!(asm.contains(".byte\t0x00,0x00,0x00,0x01,0x23,0x45,0x67,0x89"), "{asm}");
}
