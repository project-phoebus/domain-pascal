{ Execution test: locals held in registers (codegen-patterns 12.4, 12.5)
  must survive calls, which clobber D0/D1/A0/A1 (the stubs do), branches
  and loops; M$ARITH results held across later calls.

  call: REGHELD_$ACROSS_CALLS w:5 l:100
  call: REGHELD_$BRANCHES w:20 w:3 -> w
  call: REGHELD_$BRANCHES w:3 w:10 -> w
  call: REGHELD_$MARITH l:12345 l:-678
  call: REGHELD_$MARITH l:100000 l:3
  expect:
    6
    99
    12
    127
    17
    => 199
    mark 1
    0
    => 16
    -8369910
    -1195701
    -39910
    -3
    300000
    42857
    20000
    1
}
module regheld;
%include 'exec.ins.pas';

procedure regheld_$across_calls(a: integer; b: integer32); options(val_param);
var
  x: integer;
  y: integer32;
  i: integer;
begin
  x := a + 1;
  y := b - 1;
  exec_$clobber;
  exec_$logv_w(x);
  exec_$logv_l(y);
  for i := 1 to 3 do begin
    x := x + i;
    exec_$clobber;
    y := y + x;
  end;
  exec_$logv_w(x);
  exec_$logv_l(y);
end;

var
  gb: integer;

function regheld_$branches(a: integer; b: integer): integer; options(val_param);
var
  x, y: integer;
begin
  x := a;
  y := 0;
  gb := b;
  if a > gb then begin
    y := a - b;
    exec_$clobber;
  end
  else begin
    x := b;
    exec_$mark(1);
  end;
  exec_$logv_w(y);
  while x > 0 do begin
    y := y + 1;
    x := x - 7;
  end;
  regheld_$branches := y * 10 + x;
end;

procedure regheld_$marith(a: integer32; b: integer32); options(val_param);
var
  p, q, r: integer32;
begin
  p := a * b;
  q := p div 7;
  r := p mod 70000;
  exec_$clobber;
  exec_$logv_l(p);
  exec_$logv_l(q);
  exec_$logv_l(r);
  q := q * 7;
  exec_$logv_l(p - q);
end;
