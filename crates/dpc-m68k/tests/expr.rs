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
  cbuf = array[1..8] of char;
  ibuf = array[1..8] of integer;
  pbuf = record n: integer; cs: ^cbuf; end;
var
  ss: extern s16;
  w: extern integer;
  l: extern integer32;
  u: extern pinteger;
  ws: array[1..8] of integer;
  xs: extern array[1..8] of integer;
  xrs: extern array[1..8] of record a, b, c: integer32; end;
  xws: extern array[1..8] of record a, b: integer; end;
  xbs: extern array[1..8] of record a: integer32; z: array[1..13] of integer32; end;
  pp: extern univ_ptr;
  cp: extern ^cbuf;
  ip: extern ^ibuf;
  pb: extern ^pbuf;
  xh: extern array[100..107] of integer;
  xsr: extern record n: integer; a: array[1..8] of integer; end;
  rs: array[1..8] of rec;
function f(x: integer): integer; options(val_param, extern);
procedure g(n: integer; var c: integer); options(val_param, extern);
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
        // Odd part of three bits other than 11, 21, 25, 67, 145: shifts
        // (x26, odd part 13, 0xE71BBA) or a multiply (bead
        // domain-pascal-irb4).
        ("t := w * 13;", "odd parts of three bits"),
        ("t := w * 38;", "odd parts of three bits"),
        // The constant first with another unit's variable: no site.
        ("t := w * 44;", "another unit's variable, an array element"),
        // M$ARITH with a word argument and a computed operand: the pad is
        // scheduled into the computation (CAL_$WEEKDAY 0xE686B0).
        (
            "t := (l + 9) div 12;",
            "a word argument and a computed operand",
        ),
        // A word sum divided in 32 bits and narrowed, used as a longword.
        (
            "t := (w + 9) mod 12;",
            "a word `div`/`mod` of a sum, difference or product used as a longword",
        ),
        // Two M$ARITH results in one expression.
        ("t := l * l + l * l;", "two function calls in one statement"),
        // An unsigned `mod` by a power of two: mask or division?
        ("t := u mod 4;", "unsigned `mod` by a power of two"),
        // A signed `M$OIS$WLW' remainder used as a longword: no site.
        (
            "t := l mod w;",
            "a signed `mod` by a word used as a longword",
        ),
        // Both operands of an M$ARITH call computed: order not settled.
        ("t := (l + 1) * (l - 1);", "both operands computed"),
        // Two elements of other units' arrays in one statement
        // (FIM_$ACKNOWLEDGE 0xE0A994: not modelled).
        ("t := xs[w] + xs[w];", "two elements of other units' arrays"),
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
    // `in' on a one-word set with a variable element.
    let e = error("function t: boolean; internal;\nbegin\n  t := w in ss;\nend;\n");
    assert!(
        e.contains("one-word set with an element that is not constant"),
        "{e}"
    );
}

