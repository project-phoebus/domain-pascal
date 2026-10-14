//! Attributes for variables and types (ref 3.15, 3.16) and the alignment
//! environments (ref 3.10.5, 3.10.6, 4.7 `%natural_alignment`): layouts
//! from the reference's figures (read in the PDF) and from Apollo's SR10.2
//! customer inserts (codegen-patterns 0.1), checked against
//! `../source/domain_os`'s `_Static_assert`s where it has them.

use super::*;
use crate::types::DeviceAccess;

fn align(p: &Program, name: &str) -> u32 {
    p.types.get(ty(p, name)).align
}

fn field_align(p: &Program, name: &str) -> u32 {
    p.types.get(ty(p, name)).field_align
}

// ---------------------------------------------------------------------
// aligned(n) and natural (ref 3.15.6)
// ---------------------------------------------------------------------

#[test]
fn aligned_n_is_two_to_the_n() {
    // ref 3.15.6.1 p. 3-63: "aligned specifies byte alignment, aligned(1)
    // specifies word alignment, aligned(2) specifies longword alignment";
    // `[ALIGNED(1)] integer32` is "at least word aligned"; `[NATURAL]
    // integer32` "at least longword aligned".
    let p = module(
        "type
           b = [aligned] integer16;
           b0 = [aligned(0)] integer16;
           word_aligned_integer32 = [aligned(1)] integer32;
           l = [aligned(2)] integer16;
           natural_integer32 = [natural] integer32;",
    );
    assert_eq!(align(&p, "b"), 1);
    assert_eq!(align(&p, "b0"), 1);
    assert_eq!(align(&p, "word_aligned_integer32"), 2);
    assert_eq!(size(&p, "word_aligned_integer32"), 4);
    assert_eq!(align(&p, "l"), 4);
    assert_eq!(size(&p, "l"), 2);
    assert_eq!(align(&p, "natural_integer32"), 4);
}

#[test]
fn records_inherit_the_largest_field_specification() {
    // ref 3.15.6.2 pp. 3-66..3-67, Figure 3-21 (PDF): every one of these
    // declarations of S gives a at 0, b at 4 and 8 bytes, longword
    // aligned.
    let p = module(
        "type
           s_rec = [aligned(2)] record a: integer32; b: integer16 end;
           s_nat = record a: [natural] integer32; b: integer16 end;
           s_al2 = record a: [aligned(2)] integer32; b: integer16 end;
           s_pad = record a: integer32; b: integer16; padding: integer16 end;
           s_def = record a: integer32; b: integer16 end;",
    );
    for s in ["s_rec", "s_nat", "s_al2", "s_pad"] {
        assert_eq!(size(&p, s), 8, "{s}");
        assert_eq!(offset(&p, s, "b"), 4, "{s}");
        assert_eq!(align(&p, s), 4, "{s}");
    }
    // Without them: 6 bytes (Figure 3-19, 3-20 p. 3-65), longword
    // allocated alone (3.10.5.6), word aligned as an element or field.
    assert_eq!(size(&p, "s_def"), 6);
    assert_eq!(field_align(&p, "s_def"), 2);
    assert_eq!(field_align(&p, "s_nat"), 4);
}

#[test]
fn inherited_alignment_pads_the_containing_record() {
    // ref 3.15.6.2 NOTE p. 3-68, Figure 3-22 (PDF): S1 = x at 0, y: S at
    // 4 (a word of padding after x), z at 12, 16 bytes (a word at the
    // end).
    let p = module(
        "type
           s = record a: [natural] integer32; b: integer16 end;
           s1 = record x: integer16; y: s; z: integer16 end;",
    );
    assert_eq!(offset(&p, "s1", "y"), 4);
    assert_eq!(offset(&p, "s1", "z"), 12);
    assert_eq!(size(&p, "s1"), 16);
    assert_eq!(align(&p, "s1"), 4);
}

