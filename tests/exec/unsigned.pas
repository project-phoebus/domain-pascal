{ Execution test: subranges 0..65535 are unsigned words (ref 3.4):
  comparisons must use the unsigned conditions (bhi/bcs, codegen-patterns
  7.1), and widening must zero-extend.

  call: UNSIGNED_$CMP w:40000 w:100
  call: UNSIGNED_$CMP w:100 w:40000
  call: UNSIGNED_$CMP w:65535 w:65535
  call: UNSIGNED_$WIDEN w:50000
  call: UNSIGNED_$BYTES w:200 w:100
  expect:
    mark 1
    mark 4
    mark 6
    mark 3
    mark 2
    mark 6
    mark 2
    mark 4
    mark 5
    50000
    50001
    300
    mark 7
}
module unsigned;
%include 'exec.ins.pas';

type
  uword = 0..65535;
  ubyte = 0..255;
var
  gu: uword;
  gl: integer32;
  gb: ubyte;

procedure unsigned_$cmp(a: uword; b: uword); options(val_param);
begin
  gu := b;
  if a > gu then exec_$mark(1);
  if a < gu then exec_$mark(3);
  if a <= gu then exec_$mark(2);
  if a >= gu then exec_$mark(4);
  if a = gu then exec_$mark(5);
  if a <> gu then exec_$mark(6);
end;

procedure unsigned_$widen(a: uword); options(val_param);
begin
  gl := a;
  exec_$logv_l(gl);
  gl := a + 1;
  exec_$logv_l(gl);
end;

procedure unsigned_$bytes(a: uword; b: uword); options(val_param);
var
  x, y: ubyte;
begin
  x := a;
  y := b;
  gl := x;
  gl := gl + y;
  exec_$logv_l(gl);
  gb := y;
  if x > gb then exec_$mark(7);
end;
