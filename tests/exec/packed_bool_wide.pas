{ Execution test (bead domain-pascal-338r, domain-c's gap 13): elements
  of packed arrays of booleans (codegen-patterns 5.2; best effort, no
  kernel site): a 2048-element array through a pointer tested with a
  variable index (the position on the index's word; the DOMAIN C
  compiler's pds_$end_scope, reported: cc.m68k 0x06D7C6), as an operand
  of a plain `and' and as a value; constant indexes as one-bit fields
  (put_procedure_start's `tst.b (A0)' for element 0, reported), read
  and written; an integer32 variable as the index (its low word).

  call: PACKED_BOOL_WIDE_$RUN w:1 w:9 w:2000 w:2048
  expect:
    1
    0
    0
    0
    1
    1
    1
    1
    1
    0
    1
    1
    0
    1
    0
}
module packed_bool_wide;
%include 'exec.ins.pas';

type
  bits_t = packed array [1..2048] of boolean;
  zbits_t = packed array [0..1023] of boolean;
var
  big: bits_t;
  z: zbits_t;
  b: ^bits_t;
  f: boolean;
  l: integer32;

procedure one(i: integer); options(val_param);
begin
  if b^[i] then exec_$logv_w(1) else exec_$logv_w(0);
  f := (i > 4) and b^[i];
  if f then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure packed_bool_wide_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  b := addr(big);
  big[1] := true;
  big[2000] := true;
  big[2048] := true;
  one(i);
  one(j);
  one(k);
  one(l);
  z[0] := true;
  z[9] := false;
  if z[0] then exec_$logv_w(1) else exec_$logv_w(0);
  if z[9] then exec_$logv_w(1) else exec_$logv_w(0);
  f := big[2000];
  if f then exec_$logv_w(1) else exec_$logv_w(0);
  z[9] := true;
  if z[9] and not z[10] then exec_$logv_w(1) else exec_$logv_w(0);
  if big[2047] then exec_$logv_w(1) else exec_$logv_w(0);
  l := 2048;
  if b^[l] then exec_$logv_w(1) else exec_$logv_w(0);
  l := 2046;
  if b^[l] then exec_$logv_w(1) else exec_$logv_w(0);
end;