#[test]
fn aligned_0_removes_padding() {
    // ref 3.15.6.3 pp. 3-69..3-70, Figures 3-23, 3-24 (PDF): a at 0, b at
    // 1, c at 3, and still a trailing pad byte: 6 bytes ("Records created
    // by Domain Pascal are always 2-byte multiples").
    let p = module(
        "type
           s2_def = record a: char; b, c: integer16 end;
           s2 = record a: char; b, c: [aligned(0)] integer16 end;",
    );
    assert_eq!(offset(&p, "s2_def", "b"), 2);
    assert_eq!(offset(&p, "s2_def", "c"), 4);
    assert_eq!(offset(&p, "s2", "b"), 1);
    assert_eq!(offset(&p, "s2", "c"), 3);
    assert_eq!(size(&p, "s2"), 6);
}

#[test]
fn natural_alignment_environment() {
    // ref 3.15.6.3 pp. 3-70..3-72, Figures 3-25, 3-26, 3-27 (PDF).
    let p = module(
        "%natural_alignment;
         type
           s1 = record a: char; b: integer32; c: char end;
           s1w = [aligned(1)] record a: char; b: integer32; c: char end;
           s1b = [aligned(1)] record a: char; b: [aligned(1)] integer32; c: char end;
           s1z = [aligned(1)] record a: char; b: [aligned(0)] integer32; c: char end;",
    );
    // Figure 3-25: a 0, b 4, c 8, 12 bytes.
    assert_eq!(offset(&p, "s1", "b"), 4);
    assert_eq!(offset(&p, "s1", "c"), 8);
    assert_eq!(size(&p, "s1"), 12);
    assert_eq!(align(&p, "s1"), 4);
    // Figure 3-26: the final word of padding removed, 10 bytes.
    assert_eq!(offset(&p, "s1w", "b"), 4);
    assert_eq!(offset(&p, "s1w", "c"), 8);
    assert_eq!(size(&p, "s1w"), 10);
    assert_eq!(align(&p, "s1w"), 2);
    // Figure 3-27: b at 2, c at 6, 8 bytes; byte alignment for b gives
    // the same ("a byte aligned object may not cross two word
    // boundaries").
    for s in ["s1b", "s1z"] {
        assert_eq!(offset(&p, s, "b"), 2, "{s}");
        assert_eq!(offset(&p, s, "c"), 6, "{s}");
        assert_eq!(size(&p, s), 8, "{s}");
    }
}

#[test]
fn same_layout_in_all_environments() {
    // ref 3.15.6.4 pp. 3-72..3-73, Figures 3-28, 3-29 (PDF): S2 is a 0, b
    // 4 naturally and a 0, b 2 by the word rules; [ALIGNED(1)] on b
    // gives the word layout in both.
    let p = module(
        "type s2w = record a: integer16; b: integer32 end;
              s2wa = record a: integer16; b: [aligned(1)] integer32 end;
         %natural_alignment;
         type s2n = record a: integer16; b: integer32 end;
              s2na = record a: integer16; b: [aligned(1)] integer32 end;
         %word_alignment;
         type s2w2 = record a: integer16; b: integer32 end;",
    );
    assert_eq!(offset(&p, "s2n", "b"), 4);
    assert_eq!(size(&p, "s2n"), 8);
    for s in ["s2w", "s2wa", "s2na", "s2w2"] {
        assert_eq!(offset(&p, s, "b"), 2, "{s}");
        assert_eq!(size(&p, s), 6, "{s}");
    }
}

#[test]
fn aligned_and_unaligned_records() {
    // ref 3.10.6 pp. 3-33..3-34, Figures 3-15, 3-16: an aligned record is
    // laid out naturally (a 0, b 4) even under %word_alignment, an
    // unaligned one by the word rules (a 0, b 2) even under
    // %natural_alignment.
    let p = module(
        "%word_alignment;
         type nat_rec = aligned record a: integer; b: integer32 end;
         %natural_alignment;
         type not_nat_rec = unaligned record a: integer; b: integer32 end;",
    );
    assert_eq!(offset(&p, "nat_rec", "b"), 4);
    assert_eq!(size(&p, "nat_rec"), 8);
    assert_eq!(offset(&p, "not_nat_rec", "b"), 2);
    assert_eq!(size(&p, "not_nat_rec"), 6);
}

