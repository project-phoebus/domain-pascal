{ Execution test (bead domain-pascal-5kgg, best effort): comparisons
  whose image form is not catalogued (codegen-patterns 7.1): a constant
  on the left (`3 < g + w', not swapped), a computed value against a
  function's result on either side, a value tested right after the
  `and' or the multiply that formed it with `>' (a `tst' then), a
  word set intersection against `[]' whose flags the `and' did not set.

  warns: best effort (not byte-identical): a comparison whose left operand is not a variable
  warns: best effort (not byte-identical): a comparison of a computed value with one whose evaluation calls
  warns: best effort (not byte-identical): a comparison of a value whose evaluation calls
  warns: best effort (not byte-identical): this test of a value right after the operation that formed it
  call: BE_CMP_$CONST w:1 w:2
  call: BE_CMP_$CONST w:1 w:1
  call: BE_CMP_$CALL_RHS w:4 w:5
  call: BE_CMP_$CALL_RHS w:4 w:6
  call: BE_CMP_$CALL_LHS w:7 w:6
  call: BE_CMP_$CALL_LHS w:7 w:7
  call: BE_CMP_$AND_GT w:9
  call: BE_CMP_$AND_GT w:8
  call: BE_CMP_$AND_GT w:-1
  call: BE_CMP_$MUL_GT w:3
  call: BE_CMP_$MUL_GT w:0
  call: BE_CMP_$MUL_GT w:-2
  expect:
    1
    0
    1
    0
    1
    0
    1
    0
    1
    1
    0
    0
}
module be_cmp;
%include 'exec.ins.pas';

var
  g, w, h: integer;

procedure be_cmp_$const(a, b: integer); options(val_param);
begin
  g := a;
  h := b;
  if 2 < g + h then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_cmp_$call_rhs(a, b: integer); options(val_param);
begin
  g := a;
  h := b;
  if g + 1 = exec_$id_w(h) then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_cmp_$call_lhs(a, b: integer); options(val_param);
begin
  g := a;
  h := b;
  if exec_$id_w(g) = h + 1 then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_cmp_$and_gt(v: integer); options(val_param);
begin
  if (v & 7) > 0 then w := 1 else w := 0;
  exec_$log_w(w);
end;

procedure be_cmp_$mul_gt(v: integer); options(val_param);
begin
  if (v * 3) > 0 then w := 1 else w := 0;
  exec_$log_w(w);
end;
