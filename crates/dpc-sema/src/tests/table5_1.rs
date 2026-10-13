//! Table 5-1 (ref 5.2, p. 5-5, read from the PDF), row by row, plus the
//! frame offsets, pads and result conventions that follow from it.

use super::*;
use crate::conv::{Passing, ResultReg};
use crate::program::Param;

const A: Passing = Passing::ByAddress;
const V: Passing = Passing::ByValue;

/// The parameters of `procedure p(<params>); <opts>` in a module that
/// declares the types used below.
fn params(params: &str, opts: &str) -> Vec<Param> {
    let p = ok(&format!(
        "module m;
         type small = integer32;               {{ 4 bytes }}
              big = record a, b: integer32 end; {{ 8 bytes }}
              rec4 = record a: integer32 end;   {{ 4 bytes }}
              chars4 = array[1..4] of char;
              chars2 = array[1..2] of char;
              ptr = ^integer;
         procedure p({params}); {opts} extern;"
    ));
    routine(&p, "p").params.clone()
}

/// (passing, slot size, local copy) of the single parameter.
fn one(param: &str, opts: &str) -> (Passing, u32, bool) {
    let ps = params(param, opts);
    assert_eq!(ps.len(), 1);
    (
        ps[0].conv.passing,
        ps[0].conv.slot_size,
        ps[0].conv.local_copy,
    )
}

// --- Row "IN" ---------------------------------------------------------

#[test]
fn in_default_passes_address_no_copy() {
    // "Passes addresses of arguments." / "Makes indirect references".
    assert_eq!(one("in x: integer", ""), (A, 4, false));
    assert_eq!(one("in x: big", ""), (A, 4, false));
}

#[test]
fn in_small_val_param_or_internal_by_value() {
    // "val_param or internal: Causes the caller to pass the value of the
    // argument and the callee to make direct references to it."
    assert_eq!(one("in x: integer", "val_param;"), (V, 2, false));
    assert_eq!(one("in x: small", "val_param;"), (V, 4, false));
    assert_eq!(one("in x: char", "val_param;"), (V, 2, false));
    assert_eq!(one("in x: boolean", "internal;"), (V, 2, false));
    assert_eq!(one("in x: ptr", "internal;"), (V, 4, false));
}

#[test]
fn in_small_univ_cancels() {
    // "UNIV: Cancels the effect of var_param [sic] and internal."
    assert_eq!(one("in x: univ integer", "val_param;"), (A, 4, false));
    assert_eq!(one("in x: univ integer", "internal;"), (A, 4, false));
}

#[test]
fn in_big_no_effect() {
    // "> 4 bytes: val_param: No effect / internal: No effect / UNIV: No
    // effect".
    assert_eq!(one("in x: big", "val_param;"), (A, 4, false));
    assert_eq!(one("in x: big", "internal;"), (A, 4, false));
    assert_eq!(one("in x: univ big", "val_param;"), (A, 4, false));
}

// --- Row "IN OUT / VAR" (and OUT) ---------------------------------------

#[test]
fn var_and_in_out_always_by_address() {
    for mode in ["var", "in out", "out"] {
        for opts in ["", "val_param;", "internal;", "options(c_param);"] {
            for t in ["integer", "big", "univ integer"] {
                assert_eq!(
                    one(&format!("{mode} x: {t}"), opts),
                    (A, 4, false),
                    "{mode} {t} {opts}"
                );
            }
        }
    }
}

// --- Row "No mode" ------------------------------------------------------

#[test]
fn value_default_by_address_with_local_copy() {
    // "Passes addresses of arguments." / "Creates local copies of arguments
    // and makes direct references to them."
    assert_eq!(one("x: integer", ""), (A, 4, true));
    assert_eq!(one("x: big", ""), (A, 4, true));
}

#[test]
fn value_small_val_param_or_internal_by_value() {
    assert_eq!(one("x: integer", "val_param;"), (V, 2, false));
    assert_eq!(one("x: integer", "internal;"), (V, 2, false));
    assert_eq!(one("x: char", "val_param;"), (V, 2, false));
    assert_eq!(one("x: small", "val_param;"), (V, 4, false));
}

