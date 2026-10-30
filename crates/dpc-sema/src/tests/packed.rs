//! Packed record layout (ref 3.10.7, Table 3-3, pp. 3-35..3-37).
//!
//! The declarations are Apollo's own, copied from the SR10.2 customer
//! insert files (`sr10.2-install/install/ri.apollo.os.v.10.2/sys/ins/`,
//! codegen-patterns 0.1), with attributes the checker does not take yet
//! left out where they do not change the layout tested.  Each expected
//! position is cross-checked against an independent source: the image, a
//! `_Static_assert` in `../source/domain_os`, the insert file's own
//! comments, or the reference's Figure 3-17.

use super::*;
use crate::types::{BitPos, TypeKind};

fn bits(p: &Program, rec: &str, f: &str) -> (u32, u32) {
    let b = field(p, rec, f)
        .bits
        .unwrap_or_else(|| panic!("{rec}.{f} has no bits"));
    (b.bit, b.width)
}

fn align(p: &Program, name: &str) -> u32 {
    p.types.get(ty(p, name)).align
}

/// Ref 3.10.7 pp. 3-36..3-37, Figure 3-17 (read in the PDF): Mass 32 bits
/// word aligned, Shape 3 bits, B 1 bit, Purpose 4 bits, Low_temp 8 bits
/// (-100..40: "seven bits can represent only 128 values"), Class 8 bits
/// (a subrange of char, byte aligned): eight bytes, the last byte unused.
#[test]
fn reference_figure_3_17() {
    let p = module(
        "type
           Shapes = (Sphere, Cube, Ovoid, Cylinder, Tetrahedron);
           Uses = (Toy, Tool, Weapon, Food);
           Characteristics = PACKED RECORD
             Mass : Real;
             Shape : Shapes;
             B : Boolean;
             Purpose : SET OF Uses;
             Low_temp : -100..40;
             Class : 'A'..'Z';
           end;",
    );
    let r = "characteristics";
    assert_eq!(bits(&p, r, "mass"), (0, 32));
    assert_eq!(bits(&p, r, "shape"), (32, 3)); // bits 15..13 of word 2
    assert_eq!(bits(&p, r, "b"), (35, 1)); // bit 12
    assert_eq!(bits(&p, r, "purpose"), (36, 4)); // bits 11..8
    assert_eq!(bits(&p, r, "low_temp"), (40, 8)); // bits 7..0
    assert_eq!(bits(&p, r, "class"), (48, 8)); // word 3, high byte
    assert_eq!(offset(&p, r, "class"), 6);
    assert_eq!(size(&p, r), 8);
}

/// Ref 3.10.3 p. 3-21's `student` and the 16-bit minimum (p. 3-35).
#[test]
fn small_packed_records_take_a_word() {
    let p = module(
        "type
           student = PACKED record
             ages : 10..20;
             grade : (seventh, eighth, ninth, tenth, eleventh, twelfth);
             graduating : boolean;
           end;
           one = packed record b: boolean end;",
    );
    assert_eq!(bits(&p, "student", "ages"), (0, 5));
    assert_eq!(bits(&p, "student", "grade"), (5, 3));
    assert_eq!(bits(&p, "student", "graduating"), (8, 1));
    assert_eq!(size(&p, "student"), 2);
    assert_eq!(size(&p, "one"), 2);
}

/// The 16-bit boundary rule (p. 3-35): a field that would cross two or
/// more 16-bit boundaries starts at the next one; crossing one is fine.
#[test]
fn boundary_crossing_rule() {
    let p = module(
        "type r = packed record
                    a: 0..1023;        { 10 bits at 0 }
                    b: 0..16777215;    { 24 bits: 10..33 crosses 16 and 32 }
                    c: 0..4095;        { 12 bits at 40: crosses 48 only }
                    d: boolean;        { 52 }
                    e: integer;        { word aligned: 64 }
                    f: char;           { byte aligned: 80 }
                  end;",
    );
    assert_eq!(bits(&p, "r", "a"), (0, 10));
    assert_eq!(bits(&p, "r", "b"), (16, 24));
    assert_eq!(bits(&p, "r", "c"), (40, 12));
    assert_eq!(bits(&p, "r", "d"), (52, 1));
    assert_eq!(bits(&p, "r", "e"), (64, 16));
    assert_eq!(bits(&p, "r", "f"), (80, 8));
    assert_eq!(size(&p, "r"), 12);
}

