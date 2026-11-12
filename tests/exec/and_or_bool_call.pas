{ Execution test (bead domain-pascal-338r, domain-c's gap 24): a
  boolean function's result as the right operand of a plain `and'/`or'
  in a condition, also under `not' (codegen-patterns 7.1; best effort,
  no kernel site): the left operand jumping code, then the call, `tst.b'
  of its D0 and the branch on the sign.  The DOMAIN C compiler's
  get_alias_desc (reported: cc.m68k 0x0740CA) has `or f(y)'.

  call: AND_OR_BOOL_CALL_$RUN w:1 w:2 w:3 w:4
  expect:
    1
    0
    0
    1
    1
    0
    0
    0
    0
    0
    1
    1
    1
    1
    0
    0
}
module and_or_bool_call;
%include 'exec.ins.pas';

function even(n: integer): boolean; options(val_param);
begin
  even := (n & 1) = 0;
end;

procedure four(k: integer); options(val_param);
var
  r: integer;
begin
  r := 0;
  if (k = 1) or even(k) then r := 1;
  exec_$log_w(r);
  r := 0;
  if (k > 2) and even(k) then r := 1;
  exec_$log_w(r);
  r := 0;
  if (k > 2) and not even(k) then r := 1;
  exec_$log_w(r);
  r := 0;
  if (k = 3) or not even(k) then r := 1;
  exec_$log_w(r);
end;

procedure and_or_bool_call_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  four(i);
  four(j);
  four(k);
  four(l);
end;
