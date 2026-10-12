//! Expressions end to end from Pascal source (codegen-patterns 5.2, 8,
//! 3): what is selected, and what is reported as not supported yet.
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
  rec = record a: integer32; b: integer; end;
  s16 = set of 0..15;
var
  ss: extern s16;
  w: extern integer;
  l: extern integer32;
  u: extern pinteger;
  ws: array[1..8] of integer;
  xs: extern array[1..8] of integer;
  rs: array[1..8] of rec;
function f(x: integer): integer; options(val_param, extern);
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

/// Multiplication by a constant (codegen-patterns 8): shifts and adds for
/// 2^s (2^k +- 1), `mulu.w` for odd parts of four bits or more.
#[test]
fn constant_multiplication_forms() {
    let mul = |c: u32| {
        body(&format!(
            "function t: integer; internal;\nbegin\n  t := w * {c};\nend;\n"
        ))
    };
    // x2, x8, x256 (one shift of 8), x1024 (8 then 2: 0xE02AA6).
    assert_eq!(mul(2)[1], "add.w\t%d0,%d0");
    assert_eq!(mul(8)[1], "lsl.w\t#3,%d0");
    assert_eq!(mul(1024)[1..], ["lsl.w\t#8,%d0", "lsl.w\t#2,%d0"]);
    // x5 (0xE00CCA), x72 (0xE6C072), x60 (0xE5AB74).
    assert_eq!(
        mul(5)[1..],
        ["move.w\t%d0,%d1", "lsl.w\t#2,%d1", "add.w\t%d1,%d0"]
    );
    assert_eq!(
        mul(72)[1..],
        [
            "lsl.w\t#3,%d0",
            "move.w\t%d0,%d1",
            "lsl.w\t#3,%d1",
            "add.w\t%d1,%d0"
        ]
    );
    assert_eq!(
        mul(60)[1..],
        [
            "lsl.w\t#2,%d0",
            "move.w\t%d0,%d1",
            "neg.w\t%d0",
            "lsl.w\t#4,%d1",
            "add.w\t%d1,%d0"
        ]
    );
    // x150 (0xE54E24 `mulu.w #150'): odd part 75, four bits.
    assert_eq!(mul(150)[1], "mulu.w\t#0x96,%d0");
}

#[test]
fn record_array_elements() {
    // 6-byte records: x6 = 2 x 3; the array's offset (16), the field's (4)
    // and the lower bound (-6) folded into the displacement.
    assert_eq!(
        body("function t: integer; internal;\nbegin\n  t := rs[w].b;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "add.w\t%d0,%d0",
            "move.w\t%d0,%d1",
            "add.w\t%d1,%d1",
            "add.w\t%d1,%d0",
            "move.w\t(0xe,%a5,%d0.w),%d0",
        ]
    );
}

#[test]
fn unsupported_expressions() {
    let cases: &[(&str, &str)] = &[
        // Odd part of three bits: shifts (x26, 0xE71BBA) or `mulu' (x21).
        ("t := w * 21;", "not settled for an odd part of three bits"),
        // 32-bit multiplication and division call M$ARITH.
        ("t := l * l;", "a 32-bit multiplication (M$ARITH)"),
        ("t := l div 3;", "`div`/`mod` other than of an integer16"),
        // Two words multiplied for a longword (ref 4.3.1): `muls.w`?
        (
            "t := w * w;",
            "a product of two word values used as a longword",
        ),
        // Another unit's array: address split not catalogued.
        ("t := xs[w];", "an element of an array other than"),
        // Two calls in one statement.
        ("t := f(w) + f(w);", "two function calls in one statement"),
        // `!' of a variable and a constant: both orders in the image.
        ("t := w ! 4;", "`!`/`xor` of a variable and a constant"),
    ];
    for (stmt, want) in cases {
        let e = error(&format!(
            "function t: integer32; internal;\nbegin\n  {stmt}\nend;\n"
        ));
        assert!(e.contains(want), "{stmt}: {e}");
    }
    // An array element updated in place: both forms in the image
    // (AUDIT_$SUSPEND 0xE70DC8; 0xE13154).
    let e = error("procedure t; internal;\nbegin\n  ws[w] := ws[w] + 1;\nend;\n");
    assert!(e.contains("updating an array element in place"), "{e}");
    // rshft of a signed word (ref p. 4-171).
    let e = error("function t: integer; internal;\nbegin\n  t := rshft(w, 2);\nend;\n");
    assert!(e.contains("`rshft` of a signed operand"), "{e}");
    // Two words multiplied for a word.
    let e = error("function t: integer; internal;\nbegin\n  t := w * w;\nend;\n");
    assert!(e.contains("multiplying two variables"), "{e}");
    // `in' on a one-word set with a variable element.
    let e = error("function t: boolean; internal;\nbegin\n  t := w in ss;\nend;\n");
    assert!(
        e.contains("one-word set with an element that is not constant"),
        "{e}"
    );
}