/// Scalings and products by a constant of 1..127 done with a multiply
/// (codegen-patterns 5.2, 8; bead domain-pascal-r3yy;
/// `tools/arithsurvey.py scale`): the constant first, `moveq #c,Dn;
/// mul<s>.w x,Dn` (TPAD_$SET_UNIT_MODE 0xE69824 `moveq #44,D2; mulu.w
/// (A4),D2`); 128 and more: `mul<s>.w #c` on the loaded value.
#[test]
fn products_with_the_constant_first() {
    let src = |routine: &str| {
        format!(
            "module m;
type
  c44 = record a: integer; pad: array[1..20] of integer; z: integer; end;
  c84 = record a: integer; pad: array[1..41] of integer; end;
  c26 = record a: integer; pad: array[1..12] of integer; end;
  c336 = record a: integer; pad: array[1..167] of integer; end;
  pinteger = 0..65535;
var
  cfgs: array[1..8] of c44;
  c84s: array[0..7] of c84;
  c26s: array[0..7] of c26;
  c336s: array[0..7] of c336;
  k: integer;
  pk: pinteger;
  cs: array[0..255] of c44;
  ks: array[1..4] of integer;
{routine}"
        )
    };
    let body = |routine: &str| {
        let lines = compile(&src(routine)).unwrap();
        let start = lines.iter().position(|l| l == "T:").unwrap() + 2;
        let end = start
            + lines[start..]
                .iter()
                .position(|l| l.starts_with("movea.l\t(-8,%fp),%a5") || l.starts_with("unlk"))
                .unwrap();
        lines[start..end].to_vec()
    };
    let error = |routine: &str| {
        let e = compile(&src(routine)).unwrap_err();
        assert_eq!(e.len(), 1, "{e:?}");
        e[0].clone()
    };
    // A by-address parameter, its slot loaded after the `moveq'
    // (TPAD_$INQUIRE_UNIT 0xE6999C..0xE699A2); `.w': `mulu.w'.
    assert_eq!(
        body("function t(in u: integer): integer;\nbegin\n  t := cfgs[u].z;\nend;\n"),
        [
            "pea\t(%a5)",
            "lea\t(.Ldata).l,%a5",
            "moveq\t#0x2c,%d0",
            "movea.l\t(8,%fp),%a0",
            "mulu.w\t(%a0),%d0",
            "move.w\t(-2,%a5,%d0.w),%d0",
        ]
    );
    // A unit variable (TPAD_$DATA 0xE691CA).
    assert_eq!(
        body("function t: integer;\nbegin\n  t := cfgs[k].a;\nend;\n")[2..],
        [
            "moveq\t#0x2c,%d0",
            "mulu.w\t(0xf50,%a5),%d0",
            "move.w\t(-0x2c,%a5,%d0.w),%d0",
        ]
    );
    // A by-value parameter, loaded first (module-local 0xE383B0 at
    // 0xE383C8 `move.w (12,A6),D2; moveq #100,D0; mulu.w D2,D0'); ×84,
    // odd part 21.
    assert_eq!(
        body("function t(i: integer): integer;\n  options(val_param);\nbegin\n  t := c84s[i].a;\nend;\n")[2..],
        [
            "move.w\t(8,%fp),%d0",
            "moveq\t#0x54,%d1",
            "mulu.w\t%d0,%d1",
            "lea\t(0,%a5,%d1.w),%a0",
            "move.w\t(0x160,%a0),%d0",
        ]
    );
    // ×336 (odd part 21): `mulu.w #c' on the loaded value.
    assert_eq!(
        body("function t: integer;\nbegin\n  t := c336s[k].a;\nend;\n")[2..4],
        ["move.w\t(0xf50,%a5),%d0", "mulu.w\t#0x150,%d0"]
    );
    // A computed index (a zero-extended byte): in place (RIP_$BROADCAST
    // 0xE872F8 `mulu.w #92,D5').
    assert_eq!(
        body("function t(c: char): integer;\n  options(val_param);\nbegin\n  t := cs[ord(c)].a;\nend;\n")
            .iter()
            .filter(|l| l.starts_with("mul") || l.starts_with("moveq"))
            .collect::<Vec<_>>(),
        ["mulu.w\t#0x2c,%d0"]
    );
    // Word and longword products of a signed word: `muls.w' (0xE458D2;
    // TPAD_$DATA 0xE691D0).
    assert_eq!(
        body("function t: integer;\nbegin\n  t := k * 44;\nend;\n")[2..],
        ["moveq\t#0x2c,%d0", "muls.w\t(0xf50,%a5),%d0"]
    );
    assert_eq!(
        body("function t: integer32;\nbegin\n  t := k * 44;\nend;\n")[2..],
        ["moveq\t#0x2c,%d0", "muls.w\t(0xf50,%a5),%d0"]
    );
    // Refused: an unsigned word product (no site); an index read from an
    // array element (no site); ×26 (odd part 13, done bit by bit at
    // 0xE71BBA, bead domain-pascal-irb4).
    assert!(
        error("function t: integer;\nbegin\n  t := pk * 44;\nend;\n")
            .contains("a word product of a `0..65535` variable")
    );
    assert!(
        error("function t: integer;\nbegin\n  t := cfgs[ks[k]].a;\nend;\n")
            .contains("another unit's variable, an array element")
    );
    assert!(
        error("function t: integer;\nbegin\n  t := c26s[k].a;\nend;\n")
            .contains("odd parts of three bits")
    );
    // Refused: a computed value other than a zero-extended byte (a mask
    // has the constant first, HINT_$GET_HINTS 0xE4998E; bead
    // domain-pascal-q322), as an index or a factor; a longword product of
    // an unsigned word (the `.l` indices are all `muls.w`).
    for routine in [
        "function t: integer;\nbegin\n  t := cfgs[k + 1].a;\nend;\n",
        "function t: integer;\nbegin\n  t := c84s[k & 7].a;\nend;\n",
        "function t: integer;\nbegin\n  t := (k + 1) * 44;\nend;\n",
    ] {
        assert!(error(routine).contains("of a computed value"), "{routine}");
    }
    assert!(
        error("function t: integer32;\nbegin\n  t := pk * 44;\nend;\n")
            .contains("a longword product of a `0..65535` variable")
    );
}

