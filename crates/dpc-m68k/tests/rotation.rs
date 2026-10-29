//! The indices into the selected instructions that a store's rotation
//! (the destination's instructions before the value's,
//! codegen-patterns 12.4, "Stores whose destination and value both need
//! a register") must move with their instructions (bead
//! domain-pascal-c7kh).  Each store below is rotated, and the value's
//! instructions record an index that a check after selection reads.
//! Before the fix the index still pointed into the destination's
//! instructions, and the check read the wrong instruction.
use dpc_m68k::emit::{emit, Options};
use dpc_syntax::preprocess::{preprocess, Options as PpOptions};
use dpc_syntax::SourceMap;

/// Compile `src` (strict, the library default) to assembly lines without
/// comments and blank lines, spaces normalised, or the rendered
/// diagnostics.
fn compile(src: &str) -> Result<Vec<String>, String> {
    let mut sm = SourceMap::new();
    let f = sm.add("t.pas", src.to_string());
    let pp = preprocess(&mut sm, f, &PpOptions::default());
    assert!(!pp.has_errors(), "preprocessor errors");
    let unit = dpc_syntax::parser::parse(&pp.tokens).expect("parses");
    let render = |ds: Vec<dpc_syntax::Diagnostic>| -> String {
        ds.iter()
            .map(|d| d.render(&sm, "dpc"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let program = dpc_sema::analyze(&unit, &dpc_m68k::M68k::default()).map_err(render)?;
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

/// `other_base_loads`: the block of another data section loaded by the
/// value of a rotated store, then read on both sides of a call.  The
/// check that refuses that read the destination's first instruction
/// (`move.w D0,D1`, no `movea.l`) and let the block's register through
/// the call: `move.l (A2),D1` after `jsr S_$G`.
#[test]
fn block_load_in_a_rotated_store() {
    let src = "module s, wired_proc, wired_data;
var
  hits: integer32;
  limit: integer;
  g: extern integer;
function s_$g(x: integer32): integer32; extern;
[section(os_init_proc, os_init_data)]
procedure s_$p;
var
  a: array[1..10] of integer;
  i: integer;
  l: integer32;
begin
  i := g;
  a[i] := limit;
  l := hits + s_$g(1);
  hits := l;
end;
";
    let e = compile(src).unwrap_err();
    assert!(
        e.contains("another data section's block reached on both sides of a call"),
        "{e}"
    );
}

/// `zext_bases`: a zero-extended `.l` index of another unit's array as
/// the value of a rotated store.  The check after scheduling looked for
/// three of the destination's instructions in a row and refused a body
/// whose `clr.l; movea.l #CHARS; move.w` stayed together.
#[test]
fn zero_extended_index_in_a_rotated_store() {
    let src = "module wi;
type
  pinteger = 0..65535;
var
  chars: extern array[1..40000] of char;
  gj, gm: extern integer;
procedure by_char(i: pinteger);
  options(val_param);
var
  k: pinteger;
  b: array[1..40] of char;
begin
  k := i;
  b[lshft(gj, 3) + gm] := chars[k];
end;
";
    let lines = compile(src).unwrap_or_else(|e| panic!("{e}"));
    let at = lines
        .iter()
        .position(|l| l == "movea.l #CHARS,%a0")
        .unwrap_or_else(|| panic!("{lines:?}"));
    assert_eq!(
        lines[at - 1..at + 2],
        ["clr.l %d2", "movea.l #CHARS,%a0", "move.w %d1,%d2"],
        "{lines:?}"
    );
}

/// `other_held`: the base of another unit's array loaded by the value of
/// a rotated store and read again by the next statement.  Its load's
/// index named the destination's first `movea.l`, so the check of the
/// instructions since the load saw the destination's pointer loads and
/// refused the base kept.
#[test]
fn held_base_from_a_rotated_store() {
    let src = "module oh;
type
  rp = ^r;
  r = record f: char; g: char; q: rp; end;
var
  chars: extern array[1..400] of char;
procedure held(i: integer; p: rp);
  options(val_param);
var
  k: integer;
  l: rp;
begin
  k := i;
  l := p;
  p^.q^.q^.f := chars[k];
  l^.g := chars[k + 1];
end;
";
    let lines = compile(src).unwrap_or_else(|e| panic!("{e}"));
    let bases = lines.iter().filter(|l| *l == "movea.l #CHARS,%a1").count();
    assert_eq!(bases, 1, "{lines:?}");
    assert!(
        lines.contains(&"move.b (0,%a1,%d1.w),(1,%a0)".to_string()),
        "{lines:?}"
    );
}

/// `held_firsts`: a block comparison as the value of a rotated store,
/// its first address `lea (A0),A1` of a held pointer that dies there.
/// The check after allocation read two `muls.w` of the destination's
/// index and passed the second address coloured into the dead pointer's
/// A0.
#[test]
fn held_first_in_a_rotated_store() {
    let src = "module bc;
type
  r = record a, b, c, d, e, f, g, h: integer32; end;
  rp = ^r;
var
  gj, gm: extern integer;
  u: extern r;
procedure cmp(p: rp; q: rp);
  options(val_param);
var
  bs: array[1..40] of boolean;
begin
  p^.a := 0;
  bs[gj * gm * gj * gm * gj * gm * gj * gm * gj * gm] := (p^ = u);
end;
";
    let e = compile(src).unwrap_err();
    assert!(
        e.contains("a block's second address in the register of the pointer held for its first"),
        "{e}"
    );
}
