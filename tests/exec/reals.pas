{ Execution test (bead domain-pascal-338r, domain-c's gap 7): real
  arithmetic through the Pascal run time's floating point interpreter
  (`jsr FPP_$' and its operation words; the runner interprets them with
  the host's IEEE arithmetic): constants (doubles), single and double
  variables, + - * /, an integer operand, a nested right operand through a
  temporary, negation, single := double and double := single, trunc and
  round, compares in a condition and as a value, a single constant
  stored as its bits; a function returning a single (in D0, gap 19);
  a single, an expression and an integer passed to an `in double'
  parameter through a temporary (gap 6); abs of a double (`max(-x, x)`),
  `**`, sqrt, exp, ln, arctan, sin and cos.  Best effort, no kernel site.

  call: REALS_$RUN w:7
  expect:
    3
    -3
    2
    7
    10
    -10
    1
    0
    1
    1
    1065353216
    18
    35
    35
    45
    70
    25
    128
    1131
    2718
    4852
    3142
    1357
}
module reals;
%include 'exec.ins.pas';

var
  d, e: double;
  s, t: real;
  b: boolean;

function half(n: integer): real; options(val_param);
begin
  half := n / 2.0;
end;

procedure show(in v: double);
begin
  exec_$logv_l(trunc(v * 10.0));
end;

procedure reals_$run(n: integer); options(val_param);
begin
  e := 2.0;
  d := 7.0 / e;
  exec_$logv_l(trunc(d));
  exec_$logv_l(trunc(-d));
  s := d - 1.5;
  exec_$logv_l(round(s));
  t := n;
  exec_$logv_l(trunc(t));
  d := (t + 3.0) * (e / 2.0);
  exec_$logv_l(round(d));
  exec_$logv_l(round(-d));
  b := s < d;
  if b then exec_$logv_w(1) else exec_$logv_w(0);
  if d = 9.5 then exec_$logv_w(1) else exec_$logv_w(0);
  if s >= 2.0 then exec_$logv_w(1) else exec_$logv_w(0);
  e := s;
  if e = 2.0 then exec_$logv_w(1) else exec_$logv_w(0);
  s := 1.0;
  exec_$logv_l(integer32(s));
  exec_$logv_l(round(n * 2.5));
  s := half(n);
  exec_$logv_l(trunc(s * 10.0));
  show(s);
  show(s + 1.0);
  show(n);
  d := -2.5;
  exec_$logv_l(round(abs(d) * 10.0));
  e := 2.0 ** (n * 1.0);
  exec_$logv_l(round(e));
  exec_$logv_l(round(sqrt(e) * 100.0));
  exec_$logv_l(round(exp(1.0) * 1000.0));
  exec_$logv_l(round(ln(e) * 1000.0));
  exec_$logv_l(round(arctan(1.0) * 4000.0));
  exec_$logv_l(round(sin(0.5) * 1000.0 + cos(0.5) * 1000.0));
end;