/// An element of another unit's array (codegen-patterns 5.2, "Another
/// unit's array"): `movea.l #sym,An`, the lower bound in the displacement,
/// the base after the index's value and before its scaling (module-local
/// 0xE4649C `move.w (8,A6),D0; movea.l #PROC1_$TYPE,A0; move.w D0,D1;
/// add.w D1,D1; cmpi.w #2,(-2,A0,D1.w)`); after a scaling with `neg`,
/// moved above its last `add` by the scheduler (0xE1AA5A).
#[test]
fn other_units_array_elements() {
    assert_eq!(
        body("function t: integer32; internal;\nbegin\n  t := xs[w];\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "movea.l\t#XS,%a0",
            "add.w\t%d0,%d0",
            "move.w\t(-2,%a0,%d0.w),%d0",
            "ext.l\t%d0",
        ]
    );
    // ×12 (TTY_$I_WAIT 0xE1C24A for ×4, 0xE1C22E for ×12).
    assert_eq!(
        body("function t: integer32; internal;\nbegin\n  t := xrs[w].a;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "movea.l\t#XRS,%a0",
            "lsl.w\t#2,%d0",
            "move.w\t%d0,%d1",
            "add.w\t%d1,%d1",
            "add.w\t%d1,%d0",
            "move.l\t(-0xc,%a0,%d0.w),%d0",
        ]
    );
    // ×56: the base generated after the scaling (0xE1AA50..0xE1AA62).
    assert_eq!(
        body("function t: integer32; internal;\nbegin\n  t := xbs[w].a;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "lsl.w\t#3,%d0",
            "move.w\t%d0,%d1",
            "neg.w\t%d0",
            "lsl.w\t#3,%d1",
            "movea.l\t#XBS,%a0",
            "add.w\t%d1,%d0",
            "move.l\t(-0x38,%a0,%d0.w),%d0",
        ]
    );
    // Refused: in a loop (loaded in the loop or held from before it,
    // TTY_$K_DRAIN_OUTPUT 0xE67AF0); its address as a value read once
    // (`lea` into the base's register or another, bead domain-pascal-1ibr;
    // [`element_addresses_as_values`]).
    let e = error("procedure t; internal;\nbegin\n  while w > 0 do l := xs[w];\nend;\n");
    assert!(e.contains("another unit's array in a loop"), "{e}");
    let e = error("procedure t; internal;\nbegin\n  pp := addr(xs[w]);\nend;\n");
    assert!(e.contains("read once and not across a call"), "{e}");
    // Refused, review of bead domain-pascal-rlnm: the same array in a
    // later statement with no call between (the base kept, MMAP_$FREE_
    // PAGES 0xE0CF74); an index read from the same variable (SIO_$INIT
    // 0xE32CA2); a displacement beyond a byte (`lea` into the base's
    // register or another); a store whose value needs an address register
    // (TERM_$CONTROL 0xE66B4C); the first argument pushed after a pad
    // (NAME_$FREE_ASID 0xE74DDE).
    let e = error("procedure t; internal;\nbegin\n  l := xs[w];\n  l := xs[u];\nend;\n");
    assert!(
        e.contains("an earlier statement since the last call"),
        "{e}"
    );
    let e = error("procedure t; internal;\nbegin\n  if xsr.a[xsr.n] = 0 then l := 0;\nend;\n");
    assert!(e.contains("read from the same variable"), "{e}");
    let e = error("procedure t; internal;\nbegin\n  xsr.n := xsr.a[w];\nend;\n");
    assert!(e.contains("another part of the same variable"), "{e}");
    let e = error("procedure t; internal;\nbegin\n  l := xh[w];\nend;\n");
    assert!(e.contains("displacement beyond a byte"), "{e}");
    let e = error("procedure t; internal;\nbegin\n  xs[w] := pb^.n;\nend;\n");
    assert!(e.contains("whose value needs a register set up"), "{e}");
    let e = error("procedure t; internal;\nbegin\n  g(w, xs[u]);\nend;\n");
    assert!(e.contains("a call with a pad"), "{e}");
    // A call between: the base loaded again (27 reloads after a call).
    assert_eq!(
        body("procedure t; internal;\nbegin\n  l := xs[w];\n  w := f(w);\n  l := xs[u];\nend;\n")
            .iter()
            .filter(|i| i.starts_with("movea.l\t#XS,"))
            .count(),
        2
    );
}