#[test]
fn push_and_pop_alignment() {
    // ref 4.7 p. 4-58: %push_alignment saves the mode, %pop_alignment
    // restores it.
    let p = module(
        "%natural_alignment;
         %push_alignment;
         %word_alignment;
         type w = record a: integer16; b: integer32 end;
         %pop_alignment;
         type n = record a: integer16; b: integer32 end;",
    );
    assert_eq!(offset(&p, "w", "b"), 2);
    assert_eq!(offset(&p, "n", "b"), 4);
    // More pushes than pops, and the reverse, are errors (ref 9 p. 9-45:
    // "Unmatched %PUSH_ALIGNMENT directive", "Alignment stack
    // underflow").
    assert_err(
        "module m; %push_alignment; type t = integer;",
        "Unmatched %push_alignment",
    );
    assert_err(
        "module m; %pop_alignment; type t = integer;",
        "%pop_alignment without a matching %push_alignment",
    );
}

// ---------------------------------------------------------------------
// Apollo's SR10.2 inserts (codegen-patterns 0.1)
// ---------------------------------------------------------------------

#[test]
fn apollo_time_clock_t() {
    // base.ins.pas, verbatim but for the %IF (the SR10 m68k branch):
    // under %PUSH_ALIGNMENT; %WORD_ALIGNMENT.  ../source base/base.h:
    // `_Static_assert(sizeof(clock_t) == 6)`, `_Alignof(clock_t) == 2`.
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           time_$clockh_t = linteger;
         %PUSH_ALIGNMENT;
         %WORD_ALIGNMENT;
         type
           time_$clock_t = [aligned(1)] PACKED RECORD CASE integer OF
               1:  (high:  time_$clockh_t;
                    low:  pinteger);
               2:  (high16:  pinteger;
                    low32:  linteger);
               END;
         %POP_ALIGNMENT;",
    );
    assert_eq!(size(&p, "time_$clock_t"), 6);
    assert_eq!(align(&p, "time_$clock_t"), 2);
    assert_eq!(offset(&p, "time_$clock_t", "high"), 0);
    assert_eq!(offset(&p, "time_$clock_t", "low"), 4);
    assert_eq!(offset(&p, "time_$clock_t", "high16"), 0);
    assert_eq!(offset(&p, "time_$clock_t", "low32"), 2);
}

#[test]
fn apollo_cal_timezone_rec_t() {
    // cal.ins.pas: `drift: [aligned(1)] time_$clock_t` in a PACKED
    // RECORD (the m68k branch).  cal_$tz_name_t is `array [1..4] of char`.
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           time_$clock_t = [aligned(1)] PACKED RECORD CASE integer OF
               1:  (high: linteger; low: pinteger);
               2:  (high16: pinteger; low32: linteger);
               END;
           cal_$tz_name_t = array [1..4] of char;
           cal_$timezone_rec_t = PACKED RECORD
               utc_delta:  integer;
               tz_name:  cal_$tz_name_t;
               drift:  [aligned(1)] time_$clock_t;
               END;",
    );
    assert_eq!(offset(&p, "cal_$timezone_rec_t", "tz_name"), 2);
    assert_eq!(offset(&p, "cal_$timezone_rec_t", "drift"), 6);
    assert_eq!(size(&p, "cal_$timezone_rec_t"), 12);
}

#[test]
fn apollo_ec2_eventcount_t() {
    // base.ins.pas, the SR10 m68k branch.  ../source ec/ec.h: "Size: 6
    // bytes", value at 0, awaiters at 4.
    let p = module(
        "type
           pinteger = 0..65535;
           ec2_$eventcount_t = [volatile] RECORD
               value: [aligned(1)] integer32;
               awaiters: pinteger;
               END;
           ec2_$ptr_t = ^ec2_$eventcount_t;",
    );
    assert_eq!(size(&p, "ec2_$eventcount_t"), 6);
    assert_eq!(align(&p, "ec2_$eventcount_t"), 2);
    assert_eq!(offset(&p, "ec2_$eventcount_t", "awaiters"), 4);
    let t = ty(&p, "ec2_$eventcount_t");
    assert!(p.types.attrs(t).volatile);
    // A pointer does not inherit the attribute (ref 3.15.7 p. 3-77).
    assert!(!p.types.attrs(ty(&p, "ec2_$ptr_t")).is_volatile());
}

