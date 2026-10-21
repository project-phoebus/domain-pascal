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
  tz: array[1..4] of char;
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

/// Arrays of char are copied byte by byte at every address (the image:
/// ACL_ 0xE45130 `lea (A0),A2; moveq #23,D0; move.b (A1)+,(A2)+; dbf`, a
/// held pointer; CAL_$WRITE_TIMEZONE 0xE3E694 four `move.b` for a 4-byte
/// `cal_$tz_name_t`; codegen-patterns 5.2 "Arrays of char", bead
/// domain-pascal-40r2), a record holding one by longwords (0xE3E62A).
#[test]
fn char_arrays() {
    let loop24 = |from: &str, to: &str, l: u32| {
        vec![
            "moveq\t#0x17,%d0".to_string(),
            ".balignw\t4,0x2048".to_string(),
            format!("{l}:"),
            format!("move.b\t({from})+,({to})+"),
            format!("dbf\t%d0,{l}b"),
        ]
    };
    let body = |src: &str| -> Vec<String> {
        let lines = compile(&format!("{DECLS}{src}")).unwrap();
        let k = lines.iter().position(|l| l.starts_with("link.w")).unwrap();
        lines[k + 1..].to_vec()
    };
    // Another unit's object, then the pointer slot (refused before).
    let b = body(
        "procedure t(out c: chars24);
begin
  c := other_name;
end;
",
    );
    assert_eq!(
        b[..2],
        ["movea.l\t#OTHER_NAME,%a0", "movea.l\t(8,%fp),%a1"],
        "{b:?}"
    );
    assert_eq!(b[2..7], loop24("%a0", "%a1", 1)[..], "{b:?}");
    // The data area (copied by longwords before).
    let b = body(
        "procedure t(out c: chars24);
begin
  c := name;
end;
",
    );
    assert_eq!(
        b[2..4],
        ["lea\t(0xc,%a5),%a0", "movea.l\t(8,%fp),%a1"],
        "{b:?}"
    );
    assert_eq!(b[4..9], loop24("%a0", "%a1", 1)[..], "{b:?}");
    // A held pointer: `lea (A0),A2`, the destination past the dead A0
    // (ACL_ 0xE45130; refused before).
    let b = body(
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
    let k = b
        .iter()
        .position(|l| l == "lea\t(%a0),%a2")
        .expect("lea (A0),A2");
    assert_eq!(b[k + 1], "lea\t(0xc,%a5),%a3", "{b:?}");
    assert_eq!(b[k + 2..k + 7], loop24("%a2", "%a3", 2)[..], "{b:?}");
    // 4 bytes: four `move.b`, not one `move.l` (CAL_$READ_TIMEZONE
    // 0xE3E592..0xE3E5A0); the record holding them by longwords
    // (CAL_$WRITE_TIMEZONE 0xE3E626..0xE3E62E).
    let b = body(
        "procedure t(out n: integer);
var l: rec12;
begin
  l := mine;
  tz := l.b;
  n := l.a;
end;
",
    );
    assert_eq!(
        b[2..7],
        [
            "lea\t(%a5),%a0",
            "lea\t(-0xc,%fp),%a1",
            "move.l\t(%a0)+,(%a1)+",
            "move.l\t(%a0)+,(%a1)+",
            "move.l\t(%a0)+,(%a1)+",
        ],
        "{b:?}"
    );
    assert_eq!(
        b[7..13],
        [
            "lea\t(-0xa,%fp),%a0",
            "lea\t(0x24,%a5),%a1",
            "move.b\t(%a0)+,(%a1)+",
            "move.b\t(%a0)+,(%a1)+",
            "move.b\t(%a0)+,(%a1)+",
            "move.b\t(%a0)+,(%a1)+",
        ],
        "{b:?}"
    );
}

/// Char-array copies the image does not show, and arrays of other
/// one-byte elements, are refused (bead domain-pascal-4lf1).
#[test]
fn unsettled_byte_copies() {
    let cases = [
        ("c6 := c6b;", "an array of 6 char"),
        ("c2 := c2b;", "an array of 2 char"),
        ("c9 := c9b;", "an array of 9 char"),
        ("c50 := c50b;", "an array of 50 char"),
        ("c200 := c200b;", "an array of 200 char"),
        (
            "bools := bools2;",
            "an array of one-byte elements other than char",
        ),
        (
            "bytes := bytes2;",
            "an array of one-byte elements other than char",
        ),
        ("pk := pk2;", "a packed array of char"),
        ("two := two2;", "an array of arrays of char"),
        ("sub := sub2;", "an array of a subrange of char"),
    ];
    for (stmt, want) in cases {
        let e = compile(&format!(
            "module m;
type
  byte_t = [byte] 0..255;
var
  c6, c6b: array[1..6] of char;
  c2, c2b: array[1..2] of char;
  c200, c200b: array[1..200] of char;
  c9, c9b: array[1..9] of char;
  c50, c50b: array[1..50] of char;
  bools, bools2: array[1..10] of boolean;
  bytes, bytes2: array[1..4] of byte_t;
  pk, pk2: packed array[1..16] of char;
  two, two2: array[1..2] of array[1..8] of char;
  sub, sub2: array[1..16] of 'a'..'z';
procedure t;
begin
  {stmt}
end;
"
        ))
        .expect_err("refused")
        .join("\n");
        assert!(
            e.contains(want) && e.contains("domain-pascal-4lf1"),
            "{stmt}: {e}"
        );
    }
}

/// Blocks that are array elements indexed at run time (codegen-patterns
/// 5.2, "Block addresses", "Elements"; bead domain-pascal-dsr5): the
/// shapes the image does not settle are refused with their own messages;
/// the settled ones are golden `data/block_elements`, `_wide`.
#[test]
fn unsettled_element_blocks() {
    let cases = [
        // Both blocks elements (NAME_$FORK 0xE73E90).
        (
            "tbl[i] := tbl[j];",
            "two array elements indexed at run time",
        ),
        // Another unit's array (ACL_$GET_SID 0xE74C40).
        ("r := ext[i];", "an element of another unit's array"),
        ("ext[i] := r;", "an element of another unit's array"),
        // A frame array (none in compiled code).
        ("loc[i] := r;", "an element of an array in the frame"),
        // A computed index (every site's is a variable).
        ("tbl[i + j] := r;", "whose index is computed"),
        // A zero-extended byte index (none).
        ("tc[c] := r;", "a byte index zero-extended"),
        // The element second with an index of five instructions (x36).
        ("t36[i] := q36;", "takes more than three instructions"),
        // The element first in an unrolled copy (none).
        ("r := tbl[i];", "an unrolled block copy"),
        // The element first beyond a byte.
        ("q := big[i];", "or at a displacement beyond a byte"),
        // The element first with its index read from memory
        // (NAME_$GET_WDIR_UID 0xE58972 against MAC_OS_$CLOSE 0xE0B4E8).
        ("q := t32[cur];", "whose index is read from memory"),
        // The element first through a held pointer.
        (
            "q := p^[i]; p^[1].a := 0;",
            "through a pointer held in an address register",
        ),
        // The element second through a held pointer, unrolled (only `dbf`
        // copies seen: VTOCE_$WRITE 0xE3977C).
        ("pr^[i] := r; pr^[1].a := 0;", "only `dbf` copies seen"),
        // The count hoisted above the element's `lea` (VTOC_$MOUNT
        // 0xE38616..0xE3861A keeps it after).
        (
            "t32[i] := q; fl[i] := t32[i].a <> 0;",
            "would be scheduled above the element's",
        ),
        // Arrays of char (no byte copy with an element).
        ("names[i] := nm;", "a copy of an array of char"),
    ];
    for (stmt, want) in cases {
        let e = compile(&format!(
            "module m;
type
  rec12 = record a, b, c: integer32; end;
  rec36 = record a: integer32; b: array[1..8] of integer32; end;
  rec32 = record a: integer32; b: array[1..7] of integer32; end;
  tab_t = array[1..10] of rec12;
  tabp = ^tab_t;
  tab32 = array[1..10] of rec32;
  tab32p = ^tab32;
  name_t = array[1..16] of char;
var
  tbl: tab_t;
  t32: tab32;
  tc: array[char] of rec12;
  t36: array[1..10] of rec36;
  fl: array[1..10] of boolean;
  names: array[1..10] of name_t;
  cur: integer;
  pad: array[1..100] of integer32;
  big: tab32;
  ext: extern tab_t;
procedure t(i, j: integer; c: char; in out r: rec12; in out q: rec32;
            in out q36: rec36; p: tab32p; pr: tabp; in out nm: name_t);
            options(val_param);
var loc: tab_t;
begin
  loc[1].a := 0;
  {stmt}
end;
"
        ))
        .expect_err("refused")
        .join("\n");
        assert!(e.contains(want) && e.contains("dsr5"), "{stmt}: {e}");
    }
}

/// 8-byte copies (UIDs) from an array element indexed at run time
/// (codegen-patterns 5.2, "6 and 8 bytes"; bead domain-pascal-zrug): the
/// shapes the image does not settle are refused with their own messages;
/// the settled ones are golden `data/element_copy8`.
#[test]
fn unsettled_element_copies8() {
    let cases = [
        // Another unit's array (the pair's registers differ: 0xE73D20,
        // 0xE60AE8).
        (
            "u := ext[i];",
            "another unit's array, of another data section",
        ),
        // A frame array.
        ("u := loc[i];", "an element of an array in the frame"),
        // A computed index (`tbl[i + 1]` is `tbl[i]` with the 1 in the
        // displacement, codegen-patterns 5.2, "The index's constant part").
        ("u := tbl[i + j];", "whose index is computed"),
        // A zero-extended byte index (none).
        ("u := tc[c];", "a byte index zero-extended"),
        // Through a held pointer into a slot's object (none).
        (
            "u := p^[i]; p^[1].high := 0;",
            "into the object a pointer slot",
        ),
        // Through a held pointer beyond a byte (`with` records only).
        (
            "w := pb^.t[i]; pb^.pad[1] := w.low;",
            "at a displacement beyond a byte",
        ),
        // Into a slot, an index loaded from memory within a byte (none).
        ("u := tbl[cur];", "a loaded index within a byte not seen"),
        // Into a destination set up by other instructions.
        ("pp^^ := tbl[i];", "one set up by other instructions"),
        // Into the data area or an absolute address (not seen).
        ("tbl[1] := tbl[i];", "the data area, an absolute address"),
        ("ext[1] := tbl[i];", "the data area, an absolute address"),
        // Into a field of a slot's object (not seen).
        ("r.u := tbl[i];", "a field of the object a pointer slot"),
        // Into a slot, beyond a byte, an index zero-extended first (the
        // load rule needs a `move` from memory; 0xE4DED4).
        ("u := huge[k];", "a loaded index within a byte not seen"),
        // Into a slot, an index formed by an earlier statement.
        (
            "ext1(i); u := tbl[i]; u2 := tbl[i];",
            "a loaded index within a byte not seen",
        ),
        // The row of the copy just before, the index read from memory
        // (0xE5892C keeps the row, 0xE5345E recomputes it).
        (
            "w := wd[cur]; x := nd[cur]; u := w; u2 := x;",
            "of the copy of the statement just before",
        ),
        (
            "u := wd[cur]; u2 := nd[cur];",
            "of the copy of the statement just before",
        ),
        // Beyond a byte, the scaled index of an earlier statement held in
        // a register (the image keeps the row, 0xE58938).
        (
            "ext1(i); w := wd[i]; x := nd[i]; u := w; u2 := x;",
            "whose scaled index an earlier statement formed",
        ),
    ];
    for (stmt, want) in cases {
        let e = compile(&format!(
            "module m;
type
  uid_t = record high, low: integer32; end;
  tab_t = array[1..10] of uid_t;
  tabp = ^tab_t;
  big_t = record pad: array[1..100] of integer32; t: tab_t; end;
  bigp = ^big_t;
  uidp = ^uid_t;
  uidpp = ^uidp;
  rec_t = record a: integer32; u: uid_t; end;
  cnt_t = 0..65535;
var
  tbl: tab_t;
  tc: array[char] of uid_t;
  cur: integer;
  wd: tab_t;
  nd: tab_t;
  huge: array[1..5000] of uid_t;
  ext: extern tab_t;
procedure ext1(i: integer); options(val_param); extern;
procedure t(i, j: integer; c: char; out u: uid_t; v: uidp; p: tabp;
            pb: bigp; pp: uidpp; out r: rec_t; k: cnt_t; out u2: uid_t);
            options(val_param);
var loc: tab_t; w, x: uid_t;
begin
  loc[1].high := 0;
  {stmt}
end;
"
        ))
        .expect_err("refused")
        .join("\n");
        assert!(e.contains(want), "{stmt}: {e}");
    }
}

/// Within a byte, a scaled index held in a register reused by the next
/// statement's element copy is accepted (VOLX_$DISMOUNT 0xE6B4B0..0xE6B4E8
/// reuses `(d,A5,D2.w)` across statements; bead domain-pascal-zrug).
#[test]
fn held_index_reused_by_element_copies8() {
    let asm = compile(
        "module m;
type uid_t = record high, low: integer32; end;
  ent_t = record dev: integer32; a, b: uid_t; end;
var tbl: array[1..6] of ent_t;
procedure ext1(i: integer); options(val_param); extern;
procedure show(in u: uid_t); extern;
procedure t(i: integer); options(val_param);
var a, b: uid_t;
begin
  ext1(i);
  a := tbl[i].a;
  b := tbl[i].b;
  show(a);
  show(b);
end;
",
    )
    .expect("compiles");
    let leas = asm.iter().filter(|l| l.ends_with(",%d0.w),%a0")).count();
    assert_eq!(leas, 2, "{asm:?}");
}