/// The address of an element of another unit's array as a value
/// (codegen-patterns 5.2, "The element's address as a value"; bead
/// domain-pascal-o1kl): the index's value, the base, the scaling, then
/// `lea (d,An,Dx),Ar`, `Ar` the allocator's.  A `with` record read twice
/// takes the dying base's register (XPD_$FIND_INDEX 0xE5AF56..0xE5AF60
/// `movea.l #0xEA551C,A0; muls.w #228,D1; lea (0,A0,D1.l),A0`); one live
/// across a call a callee-saved register, the base another (MMAP_$AVAIL
/// 0xE0CC76..0xE0CC80 `movea.l #MMAP,A0; move.l D2,D0; lsl.l #4,D0; lea
/// (0,A0,D0.l),A2`; the pad of the call's arguments scheduled into the
/// address as at XPD_$SET_DEBUGGER 0xE5BC98..0xE5BCA4 `subq.l #2,SP; add.w
/// D2,D1; lea (0,A0,D1.w),A3; move.w #2,-(SP); jsr ML_$LOCK`).  Read once
/// and not across a call: refused (98 sites take the base's register, 99
/// another; bead domain-pascal-1ibr).
#[test]
fn element_addresses_as_values() {
    assert_eq!(
        body("procedure t; internal;\nbegin\n  with xrs[w] do begin\n    a := l;\n    c := b;\n  end;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "movea.l\t#XRS,%a0",
            "lsl.w\t#2,%d0",
            "move.w\t%d0,%d1",
            "add.w\t%d1,%d1",
            "add.w\t%d1,%d0",
            "lea\t(0,%a0,%d0.w),%a0",
            "move.l\t(L).l,(-0xc,%a0)",
            "move.l\t(-8,%a0),(-4,%a0)",
        ]
    );
    assert_eq!(
        body("procedure t; internal;\nbegin\n  with xws[w] do begin\n    g(w, a);\n    b := 0;\n  end;\nend;\n"),
        [
            "pea\t(%a2)",
            "move.w\t(W).l,%d0",
            "movea.l\t#XWS,%a0",
            "lsl.w\t#2,%d0",
            "subq.l\t#2,%sp",
            "lea\t(0,%a0,%d0.w),%a2",
            "pea\t(-4,%a2)",
            "move.w\t(W).l,-(%sp)",
            "jsr\t(G).l",
            "clr.w\t(-2,%a2)",
            // Below the reserved slot and the `with` temporary's (bead
            // domain-pascal-s9g0): `link #-8`.
            "movea.l\t(-0xc,%fp),%a2",
        ]
    );
    // Read twice, the last read a call's argument: held through the call
    // in a callee-saved register (12.4 R4(c); MMAP_$FREE_LIST 0xE0CB46..
    // 0xE0CB62 `lea (0,A0,D0.l),A2; move.w (-8182,A2),D2; ...; pea
    // (-8192,A2); bsr.w`).
    assert_eq!(
        body("procedure t; internal;\nbegin\n  with xws[w] do begin\n    w := b;\n    g(w, a);\n  end;\nend;\n"),
        [
            "pea\t(%a2)",
            "move.w\t(W).l,%d0",
            "movea.l\t#XWS,%a0",
            "lsl.w\t#2,%d0",
            "lea\t(0,%a0,%d0.w),%a2",
            "move.w\t(-2,%a2),(W).l",
            "subq.l\t#2,%sp",
            "pea\t(-4,%a2)",
            "move.w\t(W).l,-(%sp)",
            "jsr\t(G).l",
            "movea.l\t(-0xc,%fp),%a2",
        ]
    );
    // Read once: a `with` record used once, an address stored (SIO6509_
    // $INIT 0xE33548 into the base's register, the module-local routine
    // at 0xE1C204 at 0xE1C242 into another), a byte element through a
    // loaded pointer stored.
    for stmt in [
        "with xrs[w] do b := 0;",
        "pp := addr(xrs[w]);",
        "pp := addr(cp^[w]);",
    ] {
        let e = error(&format!("procedure t; internal;\nbegin\n  {stmt}\nend;\n"));
        assert!(e.contains("read once and not across a call"), "{stmt}: {e}");
    }
    // Wider elements through a loaded pointer stay refused (bead
    // domain-pascal-nqg1).
    let e = error("procedure t; internal;\nbegin\n  pp := addr(ip^[w]);\nend;\n");
    assert!(e.contains("more than 1 byte"), "{e}");
}

