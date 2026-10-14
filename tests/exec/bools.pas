{ Execution test: plain `and' / `or' on booleans (ref "And", p. 4-18;
  "Or", p. 4-134; Table 4-1 p. 4-2), in conditions, under `not' and as
  values stored; `odd' (ref "Odd", p. 4-128) of word and longword values
  held in registers, negative ones included (codegen-patterns 7.1, "Plain
  `and' and `or'"; 8, "odd"); a register-held local compared inside a
  plain `or'/`and' that ends a loop (BOOLS_$LOOP: the `s<cc>' must not
  overwrite the local's register).

  The reference leaves open whether both operands of a plain `and'/`or'
  are evaluated and in which order (p. 4-18: "the system may evaluate
  both ... there is no guarantee that the system will evaluate the two
  operands in the order in which they appear"), so the only operand with
  a side effect here (`probe', a mark) is one that is evaluated either
  way: the left operand of an `or' whose right operand has none.
  Not expressible yet: `odd' of a variable in memory, storing a
  comparison or `odd' as a value (refused, not catalogued).

  call: BOOLS_$SET w:1 w:2
  call: BOOLS_$TEST
  call: BOOLS_$SET w:-1 w:3
  call: BOOLS_$TEST
  call: BOOLS_$SET w:0 w:-5
  call: BOOLS_$TEST
  call: BOOLS_$ODD w:-3 l:70001
  call: BOOLS_$ODD w:4 l:-8
  call: BOOLS_$LOOP
  expect:
    mark 1
    mark 1
    mark 11
    mark 21
    mark 30
    false
    true
    mark 5
    mark 51
    mark 0
    mark 1
    mark 10
    mark 21
    mark 31
    false
    true
    mark 5
    mark 51
    mark 0
    mark 0
    mark 10
    mark 21
    mark 30
    false
    true
    mark 5
    mark 50
    mark 1
    mark 1
    mark 2
    mark 0
    mark 0
    mark 3
    mark 4
    mark 3
}
module bools;
%include 'exec.ins.pas';

var
  x: boolean;
  i: integer;
  y: boolean;
  j: integer;
  r: boolean;
  m: integer;

function probe(v: integer): boolean; options(val_param);
begin
  exec_$mark(5);
  probe := v > 0;
end;

procedure bools_$set(a, b: integer); options(val_param);
begin
  i := a;
  x := false;
  j := b;
  if a > 0 then x := true;
  if b > 0 then y := true else y := false;
end;

procedure bools_$test;
begin
  m := 0;
  if x and y then m := 1;
  exec_$mark(m);
  m := 0;
  if x or y then m := 1;
  exec_$mark(m);
  m := 10;
  if (i = 1) and (j = 2) then m := 11;
  exec_$mark(m);
  m := 20;
  if (i > j) or y then m := 21;
  exec_$mark(m);
  m := 30;
  if not (x or (j < 0)) then m := 31;
  exec_$mark(m);
  r := x and not y;
  exec_$log_bool(r);
  r := (i <> 0) or (j <> 0) or x;
  exec_$log_bool(r);
  m := 50;
  if probe(i) or y then m := 51;
  exec_$mark(m);
end;

procedure bools_$odd(v: integer; w: integer32); options(val_param);
var
  n: integer;
begin
  m := 0;
  if odd(v) then m := 1;
  exec_$mark(m);
  m := 0;
  if odd(w) then m := 1;
  exec_$mark(m);
  n := v + 1;
  m := 2;
  if odd(n) or x then m := 3;
  exec_$mark(m);
end;

{ A register-held local compared inside a plain `or' in a loop: the
  `s<cc>' of the comparison goes to a register of its own, not the
  local's (it is live around the loop and after it). }
procedure bools_$loop;
var
  n: integer;
begin
  n := 0;
  repeat
    n := n + 1;
  until x or (n > 3);
  exec_$mark(n);
  m := n;
  n := 0;
  repeat
    n := n + 1;
  until (n > 2) and not x;
  exec_$mark(n);
  m := n;
end;