#[test]
fn value_small_univ_cancels_and_copies() {
    // "UNIV: Cancels the effect of val_param and internal ... it makes a
    // local copy as described by the parameter."
    assert_eq!(one("x: univ integer", "val_param;"), (A, 4, true));
    assert_eq!(one("x: univ integer", "internal;"), (A, 4, true));
}

#[test]
fn value_big_no_effect() {
    assert_eq!(one("x: big", "val_param;"), (A, 4, true));
    assert_eq!(one("x: big", "internal;"), (A, 4, true));
    assert_eq!(one("x: univ big", "val_param;"), (A, 4, true));
}

// --- Notes and other options -------------------------------------------

#[test]
fn character_arrays_by_address() {
    // 5.5.7 p. 5-19 "(except for character arrays)"; see the
    // TODO(ref-question) in conv.rs about the NOTE on p. 5-4.
    assert_eq!(one("x: chars4", "val_param;"), (A, 4, true));
    assert_eq!(one("in x: chars2", "val_param;"), (A, 4, false));
}

#[test]
fn small_record_under_val_param() {
    // 5.5.7 literal reading: "all arguments four bytes or less".
    assert_eq!(one("x: rec4", "val_param;"), (V, 4, false));
}

#[test]
fn c_param_passes_records_by_value() {
    // 5.5.12 p. 5-21: "pass all record data types by value".
    assert_eq!(one("x: big", "options(c_param);"), (V, 8, false));
    assert_eq!(one("x: integer", "options(c_param);"), (V, 2, false));
}

#[test]
fn default_routine_options_apply() {
    // ref 5.6.4 p. 5-22.
    let p = module(
        "routine_option default_routine_options = val_param;
         procedure a(x: integer); extern;
         procedure b(x: integer); options(abnormal, extern);",
    );
    assert_eq!(routine(&p, "a").params[0].conv.passing, V);
    // "You can override the default_routine_options ... by specifying
    // different routine options": b specified only abnormal.
    assert_eq!(routine(&p, "b").params[0].conv.passing, A);
    assert!(routine(&p, "b").options.abnormal);
    assert!(!routine(&p, "b").options.val_param);
}

#[test]
fn user_routine_option_names() {
    // ref 5.6.2 p. 5-22.
    let p = module(
        "routine_option my_c = a0_return, d0_return, c_param;
         function f(x: integer): integer; options(extern, my_c);",
    );
    let f = routine(&p, "f");
    assert!(f.options.c_param && f.options.a0_return && f.options.extern_);
    assert_eq!(f.params[0].conv.passing, V);
    assert_err(
        "module m; routine_option o = extern;",
        "cannot appear in a routine_option",
    );
}

#[test]
fn nested_routines_take_small_values_by_value() {
    // PMAP_$FLUSH 0xE13920..0xE1392A calls its nested routine 0xE1359C
    // with `move.w D6,-(SP); move.l D5,-(SP); pea (A2)`.
    let p = module(
        "procedure outer(var r: integer32);
           procedure inner(var a: integer32; b: integer32; c: integer);
           begin end;
         begin inner(r, 1, 2) end;",
    );
    let inner = routine(&p, "inner");
    let got: Vec<_> = inner
        .params
        .iter()
        .map(|p| (p.conv.passing, p.conv.slot_size, p.slot_offset))
        .collect();
    assert_eq!(got, vec![(A, 4, 8), (V, 4, 12), (V, 2, 16)]);
    assert_eq!(inner.arg_bytes, 10);
    assert_eq!(inner.arg_pad, 2); // `subq.l #2,SP` at 0xE13920
    assert!(!inner.global);
    assert_eq!(inner.level, 1);
    // outer itself is a default-convention exported routine.
    assert!(routine(&p, "outer").global);
}

// --- Frame offsets and the pad (codegen-patterns 1.1, 1.3, 1.4) --------