/// Owner ruling (2026-10-30) on reference-clarifications item 51, in a
/// packed record: for the MC68020 targets an explicit `aligned(0)` field
/// starts at the next free byte and may cross two 16-bit boundaries
/// (../source reproducer g231: `id: [aligned(0)] integer32` at byte 3 of
/// the 8-byte PROM device record, io_$match_device 0x3C439032).  The
/// MC68010 places it by Table 3-3 (word aligned).  Bead domain-pascal-meel.
#[test]
fn byte_aligned_field_crossing_two_words() {
    let src = "type
           d_t = packed record f: 0..255; u: 0..255; c: 0..255;
                               id: [aligned(0)] integer32; z: char end;
           b_t = packed record b: boolean; id: [aligned(0)] integer32 end;";
    let p = module(src);
    assert_eq!(bits(&p, "d_t", "id"), (32, 32));
    assert_eq!(bits(&p, "d_t", "z"), (64, 8));
    assert_eq!(size(&p, "d_t"), 10);
    assert_eq!(bits(&p, "b_t", "id"), (16, 32));
    assert_eq!(size(&p, "b_t"), 6);
    let p = module_68020(src);
    assert_eq!(bits(&p, "d_t", "id"), (24, 32));
    assert_eq!(offset(&p, "d_t", "id"), 3);
    assert_eq!(bits(&p, "d_t", "z"), (56, 8));
    assert_eq!(size(&p, "d_t"), 8);
    assert_eq!(bits(&p, "b_t", "id"), (8, 32));
    assert_eq!(size(&p, "b_t"), 6);
}

/// `status_$t` (base.ins.pas).  Image: `fail` is bit 7 of the first byte
/// (PROC2_$RESUME 0xE41444 `bset #7,(-0x4,A6)`), `code` the low word
/// (0xE4142A `tst.w (-0x2,A6)`); `all` the whole longword (codegen-patterns
/// 9).
#[test]
fn apollo_status_t() {
    let p = module(
        "type
           status_$t = PACKED RECORD CASE integer OF
               1:  (fail:  boolean;
                    subsys: 0..127;
                    modc: 0..255;
                    code: integer);
               2:  (all: integer32);
               END;",
    );
    let r = "status_$t";
    assert_eq!(bits(&p, r, "fail"), (0, 1));
    assert_eq!(bits(&p, r, "subsys"), (1, 7));
    assert_eq!(bits(&p, r, "modc"), (8, 8));
    assert_eq!(bits(&p, r, "code"), (16, 16));
    assert_eq!(offset(&p, r, "code"), 2);
    assert_eq!(bits(&p, r, "all"), (0, 32));
    assert_eq!(size(&p, r), 4);
    // `all`, an integer32 at 0, is naturally aligned (ref 3.10.5.6).
    assert_eq!(align(&p, r), 4);
}

/// `uid_$t` and `time_$clock_t` (base.ins.pas; the clock's `[aligned(1)]`
/// of the SR10 68k branch left out, it is the `%ELSE` branch's
/// declaration).  `../source` base/base.h: `uid_t.high` at 0, `.low` at 4;
/// `clock_t` six bytes; tty/tty.h: the clock's `high` longword at 0x2C4 and
/// `low` word at 0x2C8.  Image: TTY_$I_WAIT 0xE1C2AA `clr.w (-0x60,A6)`,
/// 0xE1C2C0 `move.l D0,(-0x5e,A6)`: `high16` a word at 0, `low32` a
/// longword at 2.  `linteger` (0..2147483647) takes 32 bits (ref 3.4 p.
/// 3-10), not 31, where it starts on a word boundary (`low32` at bit 16
/// is stored with `move.l`); see [`unsigned_31_bit_subrange_after_a_boolean`].
#[test]
fn apollo_uid_and_clock() {
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           time_$clockh_t = linteger;
           uid_$t = PACKED RECORD
               high: integer32;
               low: integer32;
               END;
           time_$clock_t = PACKED RECORD CASE integer OF
               1:  (high:  time_$clockh_t;
                    low:  pinteger);
               2:  (high16:  pinteger;
                    low32:  linteger);
               END;",
    );
    assert_eq!(bits(&p, "uid_$t", "high"), (0, 32));
    assert_eq!(bits(&p, "uid_$t", "low"), (32, 32));
    assert_eq!(size(&p, "uid_$t"), 8);
    assert_eq!(align(&p, "uid_$t"), 4);
    let c = "time_$clock_t";
    assert_eq!(bits(&p, c, "high"), (0, 32));
    assert_eq!(bits(&p, c, "low"), (32, 16));
    assert_eq!(bits(&p, c, "high16"), (0, 16));
    assert_eq!(bits(&p, c, "low32"), (16, 32));
    assert_eq!(size(&p, c), 6);
}

