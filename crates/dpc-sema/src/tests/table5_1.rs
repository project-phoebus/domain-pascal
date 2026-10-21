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
    // The copies go in front of the first local smaller than they are
    // (codegen-patterns 2, "Where the compiler's objects go"): y's above
    // l, at a multiple of 8 (bead domain-pascal-1edl); x's after it.
    assert_eq!(r.params[0].copy_offset, Some(-14));
    assert_eq!(r.params[1].copy_offset, Some(-8));
    assert_eq!(r.params[2].copy_offset, None);
    assert_eq!(
        var(&p, "l").storage,
        crate::program::Storage::Frame { offset: -12 }
    );
    assert_eq!(r.frame_size, 14);
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

/// Frame objects are at least word aligned (codegen-patterns 2, "Frame
/// layout"; bead domain-pascal-ek7): a lone byte local at (-2,A6)
/// (FILE_$REMOVE_WHEN_UNLOCKED 0xE5FC8E, `link #-4`), byte locals one to
/// a word (PROC2_$WAIT 0xE3FDD0), longwords longword aligned, a byte
/// result variable in a word of its own below the locals.
#[test]
fn frame_objects_word_aligned() {
    let p = module(
        "procedure one_byte; var lone: boolean; begin lone := true end;
         procedure bytes; var b1: boolean; c1: char; b2: boolean; l1: integer32;
         begin b1 := true end;
         function byte_fn: boolean; var w1: integer; begin byte_fn := true end;
         procedure internal_byte; internal; var ib: char; begin ib := 'a' end;",
    );
    let frame = |off| crate::program::Storage::Frame { offset: off };
    assert_eq!(var(&p, "lone").storage, frame(-2));
    assert_eq!(routine(&p, "one_byte").frame_size, 2);
    assert_eq!(var(&p, "b1").storage, frame(-2));
    assert_eq!(var(&p, "c1").storage, frame(-4));
    assert_eq!(var(&p, "b2").storage, frame(-6));
    assert_eq!(var(&p, "l1").storage, frame(-12));
    assert_eq!(routine(&p, "bytes").frame_size, 12);
    let r = routine(&p, "byte_fn");
    assert_eq!(var(&p, "w1").storage, frame(-2));
    assert_eq!(
        p.var(r.result.as_ref().unwrap().var.unwrap()).storage,
        frame(-4)
    );
    assert_eq!(r.frame_size, 4);
    // Below the reserved slot of a routine that is not global.
    assert_eq!(var(&p, "ib").storage, frame(-6));
    assert_eq!(routine(&p, "internal_byte").frame_size, 6);
}

