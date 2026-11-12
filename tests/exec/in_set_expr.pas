{ Execution test (bead domain-pascal-338r, domain-c's gap 8): `k in a
  op b' for sets of more than a longword, `op' `+', `*' or `-', the
  element a variable: tested as `(k in a) or (k in b)', `and', `and
  not' (codegen-patterns 5.1; best effort, no kernel site).  In a
  condition and as a value under `not', as in the DOMAIN C compiler's
  c_syntax.pas (`token_kind in s + [...]').

  call: IN_SET_EXPR_$RUN w:3 w:40 w:70 w:100
  expect:
    1
    1
    0
    0
    1
    0
    0
    1
    1
    0
    1
    0
    1
    0
    0
    0
}
module in_set_expr;
%include 'exec.ins.pas';

type
  s_t = set of 0..111;
var
  a, b: s_t;
  r: boolean;

procedure one(k: integer); options(val_param);
begin
  if k in a + [40, 100..105] then exec_$logv_w(1) else exec_$logv_w(0);
  if k in a * b then exec_$logv_w(1) else exec_$logv_w(0);
  if k in a - b then exec_$logv_w(1) else exec_$logv_w(0);
  r := not (k in b + [70]);
  if r then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure in_set_expr_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  a := [3, 70];
  b := [3, 100];
  one(i);
  one(j);
  one(k);
  one(l);
end;
