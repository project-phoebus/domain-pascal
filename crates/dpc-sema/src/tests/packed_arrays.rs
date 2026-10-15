//! Packed arrays (ref 3.11.3 p. 3-45, 3.11.4.2 Table 3-5 p. 3-47, 3.15.5
//! p. 3-62; both pages read in the PDF, the OCR scrambles the table).
//!
//! Table 3-5: subranges and enumerations take the bits they need, booleans
//! one bit, characters a byte; elements under 16 bits are padded to the
//! next power of two (`packed array [1..3] of 0..7`: 4 bits each, "2
//! bytes"); 3.15.5: "arrays of 1-, 2-, and 4-bit elements.  The order of
//! elements within words is from most significant bit to least
//! significant bit", six 4-bit elements in "3 bytes".  Image: PCHIST's
//! `packed array[1..64] of boolean`, 8 bytes at +0x18 of its WIRED_DATA
//! block, element p in bit 7 - (p - 1) mod 8 of byte (p - 1) div 8
//! (PCHIST_$COUNT 0xE1A156..0xE1A16C; codegen-patterns 5.2).

use super::*;
use crate::consts::ConstValue;
use crate::program::{ExprKind, InitStore, StmtKind};
use crate::types::{BitPos, PackedElem, TypeKind};

fn elem(p: &Program, name: &str) -> TypeId {
    match p.types.kind(ty(p, name)) {
        TypeKind::Array { elem, .. } => *elem,
        k => panic!("{k:?}"),
    }
}

#[test]
fn packed_array_sizes() {
    let p = module(
        "type
           bitmap = packed array[1..64] of boolean;
           b5 = packed array[1..5] of boolean;
           b100 = packed array[1..100] of boolean;
           array1 = packed array[1..3] of 0..7;
           two = packed array[1..8] of 0..3;
           nib = packed array[1..6] of -8..7;
           colors = (red, green, blue);
           ec = packed array[1..5] of colors;
           frame_size = packed array[0..15] of 0..255;
           sb = packed array[1..3] of -128..127;
           w = packed array[1..3] of 0..1000;
           u16 = packed array[1..2] of 0..65535;
           li = packed array[1..2] of 0..2147483647;
           pc = packed array[1..3] of char;
           ints = packed array[1..3] of integer;",
    );
    // PCHIST's bitmap: 64 bits, 8 bytes.
    assert_eq!(size(&p, "bitmap"), 8);
    // Bits in whole bytes, no pad byte for an odd count (3.11.4.1's pad is
    // for unpacked boolean arrays).
    assert_eq!(size(&p, "b5"), 1);
    assert_eq!(size(&p, "b100"), 13);
    // p. 3-47: "4 bits would be used to store each element of array1 and
    // 2 bytes would be used to store the entire array1".
    assert_eq!(size(&p, "array1"), 2);
    assert_eq!(size(&p, "two"), 2);
    // -8..7 needs 4 bits (two's complement): 6 elements in 3 bytes, as the
    // [bit (4)] example of p. 3-62.
    assert_eq!(size(&p, "nib"), 3);
    // Three values: 2 bits.
    assert_eq!(size(&p, "ec"), 2);
    // "byte integers" (3.11.3): one byte each.
    assert_eq!(size(&p, "frame_size"), 16);
    assert_eq!(size(&p, "sb"), 3);
    assert_eq!(p.types.size(elem(&p, "frame_size")), 1);
    assert_eq!(p.types.size(elem(&p, "sb")), 1);
    // 9..16 bits: 16, as unpacked; 0..65535 is a word (3.4).
    assert_eq!(size(&p, "w"), 6);
    assert_eq!(size(&p, "u16"), 4);
    // An unsigned 31-bit subrange takes 32 bits (3.4 p. 3-10).
    assert_eq!(size(&p, "li"), 8);
    assert_eq!(size(&p, "pc"), 3);
    assert_eq!(size(&p, "ints"), 6);
    let bits = |n: &str| p.types.packed_elem_bits(ty(&p, n));
    assert_eq!(bits("bitmap"), Some(1));
    assert_eq!(bits("array1"), Some(4));
    assert_eq!(bits("two"), Some(2));
    assert_eq!(bits("ec"), Some(2));
    assert_eq!(bits("frame_size"), None);
    assert_eq!(bits("w"), None);
    // Arrays of more than a byte are word aligned like other arrays.
    assert_eq!(p.types.get(ty(&p, "bitmap")).field_align, 2);
}

