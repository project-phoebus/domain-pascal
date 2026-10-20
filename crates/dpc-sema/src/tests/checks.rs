//! Name resolution, type checking, storage and diagnostics.

use super::*;
use crate::consts::ConstValue;
use crate::program::{ExprKind, InitDatum, InitStore, StaticAddr, StaticBase, StmtKind, Storage};

// ---------------------------------------------------------------------
// Diagnostics carry positions
// ---------------------------------------------------------------------

#[test]
fn undefined_identifier_has_position() {
    let e = errs("module m;\nprocedure p;\nbegin\n  x := 1;\nend;\n");
    assert_eq!(e, vec!["t.pas:4:3: error: undefined identifier `x`"]);
}

#[test]
fn undefined_type_has_position() {
    let e = errs("module m;\nvar v: nosuch;\n");
    assert_eq!(e, vec!["t.pas:2:8: error: undefined type `nosuch`"]);
}

#[test]
fn several_errors_reported() {
    let e = errs("module m;\nprocedure p;\nbegin\n  a := 1;\n  b := 2;\nend;\n");
    assert_eq!(e.len(), 2, "{e:#?}");
}

#[test]
fn type_mismatch_in_assignment() {
    assert_err(
        "module m; var b: boolean; procedure p; begin b := 1 end;",
        "not compatible with the assignment variable",
    );
    assert_err(
        "module m; var i: integer; procedure p; begin i := 'x' end;",
        "not compatible",
    );
}

#[test]
fn constant_out_of_range() {
    assert_err(
        "module m; var v: integer; procedure p; begin v := 40000 end;",
        "out of range",
    );
    // Only integer subranges take an out-of-range constant (below); a
    // char or enumeration subrange does not.
    assert_err(
        "module m; type t = 'a'..'f'; var v: t; procedure p; begin v := 'z' end;",
        "out of range",
    );
    assert_err(
        "module m; type c = (r, g, b, y); t = r..g; var v: t; procedure p; begin v := y end;",
        "out of range",
    );
}

