{ Execution test (bead domain-pascal-338r, domain-c's gap 24): a right
  operand of a plain `and'/`or' that calls in some other form, a nested
  plain `or' with a comparison of a call (the DOMAIN C compiler's
  cg_assign.pas, `(rlen = 5) and ((peek(...) <> 31) or (asnop <> 0))')
  and `odd' of a quotient M$ARITH computes (final.pas): formed as a
  boolean value, `tst.b', the branch on the sign (best effort, no
  site).  Also two pointer functions' results compared (final.pas
  `br_target(x) = nxtlcc(xa)'): the first kept across the second call.

  call: AND_OR_NESTED_CALL_$RUN w:5 w:4 w:31 w:0
  expect:
    1
    1
    0
    1
    1
    1
    1
    0
    1
}
module and_or_nested_call;
%include 'exec.ins.pas';

type
  p_t = ^integer;
var
  x, y: integer;
  big: integer32;

function peek(k: integer): integer; options(val_param);
begin
  peek := k;
end;

function px(k: integer): p_t; options(val_param);
begin
  if k = 0 then px := addr(x) else px := addr(y);
end;

procedure one(k: integer; asnop: integer); options(val_param);
begin
  if (k = 5) and ((peek(asnop) <> 31) or (asnop <> 0)) then exec_$logv_w(1)
  else exec_$logv_w(0);
  if (k > 3) and odd(big div k) then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure and_or_nested_call_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  big := 100005;
  one(i, j);
  one(j, k);
  one(i, k);
  if px(l) = px(0) then exec_$logv_w(1) else exec_$logv_w(0);
  if px(1) = px(l) then exec_$logv_w(1) else exec_$logv_w(0);
  if px(l) <> px(1) then exec_$logv_w(1) else exec_$logv_w(0);
end;