/// Elements through a pointer loaded from memory (codegen-patterns 5.2,
/// "Arrays through a pointer"): 1 byte, the index then the pointer
/// (KBD_$FETCH_KEY 0xE1CB8A); wider elements and an index read through
/// the same pointer refused.
#[test]
fn elements_through_loaded_pointers() {
    assert_eq!(
        body("function t: char; internal;\nbegin\n  t := cp^[w];\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "movea.l\t(CP).l,%a0",
            "move.b\t(-1,%a0,%d0.w),%d0",
        ]
    );
    let e = error("function t: integer; internal;\nbegin\n  t := ip^[w];\nend;\n");
    assert!(e.contains("more than 1 byte"), "{e}");
    let e = error("function t: char; internal;\nbegin\n  t := pb^.cs^[pb^.n];\nend;\n");
    assert!(e.contains("its index is read through too"), "{e}");
    // A store whose value needs an address register (0xE6AA1C, the
    // value's pointer first): refused (review of bead domain-pascal-rlnm).
    let e = error("procedure t; internal;\nbegin\n  cp^[w] := pb^.cs^[1];\nend;\n");
    assert!(e.contains("whose value needs a register set up"), "{e}");
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
    // A word times a constant that is not done with shifts: `muls.w`, its
    // 32-bit product (PROC1_$LOADAV_CALLBACK 0xE14C14..0xE14C1C).
    assert_eq!(
        body("procedure t; internal;\nbegin\n  l := w * 564;\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "muls.w\t#0x234,%d0",
            "move.l\t%d0,(L).l",
        ]
    );
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

/// codegen-patterns 8: products of words used as longwords are one
/// `muls.w`/`mulu.w` (bead domain-pascal-1x1): PROC1_$LOADAV_CALLBACK
/// 0xE14C14 `muls.w #5240`; 0xE10A7E `move.w D2,D4; muls.w (-866,A2),D4`;
/// SIO_$K_TIMED_BREAK 0xE67F28 `mulu.w #250` of a `0..65535` word.
#[test]
fn word_products_used_as_longwords() {
    let f = |e: &str| {
        body(&format!(
            "function t: integer32; internal;\nbegin\n  t := {e};\nend;\n"
        ))
    };
    assert_eq!(f("w * 5240"), ["move.w\t(W).l,%d0", "muls.w\t#0x1478,%d0"]);
    assert_eq!(f("w * w"), ["move.w\t(W).l,%d0", "muls.w\t(W).l,%d0"]);
    assert_eq!(f("u * 250"), ["move.w\t(U).l,%d0", "mulu.w\t#0xfa,%d0"]);
    // A constant done with shifts: extended first (0xE07442, x12).
    assert_eq!(f("w * 12")[..2], ["move.w\t(W).l,%d0", "ext.l\t%d0"]);
    // A word times a word for a word: `mul<s>.w` (0xE30A9A).
    assert_eq!(
        body("function t: integer; internal;\nbegin\n  t := w * w;\nend;\n"),
        ["move.w\t(W).l,%d0", "muls.w\t(W).l,%d0"]
    );
}

/// codegen-patterns 8: M$ARITH calls (bead domain-pascal-5aw.5).  The pad
/// before a word argument, argument 2 pushed first, `addq.w #8,SP` unless
/// the procedure-call rule skips it, the result in D0.
#[test]
fn marith_calls() {
    let f = |e: &str| {
        body(&format!(
            "function t: integer32; internal;\nbegin\n  t := {e};\nend;\n"
        ))
    };
    // M$MIS$LLL, a constant beyond a word as argument 2 (0xE14BFA); the
    // last call of the routine is not popped (0xE16AD8).
    assert_eq!(
        f("l * 60296"),
        [
            "move.l\t#0xeb88,-(%sp)",
            "move.l\t(L).l,-(%sp)",
            "jsr\t(M$MIS$LLL).l",
        ]
    );
    // M$MIS$LLW: the word is argument 2 whichever side it is on.
    for e in ["l * w", "w * l"] {
        assert_eq!(
            f(e),
            [
                "subq.l\t#2,%sp",
                "move.w\t(W).l,-(%sp)",
                "move.l\t(L).l,-(%sp)",
                "jsr\t(M$MIS$LLW).l",
            ],
            "{e}"
        );
    }
    assert_eq!(f("l * 564")[3], "jsr\t(M$MIS$LLW).l");
    assert_eq!(f("l div 12")[3], "jsr\t(M$DIS$LLW).l");
    assert_eq!(f("l div l")[2], "jsr\t(M$DIS$LLL).l");
    assert_eq!(f("l mod l")[2], "jsr\t(M$OIS$LLL).l");
    // An M$ARITH result as argument 1 of another: the inner call popped,
    // then the outer pad (0xE1695C..0xE16970).
    assert_eq!(
        f("(l div w) * w"),
        [
            "subq.l\t#2,%sp",
            "move.w\t(W).l,-(%sp)",
            "move.l\t(L).l,-(%sp)",
            "jsr\t(M$DIS$LLW).l",
            "addq.w\t#8,%sp",
            "subq.l\t#2,%sp",
            "move.w\t(W).l,-(%sp)",
            "move.l\t%d0,-(%sp)",
            "jsr\t(M$MIS$LLW).l",
        ]
    );
    // The same operand twice: copied on the stack (OS_$SHUTDOWN
    // 0xE6D604); the result modified in place in D0 (0xE39C3E `add.l
    // (14,A6),D0`).
    assert_eq!(
        f("l * l + l"),
        [
            "move.l\t(L).l,-(%sp)",
            "move.l\t(%sp),-(%sp)",
            "jsr\t(M$MIS$LLL).l",
            "add.l\t(L).l,%d0",
        ]
    );
    // Popped when a statement with pushes follows.
    let b = body("procedure t; internal;\nbegin\n  l := l div l;\n  l := l * l;\nend;\n");
    assert_eq!(b[3], "addq.w\t#8,%sp");
    assert_eq!(b.last().unwrap(), "move.l\t%d0,(L).l");
}

/// codegen-patterns 12.4 R3: an M$ARITH call writes only D0 and A0, so a
/// value lives in D1 across it (31 of 83 calls in the survey; 0xE3CB8E
/// A1).
#[test]
fn values_live_in_d1_across_marith() {
    let b = body(
        "procedure t(x: integer32; y: integer); internal;\nbegin\n  l := x div 1000;\n  w := y;\nend;\n",
    );
    assert_eq!(
        b,
        [
            "move.l\t(8,%fp),%d0",
            "move.w\t(0xc,%fp),%d1",
            "subq.l\t#2,%sp",
            "move.w\t#0x3e8,-(%sp)",
            "move.l\t%d0,-(%sp)",
            "jsr\t(M$DIS$LLW).l",
            "move.l\t%d0,(L).l",
            "move.w\t%d1,(W).l",
        ]
    );
}

/// codegen-patterns 8: division and mod forms.
#[test]
fn division_forms() {
    let fw = |e: &str| {
        body(&format!(
            "function t: integer; internal;\nbegin\n  t := {e};\nend;\n"
        ))
    };
    let sw = |e: &str| {
        body(&format!(
            "procedure t; internal;\nbegin\n  w := {e};\nend;\n"
        ))
    };
    let fl = |e: &str| {
        body(&format!(
            "function t: integer32; internal;\nbegin\n  t := {e};\nend;\n"
        ))
    };
    // integer16 by a variable (TPAD_$INIT 0xE335F4).
    assert_eq!(
        fw("w div w"),
        ["move.w\t(W).l,%d0", "ext.l\t%d0", "divs.w\t(W).l,%d0"]
    );
    // Signed by a power of two: bias and `asr` (CAL_$WEEKDAY 0xE68698;
    // PROC1_$LOADAV_CALLBACK 0xE14BEE).
    assert_eq!(
        fw("w div 4"),
        [
            "move.w\t(W).l,%d0",
            "bpl.s\t1f",
            "addq.w\t#3,%d0",
            "1:",
            "asr.w\t#2,%d0"
        ]
    );
    assert_eq!(
        fl("l div 1024"),
        [
            "move.l\t(L).l,%d0",
            "bpl.s\t1f",
            "addi.l\t#0x3ff,%d0",
            "1:",
            "asr.l\t#8,%d0",
            "asr.l\t#2,%d0"
        ]
    );
    // Unsigned by a power of two: `lsr.w` (LOG_$READ's worker 0xE177D8;
    // no `divu.w #2^k` in compiled code), the word result zero-extended
    // as below.
    assert_eq!(
        fl("u div 4"),
        [
            "move.w\t(U).l,%d0",
            "clr.l\t%d1",
            "lsr.w\t#2,%d0",
            "move.w\t%d0,%d1",
            "move.l\t%d1,%d0",
        ]
    );
    // `mod` by a power of two is a division (CAL_$DECODE_TIME 0xE68800).
    assert_eq!(fw("w mod 4")[2], "divs.w\t#4,%d0");
    // Unsigned words: zero-extended, `divu.w` (0xE4CE1A..0xE4CE22); used as
    // a longword, the word result zero-extended into a new register
    // (0xE382CC), whose `clr.l` the scheduler moves into the stall after
    // the load (0xE382BE `clr.l D2` before the `divu.w`; codegen-patterns
    // 12.4 R11).
    assert_eq!(
        fl("u mod 101"),
        [
            "clr.l\t%d0",
            "move.w\t(U).l,%d0",
            "clr.l\t%d1",
            "divu.w\t#0x65,%d0",
            "swap\t%d0",
            "move.w\t%d0,%d1",
            "move.l\t%d1,%d0",
        ]
    );
    // A longword quotient used as a word: `divs.w` in place (0xE10CAE).
    assert_eq!(
        sw("l div 250"),
        [
            "move.l\t(L).l,%d0",
            "divs.w\t#0xfa,%d0",
            "move.w\t%d0,(W).l"
        ]
    );
    // A longword remainder by a word: `M$OIS$WLW`, a word result (0xE30D3E).
    assert_eq!(
        sw("l mod 32"),
        [
            "subq.l\t#2,%sp",
            "move.w\t#0x20,-(%sp)",
            "move.l\t(L).l,-(%sp)",
            "jsr\t(M$OIS$WLW).l",
            "move.w\t%d0,(W).l",
        ]
    );
    // A word difference divided in 32 bits (RIP_$SERVER 0xE68AE2).
    assert_eq!(
        fw("(w - 2) div 6"),
        [
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "subq.l\t#2,%d0",
            "divs.w\t#6,%d0"
        ]
    );
    // A word product divided (TPAD_$DATA 0xE69392..0xE6939A).
    assert_eq!(
        fw("(w * w) div 2200"),
        [
            "move.w\t(W).l,%d0",
            "muls.w\t(W).l,%d0",
            "divs.w\t#0x898,%d0"
        ]
    );
}

/// A word `div`/`mod` narrowed to a byte (`chr`) is the word operation,
/// its low byte used; only a longword one goes through the narrowing of
/// codegen-patterns 8 (`div<s>.w` in place, `M$OI*$WLW`).
#[test]
fn word_div_mod_narrowed_to_a_byte() {
    assert_eq!(
        body("function t: char; internal;\nbegin\n  t := chr(w mod 10);\nend;\n"),
        [
            "move.w\t(W).l,%d0",
            "ext.l\t%d0",
            "divs.w\t#0xa,%d0",
            "swap\t%d0",
        ]
    );
    let e = error("function t: char; internal;\nbegin\n  t := chr(l mod 10);\nend;\n");
    assert!(e.contains("a longword `div`/`mod` used as a byte"), "{e}");
}