#[test]
fn apollo_tty_break_mode() {
    // tty.ins.pas `timer: [aligned(1)] linteger` after `count: pinteger`
    // (../source domain_os/tty/tty.ins.pas: 8 bytes, timer at 4).
    let p = module(
        "type
           pinteger = 0..65535;
           linteger = 0..2147483647;
           kind_t = (on_timer, on_count, on_both);
           mode_t = record
               case break_type: kind_t of
                 on_timer, on_count, on_both: (
                   count: pinteger;
                   timer: [aligned(1)] linteger);
             end;",
    );
    assert_eq!(offset(&p, "mode_t", "count"), 2);
    assert_eq!(offset(&p, "mode_t", "timer"), 4);
    assert_eq!(size(&p, "mode_t"), 8);
}

#[test]
fn apollo_word_aligned_fields() {
    // ms.ins.pas `cur_len: [aligned(1)] linteger` ...; rws.ins.pas
    // `rws_$word_aligned_long = [aligned(1)] integer32`; a char before
    // shows the word alignment.
    let p = module(
        "type
           linteger = 0..2147483647;
           rws_$word_aligned_long = [aligned(1)] integer32;
           r = record c: char; cur_len: [aligned(1)] linteger; w: rws_$word_aligned_long end;",
    );
    assert_eq!(offset(&p, "r", "cur_len"), 2);
    assert_eq!(offset(&p, "r", "w"), 6);
    assert_eq!(size(&p, "r"), 10);
    assert_eq!(align(&p, "r"), 2);
}

// ---------------------------------------------------------------------
// volatile, atomic, device (ref 3.15.1-3.15.3, 3.15.7)
// ---------------------------------------------------------------------

#[test]
fn volatile_types_are_compatible_with_their_base() {
    // ref 3.15.7 p. 3-76: `semaphore = [VOLATILE] integer` is used as an
    // integer.
    let p = module(
        "type
           semaphore = [volatile] integer;
           resource = array[1..10] of semaphore;
           flag = [device] boolean;
           color = (red, green);
           vcolor = [volatile] color;
         var s: semaphore; r: resource; f: flag; c: vcolor; i: integer;
         procedure x;
         begin
           s := 1; i := s + r[2]; s := i;
           f := true; if f then c := green;
         end;",
    );
    // An array inherits volatile from its element (3.15.7).
    assert!(p.types.attrs(ty(&p, "resource")).volatile);
    assert!(p.types.attrs(ty(&p, "flag")).device.is_some());
}

#[test]
fn record_attribute_inheritance() {
    // ref 3.15.7 p. 3-77: a device field makes the record device; a
    // volatile field does not make the record volatile.
    let p = module(
        "type
           lock = [volatile] integer;
           reg = [device] integer16;
           queue = record key: lock; users: integer end;
           regs = record status: reg; data: integer16 end;
           volque = [volatile] queue;",
    );
    assert!(!p.types.attrs(ty(&p, "queue")).is_volatile());
    assert_eq!(
        p.types.attrs(ty(&p, "regs")).device,
        Some(DeviceAccess {
            read: true,
            write: true
        })
    );
    assert!(p.types.attrs(ty(&p, "volque")).volatile);
    assert_eq!(size(&p, "volque"), size(&p, "queue"));
}

#[test]
fn device_access_options() {
    // ref 3.15.3 p. 3-59.
    let p = module(
        "type
           r = [device(read)] char;
           w = [device(write)] char;
           rw = [device(read, write)] char;
           wr = [device(write, read)] char;",
    );
    let d = |n| p.types.attrs(ty(&p, n)).device.unwrap();
    assert!(d("r").read && !d("r").write);
    assert!(!d("w").read && d("w").write);
    assert!(d("rw").read && d("rw").write);
    assert!(d("wr").read && d("wr").write);
}

