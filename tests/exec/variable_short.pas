{ Execution test (bead domain-pascal-vq0h; ref 5.5.5, p. 5-17;
  codegen-patterns 1.4, "A call with fewer arguments"): a `variable' routine
  called with fewer arguments reads the ones it was given, by address and by
  value, with the stack left balanced by the caller in both.

  warns: a call with fewer arguments than parameters and a value in a slot
  call: VARS_$BY_ADDRESS -> l
  call: VARS_$BY_VALUE -> l
  expect:
    => 7
    => 9
}
module vars;

function first(in n, m, k: integer32): integer32; options(variable);
begin
  first := n;
end;

function second(a: integer; b: integer; c: integer): integer32;
  options(val_param, variable);
begin
  second := a;
end;

function vars_$by_address: integer32;
var r: integer32;
begin
  r := first(3);
  vars_$by_address := r + first(4, 5);
end;

function vars_$by_value: integer32;
var r: integer32;
begin
  r := second(4);
  vars_$by_value := r + second(5, 6);
end;