/// Ref Appendix D, ISO 6.4.6, p. D-3: "You may make an assignment from an
/// integer expression that includes a value outside one of the variables'
/// declared subranges.  Also, you may pass an integer argument that is
/// outside the corresponding parameter's declared subrange."
/// Clarification Q38: accepted quietly, the low bytes kept.  Apollo's
/// dmwin.pas: `connv[ios_$stderr] := ios_$no_stream;` (16#7FFF into an
/// array of `ios_$id_t` = 0..127).
#[test]
fn out_of_range_constant_into_an_integer_subrange() {
    let p = ok("module m;
        const ios_$no_stream = 16#7FFF;
        type ios_$id_t = 0..127; t = 0..9;
        var connv: array[0..2] of ios_$id_t; v: t; w: t := 70000;
        procedure q(x: t); extern;
        procedure r(x: t); val_param; extern;
        procedure p;
        begin connv[2] := ios_$no_stream; v := 10; v := -1; q(12); r(40000) end;");
    // A static initial value keeps its value; the data emitter keeps the
    // low bytes of the 2-byte object.
    let w = var(&p, "w").init.clone().unwrap();
    assert_eq!(w[0].value, ConstValue::Int(70000));
}

#[test]
fn duplicate_declaration() {
    assert_err("module m; var a: integer; a: char;", "already declared");
}

// ---------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------

#[test]
fn constant_folding() {
    let p = module(
        "const a = 16#10; b = a * 2 + 1; c = -b; d = 8#7 ! 16#30;
               big = 70000; neg = 16#FFFFFFFF; ch = chr(65); s = 'abc';
         var v1: integer := b; v2: integer := c; v3: integer := d;
             v4: integer32 := big; v5: integer32 := neg; v6: char := ch;
             v7: array[1..4] of char := s;",
    );
    let init = |n: &str| var(&p, n).init.clone().unwrap();
    let v = |n: &str| init(n)[0].value.clone();
    assert_eq!(v("v1"), ConstValue::Int(33));
    assert_eq!(v("v2"), ConstValue::Int(-33));
    assert_eq!(v("v3"), ConstValue::Int(0x37));
    assert_eq!(v("v4"), ConstValue::Int(70000));
    // ref 3.4 NOTE p. 3-10: "the compiler treats them as negative numbers
    // (for example, 16#FFFFFFFF)".
    assert_eq!(v("v5"), ConstValue::Int(-1));
    assert_eq!(v("v6"), ConstValue::Char(b'A'));
    assert_eq!(v("v7"), ConstValue::Str(b"abc".to_vec()));
    // maxint (ref 3.2.3, p. 3-5).
    let p = module("const m = maxint; var x: integer := m;");
    assert_eq!(
        var(&p, "x").init.as_ref().unwrap()[0].value,
        ConstValue::Int(32767)
    );
}

#[test]
fn enumeration_constants() {
    let p = module(
        "type pets = (cats, dogs, dolphins);
         var x: pets := dolphins;
             s: set of pets := [cats, dolphins];",
    );
    let pets = ty(&p, "pets");
    assert_eq!(
        var(&p, "x").init.as_ref().unwrap()[0].value,
        ConstValue::Enum { ty: pets, ord: 2 }
    );
    assert_eq!(
        var(&p, "s").init.as_ref().unwrap()[0].value,
        ConstValue::Set(vec![0, 2])
    );
}

#[test]
fn initialisers_flatten_to_stores() {
    let p = module(
        "type r = record a: char; b: integer32 end;
         var arr: array[1..4] of integer := [1, 2 of 7, 4 := 9];
             rec: r := [b := 5, a := 'x'];
             flags: set of 0..15 := [0, 3..5];",
    );
    let i16 = p.types.builtins.integer16;
    assert_eq!(
        var(&p, "arr").init.as_ref().unwrap(),
        &vec![
            InitStore {
                offset: 0,
                ty: i16,
                value: ConstValue::Int(1).into(),
                bits: None
            },
            InitStore {
                offset: 2,
                ty: i16,
                value: ConstValue::Int(7).into(),
                bits: None
            },
            InitStore {
                offset: 4,
                ty: i16,
                value: ConstValue::Int(7).into(),
                bits: None
            },
            InitStore {
                offset: 6,
                ty: i16,
                value: ConstValue::Int(9).into(),
                bits: None
            },
        ]
    );
    let rec = var(&p, "rec").init.clone().unwrap();
    assert_eq!(
        (rec[0].offset, rec[0].value.clone()),
        (2, ConstValue::Int(5).into())
    );
    assert_eq!(
        (rec[1].offset, rec[1].value.clone()),
        (0, ConstValue::Char(b'x').into())
    );
    assert_eq!(
        var(&p, "flags").init.as_ref().unwrap()[0].value,
        ConstValue::Set(vec![0, 3, 4, 5])
    );
}

/// Owner's ruling on 3.13.4, p. 3-51 (docs/reference-clarifications.md
/// "3.13.4 addr(...) as an initial value"): `addr(...)` of an object with
/// a static address is an initial value; Apollo's hardcopy
/// driver_init_ex.pas has `init := addr(my_driver_$init),`.
#[test]
fn addr_initial_values() {
    let p = module(
        "type pr = ^procedure(in x: integer32);
              epv = record init: pr; mon: pr; data: ^integer32 end;
         procedure q(in x: integer32); extern;
         procedure d(in x: integer32); forward;
         var c: integer32; t: array[1..3] of integer32; o: extern integer32;
             e: extern epv;
             v: epv := [init := addr(q), mon := nil, data := addr(c)];
             pt: ^integer32 := addr(t[2]);
             po: ^integer32 := addr(o);
             pe: univ_ptr := addr(e.data);
             dd: extern ^integer32;
         define dd := addr(t[3]);
         procedure d; var s: static ^integer32 := addr(c); begin end;",
    );
    let q = p.find_routine("q").unwrap();
    let vid = |n: &str| p.find_var(n).unwrap();
    let at = |base, offset| InitDatum::Addr(StaticAddr { base, offset });
    let v = var(&p, "v").init.clone().unwrap();
    assert_eq!(
        v.iter()
            .map(|s| (s.offset, s.value.clone()))
            .collect::<Vec<_>>(),
        vec![
            (0, at(StaticBase::Routine(q), 0)),
            (4, ConstValue::Nil.into()),
            (8, at(StaticBase::Var(vid("c")), 0)),
        ]
    );
    let one = |n: &str| var(&p, n).init.as_ref().unwrap()[0].value.clone();
    assert_eq!(one("pt"), at(StaticBase::Var(vid("t")), 4));
    assert_eq!(one("po"), at(StaticBase::Var(vid("o")), 0));
    assert_eq!(one("pe"), at(StaticBase::Var(vid("e")), 8));
    assert_eq!(one("dd"), at(StaticBase::Var(vid("t")), 8));
    assert_eq!(one("s"), at(StaticBase::Var(vid("c")), 0));

    // A routine's ordinary variable or parameter has no static address.
    assert_err(
        "module m; procedure p(x: integer32);
         var l: integer32; s: static ^integer32 := addr(l); begin end;",
        "`addr` of `l` is not a valid initial value: a routine's variable or parameter has no \
         static address",
    );
    assert_err(
        "module m; procedure p(x: integer32);
         var s: static ^integer32 := addr(x); begin end;",
        "`addr` of `x` is not a valid initial value",
    );
    // Nor what a pointer points to, nor an element at a variable index.
    assert_err(
        "module m; var p: ^integer32; q: ^integer32 := addr(p^);",
        "`addr` of an object with no static address is not a valid initial value",
    );
    assert_err(
        "module m; var i: integer; t: array[1..3] of integer32; q: ^integer32 := addr(t[i]);",
        "`addr` of an element with a variable index is not a valid initial value",
    );
    // Type checking as for an assignment: the routine pointer template
    // (ref p. 4-146), and only pointers take an address.
    assert_err(
        "module m; type pr = ^procedure(in x: integer32);
         procedure q(in x: integer); extern; var v: pr := addr(q);",
        "does not match the routine pointer type",
    );
    assert_err(
        "module m; var c: integer32; i: integer32 := addr(c);",
        "not compatible with the assignment variable (integer32 := univ_ptr)",
    );
    // ref 3.13.3 p. 3-50: no address of an internal or nested routine.
    assert_err(
        "module m; type pr = ^procedure; procedure q; internal; begin end;
         var v: pr := addr(q);",
        "the address of an internal routine cannot be taken",
    );
    // A packed record's field that is a whole byte or longword has an
    // address (ref 3.10.7), as `addr` of one in a statement; a bit field
    // has none (one error, `addr`'s).
    let p = module(
        "type pk = packed record b: 0..7; c: char; d: integer32; end;
         var r: pk; pc: univ_ptr := addr(r.c); pd: univ_ptr := addr(r.d);",
    );
    let one = |n: &str| var(&p, n).init.as_ref().unwrap()[0].value.clone();
    let r = p.find_var("r").unwrap();
    assert_eq!(one("pc"), at(StaticBase::Var(r), 1));
    assert_eq!(one("pd"), at(StaticBase::Var(r), 2));
    let e = errs(
        "module m; type pk = packed record b: 0..7; c: char; end;
         var r: pk; pb: univ_ptr := addr(r.b);",
    );
    assert_eq!(e.len(), 1, "{e:#?}");
    assert!(
        e[0].contains("has no address (ref 3.10.3, 3.10.7)"),
        "{e:#?}"
    );
    // A variable an initial value names is not a reference by the
    // routine's body (its data section need not be the routine's).
    let p = module(
        "var c: integer32; d: integer32;
         procedure q; var s: static univ_ptr := addr(c); begin d := 1 end;",
    );
    let refs: Vec<_> = routine(&p, "q").static_refs.iter().map(|r| r.0).collect();
    assert_eq!(refs, vec![p.find_var("d").unwrap()]);
    // Other non-constant values are still refused, with one message.
    let e = errs("module m; var c: integer32; d: integer32 := c;");
    assert_eq!(e.len(), 1, "{e:#?}");
    assert!(e[0].contains("initial value must be a constant"), "{e:#?}");
}

/// Owner's ruling (docs/reference-clarifications.md "A variable's own
/// address in its initializer"; bead domain-pascal-i5mf): a variable is
/// in scope in its own initial value, so `addr(itself)` is its own
/// address; a variable declared later in the var part is not.  ../source
/// network/network.pas: `network_$rqst_done_ec: ec_$eventcount_t :=
/// [value := 0, waiter_list_head := addr(network_$rqst_done_ec), ...]`
/// (the image's NETWORK D segment at +0x258: `0000 0000 00e2 4b54
/// 00e2 4b54`).
#[test]
fn own_address_initial_value() {
    let p = module(
        "type ec = record value: integer32; head: univ_ptr; tail: univ_ptr end;
         var a: ec := [value := 0, head := addr(a), tail := addr(a.tail)];
             b: static ec := [value := 0, head := addr(b), tail := addr(b)];
             c: define univ_ptr := addr(c);
             m1, m2: univ_ptr := addr(m1);
             x: extern univ_ptr;
         var (ov) o: ec := [value := 0, head := addr(o), tail := addr(o)];
         define d;
         var d: extern univ_ptr := addr(d);
         procedure q;
         var s: static ec := [value := 0, head := addr(s), tail := addr(s)];
         begin x := nil end;",
    );
    let vid = |n: &str| p.find_var(n).unwrap();
    let at = |n: &str, offset| {
        InitDatum::Addr(StaticAddr {
            base: StaticBase::Var(vid(n)),
            offset,
        })
    };
    let stores = |n: &str| {
        var(&p, n)
            .init
            .as_ref()
            .unwrap()
            .iter()
            .map(|s| (s.offset, s.value.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        stores("a"),
        vec![
            (0, ConstValue::Int(0).into()),
            (4, at("a", 0)),
            (8, at("a", 8))
        ]
    );
    for n in ["b", "o", "s"] {
        assert_eq!(
            stores(n),
            vec![(0, ConstValue::Int(0).into()), (4, at(n, 0)), (8, at(n, 0))],
            "{n}"
        );
    }
    assert_eq!(stores("c"), vec![(0, at("c", 0))]);
    assert_eq!(stores("d"), vec![(0, at("d", 0))]);
    // `m1, m2: T := addr(m1)`: each name its own copy, both m1's address.
    assert_eq!(stores("m1"), vec![(0, at("m1", 0))]);
    assert_eq!(stores("m2"), vec![(0, at("m1", 0))]);
    // A routine's static naming itself is no reference by the body.
    let refs: Vec<_> = routine(&p, "q").static_refs.iter().map(|r| r.0).collect();
    assert_eq!(refs, vec![vid("x")]);

    // A variable declared later in the var part is not yet visible.
    assert_err(
        "module m; var p: univ_ptr := addr(later); later: integer32;",
        "undefined identifier",
    );
    // A routine's ordinary variable still cannot be initialised (one
    // error; its own name is in scope).
    let e = errs("module m; procedure q; var l: univ_ptr := addr(l); begin end;");
    assert_eq!(e.len(), 1, "{e:#?}");
    assert!(
        e[0].contains("can be initialised only if it is static"),
        "{e:#?}"
    );
}

/// Bead domain-pascal-1dn (fuzzer, mutate seed 777): a set initialiser's
/// range outside the set's type is an error, not enumerated member by
/// member (`[0..2147483647]` ran for minutes).
#[test]
fn set_initialiser_out_of_the_sets_range() {
    let e = errs("module m; var s: set of 0..31 := [0..2147483647];");
    assert_eq!(
        e,
        vec!["t.pas:1:35: error: set element out of range 0..31 of the set's type"]
    );
    assert_err(
        "module m; var s: set of 0..31 := [1, 40];",
        "set element out of range 0..31",
    );
    // An empty range is no member at all.
    analyze_src("module m; var s: set of 0..31 := [5..2];").unwrap();
}

#[test]
fn variant_and_packed_record_initialisers() {
    // ref 3.10.4 p. 3-22: a variant's field by name.  status_$t as
    // Apollo's base.ins.pas declares it: `all' is the whole longword.
    let p = module(
        "type st = packed record case integer of
                 1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
                 2: (all: integer32);
               end;
             v = record n: integer; case integer of 1: (i: integer32); 2: (c: char) end;
         var s: st := [all := 16#B000E];
             x: v := [7, c := 'q'];",
    );
    let s = var(&p, "s").init.clone().unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(
        (s[0].offset, s[0].value.clone()),
        (0, ConstValue::Int(0xB000E).into())
    );
    let x = var(&p, "x").init.clone().unwrap();
    assert_eq!(
        x.iter()
            .map(|st| (st.offset, st.value.clone()))
            .collect::<Vec<_>>(),
        vec![
            (0, ConstValue::Int(7).into()),
            (2, ConstValue::Char(b'q').into())
        ]
    );
    // A set bit field of a packed record (where its elements lie is not
    // stated, ref 3.10.7 Table 3-3); a positional value for, or after, a
    // variant's field (not covered by ref 3.10.4).
    assert_err(
        "module m; type st = packed record b: boolean; s: set of 0..3 end;
         var s: st := [s := [1]];",
        "initialising a set field of a packed record that is not whole bytes \
         (where its elements lie is not stated, ref 3.10.7 Table 3-3) is not supported yet",
    );
    assert_err(
        "module m; type st = packed record b: boolean; n: 0..3 end;
         var s: st := [n := [1]];",
        "a packed record's bit field needs a single value",
    );
    assert_err(
        "module m; type v = record n: integer; case integer of 1: (i: integer32) end;
         var x: v := [1, 2];",
        "a positional initial value for a variant's field is not supported yet",
    );
    assert_err(
        "module m; type v = record case integer of 1: (i, j: integer) end;
         var x: v := [i := 1, 2];",
        "a positional initial value after a variant's field is not supported yet",
    );
}

#[test]
fn packed_bit_field_initialisers() {
    // ref 3.10.7 p. 3-35: fields left to right within bytes; a store per
    // bit field, its bits from the byte its first bit is in (KBD's
    // kbd_$state_entry_t, a char and two 4-bit subranges; image WIRED_DATA
    // 0xE2DDE4+0xC0 `FB 20').
    let p = module(
        "type e = packed record key: char; action: 0..15; next: 0..15 end;
              c = (red, green, blue);
              m = packed record f: boolean; h: c; s: -100..40; w: 0..1023 end;
         var t: array[0..1] of e := [[key := chr(16#FB), action := 2, next := 0],
                                     [chr(16#E8), 3, 1]];
             x: m := [true, blue, -100, 2 * 8];",
    );
    let at = |st: &InitStore| {
        (
            st.offset,
            st.bits.map(|b| (b.bit, b.width)),
            st.value.clone(),
        )
    };
    assert_eq!(
        var(&p, "t")
            .init
            .as_ref()
            .unwrap()
            .iter()
            .map(at)
            .collect::<Vec<_>>(),
        vec![
            (0, None, ConstValue::Char(0xFB).into()),
            (1, Some((0, 4)), ConstValue::Int(2).into()),
            (1, Some((4, 4)), ConstValue::Int(0).into()),
            (2, None, ConstValue::Char(0xE8).into()),
            (3, Some((0, 4)), ConstValue::Int(3).into()),
            (3, Some((4, 4)), ConstValue::Int(1).into()),
        ]
    );
    // f bit 0, h bits 1..2, s bits 3..10 (across a byte), w bits 11..20.
    let x = var(&p, "x").init.clone().unwrap();
    assert_eq!(
        x.iter().map(at).map(|(o, b, _)| (o, b)).collect::<Vec<_>>(),
        vec![
            (0, Some((0, 1))),
            (0, Some((1, 2))),
            (0, Some((3, 8))),
            (1, Some((3, 10))),
        ]
    );
    assert_eq!(x[0].value, ConstValue::Bool(true));
    assert_eq!(x[2].value, ConstValue::Int(-100));
    assert_eq!(x[3].value, ConstValue::Int(16));
    // An out-of-range constant for an integer-subrange bit field is
    // accepted (clarification Q38; Appendix D, ISO 6.4.6, p. D-3); the
    // data emitter keeps the field's low bits.
    ok("module m; type e = packed record a: 0..15; b: 0..15 end; var v: e := [a := 16];");
}

#[test]
fn routine_local_initialiser_needs_static() {
    // ref 3.2.2 p. 3-4.
    assert_err(
        "module m; procedure p; var x: integer := 0; begin end;",
        "only if it is static",
    );
    let p = module("procedure p; var x: static integer := 0; begin end;");
    assert!(matches!(
        var(&p, "x").storage,
        Storage::Static { global: false, .. }
    ));
}

// ---------------------------------------------------------------------
// Storage (ref 7.2)
// ---------------------------------------------------------------------

#[test]
fn unit_variable_storage() {
    let p = module(
        "var a: integer;
             b: static integer32;
             c: define char;
             d: extern ^integer;
         var (my_sec) e: integer; f: integer32;",
    );
    let st = |n: &str| var(&p, n).storage.clone();
    let stat = |sym: &str, global: bool, section: &str, offset: u32| Storage::Static {
        symbol: sym.into(),
        global,
        section: section.into(),
        offset,
    };
    assert_eq!(st("a"), stat("A", false, ".data", 0));
    assert_eq!(st("b"), stat("B", false, ".data", 4));
    assert_eq!(st("c"), stat("C", true, ".data", 8));
    assert_eq!(st("d"), Storage::Extern { symbol: "D".into() });
    assert_eq!(st("e"), stat("E", false, "my_sec", 0));
    assert_eq!(st("f"), stat("F", false, "my_sec", 4));
    assert_eq!(
        p.data_sections,
        vec![(".data".to_string(), 9), ("my_sec".to_string(), 8)]
    );
}

/// Overlay sections (ref 3.14, p. 3-52: "Domain Pascal allocates
/// variables defined in a var declaration part sequentially within the
/// specified section.  If more than one var declaration specifies the
/// same section name, the subsequent declarations are considered to be
/// continuations of the first declaration"), laid out as a data section:
/// PCHIST_UNWIRED_D (0xE85718, size 0x938; ../source pchist/pchist.pas):
/// 64 records of 0x14 bytes, 3 longwords at +0x500, a 0x42A-byte record
/// at +0x50C (4 mod 8: no 8-alignment, codegen-patterns 11), a word at
/// +0x936.  A routine's `var (name)` part is static, in that section.
/// An `extern` variable of a `var (name)` part that a `define` statement
/// defines (ref 7.2.2, Method 2) is allocated in that section.
#[test]
fn overlay_section_layout() {
    let p = module(
        "type pr = record a, b, c, d, e: integer32 end;
              hist = record w: array[1..16#215] of integer end;
         var (pchist_unwired_d)
           proc_data: array[1..64] of pr;
           wire_pages: array[1..3] of integer32;
         var x: integer;
         var (pchist_unwired_d)
           histogram: hist;
           wired_count: integer;
         var (sec) e: extern integer; f: char;
         define e;
         procedure p; var (sec) g: integer32; begin g := 0 end;",
    );
    let at = |n: &str| match var(&p, n).storage {
        Storage::Static {
            ref section,
            offset,
            global,
            ..
        } => (section.clone(), offset, global),
        ref s => panic!("{n}: {s:?}"),
    };
    assert_eq!(at("proc_data").0, "pchist_unwired_d");
    assert_eq!(at("wired_count").0, "pchist_unwired_d");
    assert_eq!(at("proc_data").1, 0);
    assert_eq!(at("wire_pages").1, 0x500);
    assert_eq!(at("histogram").1, 0x50C);
    assert_eq!(at("wired_count").1, 0x936);
    assert_eq!(at("x"), (".data".into(), 0, false));
    // f first (offset 0), then e when the define allocates it, then g.
    assert_eq!(at("f"), ("sec".into(), 0, false));
    assert_eq!(at("e"), ("sec".into(), 2, true));
    assert_eq!(at("g"), ("sec".into(), 4, false));
    assert_eq!(
        p.data_sections,
        vec![
            ("pchist_unwired_d".to_string(), 0x938),
            (".data".to_string(), 2),
            ("sec".to_string(), 8)
        ]
    );
}

/// Static objects are at least word aligned: a byte variable takes the
/// high byte of a word of its own and two are never packed into one word
/// (codegen-patterns 11, "Byte-sized module variables"; bead
/// domain-pascal-yvko).  SIO_IO's WIRED_DATA 0xE2DEB8: chars at +0x48 and
/// +0x4A (`70 00 60 00`) before a 4-byte record at +0x4C; NETWORK
/// 0xE248FC: the booleans NETWORK_$ACTIVITY_FLAG .. _$REALLY_DISKLESS 2
/// bytes apart.  Multi-byte objects keep their own alignment and size.
#[test]
fn static_byte_variables_word_aligned() {
    let p = module(
        "type rates = record tx, rx: integer end;
              c3 = array[1..3] of char;
              mr = packed record case integer of
                     1: (c: char); 2: (mode: 0..3; stop: 0..15) end;
         var
           stop: char := chr(16#70);
           start: char := chr(16#60);
           baud: rates;
           f1: define boolean;
           f2: define boolean;
           ub: [byte] 0..255;
           a3: c3;
           last: char;
           tmpl: mr;
           l: integer32;
         procedure p; var s1: static boolean; s2: static char;
         begin s1 := true; s2 := 'a' end;",
    );
    let off = |n: &str| match var(&p, n).storage {
        Storage::Static { offset, .. } => offset,
        ref s => panic!("{n}: {s:?}"),
    };
    assert_eq!((off("stop"), off("start"), off("baud")), (0, 2, 4));
    assert_eq!((off("f1"), off("f2"), off("ub")), (8, 10, 12));
    // A 3-char array keeps its 3 bytes; the next byte is not packed after
    // it.
    assert_eq!((off("a3"), off("last")), (14, 18));
    // A one-char packed record is a word (ref 3.10.7 p. 3-35).
    assert_eq!((off("tmpl"), off("l")), (20, 24));
    assert_eq!((off("s1"), off("s2")), (28, 30));
    assert_eq!(p.data_sections, vec![(".data".to_string(), 31)]);
}

#[test]
fn define_part_methods() {
    // Method 2 (ref 7.2.2 p. 7-4): extern, then define with a value.
    let p = module("var x: extern integer; define x := 8;");
    assert!(matches!(
        var(&p, "x").storage,
        Storage::Static { global: true, .. }
    ));
    assert_eq!(
        var(&p, "x").init.as_ref().unwrap()[0].value,
        ConstValue::Int(8)
    );
    // Method 3 (ref 7.2.3 p. 7-6): define first.
    let p = module("define x; var x: extern integer := 8;");
    assert!(matches!(
        var(&p, "x").storage,
        Storage::Static { global: true, .. }
    ));
}

/// `define a, b, ...;` means one uninitialised item per name (owner
/// ruling 2026-10-14 on ref 7.2.2 p. 7-5; reference-clarifications
/// "7.2.2 Lists in the define statement").  Routines may be listed too
/// (ref 7.3.4 p. 7-11), as in Apollo's streams/sasc examples.
#[test]
fn define_list_is_n_single_defines() {
    let src = "var x: extern integer; y: extern integer32;
               procedure q; extern;
               define x, q, y;
               procedure q; begin end;";
    let one = "var x: extern integer; y: extern integer32;
               procedure q; extern;
               define x; q; y;
               procedure q; begin end;";
    for s in [src, one] {
        let p = module(s);
        for n in ["x", "y"] {
            assert!(
                matches!(var(&p, n).storage, Storage::Static { global: true, .. }),
                "{n}"
            );
            assert!(var(&p, n).init.is_none(), "{n}");
        }
        let q = routine(&p, "q");
        assert!(!q.external && q.global);
    }
    let a = module(src);
    let b = module(one);
    assert_eq!(var(&a, "x").storage, var(&b, "x").storage);
    assert_eq!(var(&a, "y").storage, var(&b, "y").storage);
}

#[test]
fn define_routine_with_initial_value_is_an_error() {
    // ref 7.3.4 p. 7-11.
    assert_err(
        "module m; procedure q; extern; define q := 1;",
        "a define item for a routine cannot have an initial value",
    );
}

/// Every defined name must be declared somewhere in the unit, as a
/// variable or a routine, whichever side of the define part it is on.
#[test]
fn define_names_must_be_declared() {
    let e = errs("module m; define zz, yy;");
    assert_eq!(e.len(), 2, "{e:?}");
    assert!(e[0].contains("`zz` is named in a define statement but never declared"));
    assert!(e[1].contains("`yy` is named in a define statement but never declared"));
    // Method 3 order (ref 7.2.3 p. 7-6) with a name that is not a
    // variable or routine; reported once even when listed twice.
    let e = errs("module m; define t, t; type t = integer;");
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("`t` cannot be defined"), "{e:?}");
    assert_err("module m; const k = 1; define k;", "`k` cannot be defined");
    // Method 3 for routines, as in Apollo's streams/sasc io.pas.
    module(
        "define a, q; var a: extern integer := 3;
            procedure q; extern; procedure q; begin a := 1 end;",
    );
}

/// ref 7.3.4 p. 7-11: define says the routine is "a global symbol defined
/// in module math2"; a routine with only an `extern` heading is not.
#[test]
fn defined_routine_needs_a_body() {
    for src in [
        "module m; procedure q; extern; define q;",
        "module m; define q, q; procedure q; extern;",
    ] {
        let e = errs(src);
        assert_eq!(e.len(), 1, "{src}: {e:?}");
        assert!(
            e[0].contains("routine `q` is named in a define statement but has no body"),
            "{e:?}"
        );
    }
}

#[test]
fn module_heading_and_section_attribute() {
    let p = ok("module kitchen, wired_proc, wired_data;
                var v: integer;
                procedure a; begin end;
                [section(other_proc)] procedure b; begin end;
                [section(, other_data)] procedure c; var s: static integer; begin end;");
    assert_eq!(
        (p.code_section.as_str(), p.data_section.as_str()),
        ("wired_proc", "wired_data")
    );
    let sec = |n: &str| {
        let r = routine(&p, n);
        (r.code_section.clone(), r.data_section.clone())
    };
    assert_eq!(sec("a"), ("wired_proc".into(), "wired_data".into()));
    assert_eq!(sec("b"), ("other_proc".into(), "wired_data".into()));
    assert_eq!(sec("c"), ("wired_proc".into(), "other_data".into()));
    assert!(
        matches!(&var(&p, "s").storage, Storage::Static { section, .. } if section == "other_data")
    );
    assert!(
        matches!(&var(&p, "v").storage, Storage::Static { section, .. } if section == "wired_data")
    );
}

#[test]
fn module_heading_empty_code_section() {
    // `module name, , data_section;`: code in the default `.text`, data in
    // the named section (owner ruling 2026-10-15; ../source
    // pchist/pchist.pas).
    let p = ok("module pchist, , wired_data;
                var v: integer;
                procedure a; begin end;");
    assert_eq!(
        (p.code_section.as_str(), p.data_section.as_str()),
        (".text", "wired_data")
    );
    let r = routine(&p, "a");
    assert_eq!(
        (r.code_section.as_str(), r.data_section.as_str()),
        (".text", "wired_data")
    );
    assert!(
        matches!(&var(&p, "v").storage, Storage::Static { section, .. } if section == "wired_data")
    );
}

#[test]
fn linkage_and_symbols() {
    let p = module(
        "procedure ast_$page_zero; begin end;
         procedure worker; internal; begin end;
         procedure ml_$lock(lock: integer); val_param; extern;",
    );
    let r = routine(&p, "ast_$page_zero");
    assert_eq!(r.symbol, "AST_$PAGE_ZERO");
    assert!(r.global && !r.external);
    assert!(!routine(&p, "worker").global);
    let l = routine(&p, "ml_$lock");
    assert!(l.external && l.body.is_none());
}

#[test]
fn forward_and_body() {
    let p = module(
        "function f(x: integer): integer; forward;
         procedure g; var y: integer; begin y := f(1) end;
         function f; begin f := x end;",
    );
    let f = routine(&p, "f");
    assert!(f.body.is_some());
    assert_eq!(f.params.len(), 1);
    assert_eq!(p.routines.iter().filter(|r| r.name == "f").count(), 1);
    assert_err(
        "module m; procedure f; forward;",
        "declared forward but never defined",
    );
}

fn top_names(p: &Program) -> Vec<&str> {
    p.top_routines
        .iter()
        .map(|&id| p.routine(id).name.as_str())
        .collect()
}

/// Body order (codegen-patterns 6, "Order"): a routine declared `forward`
/// is placed at its body, not at its heading.
#[test]
fn forward_routine_is_placed_at_its_body() {
    let p = module(
        "procedure a; forward; internal;
         procedure b; begin a end;
         procedure c; begin end;
         procedure a; begin end;
         procedure d; begin a end;",
    );
    assert_eq!(top_names(&p), ["b", "c", "a", "d"]);
}

/// Ref 7.3.4 p. 7-11 (Method 2): `extern` heading (normally from an
/// %include'd file), then the body under a bare re-heading.  The routine
/// sits at its body; a routine that stays `extern` sits at its heading.
#[test]
fn extern_then_defined_is_placed_at_its_body() {
    let p = module(
        "define exponent;
         function exponent(number, power: integer): integer32; extern;
         procedure other(x: integer); extern;
         procedure first; begin other(1) end;
         function exponent; begin exponent := number + power end;
         procedure last; begin end;",
    );
    assert_eq!(top_names(&p), ["other", "first", "exponent", "last"]);
    let e = routine(&p, "exponent");
    assert!(!e.external && e.body.is_some() && e.global);
    assert!(routine(&p, "other").external);
}

// ---------------------------------------------------------------------
// Expressions and statements
// ---------------------------------------------------------------------

#[test]
fn integer_widths() {
    let p = module(
        "var a: integer; b: integer32; c: boolean;
         procedure p; begin a := a + 1; b := a * b; c := a < b end;",
    );
    let r = routine(&p, "p");
    let tys: Vec<_> = r
        .body
        .as_ref()
        .unwrap()
        .stmts
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Assign { value, .. } => p.types.display(value.ty),
            k => panic!("{k:?}"),
        })
        .collect();
    assert_eq!(tys, vec!["integer16", "integer32", "boolean"]);
}

#[test]
fn function_result_assignment_and_uplevel() {
    let p = module(
        "function f(x: integer): integer;
           var l: integer;
           procedure inner;
           begin f := l; l := x end;
         begin inner; f := 0 end;",
    );
    let inner = routine(&p, "inner");
    let stmts = &inner.body.as_ref().unwrap().stmts;
    match &stmts[0].kind {
        StmtKind::Assign { target, value } => {
            assert!(matches!(target.kind, ExprKind::Result { uplevel: 1, .. }));
            assert!(matches!(value.kind, ExprKind::Var { uplevel: 1, .. }));
        }
        k => panic!("{k:?}"),
    }
    assert_err(
        "module m; function f: integer; begin end; procedure p; begin f := 1 end;",
        "cannot assign to `f`",
    );
}

#[test]
fn assignment_to_in_parameter() {
    // ref 5.3.2 p. 5-7.
    assert_err(
        "module m; procedure p(in x: integer); begin x := 1 end;",
        "Assignment to IN argument",
    );
}

/// Bead domain-pascal-pe9: a caller holds a parameter in a register
/// across a call that passes its address to an `in` parameter
/// (codegen-patterns 12.2), so no path in the callee may write it (ref
/// 5.3.2 p. 5-7).  Writing what an `in` pointer points to is allowed.
#[test]
fn in_parameter_parts_are_not_written() {
    let rec = "type r = record a, b, c: integer; end; ip = ^integer;";
    for (body, msg) in [
        (
            "x.a := 1",
            "assigning to a field or element of an `in` parameter",
        ),
        (
            "with x do a := 1",
            "assigning to a field or element of an `in` parameter",
        ),
        ("s(x.a)", "passing an `in` parameter"),
        ("s(y)", "passing an `in` parameter"),
        ("t(x)", "passing an `in` parameter"),
    ] {
        assert_err(
            &format!(
                "module m; {rec} procedure s(var v: integer); extern; \
                 procedure t(out v: univ r); extern; \
                 procedure p(in x: r; in y: integer); begin {body} end;"
            ),
            msg,
        );
    }
    // Nested: the enclosing routine's `in` parameter.
    assert_err(
        &format!(
            "module m; {rec} procedure s(var v: integer); extern; \
             procedure p(in y: integer); procedure q; begin s(y) end; begin q end;"
        ),
        "passing an `in` parameter",
    );
    module(&format!(
        "{rec} procedure s(var v: integer); extern; \
         procedure p(in q: ip; in y: integer); var z: integer; begin q^ := y; s(q^); z := y end;"
    ));
}

/// Bead domain-pascal-mbtx: `addr` of an `in` parameter, or of a part of
/// one, is accepted ("Addr", p. 4-13: "any data type except as a procedure
/// or function data type having the internal attribute"; 5.3.2 p. 5-7
/// forbids only assignment), by address and, under `val_param`, by value.
#[test]
fn addr_of_in_parameter() {
    let rec = "type r = record a, b: integer; end; a2 = array[1..2] of integer;";
    for opts in ["", " options(val_param);"] {
        for body in [
            "u := addr(x)",
            "u := addr(y)",
            "u := addr(x.b)",
            "u := addr(z[2])",
            "with x do u := addr(b)",
            "discard(addr(y))",
        ] {
            module(&format!(
                "{rec} var u: univ_ptr; \
                 procedure p(in x: r; in y: integer; in z: a2);{opts} begin {body} end;"
            ));
        }
    }
}

#[test]
fn var_parameter_needs_variable_of_same_type() {
    assert_err(
        "module m; procedure q(var x: integer); extern; procedure p; begin q(1) end;",
        "a variable is needed",
    );
    assert_err(
        "module m; var c: char; procedure q(var x: integer); extern; procedure p; begin q(c) end;",
        "does not match",
    );
    // univ turns the check off (ref 5.3.3 p. 5-10).
    module("var c: char; procedure q(var x: univ integer); extern; procedure p; begin q(c) end;");
}

/// Ref 5.3.3 p. 5-11 and Appendix B message 246 (WARNING): an
/// expression (not a variable) smaller than its `univ` formal is extended
/// to the formal's size, with "Expression passed to UNIV formal NAME was
/// converted to NEWTYPE."  Variables, equal sizes and non-`univ` formals
/// give no warning.
#[test]
fn univ_expression_conversion_warns() {
    let p = module(
        "type r = record a, b: integer; end;
         var w: integer; l: integer32;
         procedure q(in a: univ integer32; in b: univ integer; in c: univ r; in d: integer32);
           extern;
         procedure p;
         begin
           q(0, 7, 65536, 1);
           q(w + 1, w, l, w);
           q(w, true, w, l);
         end;",
    );
    let w: Vec<&str> = p.warnings.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        w,
        [
            "Expression passed to UNIV formal a was converted to integer32.",
            "Expression passed to UNIV formal a was converted to integer32.",
            "Expression passed to UNIV formal b was converted to integer16.",
        ]
    );
    assert!(p.warnings.iter().all(|d| !d.is_error()));
}

#[test]
fn constants_passed_by_address_need_temps() {
    let p = module(
        "procedure ml_$lock(lock: integer); val_param; extern;
         procedure q(x: integer); extern;
         procedure p; begin ml_$lock(16#14); q(5) end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let arg = |i: usize| match &stmts[i].kind {
        StmtKind::Call(c) => c.args[0].clone(),
        k => panic!("{k:?}"),
    };
    assert!(!arg(0).needs_temp);
    assert!(arg(1).needs_temp);
}

#[test]
fn variables_of_another_size_passed_by_address_need_temps() {
    let p = module(
        "var l: integer32; w: integer; b: 0..100;
         procedure q(in x: integer); extern;
         procedure p; begin q(l); q(w); q(b) end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let arg = |i: usize| match &stmts[i].kind {
        StmtKind::Call(c) => c.args[0].clone(),
        k => panic!("{k:?}"),
    };
    // The callee would read the high word of `l`.
    assert!(arg(0).needs_temp);
    assert!(!arg(1).needs_temp);
    assert!(!arg(2).needs_temp);
}

#[test]
fn argument_count() {
    assert_err(
        "module m; procedure q(a, b: integer); extern; procedure p; begin q(1) end;",
        "wrong number of arguments",
    );
    // `variable` routines may get fewer (ref 5.5.5 p. 5-17).
    module("procedure q(a, b: integer); options(extern, variable); procedure p; begin q(1) end;");
}

/// Owner's ruling on 5.1, p. 5-3 (docs/reference-clarifications.md "5.1
/// Empty argument lists"): `q();` calls `q` with no arguments, like `q;`.
/// Function and routine-pointer calls (`f()`, `fp^()`) follow the lead's
/// reading, pending the owner's confirmation.
#[test]
fn empty_argument_lists() {
    let p = module(
        "type pp = ^procedure; fp = ^function: integer;
         var i: integer; vp: pp; vf: fp;
         procedure q; extern;
         function f: integer; extern;
         procedure p; begin q(); q; i := f(); i := f; vp^(); vp^; i := vf^(); i := vf^ end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let shape = |k: &StmtKind| match k {
        StmtKind::Call(c) => match &c.callee {
            crate::program::Callee::Routine(r) => {
                format!("call {} {}", p.routine(*r).name, c.args.len())
            }
            crate::program::Callee::Pointer(_) => format!("ptrcall {}", c.args.len()),
        },
        StmtKind::Assign { value, .. } => match &value.kind {
            ExprKind::Call(c) => match &c.callee {
                crate::program::Callee::Routine(r) => {
                    format!("i := {} {}", p.routine(*r).name, c.args.len())
                }
                crate::program::Callee::Pointer(_) => format!("i := ptr {}", c.args.len()),
            },
            k => format!("{k:?}"),
        },
        k => format!("{k:?}"),
    };
    let got: Vec<String> = stmts.iter().map(|s| shape(&s.kind)).collect();
    assert_eq!(
        got,
        [
            "call q 0",
            "call q 0",
            "i := f 0",
            "i := f 0",
            "ptrcall 0",
            "ptrcall 0",
            "i := ptr 0",
            "i := ptr 0",
        ]
    );
    // A routine with parameters still needs its arguments.
    assert_err(
        "module m; procedure q(a: integer); extern; procedure p; begin q() end;",
        "wrong number of arguments: 0 given, 1 expected",
    );
    assert_err(
        "module m; var i: integer; function f(a: integer): integer; extern;
         procedure p; begin i := f() end;",
        "wrong number of arguments: 0 given, 1 expected",
    );
    assert_err(
        "module m; type pp = ^procedure(a: integer); var vp: pp;
         procedure p; begin vp^() end;",
        "wrong number of arguments: 0 given, 1 expected",
    );
    // `()` does not change what kind of routine may be called where.
    assert_err(
        "module m; var i: integer; procedure q; extern; procedure p; begin i := q() end;",
        "procedure `q` used as a value",
    );
    // Only a routine (or routine pointer) can be called: `()` after a
    // variable, a data pointer, a constant or a call is no call.
    for (src, msg) in [
        (
            "var i, x: integer; procedure p; begin i := x() end;",
            "function call expected",
        ),
        (
            "var i: integer; x: ^integer; procedure p; begin i := x^() end;",
            "function call expected",
        ),
        (
            "var i: integer; procedure p; begin i := nil() end;",
            "function call expected",
        ),
        (
            "var x: integer; procedure p; begin x() end;",
            "procedure call expected",
        ),
        (
            "procedure q; extern; procedure p; begin q()() end;",
            "procedure call expected",
        ),
        (
            "var i: integer32; procedure p; begin i := integer32() end;",
            "a type transfer takes one argument",
        ),
        (
            "var i: integer; procedure p; begin i := abs() end;",
            "`abs` takes 1 argument(s)",
        ),
    ] {
        assert_err(&format!("module m; {src}"), msg);
    }
    // Inside a function, `f()` on the right is a recursive call, as `f`
    // alone is; the left `f` is the result.
    let p = module("function f: integer; begin f := f() + 1 end;");
    let StmtKind::Assign { value, .. } = &routine(&p, "f").body.as_ref().unwrap().stmts[0].kind
    else {
        panic!()
    };
    let ExprKind::Binary { lhs, .. } = &value.kind else {
        panic!("{value:?}")
    };
    assert!(
        matches!(&lhs.kind, ExprKind::Call(c) if c.args.is_empty()),
        "{lhs:?}"
    );
}

#[test]
fn sets_and_in() {
    let p = module(
        "type s = set of 0..31;
         var v: s; i: integer; b: boolean;
         procedure p; begin
           v := v + [i]; v := [1, 3..5]; b := i in v; b := 3 in [1..5];
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    match &stmts[3].kind {
        StmtKind::Assign { value, .. } => assert_eq!(value.value, Some(ConstValue::Bool(true))),
        k => panic!("{k:?}"),
    }
    assert_err(
        "module m; type e = (x, y); var v: set of 0..7; b: boolean; procedure p; begin b := x in v end;",
        "does not match the set",
    );
}

#[test]
fn with_statement_binds_fields() {
    let p = module(
        "type r = record a: integer; b: char end;
         var v: r; w: array[1..2] of r;
         procedure p; begin
           with v do a := 1;
           with w[2] : q do q.b := 'x';
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    match &stmts[0].kind {
        StmtKind::With { items, body } => {
            assert_eq!(items.len(), 1);
            match &body.kind {
                StmtKind::Assign { target, .. } => match &target.kind {
                    ExprKind::Field { base, offset, .. } => {
                        assert_eq!(*offset, 0);
                        assert!(matches!(base.kind, ExprKind::WithRecord(id) if id == items[0].0));
                    }
                    k => panic!("{k:?}"),
                },
                k => panic!("{k:?}"),
            }
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn case_statement() {
    let p = module(
        "type k = (ka, kb, kc);
         var x: k; i: integer;
         procedure p; begin
           case x of ka, kb: i := 1; kc: i := 2; otherwise i := 3; end;
         end;",
    );
    match &routine(&p, "p").body.as_ref().unwrap().stmts[0].kind {
        StmtKind::Case {
            arms, otherwise, ..
        } => {
            assert_eq!(arms[0].labels, vec![0, 1]);
            assert_eq!(arms[1].labels, vec![2]);
            assert!(otherwise.is_some());
        }
        k => panic!("{k:?}"),
    }
    assert_err(
        "module m; var i: integer; procedure p; begin case i of 1: ; 1: ; end end;",
        "duplicate case label",
    );
}

#[test]
fn loops_exit_next() {
    module(
        "var i: integer;
         procedure p; begin
           for i := 1 to 10 do begin if i = 3 then next; if i = 5 then exit end;
           while i > 0 do i := i - 1;
           repeat i := i + 1 until i >= 4;
         end;",
    );
    assert_err("module m; procedure p; begin exit end;", "outside a loop");
    assert_err(
        "module m; var c: char; procedure p; begin for c := 1 to 2 do ; end;",
        "not compatible with the index variable",
    );
}

#[test]
fn goto_and_labels() {
    module("procedure p; label 10, done; begin goto done; 10: ; done: end;");
    assert_err("module m; procedure p; begin goto 9 end;", "not declared");
    // A goto from a nested routine to its parent's label (ref p. 4-97).
    module("procedure p; label 9; procedure q; begin goto 9 end; begin q; 9: end;");
    // ref p. 9-37: "Labels not permitted at MODULE level."
    assert_err(
        "module m; label 9; procedure p; begin end;",
        "labels are not permitted at module level",
    );
    // ref p. 4-95: a label is declared in the routine whose statement it
    // prefixes.
    assert_err(
        "module m; procedure p; label 9; procedure q; begin 9: end; begin q; goto 9 end;",
        "label `9` is not declared in this routine",
    );
    assert_err(
        "module m; procedure p; label 9; begin 9: ; 9: ; goto 9 end;",
        "label `9` prefixes more than one statement",
    );
    assert_err(
        "module m; procedure p; label 9; begin goto 9 end;",
        "label `9` prefixes no statement",
    );
    // A label declared and never used nor defined is not an error.
    module("procedure p; label 9; begin end;");
}

#[test]
fn pointers_nil_univ_ptr() {
    module(
        "type pi = ^integer; pc = ^char;
         var a, b: pi; c: pc; u: univ_ptr; x: integer;
         procedure p; begin
           a := nil; a := b; u := a; a := u; x := a^;
           if a = nil then a := b;
         end;",
    );
    assert_err(
        "module m; type pi = ^integer; pc = ^char; var a: pi; c: pc; procedure p; begin a := c end;",
        "not compatible",
    );
    assert_err(
        "module m; var u: univ_ptr; x: integer; procedure p; begin x := u^ end;",
        "univ_ptr cannot be dereferenced",
    );
}

#[test]
fn forward_pointer_types() {
    let p = module("type a = ^b; b = record next: a; v: integer end;");
    match p.types.kind(ty(&p, "a")) {
        crate::types::TypeKind::Pointer { target } => assert_eq!(*target, ty(&p, "b")),
        k => panic!("{k:?}"),
    }
    assert_err("module m; type a = ^nosuch;", "undefined type `nosuch`");
}

#[test]
fn char_arrays_and_strings() {
    // ref "Array Operations", p. 4-26.
    module(
        "var greeting: array[1..12] of char; a, b: array[1..20] of char;
         procedure p; begin greeting := 'aloha'; a := b end;",
    );
    assert_err(
        "module m; var f: array[1..12] of char; procedure p; begin f := 'auf wiedersehen' end;",
        "not compatible",
    );
}

#[test]
fn type_transfer() {
    // ref p. 4-197.
    let p = module(
        "type pi = ^integer;
         var a: pi; l: integer32; i: integer;
         procedure p; begin l := integer32(a); i := integer(l) end;",
    );
    let _ = p;
    assert_err(
        "module m; var c: char; l: integer32; procedure p; begin l := integer32(c) end;",
        "equal sizes",
    );
}

#[test]
fn builtins() {
    let p = module(
        "var i: integer; l: integer32; c: char; b: boolean;
         procedure p; begin
           disable;
           i := lshft(i, 1) ! 1; l := rshft(l, 3); i := ord(c); c := chr(i);
           b := odd(i); i := xor(i, 5); i := abs(i);
           enable;
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    assert!(matches!(
        stmts[0].kind,
        StmtKind::Builtin {
            builtin: crate::program::Builtin::Disable,
            ..
        }
    ));
}

/// An integer constant transferred to a pointer type of another size
/// (owner's ruling, reference-clarifications "Type Transfer Functions"
/// (p. 4-197); bead domain-pascal-gqp): Apollo's streams/sasc misc.pas
/// `ecp := ec2_$ptr_t( ec2_$always_ready_ec )`, the constant 1.
#[test]
fn integer_constant_to_pointer_transfer() {
    let p = module(
        "type ec2_$ptr_t = ^integer32;
         const ec2_$always_ready_ec = 1;
         var e: ec2_$ptr_t := ec2_$ptr_t(0);
         procedure p(out ecp: ec2_$ptr_t); begin ecp := ec2_$ptr_t(ec2_$always_ready_ec) end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let StmtKind::Assign { value, .. } = &stmts[0].kind else {
        panic!("assignment expected: {:?}", stmts[0].kind)
    };
    assert!(matches!(value.kind, ExprKind::Transfer(_)));
    assert!(p.types.is_pointer(value.ty));
    assert_eq!(value.value, Some(ConstValue::Int(1)));
    assert_eq!(
        var(&p, "e").init.as_ref().unwrap()[0].value,
        InitDatum::from(ConstValue::Int(0))
    );
    // A negative constant: the reference does not say what it means.
    assert_err(
        "module m; type t = ^integer32; var x: t; procedure p; begin x := t(-1) end;",
        "negative constant -1",
    );
    // A run-time integer16 of another size stays refused (p. 4-197).
    assert_err(
        "module m; type t = ^integer32; var x: t; i: integer;
         procedure p; begin x := t(i) end;",
        "equal sizes",
    );
    // Not a const value: ref 2.2.2.2, p. 2-12 names nil as the pointer
    // value.
    assert_err(
        "module m; type t = ^integer32; const c = t(1);",
        "a pointer constant other than nil",
    );
    assert_err(
        "module m; type t = ^integer32; const c = (t(16#20000));",
        "a pointer constant other than nil",
    );
    // An integer32 constant (equal sizes, p. 4-197) is a pointer constant
    // as an integer16 one is, negative or not (the bits are kept).
    let p = module(
        "type t = ^integer32;
         var a: t := t(16#20000); b: t := t(-70000); c: array[1..2] of t := [t(1), t(2)];",
    );
    let v = |n: &str| var(&p, n).init.as_ref().unwrap()[0].value.clone();
    assert_eq!(v("a"), InitDatum::from(ConstValue::Int(0x20000)));
    assert_eq!(v("b"), InitDatum::from(ConstValue::Int(-70000)));
    // A pointer constant has no address: not a `var` argument, not addr's.
    assert_err(
        "module m; type t = ^integer32; procedure r(var y: t); extern;
         procedure p; begin r(t(1)) end;",
        "a variable is needed",
    );
    assert_err(
        "module m; type t = ^integer32; var u: univ_ptr;
         procedure p; begin u := addr(t(1)) end;",
        "`addr` needs a variable",
    );
}

/// Predefined functions of constants fold (ref 2.2.2.2, p. 2-12; bead
/// domain-pascal-95c), in 32 bits (ref 3.2.3, p. 3-5).  Apollo's X11
/// x.ins.pas: `KeyPressMask = LSHFT(1,0)` ... `OwnerGrabButtonMask =
/// LSHFT(1,24)`.
#[test]
fn predefined_functions_in_constants() {
    let p = module(
        "const KeyPressMask = LSHFT(1,0); ExposureMask = LSHFT(1,15);
               OwnerGrab = lshft(1,24); top = lshft(1,31); out = lshft(16#40000001, 2);
               r = rshft(16#40000000, 4); ar = arshft(-100, 3); ar2 = arshft(16#80000000, 31);
               x = xor(16#ff, 16#0f); xn = xor(-1, 16#0f0f); s = sqr(200); sn = sqr(-3);
               o = ord('A'); c = chr(16#141); a = abs(-5); od = odd(3);
               su = succ(5); pr = pred('b'); mx = max(3, 9); mn = min(3, 9);
               sz = sizeof(integer32); m = KeyPressMask ! ExposureMask;
         var v: array[1..22] of integer32 := [KeyPressMask, ExposureMask, OwnerGrab, top,
               out, r, ar, ar2, x, xn, s, sn, o, a, su, mx, mn, sz, m, 0, 0, 0];
             vc: char := c; vp: char := pr; vb: boolean := od;",
    );
    let init = var(&p, "v").init.clone().unwrap();
    let got: Vec<InitDatum> = init.iter().take(19).map(|s| s.value.clone()).collect();
    let want: Vec<InitDatum> = [
        1,
        32768,
        1 << 24,
        i32::MIN as i64,
        4,
        0x0400_0000,
        -13,
        -1,
        0xf0,
        !0x0f0f,
        40000,
        9,
        65,
        5,
        6,
        9,
        3,
        4,
        32769,
    ]
    .into_iter()
    .map(|k| ConstValue::Int(k).into())
    .collect();
    assert_eq!(got, want);
    let v = |n: &str| var(&p, n).init.as_ref().unwrap()[0].value.clone();
    assert_eq!(v("vc"), ConstValue::Char(0x41));
    assert_eq!(v("vp"), ConstValue::Char(b'a'));
    assert_eq!(v("vb"), ConstValue::Bool(true));
    // No constant value where the reference gives none: a negative shift
    // count ("Results are unpredictable", p. 4-109), rshft of a negative
    // constant (p. 4-171 gives a 32-bit and a 16-bit result).
    assert_err(
        "module m; const c = lshft(1, -1);",
        "constant expression expected",
    );
    assert_err(
        "module m; const c = rshft(-9, 3);",
        "constant expression expected",
    );
    // sqr out of the constants' range (ref 3.2.3).
    assert_err("module m; const c = sqr(70000);", "out of range");
    // A folded result is typed as any integer constant (ref 3.2.3): one
    // outside -32768..32767 is not an integer16 value, as the literal is
    // not; `xor` is not a boolean operator ("Xor", p. 4-221).
    assert_err(
        "module m; var i: integer; procedure p; begin i := lshft(1, 15) end;",
        "constant 32768 is out of range for integer16",
    );
    assert_err(
        "module m; var b: boolean := xor(true, false);",
        "integer arguments",
    );
    // Folded values serve wherever a constant does: subrange and array
    // bounds, set constructors, case labels.
    let p = module(
        "type t = 0..lshft(1, 4) - 1; s = set of 0..sqr(4) - 1;
         var a: array[0..lshft(1, 3)] of integer32; v: s := [lshft(1, 2), rshft(16, 2)..xor(5, 3)];
             i: integer;
         procedure q; begin case i of lshft(1, 2): i := 1; arshft(-8, 1): i := 2 end end;",
    );
    assert_eq!(size(&p, "t"), 2);
    assert_eq!(size(&p, "s"), 2);
    assert_eq!(p.types.size(var(&p, "a").ty), 9 * 4);
    assert_eq!(
        var(&p, "v").init.as_ref().unwrap()[0].value,
        InitDatum::from(ConstValue::Set(vec![4, 5, 6]))
    );
}

/// `addr(x)` ("Addr", p. 4-13): a univ_ptr value; x must be a variable.
#[test]
fn addr_builtin() {
    let p = module(
        "var buf: extern array[0..255] of integer32; u: univ_ptr;
         procedure take(va: univ_ptr); options(val_param, extern);
         procedure p; begin take(addr(buf)); u := addr(buf[3]) end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let StmtKind::Call(c) = &stmts[0].kind else {
        panic!("not a call")
    };
    let a = &c.args[0].expr;
    assert!(matches!(
        a.kind,
        ExprKind::Builtin {
            builtin: crate::program::Builtin::Addr,
            ..
        }
    ));
    assert_eq!(a.ty, p.types.builtins.univ_ptr);
    assert_eq!(c.args[0].conv.passing, crate::conv::Passing::ByValue);
    assert_err(
        "module m; var u: univ_ptr; procedure p; begin u := addr(3) end;",
        "`addr` needs a variable",
    );
    assert_err(
        "module m; var u: univ_ptr; i: integer; procedure p; begin u := addr(i, i) end;",
        "takes 1 argument",
    );
    // Legal (p. 4-13; C.3.5, p. C-7) but not handled yet.
    assert_err(
        "module m; var u: univ_ptr; procedure p; begin u := addr('abc') end;",
        "`addr` of a string constant (ref p. 4-13) is not supported yet",
    );
}

#[test]
fn routine_pointer_conventions() {
    // ref p. 4-146 (options after the type), Appendix B 181/182; Table
    // 5-1: `val_param` on the type passes small value and `in` parameters
    // by value, the default passes everything by address.
    let p = module(
        "type vp = ^procedure (in w: integer; b: boolean; var x: integer32); val_param;
              dp = ^procedure (in w: integer; b: boolean; var x: integer32);
         var v: vp; d: dp; l: integer32;
         procedure q; begin v^(1, true, l); d^(1, true, l) end;",
    );
    let stmts = &routine(&p, "q").body.as_ref().unwrap().stmts;
    let passing = |i: usize| -> Vec<crate::conv::Passing> {
        let StmtKind::Call(c) = &stmts[i].kind else {
            panic!("not a call")
        };
        assert!(matches!(c.callee, crate::program::Callee::Pointer(_)));
        c.args.iter().map(|a| a.conv.passing).collect()
    };
    use crate::conv::Passing::{ByAddress, ByValue};
    assert_eq!(passing(0), vec![ByValue, ByValue, ByAddress]);
    assert_eq!(passing(1), vec![ByAddress, ByAddress, ByAddress]);
    let TypeKind::RoutinePointer(sig) = p.types.kind(ty(&p, "vp")) else {
        panic!()
    };
    // 2 + 2 + 4 bytes: no pad.  Default: 12 bytes, no pad.
    assert_eq!(sig.arg_pad, 0);
    let p = module("type t = ^procedure (in w: integer; x: integer32); val_param;");
    let TypeKind::RoutinePointer(sig) = p.types.kind(ty(&p, "t")) else {
        panic!()
    };
    assert_eq!(sig.arg_pad, 2);
    assert_err(
        "module m; type t = ^procedure (x: integer); internal;",
        "INTERNAL option is illegal for PROCEDURE^ or FUNCTION^ types",
    );
    assert_err(
        "module m; type t = ^procedure (x: integer); extern;",
        "the routine option `extern` on a routine pointer type (only `val_param` is) is not \
         supported yet",
    );
    assert_err(
        "module m; routine_option default_routine_options = val_param;
         type t = ^procedure (x: integer);",
        "whether the defaults apply to it",
    );
    // Two types with different options are different types (ref
    // Appendix B 182).
    assert_err(
        "module m; type a = ^procedure (x: integer); val_param;
              b = ^procedure (x: integer);
         var va: a; vb: b; procedure p; begin va := vb end;",
        "not compatible",
    );
}

#[test]
fn addr_of_routines() {
    // ref p. 4-145/4-146: addr of a routine is a univ_ptr, checked
    // against a routine pointer type it is assigned to.
    let p = module(
        "type pinteger = 0..65535;
              t = ^procedure (x: integer); val_param;
         var u: univ_ptr; v: t;
         procedure q (x: integer); val_param; extern;
         procedure p; begin u := addr(q); v := addr(q) end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    let StmtKind::Assign { value, .. } = &stmts[1].kind else {
        panic!("not an assignment")
    };
    assert!(matches!(value.kind, ExprKind::RoutineAddr(_)));
    assert_eq!(value.ty, p.types.builtins.univ_ptr);
    // val_param on the heading but not on the type (Appendix B 182).
    assert_err(
        "module m; type t = ^procedure (x: integer);
         var v: t; procedure q (x: integer); val_param; extern;
         procedure p; begin v := addr(q) end;",
        "does not match the routine pointer type",
    );
    // Name compatibility of the parameter types (5.3.4.1, p. 5-13).
    assert_err(
        "module m; type pinteger = 0..65535; t = ^procedure (r: pinteger);
         var v: t; procedure foo (r: integer); extern;
         procedure p; begin v := addr(foo) end;",
        "does not match the routine pointer type",
    );
    // 3.13.3, p. 3-50: no address of nested or internal routines.
    assert_err(
        "module m; var u: univ_ptr;
         procedure p; procedure n; begin end; begin u := addr(n) end;",
        "the address of a nested routine cannot be taken",
    );
    assert_err(
        "module m; var u: univ_ptr; procedure i; internal; begin end;
         procedure p; begin u := addr(i) end;",
        "the address of an internal routine cannot be taken",
    );
    // p. 4-145: "you cannot de-reference the return value of a function
    // call" (`flag := rf^^`), as a value, a call with arguments, or a
    // statement.
    for body in [
        "flag := rf^^",
        "flag := getfp^",
        "flag := getfn^(1)",
        "getpr^(1)",
    ] {
        assert_err(
            &format!(
                "module m; type retbool = ^function: boolean;
                      retfunptr = ^function: retbool;
                      fn = ^function (in i: integer32): boolean;
                      pr = ^procedure (in i: integer32);
                 var rf: retfunptr; flag: boolean;
                 function getfp: retbool; extern;
                 function getfn: fn; extern;
                 function getpr: pr; extern;
                 procedure p; begin {body} end;"
            ),
            "the return value of a function call cannot be dereferenced (ref p. 4-145)",
        );
    }
}

#[test]
fn parameter_types_must_be_named() {
    // ref 5.1 p. 5-2: "Procedure range(small_range : 0..10); {WRONG!}".
    assert_err(
        "module m; procedure range(small_range: 0..10); extern;",
        "must be a type name",
    );
}

#[test]
fn kitchen_sink_module() {
    // The parser's whole-module test, with status_$t declared before use
    // (a parameter type must already be declared).
    let p = ok("{ A module touching most constructs. }
module kitchen_sink, wired_proc, wired_data;

const
  max_q = 16#20;
  msg = 'done'(13, 10);

type
  q_kind_t = (k_a, k_b);
  q_elem_t = record
    next: ^q_elem_t;
    key: integer32;
    case kind: q_kind_t of
      k_a: (a: integer);
      k_b: (b: packed array[0..3] of char);
  end;
  status_$t = integer32;
  handler_t = ^procedure (in elem: q_elem_t; out status: status_$t);

routine_option
  default_routine_options = val_param;

var (wired_data)
  queue: array[1..max_q] of q_elem_t;
  count: static integer := 0;
  flags: set of 0..15 := [0, 3..5];
  hook: extern handler_t;

define
  count;

procedure helper(var e: q_elem_t); internal;
var i: integer;
begin
  for i := 1 to max_q do begin
    if queue[i].key = 0 then next;
    with queue[i] : q do
      case q.kind of
        k_a: q.a := lshft(q.a, 1) ! 1;
        k_b: q.b[0] := 'x';
      end;
    if i >= max_q - 1 then exit;
  end;
end;

[section(wired_proc, wired_data)]
function q_$count(in which: integer): integer32;
  function inner: integer32;
  begin
    inner := count * 2;
  end;
begin
  disable;
  q_$count := inner;
  enable;
  if (which < 0) or else not (which in [1..max_q]) then
    return;
  repeat
    count := count - 1
  until count <= 0;
end;
");
    // default_routine_options = val_param applies to q_$count.
    let q = routine(&p, "q_$count");
    assert!(q.options.val_param);
    assert_eq!(q.params[0].conv.passing, crate::conv::Passing::ByValue);
    assert_eq!(q.nested.len(), 1);
    assert!(matches!(
        var(&p, "count").storage,
        Storage::Static { global: true, .. }
    ));
    assert_eq!(size(&p, "q_elem_t"), 14);
    assert_eq!(p.types.size(var(&p, "queue").ty), 14 * 32);
}

/// Ref 3.15.1-3.15.3 (pp. 3-56..3-57): `volatile`, `atomic` and `device`
/// are accepted by the checker and carried by the type; what they do to
/// the code is `dpc-ir`'s and the target's (`Addr::Volatile`).
#[test]
fn optimisation_attributes_are_accepted() {
    for a in ["volatile", "atomic", "device"] {
        let p = ok(&format!(
            "module m;\nvar x: extern [{a}] integer;\nprocedure p;\nbegin\n  x := 0;\nend;\n"
        ));
        assert!(p.types.attrs(var(&p, "x").ty).is_volatile(), "{a}");
    }
}

// ---------------------------------------------------------------------
// Predeclared routines of chapter 4 and Appendix E (bead domain-pascal-0wc)
// ---------------------------------------------------------------------

/// The value of the last statement's assigned expression in routine `p`.
fn assigned_value(prog: &Program, name: &str, k: usize) -> Option<ConstValue> {
    let stmts = &routine(prog, name).body.as_ref().unwrap().stmts;
    match &stmts[k].kind {
        StmtKind::Assign { value, .. } => value.value.clone(),
        s => panic!("not an assignment: {s:?}"),
    }
}

/// "Sizeof" (p. 4-181), "Firstof" (p. 4-87), "Lastof" (p. 4-106) fold to
/// constants, of types and of variables; "Succ"/"Pred" (pp. 4-191, 4-147),
/// "Max"/"Min" (pp. 4-111, 4-113) and "In_range" (p. 4-104) fold on
/// constant arguments.
#[test]
fn inquiry_builtins_fold() {
    let p = module(
        "type color = (red, green, blue); small = -7..7;
              rec = record a: integer32; b: integer; c: char; end;
              vec = array[2..9] of integer;
         const n = sizeof(rec) + 1;
         var w: integer; l: integer32; c: color; ch: char; b: boolean; v: vec;
         procedure p; begin
           w := sizeof(v); w := sizeof('wildebeest'); w := n; l := sizeof(integer32);
           c := lastof(color); c := succ(red); c := pred(blue);
           w := firstof(small); w := lastof(small); w := firstof(v); w := lastof(vec);
           w := lastof(integer); l := lastof(integer32); ch := lastof(char);
           b := firstof(boolean); b := succ(false); b := pred(true);
           w := max(3, 9); w := min(-3, 9); b := in_range(small(5)); b := in_range(small(8));
           ch := succ('a');
         end;",
    );
    let c = ty(&p, "color");
    let want = [
        ConstValue::Int(16),
        ConstValue::Int(10),
        ConstValue::Int(9),
        ConstValue::Int(4),
        ConstValue::Enum { ty: c, ord: 2 },
        ConstValue::Enum { ty: c, ord: 1 },
        ConstValue::Enum { ty: c, ord: 1 },
        ConstValue::Int(-7),
        ConstValue::Int(7),
        ConstValue::Int(2),
        ConstValue::Int(9),
        ConstValue::Int(32767),
        ConstValue::Int(2147483647),
        ConstValue::Char(255),
        ConstValue::Bool(false),
        ConstValue::Bool(true),
        ConstValue::Bool(false),
        ConstValue::Int(9),
        ConstValue::Int(-3),
        ConstValue::Bool(true),
        ConstValue::Bool(false),
        ConstValue::Char(b'b'),
    ];
    for (k, w) in want.iter().enumerate() {
        assert_eq!(
            assigned_value(&p, "p", k).as_ref(),
            Some(w),
            "statement {k}"
        );
    }
}

#[test]
fn run_time_builtins_are_typed() {
    let p = module(
        "type small = -7..7;
         var w: integer; l: integer32; s: small; b: boolean; sr: integer;
         procedure p; begin
           w := succ(w); l := pred(l); l := max(w, l); b := in_range(s);
           sr := set_sr(16#2700); sr := set_sr(16#F8FF); sr := set_sr(sr);
           discard(set_sr(sr));
         end;",
    );
    let stmts = &routine(&p, "p").body.as_ref().unwrap().stmts;
    assert!(matches!(
        &stmts[7].kind,
        StmtKind::Builtin {
            builtin: crate::program::Builtin::Discard,
            args,
        } if args.len() == 1
    ));
    // max of a word and a longword is a longword.
    let StmtKind::Assign { value, .. } = &stmts[2].kind else {
        panic!()
    };
    assert_eq!(value.ty, p.types.builtins.integer32);
}

#[test]
fn builtin_misuse() {
    // TODO(ref-question) Q17: p. 4-87 gives -32767 for integer.
    assert_err(
        "module m; var w: integer; procedure p; begin w := firstof(integer) end;",
        "`firstof` of integer",
    );
    assert_err(
        "module m; type r = record case k: integer of 1: (a: integer); 2: (b: integer32); end;
         var w: integer; procedure p; begin w := sizeof(r, 1) end;",
        "`sizeof` of a variant record with tags",
    );
    assert_err(
        "module m; type r = record a: integer; end; var x: r; w: integer;
         procedure p; begin w := lastof(x) end;",
        "`lastof` of",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin w := lastof(w + 1) end;",
        "needs a type name or a variable",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin discard end;",
        "`discard` takes 1 argument",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin w := discard(w) end;",
        "`discard` is a procedure",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin set_sr(w) end;",
        "`set_sr` is a function",
    );
    assert_err(
        "module m; var l: integer32; procedure p; begin l := set_sr(l) end;",
        "16-bit integer argument",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin w := set_sr(16#10000) end;",
        "16-bit integer argument",
    );
    assert_err(
        "module m; var w: integer; c: char; procedure p; begin w := max(w, c) end;",
        "`max` needs two ordinal or pointer arguments",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin w := min(w) end;",
        "`min` takes 2 argument(s)",
    );
    assert_err(
        "module m; var w: integer; procedure p; begin disable(w) end;",
        "`disable` takes no arguments",
    );
}

/// The predeclared names are in the outermost scope: a unit may declare
/// its own `min` or `max`.
#[test]
fn builtins_can_be_redeclared() {
    module(
        "var max: integer; procedure p; begin max := 1 end;
         function min(a, b: integer): integer; begin min := a end;",
    );
}

#[test]
fn univ_ptr_by_reference_parameters_take_any_pointer() {
    // ref 3.13.2, p. 3-50: univ_ptr as a "Formal or actual parameter for
    // any pointer type"; Apollo's ios.ins.pas `ios_$locate(...; OUT b:
    // univ_ptr; ...)` is passed a `^name_$pname_t` (enscramble.pas).
    ok("module m;
        type ip = ^integer; cp = ^char;
        procedure loc(out b: univ_ptr); extern;
        procedure typed(var b: ip); extern;
        procedure q;
        var p: ip; u: univ_ptr;
        begin loc(p); loc(u); typed(u); typed(p) end;");
    assert_err(
        "module m;
         type ip = ^integer; cp = ^char;
         procedure typed(var b: ip); extern;
         procedure q; var c: cp; begin typed(c) end;",
        "argument type cp does not match parameter type ip",
    );
}

/// Ref Appendix D, ISO 6.6.3.5/6.6.3.6, p. D-3: "Domain Pascal treats
/// integer and subrange of integer as identical."  Clarification Q38: an
/// integer subrange and an integer type, or two integer subranges, are
/// accepted for each other as `var`, `out` and `in out` arguments, quietly,
/// when they have the same size.  Sites: ../source net_io/init.pas (an
/// `integer` for `var socket: pinteger` through a routine pointer);
/// Apollo's sasc examples (an `integer32` for a `linteger` = 0..2147483647
/// parameter); dmwin.pas (`ios_$id_t` = 0..127 for an `integer16` `OUT`).
#[test]
fn integer_subrange_var_arguments_of_the_same_size() {
    ok("module m;
        type pinteger = 0..32767; linteger = 0..2147483647; id_t = 0..127;
             w = 0..65535;
             sendp_t = ^procedure(var socket: pinteger; out st: integer32);
        procedure v16(var x: integer16); extern;
        procedure vpi(var x: pinteger); extern;
        procedure vli(var x: linteger); extern;
        procedure v32(var x: integer32); extern;
        procedure o16(out x: integer16); extern;
        procedure iow(in out x: w); extern;
        procedure q(sp: sendp_t);
        var i: integer; i16: integer16; i32: integer32; pi: pinteger;
            li: linteger; id: id_t; st: integer32;
        begin
          v16(pi); vpi(i16); vpi(i); vpi(id); vli(i32); v32(li);
          o16(id); iow(i16); iow(id); sp^(i, st)
        end;");
}

/// Q38's limit: the argument is passed by address, so one of another size
/// would make the callee read or write the wrong bytes.
#[test]
fn integer_subrange_var_arguments_of_another_size() {
    let pre = "module m;
        type pinteger = 0..32767; linteger = 0..2147483647;
        procedure v32(var x: integer32); extern;
        procedure vli(var x: linteger); extern;
        procedure vpi(out x: pinteger); extern;";
    assert_err(
        &format!("{pre} procedure q; var i: integer16; begin vli(i) end;"),
        "argument type integer16 (2 bytes) does not match parameter type linteger (4 bytes)",
    );
    assert_err(
        &format!("{pre} procedure q; var p: pinteger; begin v32(p) end;"),
        "argument type pinteger (2 bytes) does not match parameter type integer32 (4 bytes)",
    );
    assert_err(
        &format!("{pre} procedure q; var i: integer32; begin vpi(i) end;"),
        "must have the parameter's size",
    );
    assert_err(
        "module m; type b = [byte] 0..127; procedure v(var x: integer16); extern;
         procedure q; var y: b; begin v(y) end;",
        "argument type b (1 byte) does not match parameter type integer16 (2 bytes)",
    );
    // integer16 for integer32 (no subrange) keeps the plain message.
    assert_err(
        &format!("{pre} procedure q; var i: integer16; begin v32(i) end;"),
        "argument type integer16 does not match parameter type integer32",
    );
}

/// Q38 covers integers only: enumeration, char and boolean subranges, and
/// structures, stay refused as `var` arguments of another type.
#[test]
fn non_integer_subrange_var_arguments_stay_refused() {
    let pre = "module m;
        type col = (r, g, b); rg = r..g; lc = 'a'..'z'; bs = false..true;
             rec = record a: integer end; rec2 = record a: integer end;
        procedure vc(var x: col); extern;
        procedure vch(var x: char); extern;
        procedure vb(var x: boolean); extern;
        procedure vr(var x: rec); extern;
        procedure vi(var x: integer16); extern;";
    for (decl, call, needle) in [
        (
            "v: rg",
            "vc(v)",
            "argument type rg does not match parameter type col",
        ),
        (
            "v: lc",
            "vch(v)",
            "argument type lc does not match parameter type char",
        ),
        (
            "v: bs",
            "vb(v)",
            "argument type bs does not match parameter type boolean",
        ),
        (
            "v: rec2",
            "vr(v)",
            "argument type rec2 does not match parameter type rec",
        ),
        (
            "v: rg",
            "vi(v)",
            "argument type rg does not match parameter type integer16",
        ),
        (
            "v: char",
            "vi(v)",
            "argument type char does not match parameter type integer16",
        ),
    ] {
        assert_err(
            &format!("{pre} procedure q; var {decl}; begin {call} end;"),
            needle,
        );
    }
}

#[test]
fn constant_index_out_of_range_under_addr_only() {
    // Best effort (bead domain-pascal-h4ba): one past the array as the
    // operand of `addr` is accepted (a warning); an access is not, nor an
    // index that is not the operand's last.
    let src = "module m; var a: array[1..8] of char; p: univ_ptr;
        procedure q; begin p := addr(a[9]) end;";
    assert_err(src, "index out of range"); // strict, the library default
    let before = dpc_syntax::diag::set_strict(false);
    ok(src);
    dpc_syntax::diag::take_best_effort();
    dpc_syntax::diag::set_strict(before);
    assert_err(
        "module m; var a: array[1..8] of char; c: char; procedure q; begin c := a[9] end;",
        "index out of range",
    );
    assert_err(
        "module m; type r = record b: array[1..2] of integer; end;
         var a: array[1..8] of r; p: univ_ptr;
         procedure q; begin p := addr(a[9].b[1]) end;",
        "index out of range",
    );
}
