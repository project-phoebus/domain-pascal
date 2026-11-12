{ Execution test (bead domain-pascal-338r, domain-c's gap 15): a
  pointer function's result assigned a parameter in a body of several
  statements, on a path that returns early and on the last statement
  (codegen-patterns 3; best effort, no kernel site): the parameter's
  register moved into the result's.  The DOMAIN C compiler's
  clex_$catstring (reported: cc.m68k 0x055FE4) `movea.l A2,A0'.

  call: PTR_RESULT_PARAM_$RUN
  expect:
    20
    0
    10
    1
}
module ptr_result_param;
%include 'exec.ins.pas';

type
  p_t = ^integer;
var
  n: integer;
  x, y: integer;

function f(a: p_t; b: p_t): p_t; options(val_param);
begin
  if a = nil then begin
    f := b;
    return;
  end;
  n := 1;
  f := a;
end;

procedure ptr_result_param_$run;
begin
  x := 10;
  y := 20;
  n := 0;
  exec_$logv_w(f(nil, addr(y))^);
  exec_$logv_w(n);
  exec_$logv_w(f(addr(x), addr(y))^);
  exec_$logv_w(n);
end;
