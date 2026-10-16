//! Layout tests.  Every expected number is taken from the reference's own
//! examples (cited) or from a `_Static_assert` in `../source/domain_os`.

use super::*;
use crate::types::TypeKind;

// ---------------------------------------------------------------------
// Scalars (ref 3.2.4, 3.5.3, 3.6.4, 3.7.1, 3.8.1, 3.13.5)
// ---------------------------------------------------------------------

#[test]
fn scalar_sizes() {
    let p = module(
        "type
           i16 = integer; i = integer16; i32 = integer32;
           b = boolean; c = char;
           pets = (cats, dogs, dolphins, gorillas, pythons);
           teen = 13..19;
           caps = 'A'..'Z';
           small_pets = dogs..gorillas;
           u31 = 0..2147483647;
           s32 = -2147483648..2147483647;
           p = ^integer32;
           up = univ_ptr;",
    );
    // integer16 and integer are one type (ref 3.2.1, p. 3-4).
    assert_eq!(size(&p, "integer16"), 2);
    assert_eq!(size(&p, "integer32"), 4);
    assert_eq!(size(&p, "boolean"), 1);
    assert_eq!(size(&p, "char"), 1);
    // ref 3.7.1 p. 3-13: "one 16-bit word".
    assert_eq!(size(&p, "pets"), 2);
    // ref 3.8.1 p. 3-14: a subrange is stored like its base type; `13..19`
    // is a "Subrange of INTEGER" (3.8, p. 3-14).
    assert_eq!(size(&p, "teen"), 2);
    assert_eq!(size(&p, "caps"), 1);
    assert_eq!(size(&p, "small_pets"), 2);
    // ref 3.4 p. 3-10: "an unsigned subrange that is large enough to
    // require 31 bits, the compiler allocates 32 bits".
    assert_eq!(size(&p, "u31"), 4);
    assert_eq!(size(&p, "s32"), 4);
    // ref 3.13.5 p. 3-51: 32 bits, longword aligned.
    assert_eq!(size(&p, "p"), 4);
    assert_eq!(p.types.get(ty(&p, "p")).align, 4);
    assert_eq!(size(&p, "univ_ptr"), 4);
    // Freestanding alignment (ref 3.2.4 p. 3-5).
    assert_eq!(p.types.get(ty(&p, "integer16")).align, 2);
    assert_eq!(p.types.get(ty(&p, "integer32")).align, 4);
    assert_eq!(p.types.get(ty(&p, "char")).align, 1);
}

#[test]
fn predeclared_string_is_80_chars() {
    // ref 3.11, p. 3-38: "string = array[1..80] of CHAR;"; "Sizeof",
    // p. 4-181: "sizeof returns 80 because animal is declared to be a
    // string".
    let p = module(
        "type line = string;
         var animal: string; n: integer;
         procedure q; begin n := sizeof(animal); animal[80] := 'x' end;",
    );
    assert_eq!(size(&p, "string"), 80);
    // Like the other predeclared names, it can be redeclared.
    module("type string = array[1..10] of char; var s: string;");
}

// ---------------------------------------------------------------------
// Sets (ref 3.9.3, pp. 3-16..3-17)
// ---------------------------------------------------------------------

#[test]
fn set_sizes_from_the_reference() {
    let p = module(
        "type
           greek_letters = (alpha, beta, gamma, delta, epsilon, zeta, eta, theta,
                            iota, kappa, lambda, mu, nu, xi, omicron, pi, rho,
                            sigma, tau, upsilon, phi, chi, psi, omega);
           greek_set = set of greek_letters;
           possible_values = 80..170;
           small_letters = 'a'..'z';
           pos_t = set of possible_values;
           sma_t = set of small_letters;
           chars = set of char;
           bools = set of boolean;
           w1 = set of 0..15;
           w2 = set of 0..16;
           locks = set of 0..31;
           biggest = set of 0..1023;",
    );
    // "Greek_alphabet has 24 values ... allocates 32 bits (2 words)".
    assert_eq!(size(&p, "greek_set"), 4);
    // "Domain Pascal stores variable pos in 11 words (176 bits)".
    assert_eq!(size(&p, "pos_t"), 22);
    // "Domain Pascal stores variable sma in eight words (128 bits)".
    assert_eq!(size(&p, "sma_t"), 16);
    // 256 elements: the 32-byte `set of char` tables of OLD_DIR 0xE7FD24
    // (codegen-patterns 5.1).
    assert_eq!(size(&p, "chars"), 32);
    // "the minimum storage size for a set is one word".
    assert_eq!(size(&p, "bools"), 2);
    assert_eq!(size(&p, "w1"), 2);
    assert_eq!(size(&p, "w2"), 4);
    // PROC1 resource_locks_held: one longword (PROC1_$TST_LOCK 0xE148D8).
    assert_eq!(size(&p, "locks"), 4);
    // "the maximum size is 64 words (1024 bits)".
    assert_eq!(size(&p, "biggest"), 128);
}