#[test]
fn packed_element_classes() {
    let p = module(
        "type
           e17 = (a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16);
           byte = [byte] 0..255;
           s17 = 0..100000;",
    );
    let t = &p.types;
    let b = p.types.builtins;
    assert_eq!(t.packed_array_elem(b.boolean), Ok(PackedElem::Bits(1)));
    assert_eq!(t.packed_array_elem(b.char), Ok(PackedElem::Unpacked));
    assert_eq!(t.packed_array_elem(b.integer16), Ok(PackedElem::Unpacked));
    assert_eq!(t.packed_array_elem(b.integer32), Ok(PackedElem::Unpacked));
    // A [byte] subrange is a byte already.
    assert_eq!(
        t.packed_array_elem(ty(&p, "byte")),
        Ok(PackedElem::Unpacked)
    );
    // 17 values: 5 bits, padded to 8 (a byte enumeration: refused).
    assert!(t.packed_array_elem(ty(&p, "e17")).is_err());
    // Footnote 2 (over 16 bits): refused.
    assert!(t.packed_array_elem(ty(&p, "s17")).is_err());
}

#[test]
fn packed_arrays_refused() {
    // Footnote 2, "word aligned": the size of a 17-bit element is not
    // stated.
    assert_err(
        "module m; type a = packed array[1..4] of 0..100000;",
        "packed array of 17-bit elements",
    );
    assert_err(
        "module m; type a = packed array[1..4] of set of 0..7;",
        "packed array of sets",
    );
    assert_err(
        "module m; type e = (a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, \
         a15, a16); a = packed array[1..4] of e;",
        "one-byte enumeration",
    );
    // An array of non-characters of an odd size takes "exact number of
    // words" in a packed array: the padding is not settled.
    assert_err(
        "module m; type a = packed array[1..2] of packed array[1..3] of 0..255;",
        "odd number of bytes",
    );
}

#[test]
fn packed_array_dimensions_are_each_packed() {
    let p = module("type m = packed array[1..2, 1..16] of boolean;");
    // Each row is a packed array of 16 bits.
    assert_eq!(size(&p, "m"), 4);
    assert_eq!(p.types.size(elem(&p, "m")), 2);
}

#[test]
fn packed_array_initialisers() {
    let p = module(
        "var
           bi: packed array[1..16] of boolean := [true, false, true, 9 := true, * of false];
           nib: packed array[1..3] of 0..7 := [5, 1, 7];
           fs: packed array[0..3] of 0..255 := [16#08, 16#0C, 16#3A, 16#5C];",
    );
    let init = |n: &str| var(&p, n).init.clone().unwrap();
    let bi = init("bi");
    let at = |s: &InitStore| (s.offset, s.bits, s.value.clone());
    let b = |bit, width| Some(BitPos { bit, width });
    assert_eq!(at(&bi[0]), (0, b(0, 1), ConstValue::Bool(true).into()));
    assert_eq!(at(&bi[1]), (0, b(1, 1), ConstValue::Bool(false).into()));
    assert_eq!(at(&bi[2]), (0, b(2, 1), ConstValue::Bool(true).into()));
    // Element 9: bit 0 of byte 1.
    assert_eq!(at(&bi[3]), (1, b(0, 1), ConstValue::Bool(true).into()));
    // Elements 1..3, 9, then 10..16.
    assert_eq!(bi.len(), 11);
    let nib = init("nib");
    assert_eq!(at(&nib[0]), (0, b(0, 4), ConstValue::Int(5).into()));
    assert_eq!(at(&nib[1]), (0, b(4, 4), ConstValue::Int(1).into()));
    assert_eq!(at(&nib[2]), (1, b(0, 4), ConstValue::Int(7).into()));
    let fs = init("fs");
    assert_eq!(
        fs.iter().map(|s| (s.offset, s.bits)).collect::<Vec<_>>(),
        vec![(0, None), (1, None), (2, None), (3, None)]
    );
    assert_eq!(p.types.size(fs[0].ty), 1);
}

#[test]
fn bit_elements_are_indexed_by_bits() {
    let p = module(
        "var bm: packed array[1..64] of boolean; i: integer; b: boolean;
         procedure q; begin b := bm[i]; end;",
    );
    let r = routine(&p, "q");
    let StmtKind::Assign { value, .. } = &r.body.as_ref().unwrap().stmts[0].kind else {
        panic!()
    };
    match &value.kind {
        ExprKind::Index { elem_bits, lo, .. } => {
            assert_eq!(*elem_bits, Some(1));
            assert_eq!(*lo, 1);
        }
        k => panic!("{k:?}"),
    }
}
