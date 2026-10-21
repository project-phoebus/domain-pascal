{ Golden: codegen-patterns 7.1 ("Signed or unsigned"; bead
  domain-pascal-wkm5).  A word type that uses the top bit (pinteger,
  0..65535) against 0: `tst.w' (or the load's flags) and the signed
  condition, as for an integer, while its bounds against other constants
  stay unsigned (`bhi'/`bls').
  - `(length <= 0) or else (length > 16#4000)': MT_$ACQUIRE SAU5
    0xE760B6 `move.w (A0),D1w; ble; cmpi.w #0x4000,D1w; bls'; the
    module-local after MT_PURGE_UCB, SAU5 0xE75916 `move.w (0x16,A6),D1w;
    ble.b; cmpi.w #0x4000,D1w; bhi.b'; SOCK_$PUT_INT SAU2 0xE161AA
    `tst.w D0w; ble.b; cmpi.w #0xe0,D0w; bls.b' (SAU5 0xE167F6).
    The image holds the length in D1; dpc tests it in memory.
  - `x > 0': PKT_$BLD_INTERNET_HDR 0xE122D6 `tst.w D2w; ble.b' around
    the part.
  - `(unit < 0) or else (unit > 15)': PBU_$GET_DCTE SAU5 0xE59212
    `move.w (A0),D1w; bmi.b; cmpi.w #0xf,D1w; bls.b'. }
module top_bit_zero;
type
  pinteger = 0..65535;
var
  n: extern pinteger;
  r: extern integer;

procedure check(in length: pinteger; out status: integer32);
begin
  if (length <= 0) or else (length > 16#4000) then
    status := 16#220003
  else
    status := 0;
end;

procedure positive;
begin
  if n > 0 then r := 1;
end;

procedure unit_check(in unit: pinteger; out status: integer32);
begin
  status := 0;
  if (unit < 0) or else (unit > 15) then
    status := 16#1e000c;
end;