#[test]
fn attribute_declaration_part() {
    // ref 3.16 p. 3-78 (address left out: not supported yet).
    let p = module(
        "attribute
           integer_attributes = natural;
           keyboard_attributes = device;
           array_attributes = volatile;
           both = keyboard_attributes, aligned(1);
         type
           int1 = [integer_attributes] integer32;
           kb = [keyboard_attributes] char;
           arr = [array_attributes] array[1..10] of int1;
           bw = [both] integer32;",
    );
    assert_eq!(align(&p, "int1"), 4);
    assert!(p.types.attrs(ty(&p, "kb")).device.is_some());
    assert!(p.types.attrs(ty(&p, "arr")).volatile);
    assert!(p.types.attrs(ty(&p, "bw")).device.is_some());
    assert_eq!(align(&p, "bw"), 2);
}

#[test]
fn attribute_errors() {
    assert_err(
        "module m; attribute volatile = device;",
        "Illegal to redefine `volatile`",
    );
    // ref 3.15.2 p. 3-57: atomic only on scalars.
    assert_err(
        "module m; type r = record a: integer end; var x: [atomic] r;",
        "`atomic` can only be used with scalar variables",
    );
    assert_err("module m; var x: [aligned(5)] integer;", "n from 0 to 4");
    assert_err(
        "module m; var x: [frobnicate] integer;",
        "is not an attribute",
    );
    assert_err(
        "module m; var x: [address(16#ff7000)] char;",
        "`address` attribute (ref 3.15.4) is not supported yet",
    );
    // ref 3.15.5 p. 3-60, 7.5.1 p. 7-15: `bool4 = [long] boolean`, a
    // size larger than the type's storage (not implemented).
    assert_err(
        "module m; type t = [long] boolean;",
        "a size attribute larger than the type's own storage",
    );
    assert_err(
        "module m; var x: [device(sideways)] char;",
        "takes `read` and/or `write`",
    );
}

#[test]
fn erroneous_field_types_do_not_upset_layout() {
    // A field whose type failed to check has size 0 (sfcb.ins.pas without
    // mutex.ins.pas: `slock: mutex_$lock_rec_t` undefined).
    assert_err(
        "module m; type r = record a: integer; b: nosuch_t; c: [aligned(1)] ^r end;",
        "undefined type `nosuch_t`",
    );
}

// ---------------------------------------------------------------------
// Attribute lists on parameter types (owner ruling, docs/reference-
// clarifications.md "5.1 Attribute lists on parameter types"; meaning
// from ref 3.15.6.8, p. 3-76)
// ---------------------------------------------------------------------

/// Apollo's rws.ins.pas and error.ins.pas forms, with the declarations
/// they rely on reduced.
const PARAM_ATTRS: &str = "type
       status_$t = record all: integer32 end;
       error_$integer32 = integer32;
       error_$status_t = [aligned(1)] status_$t;
     procedure rws_like(in n: integer32; out status: [aligned(1)] status_$t); extern;
     procedure error_like(in a1, a2: univ [aligned(0)] error_$integer32); extern;
     procedure vp(i: [aligned(1)] integer32); val_param; extern;
     procedure byval(i: [aligned(0)] integer32); extern;
     procedure vol(var x: [volatile] integer); extern;
     procedure named(out status: error_$status_t); extern;
     ";

#[test]
fn attributed_parameter_types_are_the_declared_types() {
    // `[attrs] T` is the type `= [attrs] T` declares (3.15 p. 3-53).
    let p = module(PARAM_ATTRS);
    let ps = &routine(&p, "rws_like").params;
    assert_eq!(p.types.get(ps[1].ty).align, 2);
    assert_eq!(p.types.size(ps[1].ty), 4);
    assert_eq!(p.types.canon(ps[1].ty), p.types.canon(ty(&p, "status_$t")));
    assert_eq!(p.types.attrs(ps[1].ty).align, Some(2));
    let ps = &routine(&p, "error_like").params;
    assert!(ps[0].univ && ps[1].univ);
    assert_eq!(p.types.get(ps[0].ty).align, 1);
    assert_eq!(p.types.attrs(ps[1].ty).align, Some(1));
    assert!(p.types.attrs(routine(&p, "vol").params[0].ty).volatile);
    // Same as the named declaration.
    let named = &routine(&p, "named").params[0];
    assert_eq!(
        p.types.attrs(named.ty),
        p.types.attrs(routine(&p, "rws_like").params[1].ty)
    );
}