/// `cal_$timezone_rec_t` (cal.ins.pas; the `[aligned(1)]` on `drift` left
/// out).  `../source` cal/cal.h: `utc_delta` 0, `tz_name` 2, `drift` 6,
/// size 12 (three longword moves in the image, CAL_$GET_INFO 0xE68576).
/// Also `cal_$timedate_rec_t`, six integers.
#[test]
fn apollo_cal_records() {
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           time_$clock_t = PACKED RECORD CASE integer OF
               1:  (high:  linteger; low:  pinteger);
               2:  (high16:  pinteger; low32:  linteger);
               END;
           cal_$tz_name_t = array[1..4] of char;
           cal_$timedate_rec_t = PACKED RECORD
               year: integer; month: integer; day: integer;
               hour: integer; minute: integer; second: integer
               END;
           cal_$timezone_rec_t = PACKED RECORD
               utc_delta:  integer;
               tz_name:  cal_$tz_name_t;
               drift:  time_$clock_t;
               END;",
    );
    let r = "cal_$timezone_rec_t";
    assert_eq!(offset(&p, r, "utc_delta"), 0);
    assert_eq!(offset(&p, r, "tz_name"), 2);
    assert_eq!(offset(&p, r, "drift"), 6);
    assert_eq!(size(&p, r), 12);
    assert_eq!(offset(&p, "cal_$timedate_rec_t", "second"), 10);
    assert_eq!(size(&p, "cal_$timedate_rec_t"), 12);
}

/// `fontn_$table_tver1` (fontn.ins.pas), the version-3 font header that
/// `../source` smd/smd_internal.h declares as `smd_font_v3_t`: its asserted
/// offsets 0x28 (`image_offset`, there `data_offset`), 0x2C, 0x34
/// (`index_table_offset`, `char_map_offset`), 0x38, 0x5A (`char_spacing`,
/// here `inter_space_dflt`) and 0x6E (`default_missing`, here
/// `space_size`) need `linteger` to be 32 bits and `old_proportional`
/// (1 bit) with `space_type` (0..16#7FFF, 15 bits) to share one word.
#[test]
fn apollo_font_header() {
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           fontn_$spacing_t = 0..16#7FFF;
           fontn_$rel_number_t = PACKED RECORD
                                   fontn_major, fontn_minor : integer16;
                                   END;
           fontn_$table_tver1 = PACKED RECORD
               version:           pinteger;
               font_type:         pinteger;
               release_number:    fontn_$rel_number_t;
               header_length:     pinteger;
               old_proportional:  boolean;
               space_type:        fontn_$spacing_t;
               comparison_set:    univ_ptr;
               scaling_factor:    real;
               stroke_multiplier: real;
               chars_in_font:     linteger;
               lowest_char_code:  linteger;
               highest_char_code: linteger;
               max_char_index:    linteger;
               image_offset:      linteger;
               image_size:        linteger;
               char_string_offset:linteger;
               index_table_offset:linteger;
               desc_table_offset: linteger;
               point_size:        pinteger;
               point_res:         pinteger;
               bytes_per_char:    pinteger;
               raster_lines:      pinteger;
               max_height:        pinteger;
               max_width:         pinteger;
               max_up:            pinteger;
               max_down:          pinteger;
               max_right:         pinteger;
               v_spacing:         pinteger;
               font_height:       pinteger;
               x_height:          pinteger;
               cap_offset:        pinteger;
               ascender_height:   pinteger;
               descender_depth:   pinteger;
               inter_space_dflt:  pinteger;
               default_char:      pinteger;
               em_space:          pinteger;
               en_space:          pinteger;
               digit_width:       pinteger;
               baseline_offset:   pinteger;
               underline_offset:  pinteger;
               underline_width:   pinteger;
               overstrike_offset: pinteger;
               overstrike_width:  pinteger;
               space_size:        pinteger;
               pad_01:            linteger;
               pad_02:            linteger;
               pad_03:            linteger;
               pad_04:            linteger;
               END ;",
    );
    let r = "fontn_$table_tver1";
    assert_eq!(bits(&p, r, "old_proportional"), (80, 1));
    assert_eq!(bits(&p, r, "space_type"), (81, 15));
    assert_eq!(offset(&p, r, "comparison_set"), 0x0C);
    assert_eq!(offset(&p, r, "image_offset"), 0x28);
    assert_eq!(offset(&p, r, "image_size"), 0x2C);
    assert_eq!(offset(&p, r, "index_table_offset"), 0x34);
    assert_eq!(offset(&p, r, "desc_table_offset"), 0x38);
    assert_eq!(offset(&p, r, "inter_space_dflt"), 0x5A);
    assert_eq!(offset(&p, r, "space_size"), 0x6E);
    // "pad...reserved for future use...8-byte multiple"
    assert_eq!(size(&p, r), 128);
}

