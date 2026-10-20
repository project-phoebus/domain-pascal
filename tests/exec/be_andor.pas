{ Execution test (bead domain-pascal-5kgg, best effort): plain `and'
  with a function call in its right operand (codegen-patterns 7.1, "Plain
  `and` and `or`"; both operands evaluated, ref p. 4-18): as a value
  (the left operand's boolean held across the call), and in a condition
  comparing the result with a static variable (the `s<cc>' register not
  explained at the image's sites).

  warns: best effort (not byte-identical): a plain `and`/`or` with a call in its right operand
  warns: best effort (not byte-identical): a comparison of a function's result with a static variable
  call: BE_ANDOR_$VALUE w:1 w:4
  call: BE_ANDOR_$VALUE w:1 w:3
  call: BE_ANDOR_$VALUE w:0 w:4
  call: BE_ANDOR_$STATIC w:5 w:5
  call: BE_ANDOR_$STATIC w:5 w:6
  call: BE_ANDOR_$STATIC w:-5 w:-5
  expect:
    true
    false
    false
    1
    0
    0
}
module be_andor;
%include 'exec.ins.pas';

var
  g, h, w: integer;
  bo: boolean;

procedure be_andor_$value(a, b: integer); options(val_param);
begin
  w := a;
  g := b;
  bo := (w > 0) and (exec_$id_w(g) > 3);
  exec_$log_bool(bo);
end;

procedure be_andor_$static(a, b: integer); options(val_param);
begin
  w := a;
  g := b;
  h := 0;
  if (w > 0) and (exec_$id_w(w) = g) then h := 1;
  exec_$log_w(h);
end;
