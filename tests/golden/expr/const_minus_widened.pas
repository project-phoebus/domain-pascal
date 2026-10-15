{ Golden: a constant minus a word widened to a longword (ref 4.3.1,
  p. 4-5; codegen-patterns 12.4 R8; bead domain-pascal-bzkf): the word
  loaded and extended, the constant loaded into a register of its own and
  the word subtracted from it, the constant's load scheduled between the
  word's load and its extension (12.4 R11).
  - from_word: NETWORK_$READ_AHEAD 0xE10000..0xE1000A `move.w D4,D0;
    move.l #0x400,D1; ext.l D0; sub.l D0,D1' (XNS_ERROR_$SEND 0xE17B22
    with `moveq #76,D1').
  - from_uword: AREA_$INVALIDATE 0xE08EF4..0xE08EFC `clr.l D0; moveq
    #32,D1; move.w (0xe,A6),D0; sub.l D0,D1', a `0..65535' word widened
    with zeros. }
module const_minus_widened;
type
  pinteger = 0..65535;
var
  w: integer;
  l: integer32;
procedure from_word;
begin
  l := 1024 - w;
end;
procedure from_uword(n: pinteger); options(val_param);
begin
  l := 32 - n;
end;