/// The version-1 font table and its descriptor entries (smdu.ins.pas,
/// fontn.ins.pas): `../source` smd_font_v1_t (13 words, `char_map` at
/// 0x1A) and smd_glyph_metrics_t (bytes at 0..5, `bitmap_row` at 6, eight
/// bytes per glyph).
#[test]
fn apollo_font_tables() {
    let p = module(
        "type
           pinteger = 0..65535;
           binteger = 0..255;
           fontn_$desc = PACKED RECORD
                           left:       -128..127;
                           right:      -128..127;
                           up:         -128..127;
                           down:       -128..127;
                           width:      binteger;
                           x_pos:      binteger;
                           y_pos:      pinteger;
                         END;
           smd_$v1_fdte_t = PACKED RECORD
               left:  char ; right:  char ; up:  char ; down:  char ;
               width:  char ; x_pos:  char ; y_pos:  pinteger ;
               END ;
           smd_$font_table_t = PACKED RECORD
               version:  pinteger ; image_offset:  pinteger ;
               chars_in_font:  pinteger ; raster_lines:  pinteger ;
               image_size:  pinteger ; max_height:  pinteger ;
               max_width:  pinteger ; v_spacing:  pinteger ;
               h_spacing:  pinteger ; space_size:  pinteger ;
               max_right:  pinteger ; max_up:  pinteger ;
               max_down:  pinteger ;
               index_table:  ARRAY [0..127] OF char ;
               desc_table:  ARRAY [1..127] OF smd_$v1_fdte_t ;
               END ;",
    );
    for (f, b) in [("left", 0), ("right", 8), ("up", 16), ("down", 24)] {
        assert_eq!(bits(&p, "fontn_$desc", f), (b, 8));
    }
    assert_eq!(bits(&p, "fontn_$desc", "width"), (32, 8));
    assert_eq!(bits(&p, "fontn_$desc", "x_pos"), (40, 8));
    assert_eq!(offset(&p, "fontn_$desc", "y_pos"), 6);
    assert_eq!(size(&p, "fontn_$desc"), 8);
    assert_eq!(offset(&p, "smd_$v1_fdte_t", "x_pos"), 5);
    assert_eq!(offset(&p, "smd_$v1_fdte_t", "y_pos"), 6);
    assert_eq!(size(&p, "smd_$v1_fdte_t"), 8);
    assert_eq!(offset(&p, "smd_$font_table_t", "max_down"), 0x18);
    assert_eq!(offset(&p, "smd_$font_table_t", "index_table"), 0x1A);
    assert_eq!(offset(&p, "smd_$font_table_t", "desc_table"), 0x9A);
    assert_eq!(size(&p, "smd_$font_table_t"), 0x9A + 127 * 8);
}

