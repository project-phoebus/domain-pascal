{ Execution test: the index's constant part and computed indices
  (codegen-patterns 5.2, "The index's constant part", "Computed indices";
  bead domain-pascal-69fe).
  - `a[x + k]', `a[k + x]', `a[x - k]': `x' is the index register, `k'
    goes into the displacement with the lower bound (MMAP_$INIT 0xE31A50,
    PEB_UNWIRED's module-local 0xE31DD4 at 0xE31E2C..0xE31E36): ip_$w8 1..8 of
    words, at both ends.
  - A computed longword index into an array whose bounds times the
    element size fit in 16 bits: its low word scaled at the word's width,
    `.w' (SAU5 mt_$set_iomap 0xE75CBA `lsr.l #8; lsr.l #2; lsl.w #2,D2;
    move.l D0,(0,A0,D2.w)'; SAU5 MT_$GET_IOVA 0xE76DDA with `+ 1';
    MMAP_$INIT 0xE31A44..0xE31A50 `div 1024 + 1'): ip_$tbl 0..1023 of
    longwords at 0, 512 and 1023 (offset 4092); ip_$t56 1..56.
  - A longword variable stays `.l' (EC2_$REGISTER_EC1 0xE429A4), with a
    constant part too (audit_$log_cname_op 0xE4BF0E `lea (0,A6,D3.l),A0;
    lea (-527,A0),A2', `data.names[n1 + 1]').
  - Not folded where the displacement would pass 16 bits (k + 1000 of
    48-byte records: the sum computed, as
    before), and inside `with' (the fields' displacements).
  - A word variable with a constant part into an array that does not fit
    in 16 bits, zero-extended when the register cannot be negative
    (AREA_'s module-local 0xE075CA at 0xE076C6): wi_$r48 1..16#540 of 48
    bytes at offsets past 32767 (wide_index_data.mod.pas); sign-extended,
    best effort, when it can be: wi_$r24 -1000..1400 of 24 bytes, its
    register from -999.

  link: index_parts_data.mod.pas wide_index_data.mod.pas
  call: IP_$FILL
  call: WI_$FILL
  call: IP_$SH l:0 -> l
  call: IP_$SH l:524288 -> l
  call: IP_$SH l:1048552 -> l
  call: IP_$SH1 l:0 -> l
  call: IP_$SH1 l:1046528 -> l
  call: IP_$MM l:512 -> l
  call: IP_$MM l:1152 -> l
  call: IP_$MM l:4032 -> l
  call: IP_$LV l:1000 -> l
  call: IP_$LV1 l:1000 -> l
  call: IP_$LV1 l:1022 -> l
  call: IP_$FAR w:0 -> l
  call: IP_$FAR w:344 -> l
  call: IP_$WFAR w:344 -> l
  call: IP_$WP1 w:0 -> w
  call: IP_$WP1 w:7 -> w
  call: IP_$WK w:-2 -> w
  call: IP_$WK w:5 -> w
  call: IP_$WM2 w:3 -> w
  call: IP_$WM2 w:10 -> w
  call: IP_$Z48 w:0 -> l
  call: IP_$Z48 w:699 -> l
  call: IP_$Z48 w:1343 -> l
  call: IP_$S24 w:-999 -> l
  call: IP_$S24 w:1 -> l
  call: IP_$S24 w:1401 -> l
  warns: best effort (not byte-identical): a computed word index (a variable with a constant part) that may be negative
  expect:
    => 3
    => 3587
    => 7164
    => 10
    => 7164
    => 1001
    => 1011
    => 1056
    => 7003
    => 7010
    => 7164
    => 201000
    => 201344
    => 201344
    => 10
    => 80
    => 10
    => 80
    => 10
    => 80
    => 200001
    => 200700
    => 201344
    => 299000
    => 300000
    => 301400
}
module index_parts;

type
  linteger = 0..2147483647;
  pinteger = 0..65535;
  r48 = record z: integer32; pad: array[1..22] of integer; end;
  r24 = record z: integer32; pad: array[1..10] of integer; end;
var
  ip_$tbl: extern array[0..1023] of integer32;
  ip_$t56: extern array[1..56] of integer32;
  ip_$w8: extern array[1..8] of integer;
  wi_$r48: extern array[1..16#540] of r48;
  wi_$r24: extern array[-1000..1400] of r24;

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

{ A longword variable with a constant part: `.l', the 1 folded. }
function ip_$lv1(l: integer32): integer32;
  options(val_param);
begin
  ip_$lv1 := ip_$tbl[l + 1];
end;

{ A displacement beyond 16 bits: not folded. }
function ip_$far(k: pinteger): integer32;
  options(val_param);
begin
  ip_$far := wi_$r48[k + 1000].z;
end;

function ip_$wfar(k: pinteger): integer32;
  options(val_param);
begin
  with wi_$r48[k + 1000] do
    ip_$wfar := z;
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

{ Sign-extended (best effort). }
function ip_$s24(k: integer): integer32;
  options(val_param);
begin
  ip_$s24 := wi_$r24[k - 1].z;
end;