#[test]
fn attributed_parameter_types_keep_table_5_1() {
    // Table 5-1 (ref 5.2, p. 5-5) decides from mode, size and options:
    // the attributes change none of them.
    use crate::conv::Passing;
    let p = module(PARAM_ATTRS);
    let c = |r: &str, i: usize| routine(&p, r).params[i].conv;
    assert_eq!(c("rws_like", 1).passing, Passing::ByAddress);
    assert_eq!(c("error_like", 0).passing, Passing::ByAddress);
    assert!(!c("error_like", 0).local_copy);
    assert_eq!(c("vp", 0).passing, Passing::ByValue);
    assert_eq!(c("vp", 0).slot_size, 4);
    assert_eq!(c("byval", 0).passing, Passing::ByAddress);
    assert!(c("byval", 0).local_copy);
}

#[test]
fn plain_and_attributed_arguments_match_either_formal() {
    // As `error_$status_t` and `status_$t` do: an attributed type is its
    // base type for compatibility (TypeInfo::canon).
    module(&format!(
        "{PARAM_ATTRS}
         procedure plain(out status: status_$t); extern;
         procedure q;
         var s: status_$t; e: error_$status_t; w: [aligned(1)] status_$t;
             l: integer32;
         begin
           rws_like(1, s); rws_like(1, e); rws_like(1, w);
           plain(s); plain(e); plain(w);
           named(s);
           error_like(l, 5);
           vp(l); byval(l);
         end;"
    ));
}

#[test]
fn an_anonymous_type_under_attributes_is_refused() {
    // ref 5.1 p. 5-2: "you cannot specify an anonymous data type".
    assert_err(
        "module m;
         procedure p(var r: [aligned(1)] record a: integer32 end); extern;",
        "a parameter's type must be a type name (ref 5.1, p. 5-2)",
    );
    assert_err(
        "module m;
         procedure p(var r: [aligned(1)] 1..10); extern;",
        "a parameter's type must be a type name (ref 5.1, p. 5-2)",
    );
}

#[test]
fn routine_pointer_templates_and_attributed_parameters() {
    // ref p. 4-146: the routine must "exactly match" the template.  The
    // same type with an alignment attribute that leaves its size alone
    // (`[aligned(1)] status_$t`, 4 bytes) matches the plain one, as a
    // call's argument does; one whose size the attribute changes (ref
    // 3.15.6.2, Figure 3-21 p. 3-67: `[aligned(2)]` rounds a record up)
    // does not.
    module(
        "type
           status_$t = record all: integer32 end;
           pt = ^procedure(out s: status_$t);
           pa = ^procedure(out s: [aligned(1)] status_$t);
         var x: pt; y: pa;
         procedure a1(out s: [aligned(1)] status_$t); extern;
         procedure a2(out s: status_$t); extern;
         procedure z; begin x := addr(a1); y := addr(a2) end;",
    );
    assert_err(
        "module m;
         type
           rec2 = record a: integer end;
           pt = ^procedure(out r: rec2);
         var x: pt;
         procedure a3(out r: [aligned(2)] rec2); extern;
         procedure z; begin x := addr(a3) end;",
        "`a3` does not match the routine pointer type",
    );
}

// ---------------------------------------------------------------------
// Size attributes (ref 3.15.5, pp. 3-60..3-62)
// ---------------------------------------------------------------------

