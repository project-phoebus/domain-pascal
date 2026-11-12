{ Execution test (bead domain-pascal-338r, domain-c's gap 21): routine
  parameters (ref 5.4), procedures and functions, passed a top-level
  routine's address by value (the DOMAIN C compiler's `move.l
  #routine,-(SP)', reported) and passed on; called by their names
  through the slot, a parameterless one also without parentheses (ref
  5.4's examples, `A;' and `a * a * a') (best effort, no kernel site).  The routines passed
  take their parameters by address, as the routine parameters' own
  parameters do (no options given, ref 5.4).

  call: ROUTINE_PARAMS_$RUN w:5
  expect:
    10
    15
    6
    25
    16
    64
    114
}
module routine_params;
%include 'exec.ins.pas';

var
  acc: integer;

procedure add(n: integer);
begin
  acc := acc + n;
end;

function twice(n: integer): integer;
begin
  twice := n + n;
end;

function square(n: integer): integer;
begin
  square := n * n;
end;

procedure apply(procedure p(n: integer); n: integer); options(val_param);
begin
  p(n);
  p(n);
end;

function map(function f(n: integer): integer; n: integer): integer; options(val_param);
begin
  map := f(n);
end;

function pass_on(function f(n: integer): integer; n: integer): integer; options(val_param);
begin
  pass_on := map(f, n + 1);
end;

function four: integer;
begin
  four := 4;
end;

function cube(function a: integer): integer; options(val_param);
begin
  cube := a * a * a;
end;

procedure proc_once(procedure b);
begin
  b;
end;

procedure bump;
begin
  acc := acc + 100;
end;

procedure routine_params_$run(k: integer); options(val_param);
begin
  acc := 0;
  apply(add, k);
  exec_$logv_w(acc);
  apply(add, k div 2);
  exec_$logv_w(acc + 1);
  exec_$logv_w(map(twice, 3));
  exec_$logv_w(map(square, k));
  exec_$logv_w(pass_on(square, 3));
  exec_$logv_w(cube(four));
  proc_once(bump);
  exec_$logv_w(acc);
end;