#[test]
fn set_bit_numbering() {
    // ref 3.9.3 Figure 3-6 (PDF p. 71): Alpha is bit 0 of word 2, Omega
    // bit 7 of word 1.
    let p = module(
        "type
           greek_letters = (alpha, beta, gamma, delta, epsilon, zeta, eta, theta,
                            iota, kappa, lambda, mu, nu, xi, omicron, pi, rho,
                            sigma, tau, upsilon, phi, chi, psi, omega);
           greek_set = set of greek_letters;
           big = set of 0..63;",
    );
    let g = ty(&p, "greek_set");
    assert_eq!(p.types.set_bit_location(g, 0), (3, 0)); // alpha
    assert_eq!(p.types.set_bit_location(g, 1), (3, 1)); // beta
    assert_eq!(p.types.set_bit_location(g, 23), (1, 7)); // omega
                                                         // MST_$ALLOC_ASID 0xE42D5C: byte (63-k)>>3, bit k mod 8.
    let b = ty(&p, "big");
    for k in [0u32, 7, 8, 42, 63] {
        assert_eq!(p.types.set_bit_location(b, k), ((63 - k) >> 3, k % 8));
    }
}

#[test]
fn set_base_type_limits() {
    // ref 3.9.1 p. 3-15: no negative low end, high end at most 1023.
    assert_err("module m; type s = set of -1..5;", "0..1023");
    assert_err("module m; type s = set of 0..1024;", "0..1023");
    assert_err("module m; type s = set of integer;", "0..1023");
}

// ---------------------------------------------------------------------
// Records (ref 3.10.5, pp. 3-22..3-32; figures checked in the PDF)
// ---------------------------------------------------------------------

#[test]
fn record_s1_figure_3_8() {
    let p = module("type s1 = record a: integer32; b: char; c: integer16 end;");
    assert_eq!(offset(&p, "s1", "a"), 0);
    assert_eq!(offset(&p, "s1", "b"), 4);
    // "a byte of padding inserted after b to ensure that c is aligned on a
    // word boundary".
    assert_eq!(offset(&p, "s1", "c"), 6);
    assert_eq!(size(&p, "s1"), 8);
}

#[test]
fn record_s2_figure_3_9() {
    let p = module("type s2 = record c1: char; s1: integer16; c2: char end;");
    assert_eq!(offset(&p, "s2", "c1"), 0);
    assert_eq!(offset(&p, "s2", "s1"), 2);
    assert_eq!(offset(&p, "s2", "c2"), 4);
    // "a gap in the middle and a gap at the end".
    assert_eq!(size(&p, "s2"), 6);
}

#[test]
fn record_allocation_s3_s4_figures_3_10_3_11() {
    let p = module(
        "type s3 = record a: integer32; b: char; c: integer16 end;
              s4 = record a: integer16; b: integer32 end;",
    );
    // S3: every field naturally aligned; the largest is `a` (longword), so
    // "the compiler allocates records of type S3 on longword boundaries".
    assert_eq!(p.types.get(ty(&p, "s3")).align, 4);
    // S4: b at 2 is not naturally aligned; allocated on word boundaries.
    assert_eq!(offset(&p, "s4", "b"), 2);
    assert_eq!(size(&p, "s4"), 6);
    assert_eq!(p.types.get(ty(&p, "s4")).align, 2);
}

#[test]
fn record_foo_figures_3_12_3_13() {
    let p = module(
        "type foo = record a: char; b: integer16; c: char; d: double; e: integer32 end;
              foo2 = record d: double; e: integer32; b: integer16; a, c: char end;",
    );
    // Figure 3-12 (PDF p. 85): a 0, b 2, c 4, d 6..13, e 14..17.
    for (f, o) in [("a", 0), ("b", 2), ("c", 4), ("d", 6), ("e", 14)] {
        assert_eq!(offset(&p, "foo", f), o, "foo.{f}");
    }
    assert_eq!(size(&p, "foo"), 18);
    // Figure 3-13 (PDF p. 86): d 0, e 8, b 12, a 14, c 15.
    for (f, o) in [("d", 0), ("e", 8), ("b", 12), ("a", 14), ("c", 15)] {
        assert_eq!(offset(&p, "foo2", f), o, "foo2.{f}");
    }
    assert_eq!(size(&p, "foo2"), 16);
}

