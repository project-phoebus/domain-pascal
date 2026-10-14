{ Execution test: a reused value (codegen-patterns 12.4 R7, the CSE of
  csesurvey) must not survive a store that may change it: an array
  element through another index, a variable passed to a var parameter,
  a field stored again, a store through a pointer.

  call: CSE_$MAIN w:2 w:2
  call: CSE_$MAIN w:2 w:3
  expect:
    7
    16
    103
    105
    205
    4
    4
    9
    5
    10
    103
    105
    205
    4
    6
    1
}
module cse;
%include 'exec.ins.pas';

type
  rec = record
    a: integer;
    b: integer;
  end;
  intp = ^integer;
var
  arr: array[1..8] of integer;
  g: integer;
  r: rec;

procedure bump(var v: integer);
begin
  v := v + 100;
end;

procedure cse_$main(i: integer; j: integer); options(val_param);
var
  x, y: integer;
  p: intp;
begin
  arr[i] := 5;
  arr[j] := 7;
  x := arr[i];
  y := arr[j];
  y := y + 1;
  arr[j] := y;
  y := arr[i] + arr[i];
  exec_$logv_w(x);
  exec_$logv_w(y);
  g := 3;
  bump(g);
  exec_$logv_w(g);
  x := g + i;
  bump(g);
  y := g + i;
  exec_$logv_w(x);
  exec_$logv_w(y);
  r.a := i;
  x := r.a * 2;
  r.a := j;
  y := r.a * 2;
  exec_$logv_w(x);
  exec_$logv_w(y);
  arr[i] := 1;
  p := exec_$id_p(addr(arr[j]));
  p^ := 9;
  exec_$logv_w(arr[i]);
end;