#[test]
fn byte_subranges_are_one_byte() {
    // ref 3.15.5 p. 3-62: "byte_integer = [byte] 0..255; byte_int_array =
    // array[1..1000] of byte_integer"; "[byte] 0..255 corresponds to the
    // unsigned char data type in Domain/C".  Table 3-7 p. 3-61: a
    // subrange's minimum is the bits it needs, so -128..127 fits a byte.
    let p = module(
        "type
           byte_integer = [byte] 0..255;
           byte_int_array = array[1..1000] of byte_integer;
           sbyte = [byte] -128..127;
           small = [byte] 0..15;
           odd3 = array[1..3] of byte_integer;
           { tpad.ins.pas (../source): bytes at 0xA..0xF }
           pkt = record
             elapsed: integer32;
             clock: integer32;
             w: integer;
             dev_id: byte_integer;
             flags: byte_integer;
             b0: byte_integer;
             b1: sbyte;
             b2: byte_integer;
             b3: byte_integer;
           end;
           mixed = record a: byte_integer; b: integer; c: sbyte end;",
    );
    assert_eq!(size(&p, "byte_integer"), 1);
    assert_eq!(align(&p, "byte_integer"), 1);
    assert_eq!(field_align(&p, "byte_integer"), 1);
    assert_eq!(size(&p, "sbyte"), 1);
    assert_eq!(size(&p, "small"), 1);
    // ref 3.11.4.1 pp. 3-45..3-46: elements times element size; only arrays of
    // booleans get a pad byte.
    assert_eq!(size(&p, "byte_int_array"), 1000);
    assert_eq!(size(&p, "odd3"), 3);
    for (f, o) in [
        ("dev_id", 0xA),
        ("flags", 0xB),
        ("b0", 0xC),
        ("b1", 0xD),
        ("b2", 0xE),
        ("b3", 0xF),
    ] {
        assert_eq!(offset(&p, "pkt", f), o, "{f}");
    }
    assert_eq!(size(&p, "pkt"), 0x10);
    // A byte field, then a word-aligned integer, then a byte; records are
    // a multiple of 2 bytes (ref 3.15.6.3 p. 3-70).
    assert_eq!(offset(&p, "mixed", "b"), 2);
    assert_eq!(offset(&p, "mixed", "c"), 4);
    assert_eq!(size(&p, "mixed"), 6);
    // The sized type is its subrange for compatibility (3.15.7 p. 3-76,
    // as for the other attributes) and keeps its range.
    assert_eq!(p.types.ordinal_range(ty(&p, "sbyte")), Some((-128, 127)));
    assert_eq!(p.types.attrs(ty(&p, "byte_integer")).size_bits, Some(8));
}