#[test]
fn array_of_records_figure_3_14() {
    let p = module(
        "type s1 = record x: integer32; y: integer16 end;
              arr = array[1..3] of s1;",
    );
    // The second element is at 6: "aligned on a word (2-byte) boundary,
    // not a longword (4-byte) boundary" (p. 3-32).
    assert_eq!(size(&p, "s1"), 6);
    assert_eq!(size(&p, "arr"), 18);
}

#[test]
fn unaligned_record_figure_3_16() {
    // ref 3.10.6 p. 3-34: unaligned records use the word rules.
    let p = module("type not_nat_rec = unaligned record a: integer; b: integer32; end;");
    assert_eq!(offset(&p, "not_nat_rec", "b"), 2);
    assert_eq!(size(&p, "not_nat_rec"), 6);
}

#[test]
fn smallest_record_is_two_bytes() {
    // ref 3.10.5.5 p. 3-26: "the smallest possible record is 2 bytes".
    let p = module("type r = record c: char end; e = record end;");
    assert_eq!(size(&p, "r"), 2);
    assert_eq!(size(&p, "e"), 2);
}

#[test]
fn nested_record_field_is_word_aligned() {
    // "all objects longer than a byte are aligned on word boundaries".
    let p = module(
        "type inner = record c: char end;
              outer = record a: char; i: inner; s: array[1..3] of char; z: char end;",
    );
    assert_eq!(offset(&p, "outer", "i"), 2);
    assert_eq!(offset(&p, "outer", "s"), 4);
    assert_eq!(offset(&p, "outer", "z"), 7);
    assert_eq!(size(&p, "outer"), 8);
}

#[test]
fn variant_record_arms_overlay() {
    let p = module(
        "type q_kind_t = (k_a, k_b);
              q_elem_t = record
                next: ^q_elem_t;
                key: integer32;
                case kind: q_kind_t of
                  k_a: (a: integer);
                  k_b: (b: packed array[0..3] of char; c: integer32);
              end;",
    );
    assert_eq!(offset(&p, "q_elem_t", "next"), 0);
    assert_eq!(offset(&p, "q_elem_t", "key"), 4);
    assert_eq!(offset(&p, "q_elem_t", "kind"), 8);
    assert_eq!(offset(&p, "q_elem_t", "a"), 10);
    assert_eq!(offset(&p, "q_elem_t", "b"), 10);
    assert_eq!(offset(&p, "q_elem_t", "c"), 14);
    assert_eq!(size(&p, "q_elem_t"), 18);
    // The forward pointer was resolved to the record itself.
    match p.types.kind(field(&p, "q_elem_t", "next").ty) {
        TypeKind::Pointer { target } => assert_eq!(*target, ty(&p, "q_elem_t")),
        k => panic!("{k:?}"),
    }
}

// ---------------------------------------------------------------------
// Arrays (ref 3.11.4, pp. 3-45..3-47)
// ---------------------------------------------------------------------

#[test]
fn array_sizes() {
    let p = module(
        "type bs = array[1..5] of boolean;
              cs = array[1..5] of char;
              two = array[1..2, 1..3] of integer16;
              v2 = varying[2] of char;
              v7 = varying[7] of char;
              v8 = varying[8] of char;
              pc = packed array[1..3] of char;",
    );
    // "Domain Pascal reserves six bytes of memory for b" (p. 3-46).
    assert_eq!(size(&p, "bs"), 6);
    assert_eq!(size(&p, "cs"), 5);
    assert_eq!(size(&p, "two"), 12);
    // Row-major (p. 3-46): one row is array[1..3] of integer16.
    match p.types.kind(ty(&p, "two")) {
        TypeKind::Array { elem, .. } => assert_eq!(p.types.size(*elem), 6),
        k => panic!("{k:?}"),
    }
    // "{8 bytes are allocated}", "{12 ...}", "{14 ...}" (p. 3-46).
    assert_eq!(size(&p, "v2"), 8);
    assert_eq!(size(&p, "v7"), 12);
    assert_eq!(size(&p, "v8"), 14);
    // Table 3-5: packed chars are bytes.
    assert_eq!(size(&p, "pc"), 3);
}

