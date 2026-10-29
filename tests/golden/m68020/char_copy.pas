{ codegen-patterns 14 ("Arrays of char"); bead domain-pascal-0p9w; ../source
  finding 361.  With `-cpu mathchip' an array of char is copied by
  longwords like a record (the MC68010 code copies it by bytes,
  codegen-patterns 5.2 "Arrays of char"): unrolled up to 18 bytes, a
  `dbf' loop above, and a byte for an odd remainder:
  - 12 bytes: `lea (-84,A6),A0; lea (8,A2),A1; move.l (A0)+,(A1)+' x3
    (SAU9 AST_$GET_COMMON_ATTRIBUTES 0x3C405D1C..0x3C405D28);
  - 13 bytes: three `move.l' and a `move.b' (XNS_IDP_$OS_SEND
    0x3C41A4E4..0x3C41A4F2);
  - 25 bytes: `moveq #5,D2; move.l (A0)+,(A1)+; dbf D2; move.b
    (A0)+,(A1)+' (MAC_$SEND 0x3C40DAE0..0x3C40DAEA);
  - 26 bytes: the loop and a `move.w' (a record's remainder; no char
    site);
  - 4 bytes: one longword store by displacement, as a 4-byte record's
    (the SAU9 CAL_$WRITE_TIMEZONE 0x3C445B7C copies a word and 4 chars as
    `move.l (A5),(224,A3); move.w (4,A5),(228,A3)'; Uncertain). }
module char_copy;
type
  a4 = array [1..4] of char;
  a12 = array [0..11] of char;
  a13 = array [1..13] of char;
  a26 = array [1..26] of char;
  a25 = array [1..25] of char;
  r_t = record a: integer32; b: integer32; name: a12; w: integer; end;
  pr_t = ^r_t;
var
  s4: a4; d4: extern a4;
  s12: a12; s13: a13; s26: a26; s25: a25;
  d13: extern a13; d26: extern a26; d25: extern a25;
procedure p(r: pr_t); options(val_param);
var l12: a12;
begin
  r^.name := l12;
  d13 := s13;
  d26 := s26;
  d25 := s25;
  d4 := s4;
end;
