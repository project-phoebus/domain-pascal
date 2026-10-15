//! Packed arrays of booleans (ref 3.11.4.2 Table 3-5, 3.15.5 p. 3-62;
//! codegen-patterns 5.2, "Packed arrays of booleans"; bead
//! domain-pascal-51e), end to end from Pascal source: the instruction
//! forms of PCHIST's and ACL's sites and the refusals.  What the code
//! computes is checked on the 68010 by `tests/exec/packed_bools.pas`.

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
        ds.iter().map(|d| d.render(&sm, "dpc")).collect()
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k).map_err(render)?;
    let module = dpc_ir::lower(&program).map_err(render)?;
    let asm = emit(&module, "t.pas", &Options::default())
        .map_err(|ds| ds.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    Ok(asm
        .lines()
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// PCHIST's block as ../source's pchist/pchist.pas lays it out: the
/// bitmap at +0x18.
const DECLS: &str = "module m;
var
  lock: array[1..9] of integer;
  pad: array[1..3] of integer;
  bitmap: packed array[1..64] of boolean;
  gw: integer;
  current: extern integer;
";

/// The body of routine `T` (after the data base load, up to `movem`).
fn body(routine: &str) -> Vec<String> {
    let lines = compile(&format!("{DECLS}{routine}")).unwrap();
    let t = lines.iter().position(|l| l == "T:").unwrap();
    let start = t
        + 1
        + lines[t..]
            .iter()
            .position(|l| l.starts_with("lea\t(.Ldata)"))
            .unwrap();
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.starts_with("movem.l\t(") || l.starts_with("movea.l\t(-"))
            .unwrap();
    lines[start..end].to_vec()
}

fn error(routine: &str) -> String {
    let e = compile(&format!("{DECLS}{routine}")).unwrap_err();
    assert_eq!(e.len(), 1, "{e:?}");
    e[0].clone()
}

fn lines(s: &str) -> Vec<String> {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.replacen(' ', "\t", 1))
        .collect()
}

/// The test: on the index's low byte, the sequence of PCHIST_$COUNT
/// 0xE1A156..0xE1A170 and PCHIST_$UNIX_PROFIL_FORK 0xE5CC44..0xE5CC62
/// (another unit's word read at its address + 1).
#[test]
fn test_on_the_low_byte() {
    assert_eq!(
        body("procedure t; begin if bitmap[current] then gw := 1; end;"),
        lines(
            "move.b (CURRENT+1).l,%d0
             clr.w %d1
             subq.b #1,%d0
             move.b %d0,%d1
             lsr.w #3,%d1
             lea (0,%a5,%d1.w),%a0
             moveq #7,%d1
             moveq #7,%d2
             and.b %d0,%d1
             sub.b %d1,%d2
             btst %d2,(0x18,%a0)
             beq.s 1f
             move.w #1,(0x20,%a5)
             1:"
        )
    );
    // `not`: `bne` (PCHIST_$UNIX_PROFIL_CNTL 0xE5CB56); a variable of the
    // unit, its low byte at +1 (0xE5CB3A `move.b (-0x21,A6),D0` for the
    // word at -0x22).
    assert_eq!(
        body("procedure t; begin if not bitmap[gw] then gw := 1; end;")[..2],
        lines(
            "move.b (0x21,%a5),%d0
             clr.w %d1"
        )
    );
    assert!(
        body("procedure t; begin if not bitmap[gw] then gw := 1; end;")
            .contains(&"bne.s\t1f".to_string())
    );
}

/// The stores: on the index's word, PCHIST_$UNIX_PROFIL_FORK
/// 0xE5CCDC..0xE5CCF4 (set) and PCHIST_$UNIX_PROFIL_CNTL
/// 0xE5CBF2..0xE5CC0C (clear, `#-0x81` and `and.b`).
#[test]
fn stores_on_the_word() {
    assert_eq!(
        body("procedure t; begin bitmap[current] := true; end;"),
        lines(
            "move.w (CURRENT).l,%d0
             moveq #7,%d1
             subq.w #1,%d0
             move.l #0x80,%d2
             and.w %d0,%d1
             lsr.l %d1,%d2
             move.w %d0,%d1
             lsr.w #3,%d1
             lea (0,%a5,%d1.w),%a0
             or.b %d2,(0x18,%a0)"
        )
    );
    let clear = body("procedure t; begin bitmap[gw] := false; end;");
    assert_eq!(clear[0], "move.w\t(0x20,%a5),%d0");
    assert_eq!(clear[3], "move.l\t#-0x81,%d2");
    assert_eq!(clear[9], "and.b\t%d2,(0x18,%a0)");
}

/// A function result: `btst; sne D0` (ACL_$USED_SUSER 0xE492FA..0xE492FE).
#[test]
fn function_result() {
    let b = body("function t: boolean; begin t := bitmap[current]; end;");
    assert_eq!(b[b.len() - 2..], lines("btst %d2,(0x18,%a0)\nsne %d0"));
}

#[test]
fn refusals() {
    for (src, why) in [
        (
            "procedure t; begin bitmap[3] := true; end;",
            "constant index",
        ),
        (
            "procedure t; begin if bitmap[3] then gw := 1; end;",
            "constant index",
        ),
        (
            "procedure t(in b: boolean); begin bitmap[gw] := b; end;",
            "other than the constant `true` or `false`",
        ),
        (
            "var b: boolean;\nprocedure t; begin b := bitmap[gw]; end;",
            "as a value other than a condition",
        ),
        (
            "procedure t; begin if bitmap[gw + 1] then gw := 1; end;",
            "computed index",
        ),
        (
            "procedure t; var l: integer32; begin l := 5; if bitmap[l] then gw := 1; end;",
            "4-byte index",
        ),
        (
            "var z: packed array[0..63] of boolean;\n\
             procedure t; begin z[gw] := true; end;",
            "lower bound is not 1..8",
        ),
        (
            "var big: packed array[1..264] of boolean;\n\
             procedure t; begin if big[gw] then gw := 1; end;",
            "more than 256 booleans",
        ),
        (
            "var other: extern packed array[1..64] of boolean;\n\
             procedure t; begin other[gw] := true; end;",
            "not in the unit's data",
        ),
        (
            "procedure t; var f: packed array[1..64] of boolean; begin f[gw] := true; end;",
            "not in the unit's data",
        ),
        (
            "var two: packed array[1..8] of 0..3;\n\
             procedure t; begin gw := two[gw]; end;",
            "2-bit elements",
        ),
        (
            "procedure s(var b: boolean); extern;\n\
             procedure t; begin s(bitmap[gw]); end;",
            "address of an element of a packed array of bits",
        ),
    ] {
        let e = error(src);
        assert!(e.contains(why), "{src}: {e}");
    }
}

/// Byte elements (a subrange of 5..8 bits) are bytes, read like a
/// `[byte]` subrange's (the FIM_ frame size table, 0xE0A486..0xE0A492
/// `clr.l D2; ...; lea (0,A5,D1.w),A2; move.b (0x124,A2),D2`).
#[test]
fn byte_elements() {
    let b = body(
        "var fs: packed array[0..15] of 0..255;\n\
         procedure t; begin gw := fs[gw]; end;",
    );
    assert!(
        b.iter().any(|l| l.starts_with("move.b\t(0x22,%a5,")),
        "{b:?}"
    );
}