/// FRAME_MIN_ALIGN only lifts 1-byte objects: odd-sized arrays, records
/// and varying strings already have word alignment from their own type
/// (ref 3.10.5.5 p. 3-26) and keep their own size (the frame does not
/// round an object: a 3-char array takes 3 bytes), and a byte value parameter's local copy is word aligned like a local,
/// while its argument slot (high byte of a word, codegen-patterns 1.3)
/// is not a frame object and does not move.
#[test]
fn frame_min_align_composites_and_copies() {
    let p = module(
        "type c3 = array[1..3] of char;
              r3 = record a, b, c: char end;
         procedure comp; var a3: c3; r: r3; v: varying[3] of char; b: boolean;
         begin b := true end;
         procedure copy(ch: char; flag: boolean); val_param;
         var x: char; begin x := ch end;
         procedure dflt(ch: char); var y: char; begin y := ch end;",
    );
    let frame = |off| crate::program::Storage::Frame { offset: off };
    let sz = |n| p.types.size(var(&p, n).ty);
    // A record's size is a whole number of words (its own rule).
    assert_eq!((sz("a3"), sz("r")), (3, 4));
    assert_eq!(var(&p, "a3").storage, frame(-4));
    assert_eq!(var(&p, "r").storage, frame(-8));
    assert_eq!(var(&p, "v").storage.clone(), frame(-8 - sz("v") as i32));
    assert_eq!(var(&p, "b").storage, frame(-8 - sz("v") as i32 - 2));
    // val_param: bytes by value in the high byte of a word slot, no copy.
    let r = routine(&p, "copy");
    let slots: Vec<(i32, u32, Option<i32>)> = r
        .params
        .iter()
        .map(|q| (q.slot_offset, q.conv.slot_size, q.copy_offset))
        .collect();
    assert_eq!(slots, vec![(8, 2, None), (10, 2, None)]);
    assert_eq!(var(&p, "x").storage, frame(-2));
    // Default convention: by address, the callee's copy word aligned,
    // after the local of its own size (codegen-patterns 2, "Where the
    // compiler's objects go").
    let r = routine(&p, "dflt");
    assert_eq!(r.params[0].copy_offset, Some(-4));
    assert_eq!(var(&p, "y").storage, frame(-2));
    assert_eq!(r.frame_size, 4);
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

/// Frame objects of 8 bytes or more lie at multiples of 8 from the frame
/// pointer (codegen-patterns 2, "Objects of 8 bytes or more at multiples
/// of 8"; bead domain-pascal-1edl), smaller ones keep their type's
/// alignment, and the total is not rounded to 8.
#[test]
fn frame_big_objects_8_aligned() {
    let p = module(
        "type value_t = record raw: array[1..16#34] of char end;
              ec_t = record v, a, t: integer32 end;
              q_t = record x: array[1..13] of integer end;
              list_t = array[1..3] of integer32;
              clock_t = record high: integer32; low: integer end;
              six_t = array[1..6] of char;
              uid_$t = record high, low: integer32 end;
         procedure set_trouble; var value: value_t; begin end;
         procedure i_wait; internal;
         var ec: ec_t; q: q_t; l1, l2: list_t; c1, c2: clock_t;
         begin end;
         procedure mount; var rec: record r: array[1..16#32] of char end;
           st: integer32; bst: integer32; flags: integer;
         begin end;
         function small: integer32; var w: integer; s: six_t; begin small := 0 end;
         function uid_fn(n: integer): uid_$t; internal;
         var u: uid_$t; begin uid_fn := u end;",
    );
    let frame = |off| crate::program::Storage::Frame { offset: off };
    // FILE_$SET_TROUBLE 0xE5DC42: 0x34 bytes at -0x38, `link #-0x38`.
    assert_eq!(var(&p, "value").storage, frame(-0x38));
    assert_eq!(routine(&p, "set_trouble").frame_size, 0x38);
    // tty_$i_wait 0xE1C204: below the reserved slot, -0x10, -0x30, -0x40,
    // -0x50; the 6-byte clocks longword placed, -0x58 and -0x60.
    let offs: Vec<_> = ["ec", "q", "l1", "l2", "c1", "c2"]
        .iter()
        .map(|n| var(&p, n).storage.clone())
        .collect();
    let want: Vec<_> = [-0x10, -0x30, -0x40, -0x50, -0x58, -0x60]
        .into_iter()
        .map(frame)
        .collect();
    assert_eq!(offs, want);
    // VTOC_$MOUNT 0xE38658 (`link #-0x4c`): the 0x32-byte record at
    // -0x38, the statuses below it longword placed; the total is a
    // multiple of 4, not of 8.
    assert_eq!(var(&p, "rec").storage, frame(-0x38));
    assert_eq!(var(&p, "st").storage, frame(-0x3c));
    assert_eq!(var(&p, "bst").storage, frame(-0x40));
    assert_eq!(routine(&p, "mount").frame_size, 0x42);
    // A 6-byte array of char stays word aligned (WIN_$CINIT 0xE30340:
    // the address (-0xA,A6) passed under a longword at -4); here the
    // result variable goes above the word local.
    assert_eq!(var(&p, "s").storage, frame(-12));
    // The module-local 0xE4232A (`link #-16`): an 8-byte local at -0x10
    // below the reserved slot.
    assert_eq!(var(&p, "u").storage, frame(-0x10));
}

/// The objects the compiler makes, value parameters' copies and the
/// function result variable, go in front of the first declared local
/// that takes fewer frame bytes than they do, or after all of them; the
/// larger first, and the copies of one size at rising addresses in
/// parameter order (codegen-patterns 2, "Where the compiler's objects
/// go"; bead domain-pascal-67c).
#[test]
fn compiler_objects_before_smaller_locals() {
    let p = module(
        "type uid_$t = record high, low: integer32 end;
              opts_t = record a, b, c: integer32; d: integer end;
              tmpl_t = record w: array[1..15] of integer end;
              v_t = record raw: array[1..16#34] of char end;
         function rtnva: integer32; var token: integer;
         begin rtnva := 0 end;
         function next_id: integer; var token2: integer;
         begin next_id := 0 end;
         function get_pid: integer; var st: integer32; ix: integer;
         begin get_pid := 0 end;
         procedure send(a: integer32; b, c, d: integer);
         var tmpl: tmpl_t; l: integer32; begin end;
         procedure ptrace(u: uid_$t; o: opts_t); var n: integer32; begin end;
         procedure set_type(t: uid_$t); var value: v_t; begin end;",
    );
    let frame = |off| crate::program::Storage::Frame { offset: off };
    let res = |name: &str| {
        let r = routine(&p, name);
        p.var(r.result.as_ref().unwrap().var.unwrap())
            .storage
            .clone()
    };
    let copies = |name: &str| -> Vec<Option<i32>> {
        routine(&p, name)
            .params
            .iter()
            .map(|q| q.copy_offset)
            .collect()
    };
    // NETBUF_$RTNVA 0xE0ED26: the 4-byte result above the word token.
    assert_eq!(res("rtnva"), frame(-4));
    assert_eq!(var(&p, "token").storage, frame(-6));
    // PKT_$NEXT_ID 0xE1248E: a word result below the word token.
    assert_eq!(var(&p, "token2").storage, frame(-2));
    assert_eq!(res("next_id"), frame(-4));
    // PROC2_$GET_PID 0xE40CBA: status, index, the word result last.
    assert_eq!(res("get_pid"), frame(-8));
    // MSG_$SEND 0xE599FC: after the locals, the longword's copy, then the
    // words', parameter 2 lowest.
    assert_eq!(var(&p, "tmpl").storage, frame(-0x20));
    assert_eq!(var(&p, "l").storage, frame(-0x24));
    assert_eq!(
        copies("send"),
        vec![Some(-0x28), Some(-0x2e), Some(-0x2c), Some(-0x2a)]
    );
    // XPD_$SET_PTRACE_OPTS 0xE5AF9E: the 14-byte copy at -0x10, the
    // UID's at -0x18, both above the longword local at -0x1C.
    assert_eq!(copies("ptrace"), vec![Some(-0x18), Some(-0x10)]);
    assert_eq!(var(&p, "n").storage, frame(-0x1c));
    // FILE_$SET_TYPE 0xE5E176: the UID's copy below the 0x34-byte local.
    assert_eq!(var(&p, "value").storage, frame(-0x38));
    assert_eq!(copies("set_type"), vec![Some(-0x40)]);
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
    // Sets of one word or one longword: in D0 (codegen-patterns 3, "Set
    // results": ACL_$MIN_RIGHTS 0xE469F2, PROC2_$SIGBLOCK 0xE3F6B4).
    assert_eq!(
        result_reg("type s = set of 0..7; function f: s; extern;"),
        Data
    );
    assert_eq!(
        result_reg("type s = set of 0..31; function f: s; extern;"),
        Data
    );
    // Larger sets, reals and records other than 4-byte ones: not known
    // yet; the declaration is accepted (Apollo's inserts declare such
    // functions), and `dpc-ir` refuses definitions and calls.
    assert_eq!(
        result_reg("type s = set of 0..63; function f: s; extern;"),
        Unknown
    );
    assert_eq!(result_reg("function f: real; extern;"), Unknown);
    // A 4-byte record, base.ins.pas's `status_$t` (codegen-patterns 3,
    // "Record results"): in D0.  `d0_return`/`c_param` (pointers only,
    // 5.5.10, 5.5.12) change nothing.
    let status = "type status_$t = packed record case integer of
        1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
        2: (all: integer32);
      end;";
    assert_eq!(
        result_reg(&format!("{status} function f: status_$t; extern;")),
        Record
    );
    assert_eq!(
        result_reg(&format!(
            "{status} function f: status_$t; options(extern, d0_return);"
        )),
        Record
    );
    assert_eq!(
        result_reg("type r = record a, b: integer; end; function f: r; extern;"),
        Record
    );
    // a0_return on one (5.5.11): no site, not known.
    assert_eq!(
        result_reg(&format!(
            "{status} function f: status_$t; options(extern, a0_return);"
        )),
        Unknown
    );
    // Smaller records: not known.
    assert_eq!(
        result_reg("type r = record a: integer; end; function f: r; extern;"),
        Unknown
    );
    // Larger ones through a hidden address (codegen-patterns 3, "Record
    // results over 4 bytes": uid_$t at ACL_$DEFAULT_ACL 0xE4787E,
    // time_$clock_t at the module-local 0xE58C02), except under a0_return,
    // d0_return and c_param (no site).
    assert_eq!(
        result_reg("type r = record a, b: integer32; end; function f: r; extern;"),
        Hidden
    );
    assert_eq!(
        result_reg("type r = record a: integer32; b: integer; end; function f: r; extern;"),
        Hidden
    );
    for opt in ["a0_return", "d0_return", "c_param"] {
        assert_eq!(
            result_reg(&format!(
                "type r = record a, b: integer32; end; function f: r; options(extern, {opt});"
            )),
            Unknown,
            "{opt}"
        );
    }
    // A 4-byte array is no record.
    assert_eq!(
        result_reg("type a = array[1..2] of integer; function f: a; extern;"),
        Unknown
    );
}

/// A result through a hidden address takes the first argument slot: the
/// parameters follow it, the caller pushes no pad whatever the total
/// (PROC2_$UPGID_TO_UID 0xE41030..0xE4103C, a word and the address, 6
/// bytes), and the result variable is that slot's object, with no frame
/// space (ACL_$DEFAULT_ACL 0xE4787E `link #0`).
#[test]
fn hidden_result_slot() {
    let p = ok("module m; type r = record a, b: integer32; end; var v: r;
        function f(w: integer): r; internal;
        begin f := v; end;");
    let f = &p.routines[0];
    assert_eq!(f.params[0].slot_offset, 0xc);
    assert_eq!(f.params[0].value_offset, 0xc);
    assert_eq!(f.arg_bytes, 6);
    assert_eq!(f.arg_pad, 0);
    let res = f.result.as_ref().unwrap();
    assert_eq!(res.reg, ResultReg::Hidden);
    assert_eq!(
        p.vars[res.var.unwrap().0 as usize].storage,
        crate::program::Storage::HiddenResult { slot: 8 }
    );
    // The frame: the reserved slot of an `internal` routine only.
    assert_eq!(f.frame_size, 4);
    let p = ok("module m; type r = record a, b: integer32; end;
        function g(in x: integer32; in y: r): r; begin g := y; end;");
    let g = &p.routines[0];
    assert_eq!(
        g.params.iter().map(|q| q.slot_offset).collect::<Vec<_>>(),
        [0xc, 0x10]
    );
    assert_eq!((g.arg_bytes, g.arg_pad), (12, 0));
    assert_eq!(g.frame_size, 0);
}