/// `smd_$display_modereg_t` (smdu.ins.pas), whose comments give each
/// field's mask in the register word: blt 8000, dummy 7E00, noninterlace
/// 100, from_mm 80, ..., dispon 1.
#[test]
fn apollo_display_mode_register() {
    let p = module(
        "type smd_$display_modereg_t = PACKED RECORD
               blt:  boolean ;
               dummy:  0..63 ;
               noninterlace:  boolean ;
               from_mm:  boolean ;
               to_mm:  boolean ;
               ieof:  boolean ;
               idone:  boolean ;
               nonconform:  boolean ;
               decr:  boolean ;
               clrmode:  boolean ;
               dispon:  boolean ;
               END ;",
    );
    let r = "smd_$display_modereg_t";
    // Bit b of the record is bit 15 - b of the word: mask 1 << (15 - b).
    let mask = |f: &str| {
        let (b, w) = bits(&p, r, f);
        ((1u32 << w) - 1) << (16 - b - w)
    };
    assert_eq!(mask("blt"), 0x8000);
    assert_eq!(mask("dummy"), 0x7E00);
    assert_eq!(mask("noninterlace"), 0x100);
    assert_eq!(mask("from_mm"), 0x80);
    assert_eq!(mask("to_mm"), 0x40);
    assert_eq!(mask("ieof"), 0x20);
    assert_eq!(mask("idone"), 0x10);
    assert_eq!(mask("nonconform"), 0x8);
    assert_eq!(mask("decr"), 0x4);
    assert_eq!(mask("clrmode"), 0x2);
    assert_eq!(mask("dispon"), 0x1);
    assert_eq!(size(&p, r), 2);
}

/// `stream_$IR_rec_t` (streams.ins.pas, the fields before `hdr_lgth`).
/// Its comment on `unused6 : set of 0..21; { nxt field needs word
/// alignment }` holds only with `rec_lgth: linteger` taking 32 bits and
/// a `set of 0..n` taking n + 1 bits: then `hdr_lgth` (a pinteger, bit
/// aligned) starts at bit 112, a word boundary.
#[test]
fn apollo_stream_attribute_record() {
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           stream_$id_t = integer;
           stream_$opos_t = (stream_$read, stream_$write, stream_$overwrite,
                             stream_$update, stream_$append, stream_$make_backup);
           stream_$omode_t = (stream_$no_conc_write, stream_$controlled_sharing,
                              stream_$unregulated);
           stream_$fconc_t = (stream_$N_or_1, stream_$N_and_1, stream_$N_and_N,
                              stream_$strict_N_or_1);
           stream_$cc_t = (stream_$Apollo_cc, stream_$F77_cc);
           stream_$rtype_t = (stream_$V1, stream_$F2, stream_$UNDEF);
           r = packed record
               strid : stream_$id_t;
               obj_namlen : pinteger;
               rec_lgth : linteger;
               temporary : boolean;
               explicit_type : boolean;
               AB_flag : boolean;
               explicit_ML : boolean;
               unused1 : set of 0..1;
               CC : stream_$cc_t;
               unused2 : set of 0..2;
               rec_type : stream_$rtype_t;
               unused3 : set of 0..1;
               conc : stream_$fconc_t;
               unused4 : set of 0..1;
               oconc : stream_$omode_t;
               unused5 : set of 0..1;
               opos : stream_$opos_t;
               pre_exist : boolean;
               unused6 : set of 0..21;
               hdr_lgth : pinteger;
               file_lgth : linteger;
           end;",
    );
    assert_eq!(bits(&p, "r", "rec_lgth"), (32, 32));
    assert_eq!(bits(&p, "r", "opos"), (86, 3));
    assert_eq!(bits(&p, "r", "unused6"), (90, 22));
    assert_eq!(bits(&p, "r", "hdr_lgth"), (112, 16));
    assert_eq!(offset(&p, "r", "file_lgth"), 16);
}

