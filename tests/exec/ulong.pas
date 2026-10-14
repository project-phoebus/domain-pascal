{ Execution test: a non-negative subrange stored in 4 bytes
  (0..2147483647, base.ins.pas's `linteger') is unsigned (ref 3.4 p. 3-9;
  4.3.3 pp. 4-6..4-7; codegen-patterns 5, 7.1, 8; bead domain-pascal-5j9).
  Its bits can exceed 2^31 - 1 (ref 3.4's NOTE: the full unsigned range
  is reached only by type transfer; here the caller passes -2, i.e.
  16#FFFFFFFE): compares with a constant or another unsigned value use
  the unsigned conditions, `div'/`mod' the unsigned M$ARITH entries and
  `lsr.l'.  Against an integer32 or an integer the compare is signed
  (ref 4.3.3: "signed except" when both are unsigned): -1 < 3.

  call: ULONG_$MAIN l:-2 l:3 l:-1 w:-1
  expect:
    mark 1
    mark 2
    0x0fffffff
    0x00418937
    294
    0x55555554
    mark 4
    mark 5
}
module ulong;
%include 'exec.ins.pas';

type
  linteger = 0..2147483647;
var
  gu: linteger;
  gl: integer32;
  gs: integer32;
  gw: integer;

procedure ulong_$main(a: linteger; b: linteger; s: integer32; w: integer);
  options(val_param);
begin
  gu := b;
  if a > 1000 then exec_$mark(1);
  if a > gu then exec_$mark(2);
  if a < gu then exec_$mark(3);
  gl := a div 16;
  exec_$log_x(gl);
  gl := a div 1000;
  exec_$log_x(gl);
  gl := a mod 1000;
  exec_$logv_l(gl);
  gl := a div gu;
  exec_$log_x(gl);
  gs := s;
  if gs < gu then exec_$mark(4);
  gw := w;
  if gw < gu then exec_$mark(5);
end;
