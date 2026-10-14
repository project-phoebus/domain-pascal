{ Execution test: values held in registers around a loop that calls
  M$ARITH and a stub each time round (codegen-patterns 12.4: what
  survives a call is in a callee-saved register or in memory), and a
  value read before the loop that the loop changes.

  call: REGLOOP_$MAIN l:2
  call: REGLOOP_$MAIN l:-3
  expect:
    1296
    4
    7
    6561
    4
    7
}
module regloop;
%include 'exec.ins.pas';

var
  g: integer;

procedure regloop_$main(n: integer32); options(val_param);
var
  acc: integer32;
  k: integer;
  x: integer;
begin
  acc := 1;
  k := 0;
  g := 3;
  x := g;
  repeat
    acc := acc * 3;
    acc := acc * n;
    k := k + 1;
    g := g + 1;
    exec_$clobber;
  until k >= 4;
  exec_$logv_l(acc);
  exec_$logv_w(k);
  x := g;
  exec_$logv_w(x);
end;