/// `num_var_t` (prf.ins.pas): variants that overlay the low word and low
/// byte of a longword through padding fields: `pad_byte: 0..16777215`
/// takes 24 bits, so `int8` is the fourth byte.
#[test]
fn apollo_overlay_variants() {
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           binteger = 0..255;
           num_var_t = PACKED RECORD CASE INTEGER OF
               1   : (real32 : real);
               2   : (int32  : linteger);
               3   : (pad_int16 : 0..65535; int16  : pinteger);
               4   : (pad_in16n : 0..65535; int16_n : integer);
               5   : (pad_byte : 0..16777215; int8 : BINTEGER);
               6   : (int1 : BOOLEAN);
               END;",
    );
    let r = "num_var_t";
    assert_eq!(bits(&p, r, "int16"), (16, 16));
    assert_eq!(bits(&p, r, "int16_n"), (16, 16));
    assert_eq!(bits(&p, r, "pad_byte"), (0, 24));
    assert_eq!(bits(&p, r, "int8"), (24, 8));
    assert_eq!(bits(&p, r, "int1"), (0, 1));
    assert_eq!(size(&p, r), 4);
}

/// A packed record as a field and as a variable: a word-aligned structured
/// field inside a packed record (Table 3-3: "Structured types, except for
/// sets, are aligned on word boundaries"), a record field of an unpacked
/// record.
#[test]
fn nested_packed_records() {
    let p = module(
        "type
           inner = packed record a: 0..7; b: boolean end;
           outer = packed record f: boolean; i: inner; g: boolean end;
           plain = record c: char; i: inner end;",
    );
    assert_eq!(bits(&p, "outer", "i"), (16, 16));
    assert_eq!(bits(&p, "outer", "g"), (32, 1));
    assert_eq!(size(&p, "outer"), 6);
    assert_eq!(offset(&p, "plain", "i"), 2);
    assert!(matches!(p.types.kind(ty(&p, "inner")), TypeKind::Record(r) if r.packed));
    assert_eq!(
        field(&p, "plain", "i").bits,
        None::<BitPos>,
        "a field of an unpacked record has no bits"
    );
}

#[test]
fn packed_sets_not_from_zero_are_refused() {
    assert_err(
        "module m; type r = packed record s: set of 3..5 end;",
        "does not start at ordinal 0",
    );
}

/// A 31-bit unsigned subrange that does not start on a 16-bit boundary
/// takes its 31 bits, not 32 (`TypeTable::layout_packed_fields`): the SAU9
/// page-map entry of ../source ast/ast.ins.pas, `(unused_31: boolean;
/// fm_daddr: 0..16#7FFFFFFF)` in a 4-byte packed record, read `bfextu
/// (A0){1:31},D1` at ast_$load_aote 0x3C403ADA (../source gap 361; bead
/// domain-pascal-nvqu).  On a boundary it takes 32 (ref 3.4 p. 3-10), and
/// after a 2-bit field its 31 bits would cross two boundaries, so it moves
/// to the next boundary and takes 32 there.
#[test]
fn unsigned_31_bit_subrange_after_a_boolean() {
    let p = module(
        "type
           entry_t = PACKED RECORD CASE integer OF
               1: (all: integer32);
               2: (unused_31: boolean;
                   fm_daddr: 0..16#7FFFFFFF);
               END;
           two_t = PACKED RECORD
               k: 0..3;
               v: 0..16#7FFFFFFF;
               END;
           word_t = PACKED RECORD
               w: 0..65535;
               v: 0..16#7FFFFFFF;
               b: boolean;
               END;",
    );
    assert_eq!(bits(&p, "entry_t", "unused_31"), (0, 1));
    assert_eq!(bits(&p, "entry_t", "fm_daddr"), (1, 31));
    assert_eq!(size(&p, "entry_t"), 4);
    assert_eq!(bits(&p, "two_t", "k"), (0, 2));
    assert_eq!(bits(&p, "two_t", "v"), (16, 32));
    assert_eq!(size(&p, "two_t"), 6);
    assert_eq!(bits(&p, "word_t", "v"), (16, 32));
    assert_eq!(bits(&p, "word_t", "b"), (48, 1));
    assert_eq!(size(&p, "word_t"), 8);
}