/// Ref 4.3.1, p. 4-5: a word sum, difference, product or negation used as
/// a longword is computed in 32 bits, the operands extended first
/// (0xE0863C..0xE08642 `ext.l D3; subq.l #2,D3; move.l D3,...`;
/// 0xE07442..0xE0744A `ext.l D6; lsl.l #2,D6; ...`), not computed in 16
/// bits and extended (which loses the carry: 30000 + 30000).
#[test]
fn word_arithmetic_widened_is_done_in_32_bits() {
    assert_eq!(
        body("procedure t; internal;\nbegin\n  l := w - 2;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "subq.l\t#2,%d0",
            "move.l\t%d0,(L).l",
        ]
    );
    assert_eq!(
        body("procedure t; internal;\nbegin\n  l := w * 12;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "lsl.l\t#2,%d0",
            "move.l\t%d0,%d1",
            "add.l\t%d1,%d1",
            "add.l\t%d1,%d0",
            "move.l\t%d0,(L).l",
        ]
    );
    assert_eq!(
        body("procedure t; internal;\nbegin\n  l := -w;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "neg.l\t%d0",
            "move.l\t%d0,(L).l",
        ]
    );
    // `mulu.w #c' gives the low word only for a longword: refused.
    let e = error("procedure t; internal;\nbegin\n  l := w * 564;\nend;\n");
    assert!(e.contains("32-bit multiplication by a constant"), "{e}");
    // A word `lshft' used as a longword: not settled (p. 4-109).
    let e = error("procedure t; internal;\nbegin\n  l := lshft(w, 8);\nend;\n");
    assert!(
        e.contains("`lshft` of a word operand used as a longword"),
        "{e}"
    );
    // `arshft' gives the same value either way: kept.
    assert_eq!(
        body("procedure t; internal;\nbegin\n  l := arshft(w, 3);\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "asr.w\t#3,%d0",
            "ext.l\t%d0",
            "move.l\t%d0,(L).l",
        ]
    );
}

/// `(d,An,Dn.w)' sign-extends the index register: a word index is only
/// taken when every index x size of the array's bounds fits in 16 bits.
#[test]
fn word_index_must_fit_in_16_bits() {
    let decls = "type big = array[0..40000] of char; wide = array[0..20000] of integer;
var bs: big; ws2: wide;
";
    for stmt in ["bs[u] := 'a';", "t := ws2[w];"] {
        let e = compile(&format!(
            "{DECLS}{decls}procedure t; internal;\nvar t: integer;\nbegin\n  {stmt}\nend;\n"
        ))
        .unwrap_err();
        assert!(
            e[0].contains("a word index whose scaled value may not fit in 16 bits"),
            "{stmt}: {e:?}"
        );
    }
}

/// `btst #k,Dn' takes k mod 32: an element beyond the set's bits is refused.
#[test]
fn set_element_outside_the_set_is_refused() {
    let e = error("function t: boolean; internal;\nbegin\n  t := 20 in ss;\nend;\n");
    assert!(e.contains("constant element outside the set's bits"), "{e}");
}
