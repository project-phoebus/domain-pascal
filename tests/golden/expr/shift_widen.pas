{ Golden: `lshft'/`rshft' of a 16-bit operand, expanded to 32 bits by
  its sign and shifted there, the result an integer32, unsigned for an
  unsigned operand (owner's ruling, reference-clarifications "Lshft of a
  16-bit operand; the sign of a shift of an unsigned word";
  codegen-patterns 8, "Shifts of a 16-bit operand"; bead
  domain-pascal-4rfc).
  - A `0..65535' word into a longword: widened with zeros, shifted in 32
    bits, a count above 8 split: AS_$GET_ADDR 0xE58418 `clr.l D0; move.w
    (0x00e2445c).l,D0w; lsl.l #8,D0; lsl.l #7,D0; move.l D0,(4,A0)'
    (`addr_range.size := lshft(mst_$private_a_size, 15)'); UID_$GEN
    0xE1A04A `clr.l D2; move.w (-0x10,A6),D2w; lsr.l #8,D2; lsr.l #4,D2'
    (`rshft(abs_clock.low, 12)').
  - A count in a word variable: module-local 0xE3C1CE..0xE3C1EA `move.w
    (0xb0,A1),D6w; ...; lsl.l D6,D5' (the word in D5 widened by `clr.l
    D5; move.w (4,A2),D5w').
  - Stored as a word: shifted at the word's width, AREA_'s module-local
    0xE07386 `move.w D0w,D4w; lsr.w #3,D4w' (`seg_group :=
    rshft(seg_idx, 3)'), module-local 0xE3CB9E `move.w (0xa6,A1),D6w;
    lsl.w D6,D1w' (`sectors := lshft(rem, sector_size_code)').
  - Unsigned: compared with an unsigned longword, `bhi'/`bls'
    (AREA_$REMOVE_SEG 0xE0998E..0xE09996, MST_$INVALIDATE
    0xE44222..0xE4422E); added to another unsigned word, the sum
    unsigned (module-local 0xE09412..0xE0942A, `bls'); `div' and `mod'
    in the unsigned forms (ref 4.3.3, pp. 4-6..4-7: M$DIU$LLL,
    M$OIU$LLL; `lsr' for a power of two, as for any unsigned longword,
    codegen-patterns 8).
  - An `integer' into a longword: `ext.l', then the 32-bit shift
    (`ext.l; lsl.l' 212 times in compiled code; `rshft' at PEB_'s
    module-local 0xE31DEA `move.w D0w,D1w; ext.l D1; lsr.l #7,D1'); its
    `lshft' stored into an `integer' at the word's width, its `rshft'
    computed in 32 bits and the low word stored (ref p. 4-171,
    `rshft(-9, 3)' = -2).
  - `arshft' of an `integer' as a longword: `asr.w', then `ext.l' (the
    same value as p. 4-29's expansion; no image site). }
module sw;

type
  pinteger = 0..65535;
  linteger = 0..2147483647;

var
  w1, w2, w3, w4, w5, w6, w7, w8, w9, w10: pinteger;
  v1, v2, n: pinteger;
  i1, i2: integer;
  l: integer32;
  u: linteger;
  b: boolean;
  s1, s2, s3, s4, s5, s6: integer;
  j1, j2: integer;
  w11, w12, w13: pinteger;

procedure sw_$long;
begin
  l := lshft(w1, 15);
  l := rshft(w2, 12);
  l := lshft(w3, n);
  l := rshft(w4, n);
  u := lshft(w5, 5);
end;

procedure sw_$word;
begin
  v1 := rshft(w6, 3);
  i1 := rshft(w7, 3);
  v2 := lshft(w8, n);
  i2 := lshft(w9, 5);
end;

procedure sw_$compare;
begin
  b := lshft(w10, 5) > u;
  b := v1 + lshft(w1, 5) > u;
end;

procedure sw_$div;
begin
  l := rshft(w11, 3) div u;
  l := lshft(w12, 3) div 8;
  l := rshft(w13, 3) mod u;
end;

procedure sw_$signed;
begin
  l := lshft(s1, 7);
  j1 := lshft(s2, 7);
  l := rshft(s3, 3);
  j2 := rshft(s4, 3);
  b := lshft(s5, 7) > 5;
  l := arshft(s6, 3);
end;
