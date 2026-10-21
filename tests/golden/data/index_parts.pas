{ Golden: the index's constant part and computed indices (codegen-patterns
  5.2, "The index's constant part", "Computed indices"; bead
  domain-pascal-69fe; exec test index_parts).
  - ip_$sh: SAU5 mt_$set_iomap 0xE75CBA `lsr.l #8; lsr.l #2; lsl.w #2,D2;
    move.l D0,(0,A0,D2.w)' (`iomap[rshft(iova, 10)]', 0..1023 of
    longwords): a computed longword index into an array whose bounds times
    the element size fit in 16 bits, its low word scaled, `.w'.
  - ip_$sh1: SAU5 MT_$GET_IOVA 0xE76DDA..0xE76E02 `lsr.l #8; lsr.l #2;
    lsl.w #2,D1; or.l (0x1a,A0,D1.w),D0': the same with `+ 1', the 1 in
    the displacement.
  - ip_$mm: MMAP_$INIT 0xE31A44..0xE31A50 `asr.l #8; asr.l #2; move.w
    D0,D1; lsl.w #2,D1; clr.l (0,A0,D1.w)' (`div 1024 + 1' of a 1-based
    table; the copy there is the image's, not modelled).
  - ip_$lv: a longword variable stays `.l', EC2_$REGISTER_EC1
    0xE429A4..0xE429AA `move.l (3736,A5),D0; lsl.l #2,D0; lea
    (0,A5,D0.l),A0' (a 257-entry table); ip_$lv1 with a constant part,
    audit_$log_cname_op 0xE4BF0E `lea (0,A6,D3.l),A0; lea (-527,A0),A2'
    (`data.names[n1 + 1]', n1 a longword).
  - ip_$wp1, ip_$wk, ip_$wm2: `x + k', `k + x', `x - k' of a word, `k'
    in the displacement (PEB_UNWIRED's module-local 0xE31DD4 at 0xE31E2C..0xE31E36
    `add.w D2,D2; lea (0,A1,D2.w),A1' for `x + 1').
  - ip_$z48: a word with a constant part into an array that does not fit
    in 16 bits, zero-extended (AREA_'s module-local 0xE075CA at
    0xE076C6..0xE076D8 `clr.l D3; movea.l #sym,A3; move.w D2,D3; lsl.l
    #4,D3; ...', `areas[id + 1]').
  - ip_$m12: a computed longword index with an element size that is not a
    power of two: unchanged, `.l'. }
module ipg;

type
  linteger = 0..2147483647;
  pinteger = 0..65535;
  r48 = record z: integer32; pad: array[1..22] of integer; end;
  r12 = record a, b, c: integer32; end;
var
  ip_$tbl: extern array[0..1023] of integer32;
  ip_$t56: extern array[1..56] of integer32;
  ip_$w8: extern array[1..8] of integer;
  wi_$r48: extern array[1..16#540] of r48;
  ip_$r12: extern array[0..7] of r12;

{ `rshft' of a longword, its low word scaled: `lsl.w #2', `.w'. }
function ip_$sh(x: linteger): integer32;
  options(val_param);
begin
  ip_$sh := ip_$tbl[rshft(x, 10)];
end;

{ The same plus 1, the 1 in the displacement. }
function ip_$sh1(x: linteger): integer32;
  options(val_param);
begin
  ip_$sh1 := ip_$tbl[rshft(x, 10) + 1];
end;

{ MMAP_$INIT's index, of a 1-based table. }
function ip_$mm(v: linteger): integer32;
  options(val_param);
begin
  ip_$mm := ip_$t56[((v - 512) * 16) div 1024 + 1];
end;

{ A longword variable: `.l'. }
function ip_$lv(l: integer32): integer32;
  options(val_param);
begin
  ip_$lv := ip_$tbl[l];
end;

function ip_$wp1(i: integer): integer;
  options(val_param);
begin
  ip_$wp1 := ip_$w8[i + 1];
end;

function ip_$wk(i: integer): integer;
  options(val_param);
begin
  ip_$wk := ip_$w8[3 + i];
end;

function ip_$wm2(i: integer): integer;
  options(val_param);
begin
  ip_$wm2 := ip_$w8[i - 2];
end;

{ Zero-extended, ×48 at a longword's width (the parameter tested first,
  so held in a register, as wide_index.pas's wi_$u48). }
function ip_$z48(k: pinteger): integer32;
  options(val_param);
begin
  if k = 65535 then
    ip_$z48 := 0
  else
    ip_$z48 := wi_$r48[k + 1].z;
end;

{ A computed longword index scaled by a product, 12: unchanged, `.l' (the
  one product seen, HINT_$GET_HINTS 0xE4998E, narrows to a word but
  multiplies with `muls.w' and indexes `.l'; not this form). }
function ip_$m12(x: linteger): integer32;
  options(val_param);
begin
  ip_$m12 := ip_$r12[x & 7].b;
end;

function ip_$lv1(l: integer32): integer32;
  options(val_param);
begin
  ip_$lv1 := ip_$tbl[l + 1];
end;
