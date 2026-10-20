{ Execution test (bead domain-pascal-5kgg, best effort): `in' forms
  whose image form is not catalogued (codegen-patterns 5.1): member 15 of
  an indexed word set and member 7 of a set held in a register (`btst
  #15'/`#7' in the register), a run-time element on a one-word constant
  set that `moveq' cannot load (the constant a longword, bits 16..31
  clear).

  warns: best effort (not byte-identical): `in` testing member 7, 15 or 31 of a set in a register
  warns: best effort (not byte-identical): `in` with a run-time element on a one-word constant set
  call: BE_SETS_$SIGN w:2 w:-32768
  call: BE_SETS_$SIGN w:2 w:32767
  call: BE_SETS_$HELD w:128
  call: BE_SETS_$HELD w:32
  call: BE_SETS_$CONST w:9
  call: BE_SETS_$CONST w:12
  call: BE_SETS_$CONST w:2
  call: BE_SETS_$CONST w:15
  expect:
    1
    0
    1
    2
    1
    1
    0
    0
}
module be_sets;
%include 'exec.ins.pas';

type
  s16 = set of 0..15;
  uw = 0..65535;
var
  x: array[1..3] of s16;
  w: integer;

procedure be_sets_$sign(i: integer; bits: uw); options(val_param);
begin
  x[i] := [];
  if bits >= 32768 then x[i] := [15];
  if 15 in x[i] then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_sets_$held(v: s16); options(val_param);
begin
  w := 0;
  if 7 in v then w := 1;
  if 5 in v then w := 2;
  exec_$log_w(w);
end;

procedure be_sets_$const(i: integer); options(val_param);
begin
  if i in [1, 9, 12] then w := 1 else w := 0;
  exec_$log_w(w);
end;
