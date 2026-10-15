{ Golden: codegen-patterns 11 ("Initial values", packed records), ref
  3.10.7 p. 3-35 (bead domain-pascal-c0j8).  A packed record's bit
  fields initialised in the data block: fields left to right within
  bytes, the first in the most significant bits.  The table is KBD's
  kbd_$state_table, WIRED_DATA 0xE2DDE4+0xC0 (`FB 20 E8 31 DF 31 FE B4
  FF 10 00 10 00 42 00 53 00 6F 00 C0'): a char, then two 4-bit
  subranges in one byte.  Then a boolean (one bit, set for true), an
  enumeration, a negative subrange (two's complement, across a byte
  boundary), a 10-bit field across a byte, folded constant expressions
  and a repeat count. }
module pkinit, wired_proc, wired_data;
type
  entry = packed record
    key: char;
    action: 0..15;
    next_state: 0..15;
  end;
  colour = (red, green, blue, cyan, magenta);
  mixed = packed record
    b: boolean;
    c: colour;
    s: -100..40;
    w: 0..1023;
    t: boolean;
  end;
var
  table: array[0..9] of entry := [
    [key := chr(16#FB), action := 2, next_state := 0],
    [key := chr(16#E8), action := 3, next_state := 1],
    [key := chr(16#DF), action := 3, next_state := 1],
    [key := chr(16#FE), action := 11, next_state := 4],
    [key := chr(16#FF), action := 1, next_state := 0],
    [key := chr(0), action := 1, next_state := 0],
    [key := chr(0), action := 4, next_state := 2],
    [key := chr(0), action := 5, next_state := 3],
    [key := chr(0), action := 6, next_state := 15],
    [key := chr(0), action := 12, next_state := 0]];
  m: array[1..3] of mixed := [2 of [true, magenta, -100, 1023, true],
                              [w := 5 * 3 + 1, s := -1]];