#[test]
fn frame_offsets_word_slots_and_pad() {
    // IO_$TRAP: (4) vector.w, (6) handler.l on entry, i.e. 8 and 10 after
    // link (../source/docs/pascal-abi.md); 6 bytes, so a 2-byte pad.
    let ps = params("vector: integer; handler: ptr", "val_param;");
    assert_eq!(ps[0].slot_offset, 8);
    assert_eq!(ps[1].slot_offset, 10);
    let p = ok("module m; type ptr = ^integer;
                procedure io_$trap(vector: integer; handler: ptr); val_param; extern;");
    let r = routine(&p, "io_$trap");
    assert_eq!((r.arg_bytes, r.arg_pad), (6, 2));

    // TIME_$Q_INIT_QUEUE 0xE16C5E: a byte at (8,A6), a word at (0xA,A6).
    let ps = params("c: char; w: integer", "val_param;");
    assert_eq!((ps[0].slot_offset, ps[0].value_offset), (8, 8));
    assert_eq!(ps[1].slot_offset, 10);

    // MMU_$INSTALL: ppn.l, va.l, asid.w, prot.w = 12 bytes, no pad.
    let p = ok("module m; procedure mmu_$install(ppn, va: integer32; asid, prot: integer); val_param; extern;");
    let r = routine(&p, "mmu_$install");
    let offs: Vec<i32> = r.params.iter().map(|p| p.slot_offset).collect();
    assert_eq!(offs, vec![8, 12, 16, 18]);
    assert_eq!((r.arg_bytes, r.arg_pad), (12, 0));

    // Default convention: everything is a 4-byte address.
    let ps = params("a: integer; var b: char; in c: big", "");
    let offs: Vec<i32> = ps.iter().map(|p| p.slot_offset).collect();
    assert_eq!(offs, vec![8, 12, 16]);
}

#[test]
fn local_copies_get_frame_slots() {
    let p = module(
        "type big = record a, b: integer32 end;
         procedure p(x: integer; y: big; in z: integer);
         var l: integer32;
         begin l := x; end;",
    );
    let r = routine(&p, "p");
    assert_eq!(r.params[0].copy_offset, Some(-2));
    assert_eq!(r.params[1].copy_offset, Some(-12));
    assert_eq!(r.params[2].copy_offset, None);
    assert_eq!(
        var(&p, "l").storage,
        crate::program::Storage::Frame { offset: -16 }
    );
    assert_eq!(r.frame_size, 16);
}

/// The frame-size model of codegen-patterns 2 ("Frame size"): a 4-byte
/// slot at the top for a routine that is not a global symbol, then the
/// locals, then the function result variable; the backend rounds the
/// total up to 4 (`M68k::link_size`, the `link` in each comment).
#[test]
fn frame_size_model() {
    let p = module(
        "type ptr = ^rec; rec = record next: ptr end;
         procedure exported_proc; begin end;
         procedure internal_proc; internal; begin end;
         function internal_fn(n: integer): integer; internal;
         begin internal_fn := n end;
         procedure internal_local(q: ptr); internal;
         var walk: ptr;
         begin walk := q end;
         function exported_fn: integer;
         var token: integer;
         begin token := 1; exported_fn := token end;
         function internal_fn_local: integer; internal;
         var packed_word: integer32;
         begin internal_fn_local := 0 end;",
    );
    let res_offset = |name: &str| {
        let r = routine(&p, name);
        p.var(r.result.as_ref().unwrap().var.unwrap())
            .storage
            .clone()
    };
    let frame = |off| crate::program::Storage::Frame { offset: off };
    // A map-named procedure without locals: `link #0` (TIME_$Q_INIT_QUEUE
    // 0xE16C5E, LOG_$READ 0xE17800).
    assert_eq!(routine(&p, "exported_proc").frame_size, 0);
    // An internal one: the slot alone, `link #-4`, never addressed
    // (0xE32B76).
    assert_eq!(routine(&p, "internal_proc").frame_size, 4);
    // Slot + a word result: `link #-8` (0xE68864).
    assert_eq!(routine(&p, "internal_fn").frame_size, 6);
    assert_eq!(res_offset("internal_fn"), frame(-6));
    // Slot + a pointer local, held in A1 in the image: `link #-8`
    // (0xE3CCCE).
    assert_eq!(var(&p, "walk").storage, frame(-8));
    assert_eq!(routine(&p, "internal_local").frame_size, 8);
    // A map-named function: its first local at the top, the result below
    // it, `link #-4` (PKT_$NEXT_ID 0xE1248E: a word at (-2,A6), the
    // result in D2).
    assert_eq!(var(&p, "token").storage, frame(-2));
    assert_eq!(res_offset("exported_fn"), frame(-4));
    assert_eq!(routine(&p, "exported_fn").frame_size, 4);
    // Slot + a 4-byte local + a word result: `link #-12` (0xE5A9CA, whose
    // local at (-8,A6) is read as a word and returned).
    assert_eq!(var(&p, "packed_word").storage, frame(-8));
    assert_eq!(res_offset("internal_fn_local"), frame(-10));
    assert_eq!(routine(&p, "internal_fn_local").frame_size, 10);
}

/// The frame objects of one routine never overlap and stay inside its
/// frame: the reserved slot, a value parameter's local copy, locals
/// declared before and after a nested routine, and the result variable
/// (placed after all the declarations, so after the nested routine's own
/// frame has been laid out).
#[test]
fn frame_objects_do_not_overlap() {
    let p = module(
        "type big = record a, b: integer32 end;
         function outer(y: big): integer; internal;
         var before: integer32;
           procedure inner;
           var deep: integer;
           begin deep := 0; before := 0 end;
         var after: char;
         begin outer := 0 end;",
    );
    let r = routine(&p, "outer");
    let frame = |v: crate::program::VarId| match p.var(v).storage {
        crate::program::Storage::Frame { offset } => offset,
        ref s => panic!("not in the frame: {s:?}"),
    };
    let mut objs: Vec<(i32, u32)> = r
        .frame_vars
        .iter()
        .map(|&v| (frame(v), p.types.size(p.var(v).ty)))
        .collect();
    objs.extend(
        r.params
            .iter()
            .filter_map(|q| q.copy_offset.map(|o| (o, p.types.size(q.ty)))),
    );
    // The slot of a routine that is not a global symbol: (-4..-1).
    objs.push((-4, 4));
    objs.sort();
    for w in objs.windows(2) {
        assert!(w[0].0 + w[0].1 as i32 <= w[1].0, "overlap: {objs:?}");
    }
    assert!(objs[0].0 >= -(r.frame_size as i32), "{objs:?}");
    // The result variable is the lowest object.
    let res = r.result.as_ref().unwrap().var.unwrap();
    assert_eq!(objs[0].0, frame(res));
    // The nested routine's own frame starts with its static-link slot.
    assert_eq!(
        var(&p, "deep").storage,
        crate::program::Storage::Frame { offset: -6 }
    );
}

// --- Function results (codegen-patterns 3; ref 5.5.10, 5.5.11) ----------

fn result_reg(decl: &str) -> ResultReg {
    let p = ok(&format!("module m; type ptr = ^integer; {decl}"));
    p.routines[0].result.as_ref().unwrap().reg
}

#[test]
fn result_registers() {
    use ResultReg::*;
    assert_eq!(result_reg("function f: boolean; extern;"), Data);
    assert_eq!(result_reg("function f: integer32; extern;"), Data);
    // Pointers in the address register by default (5.5.10).
    assert_eq!(result_reg("function f: ptr; extern;"), Address);
    assert_eq!(result_reg("function f: univ_ptr; extern;"), Address);
    // d0_return: both, callers read the data register.
    assert_eq!(
        result_reg("function f: ptr; options(extern, d0_return);"),
        AddressAndData
    );
    assert_eq!(
        result_reg("function f: ptr; options(extern, c_param);"),
        AddressAndData
    );
    // d0_return on a non-pointer changes nothing.
    assert_eq!(
        result_reg("function f: integer; options(extern, d0_return);"),
        Data
    );
    // a0_return: every register result in the address register (5.5.11).
    assert_eq!(
        result_reg("function f: integer; options(extern, a0_return);"),
        Address
    );
    assert_err(
        "module m; type s = set of 0..7; function f: s; extern;",
        "not supported yet",
    );
}