#[test]
fn byte_variables_parameters_and_results() {
    // ref 3.15.5 p. 3-60: size attributes reserve storage for variables,
    // formal parameters, function results, record fields and array
    // components, given with the type or with the object.
    let p = ok("module m;
        type ub = 0..255; tri = -1..1;
        var
          x: [byte] 0..255;
          y: integer;
          z: [byte] -128..127;
        function f(in a: [byte] ub; b: [byte] tri): [byte] ub; extern;");
    let st = |n: &str| match &var(&p, n).storage {
        crate::program::Storage::Static { offset, .. } => *offset,
        s => panic!("{n}: {s:?}"),
    };
    assert_eq!(p.types.size(var(&p, "x").ty), 1);
    assert_eq!(st("x"), 0);
    assert_eq!(st("y"), 2);
    assert_eq!(st("z"), 4);
    let r = routine(&p, "f");
    assert_eq!(p.types.size(r.result.as_ref().unwrap().ty), 1);
    let a = r.params[0].ty;
    assert_eq!(p.types.size(a), 1);
    assert_eq!(p.types.size(r.params[1].ty), 1);
}

#[test]
fn sizes_equal_to_the_storage_change_nothing() {
    // ref 3.15.5 p. 3-60: `(n)` units, 1 by default; "big_number = [long]
    // integer16" reserves 32 bits, so `[byte(2)]` is 16 bits.
    let p = module(
        "type
           w1 = [word] integer16;
           w2 = [byte(2)] integer;
           l1 = [long] integer32;
           l2 = [word(2)] integer32;
           c1 = [byte] char;
           b1 = [byte] boolean;
           e1 = [word] (red, green);
           r1 = [long] real;
           u16 = [word] 0..65535;
           s8w = [word] 0..255;",
    );
    for (t, n) in [
        ("w1", 2),
        ("w2", 2),
        ("l1", 4),
        ("l2", 4),
        ("c1", 1),
        ("b1", 1),
        ("e1", 2),
        ("r1", 4),
        ("u16", 2),
        ("s8w", 2),
    ] {
        assert_eq!(size(&p, t), n, "{t}");
    }
    // [byte] and [word] on a type that already says the same size.
    let p = module("type b = [byte] 0..255; bb = [byte] b; arr = [byte, byte] 0..3; var v: bb;");
    assert_eq!(p.types.size(var(&p, "v").ty), 1);
    assert_eq!(size(&p, "arr"), 1);
}

#[test]
fn size_attribute_errors() {
    // ref p. 9-40, message 300: "Size of token1 bits is invalid for
    // specified type", `bad : [byte] number` with number = integer32;
    // Table 3-7 p. 3-61 minima.
    assert_err(
        "module m; type number = integer32; var bad: [byte] number;",
        "Size of 8 bits is invalid for specified type",
    );
    assert_err(
        "module m; var x: [byte] integer;",
        "Size of 8 bits is invalid",
    );
    assert_err(
        "module m; var x: [byte] 0..256;",
        "Size of 8 bits is invalid",
    );
    assert_err(
        "module m; var x: [byte] -129..0;",
        "Size of 8 bits is invalid",
    );
    assert_err(
        "module m; var x: [byte] (a, b);",
        "Size of 8 bits is invalid",
    );
    assert_err(
        "module m; var x: [word] 0..65536;",
        "Size of 16 bits is invalid",
    );
    // p. 3-61: more bits than the minimum only for integer, boolean and set.
    assert_err(
        "module m; var x: [word] char;",
        "Size of 16 bits is invalid",
    );
    assert_err(
        "module m; var x: [long] (a, b);",
        "Size of 32 bits is invalid",
    );
    assert_err(
        "module m; var x: [byte(0)] char;",
        "Size of 0 bits is invalid",
    );
    // p. 9-40, message 298: `int = [word, long] -32768..32767`.
    assert_err(
        "module m; type int = [word, long] -32768..32767;",
        "Argument to `long` attribute conflicts with value already specified",
    );
}

#[test]
fn unsettled_size_attributes_are_refused() {
    // p. 9-40, message 299 (a warning; the size that results not stated):
    // `int = [word] -32768..32767; long_int = [long] int`.
    assert_err(
        "module m; type int = [word] -32768..32767; long_int = [long] int;",
        "a size attribute on a type that has another",
    );
    assert_err(
        "module m; type t = [long] integer16;",
        "a size attribute larger than the type's own storage",
    );
    assert_err(
        "module m; type t = [bit(4)] 0..1;",
        "the `bit` size attribute (bit arrays, ref 3.15.5 p. 3-62) is not supported yet",
    );
    assert_err(
        "module m; type t = [quad] double;",
        "the `quad` size attribute (ref 3.15.5) is not supported yet",
    );
    assert_err(
        "module m; type t = [word] record a: integer end;",
        "Table 3-7 (ref 3.15.5, p. 3-61) gives no minimum size",
    );
    assert_err(
        "module m; type t = [byte] 'a'..'z';",
        "a size attribute on a subrange of char",
    );
    // A packed record field whose size attribute gives other bits than
    // Table 3-3 (ref 3.10.7 p. 3-36) gives the field.
    assert_err(
        "module m; type t = packed record a: [byte] 0..15; b: boolean end;",
        "a packed record field whose size attribute gives other bits",
    );
    assert_err(
        "module m; type t = packed record a: [byte] boolean end;",
        "a packed record field whose size attribute gives other bits",
    );
}

#[test]
fn byte_fields_of_packed_records_take_their_bits() {
    // ref 3.10.7 Table 3-3 p. 3-36: an integer subrange field takes the
    // bits of its extreme values, bit aligned; `[byte] 0..255` needs 8.
    let p = module(
        "type
           b8 = [byte] 0..255;
           t = packed record a: boolean; b: b8; c: [byte] -128..127 end;",
    );
    let f = |n: &str| field(&p, "t", n).bits.unwrap();
    assert_eq!((f("b").bit, f("b").width), (1, 8));
    assert_eq!((f("c").bit, f("c").width), (9, 8));
}

#[test]
fn attribute_declarations_carry_sizes() {
    // ref 3.16 p. 3-78: an attribute declaration names a list.
    let p = module("attribute small = byte; type t = [small] 0..255; u = [small, volatile] 0..7;");
    assert_eq!(size(&p, "t"), 1);
    assert_eq!(size(&p, "u"), 1);
    assert!(p.types.attrs(ty(&p, "u")).volatile);
}