// ---------------------------------------------------------------------
// Kernel records with `_Static_assert`ed layouts in ../source
// ---------------------------------------------------------------------

/// `proc1_t`, ../source/domain_os/proc1/proc1.h (the asserts at lines
/// 86..111).  uint8_t fields are written as `char`, uint16_t as integer,
/// uint32_t as integer32, pointers as `^proc1_t`.
#[test]
fn proc1_pcb_matches_source_static_asserts() {
    let p = module(
        "type
           proc1_t = record
             nextp, prevp: ^proc1_t;
             save_d2, save_d3, save_d4, save_d5, save_d6, save_d7: integer32;
             save_a2, save_a3, save_a4, save_a5, save_a6, save_a7: integer32;
             save_usp: univ_ptr;
             wait_start: integer32;
             resource_locks_held: set of 0..31;
             mypid, asid, vtimer, pad_4a: integer;
             cpu_total: integer32;
             cpu_usage: integer;
             state: integer;
             pri_min, pri_max: char;
             inh_count, sw_bsr, nesting_depth: integer;
             field_5c, field_60, field_64: integer32;
           end;",
    );
    let expect = [
        ("nextp", 0x00),
        ("prevp", 0x04),
        ("save_d2", 0x08),
        ("save_d3", 0x0C),
        ("save_d4", 0x10),
        ("save_d5", 0x14),
        ("save_d6", 0x18),
        ("save_d7", 0x1C),
        ("save_a2", 0x20),
        ("save_a3", 0x24),
        ("save_a4", 0x28),
        ("save_a5", 0x2C),
        ("save_a6", 0x30),
        ("save_a7", 0x34),
        ("save_usp", 0x38),
        ("wait_start", 0x3C),
        ("resource_locks_held", 0x40),
        ("mypid", 0x44),
        ("asid", 0x46),
        ("vtimer", 0x48),
        ("pad_4a", 0x4A),
        ("cpu_total", 0x4C),
        ("cpu_usage", 0x50),
        ("state", 0x52),
        ("pri_min", 0x54),
        ("pri_max", 0x55),
        ("inh_count", 0x56),
        ("sw_bsr", 0x58),
        ("nesting_depth", 0x5A),
        ("field_5c", 0x5C),
        ("field_60", 0x60),
        ("field_64", 0x64),
    ];
    for (f, o) in expect {
        assert_eq!(offset(&p, "proc1_t", f), o, "proc1_t.{f}");
    }
    assert_eq!(size(&p, "proc1_t"), 0x68);
}

/// `route_$port_stats_t`, ../source/domain_os/route/route.h (asserts after
/// the struct): longwords on odd word boundaries, which the word-alignment
/// rules produce and natural alignment would not.  Byte 0 is a boolean
/// (NET_IO_$CREATE_PORT `tst.b` 0xE5A5D0).
#[test]
fn route_port_stats_matches_source_static_asserts() {
    let p = module(
        "type route_$port_stats_t = record
                in_use: boolean;
                deep_queue_puts: integer32;
                failed_puts: integer32;
                queue_depth: array[0..32] of integer32;
              end;",
    );
    assert_eq!(offset(&p, "route_$port_stats_t", "deep_queue_puts"), 0x02);
    assert_eq!(offset(&p, "route_$port_stats_t", "failed_puts"), 0x06);
    assert_eq!(offset(&p, "route_$port_stats_t", "queue_depth"), 0x0A);
    assert_eq!(size(&p, "route_$port_stats_t"), 0x8E);
}

#[test]
fn unsupported_layouts_are_diagnosed() {
    // Under the natural rules an array, set or varying field's alignment
    // is not stated (ref 3.10.5.2).
    assert_err(
        "module m; type r = aligned record a: char; b: array[1..2] of char end;",
        "natural alignment rules",
    );
    // Table 3-5 footnote 2: the size of an element over 16 bits is not
    // stated (packed_arrays.rs).
    assert_err(
        "module m; type a = packed array[1..8] of -40000..40000;",
        "packed array",
    );
}

// ---------------------------------------------------------------------
// Unsigned 16-bit subranges (reference-clarifications question 11,
// settled by the reference)
// ---------------------------------------------------------------------

#[test]
fn unsigned_16_bit_subranges_are_words() {
    // 5.3.4.1 p. 5-13: "pinteger = 0..65535 ... Even though pinteger and
    // integer are both 16-bit integers"; Table 7-2 p. 7-30: `0..65535` is
    // Domain/C's `unsigned short`; Table 3-7 p. 3-61: a subrange's size is
    // the "Number of bits needed to store subrange".
    let p = module(
        "type pinteger = 0..65535;
              byte_range = 0..255;
              u17 = 0..65536;
              s17 = -1..32768;
              r = record a: char; b: pinteger; c: integer32; d: pinteger end;
              a = array[1..3] of pinteger;",
    );
    assert_eq!(size(&p, "pinteger"), 2);
    assert_eq!(p.types.get(ty(&p, "pinteger")).align, 2);
    // Without a [byte] size attribute 0..255 is a word (3.15.5 p. 3-62:
    // "[byte] 0..255 corresponds to the unsigned char data type").
    assert_eq!(size(&p, "byte_range"), 2);
    // Ranges that need more than 16 bits are longwords (3.4 p. 3-10).
    assert_eq!(size(&p, "u17"), 4);
    assert_eq!(size(&p, "s17"), 4);
    assert_eq!(offset(&p, "r", "b"), 2);
    assert_eq!(offset(&p, "r", "c"), 4);
    assert_eq!(offset(&p, "r", "d"), 8);
    assert_eq!(size(&p, "r"), 10);
    assert_eq!(size(&p, "a"), 6);
    // Its values do not fit integer16: they may be assigned, and the
    // parameter goes in a word slot under val_param.
    let p = module(
        "type pinteger = 0..65535;
         var x: pinteger;
         procedure q(v: pinteger); val_param; extern;
         procedure t; begin x := 65535; q(40000) end;",
    );
    let q = routine(&p, "q");
    assert_eq!(q.params[0].conv.slot_size, 2);
    assert_eq!(p.types.size(var(&p, "x").ty), 2);
    // Out of its range: accepted, the low bytes kept (Appendix D, ISO
    // 6.4.6, p. D-3; clarification Q38).
    ok("module m; type pinteger = 0..65535; var x: pinteger;
        procedure t; begin x := 65536 end;");
}

// ---------------------------------------------------------------------
// More kernel records (../source/domain_os/dir/dir_internal.h, whose
// structs are `packed, aligned(2)` in C precisely because the original
// layout is word-aligned).  uint8_t -> char, uint16_t -> 0..65535,
// uint32_t/status_$t -> integer32, uid_t -> record of two integer32.
// ---------------------------------------------------------------------

const UID: &str = "uid_t = record high, low: integer32 end;";

/// `Dir_$OpResponse`: a variant part whose arms put UIDs and longwords at
/// word (not longword) offsets.  Asserts at dir_internal.h:117..139.
#[test]
fn dir_op_response_matches_source_static_asserts() {
    let p = module(&format!(
        "type {UID}
           dir_$op_response_t = record
             f12, f13, f14, f15: char;
             status: integer32;
             f18: array[1..12] of char;
             case integer of
               1: (uid: uid_t);
               2: (cookie: integer32);
               3: (word: 0..65535; entry_uid: uid_t; extra: integer32);
               4: (more, loop: char;
                   start_uid, resolved_uid: uid_t;
                   param5, param6, param7, param8, link_count: 0..65535;
                   redirect: integer32);
               5: (continuation, count, rsize, last_offset: integer32);
               6: (raw: array[1..16#34] of char);
           end;"
    ));
    let r = "dir_$op_response_t";
    assert_eq!(offset(&p, r, "status"), 0x04);
    assert_eq!(offset(&p, r, "f18"), 0x08);
    assert_eq!(offset(&p, r, "uid"), 0x14);
    assert_eq!(offset(&p, r, "cookie"), 0x14);
    assert_eq!(offset(&p, r, "word"), 0x14);
    assert_eq!(offset(&p, r, "entry_uid"), 0x16);
    assert_eq!(offset(&p, r, "extra"), 0x1E);
    assert_eq!(offset(&p, r, "start_uid"), 0x16);
    assert_eq!(offset(&p, r, "resolved_uid"), 0x1E);
    assert_eq!(offset(&p, r, "param5"), 0x26);
    assert_eq!(offset(&p, r, "link_count"), 0x2E);
    assert_eq!(offset(&p, r, "redirect"), 0x30);
    assert_eq!(offset(&p, r, "count"), 0x18);
    assert_eq!(offset(&p, r, "rsize"), 0x1C);
    assert_eq!(offset(&p, r, "last_offset"), 0x20);
    assert_eq!(size(&p, r), 0x48);
}

/// `dir_$do_op_request_t` with three of its body variants (asserts at
/// dir_internal.h:403..441): an odd-length byte array before a UID, and
/// bodies at 0x8E with longwords at 0x92 and 0x98.
#[test]
fn dir_do_op_request_matches_source_static_asserts() {
    let p = module(&format!(
        "type {UID}
           leaf_t = array[1..255] of char;
           dir_$req_name_t = record path_len: 0..65535; name: leaf_t end;
           dir_$req_add_entry_t = record
             path_len: 0..65535; file_uid: uid_t; flags: integer32; name: leaf_t;
           end;
           dir_$req_add_link_t = record
             path_len, target_len: 0..65535; target_ptr: integer32; name: leaf_t;
           end;
           dir_$req_uid_name_t = record
             path_len: 0..65535; target_uid: uid_t; name: leaf_t;
           end;
           dir_$do_op_request_t = record
             pad_00: array[1..3] of char;
             op: char;
             uid: uid_t;
             pad_0c, version, pad_10, reply_version: 0..65535;
             pad_14: array[1..16#7A] of char;
             case integer of
               1: (name: dir_$req_name_t);
               2: (add_entry: dir_$req_add_entry_t);
               3: (add_link: dir_$req_add_link_t);
               4: (uid_name: dir_$req_uid_name_t);
           end;"
    ));
    let r = "dir_$do_op_request_t";
    assert_eq!(offset(&p, r, "op"), 0x03);
    assert_eq!(offset(&p, r, "uid"), 0x04);
    assert_eq!(offset(&p, r, "version"), 0x0E);
    assert_eq!(offset(&p, r, "reply_version"), 0x12);
    for body in ["name", "add_entry", "add_link", "uid_name"] {
        assert_eq!(offset(&p, r, body), 0x8E, "{body}");
    }
    // Field offsets inside the bodies, plus 0x8E.
    let at = |rec: &str, f: &str| 0x8E + offset(&p, rec, f);
    assert_eq!(at("dir_$req_name_t", "name"), 0x90);
    assert_eq!(at("dir_$req_add_entry_t", "file_uid"), 0x90);
    assert_eq!(at("dir_$req_add_entry_t", "flags"), 0x98);
    assert_eq!(at("dir_$req_add_entry_t", "name"), 0x9C);
    assert_eq!(at("dir_$req_add_link_t", "target_len"), 0x90);
    assert_eq!(at("dir_$req_add_link_t", "target_ptr"), 0x92);
    assert_eq!(at("dir_$req_add_link_t", "name"), 0x96);
    assert_eq!(at("dir_$req_uid_name_t", "target_uid"), 0x90);
    assert_eq!(at("dir_$req_uid_name_t", "name"), 0x98);
}

/// `dir_$find_uid_result_t` (asserts at dir_internal.h:268..277).
#[test]
fn dir_find_uid_result_matches_source_static_asserts() {
    let p = module(&format!(
        "type {UID}
           dir_$find_uid_result_t = record
             word0, name_len: 0..65535;
             name: array[1..16#20] of char;
             uid: uid_t;
             extra: integer32;
           end;"
    ));
    let r = "dir_$find_uid_result_t";
    assert_eq!(offset(&p, r, "name_len"), 0x02);
    assert_eq!(offset(&p, r, "name"), 0x04);
    assert_eq!(offset(&p, r, "uid"), 0x24);
    assert_eq!(offset(&p, r, "extra"), 0x2C);
    assert_eq!(size(&p, r), 0x30);
}

#[test]
fn oversized_types_are_errors_not_overflows() {
    assert_err(
        "module m; type t = array[0..2147483647] of integer32;",
        "too large",
    );
    // 2^32 elements: the element count must not wrap to 0.
    assert_err(
        "module m; type t = array[-2147483648..2147483647] of char;",
        "too large",
    );
    assert_err(
        "module m; type t = array[1..500000000] of integer32;
         procedure p; var a, b, c: t; begin end;",
        "too large",
    );
    assert_err(
        "module m; type t = array[1..500000000] of integer32; var a, b, c: t;",
        "too large",
    );
    assert_err(
        "module m; type t = record a: array[1..1000000000] of integer16 end;
         procedure p(a, b, c: t); options(c_param, extern);",
        "too large",
    );
}
