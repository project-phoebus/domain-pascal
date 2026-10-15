{ Execution test: a plain `or' / `and' whose right operand compares a
  function's result (bead domain-pascal-i1by; codegen-patterns 7.1, "A
  plain `or'/`and' whose right operand calls a function"; ref "Or",
  p. 4-134, "And", p. 4-18).  dpc short-circuits the left operand and
  forms the right one as a boolean (`s<cc> Dn; tst.b Dn'); the result
  must be the relation's, with the call on either side of the source
  (the relation reversed when the call is on the right), against a word
  or longword variable, 0, a constant, a register-held local, signed
  and unsigned (`0..65535', ref 3.4: compared unsigned).  Boundary
  values: -32768, -128, -1, 0, 1, 127, 128, 255, 32767 and the longword
  extremes.  exec_$id_w / _l return their argument in D0 (stubs).

  call: AOC_$OR @w:-32768 @w:5
  call: AOC_$OR @w:-1 @w:5
  call: AOC_$OR @w:0 @w:5
  call: AOC_$OR @w:1 @w:5
  call: AOC_$OR @w:5 @w:5
  call: AOC_$OR @w:6 @w:5
  call: AOC_$OR @w:127 @w:127
  call: AOC_$OR @w:128 @w:127
  call: AOC_$OR @w:255 @w:128
  call: AOC_$OR @w:32767 @w:32767
  call: AOC_$OR @w:1 @w:-128
  call: AOC_$OR @w:127 @w:-1
  call: AOC_$AND @w:-1 @w:5
  call: AOC_$AND @w:0 @w:5
  call: AOC_$AND @w:1 @w:5
  call: AOC_$AND @w:5 @w:5
  call: AOC_$AND @w:6 @w:5
  call: AOC_$AND @w:128 @w:255
  call: AOC_$AND @w:255 @w:255
  call: AOC_$AND @w:256 @w:255
  call: AOC_$AND @w:32767 @w:-32768
  call: AOC_$AND @w:1 @w:0
  call: AOC_$LEFT @w:-128 @w:-128
  call: AOC_$LEFT @w:0 @w:0
  call: AOC_$LEFT @w:127 @w:127
  call: AOC_$LEFT @w:127 @w:128
  call: AOC_$LEFT @w:255 @w:-1
  call: AOC_$ZERO @w:1 @w:0
  call: AOC_$ZERO @w:1 @w:5
  call: AOC_$ZERO @w:0 @w:0
  call: AOC_$ZERO @w:0 @w:-1
  call: AOC_$ZERO @w:255 @w:128
  call: AOC_$CONST @w:3 @w:0
  call: AOC_$CONST @w:0 @w:127
  call: AOC_$CONST @w:0 @w:128
  call: AOC_$CONST @w:0 @w:255
  call: AOC_$CONST @w:0 @w:-128
  call: AOC_$CONST @w:0 @w:-1
  call: AOC_$CONST @w:0 @w:32767
  call: AOC_$LONG @l:1 @l:0
  call: AOC_$LONG @l:0 @l:1
  call: AOC_$LONG @l:0 @l:2
  call: AOC_$LONG @l:-1 @l:0
  call: AOC_$LONG @l:-2147483648 @l:-2147483647
  call: AOC_$LONG @l:-129 @l:-128
  call: AOC_$LONG @l:-129 @l:-127
  call: AOC_$LONG_LT @l:0 @l:-1
  call: AOC_$LONG_LT @l:-1 @l:-1
  call: AOC_$LONG_LT @l:-1 @l:-2
  call: AOC_$LONG_LT @l:-2147483648 @l:2147483647
  call: AOC_$LONG_LT @l:255 @l:128
  call: AOC_$LONG_LT @l:128 @l:255
  call: AOC_$UNS @w:0 @w:1
  call: AOC_$UNS @w:-25536 @w:255
  call: AOC_$UNS @w:255 @w:-25536
  call: AOC_$UNS @w:-1 @w:-2
  call: AOC_$UNS @w:-1 @w:-1
  call: AOC_$UNS @w:-32768 @w:32767
  call: AOC_$UNS @w:32767 @w:-32768
  expect:
    mark 1
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 0
    mark 1
    mark 1
    mark 1
    mark 0
    mark 0
    mark 0
    mark 0
    mark 0
    mark 1
    mark 1
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mark 0
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 1
    mark 0
    mark 1
    mark 0
    mark 1
    mark 0
}
module and_or_call;
%include 'exec.ins.pas';

type
  uword = 0..65535;

function uid(in v: uword): uword;
begin
  uid := v;
end;

procedure aoc_$or(in u, n: integer);
begin
  if (u <= 0) or (u > exec_$id_w(n)) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$and(in u, n: integer);
begin
  if (u > 0) and (u <= exec_$id_w(n)) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$left(in u, n: integer);
begin
  if (u < 0) or (exec_$id_w(n) = u) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$zero(in u, n: integer);
begin
  if (u <> 1) and (exec_$id_w(n) <> 0) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$const(in u, n: integer);
begin
  if (u = 3) or (exec_$id_w(n) >= 128) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$long(in l, n: integer32);
var
  m: integer32;
begin
  m := l + 1;
  if (l > 0) or (exec_$id_l(n) <> m) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$long_lt(in l, n: integer32);
begin
  if (l = 0) or (l > exec_$id_l(n)) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure aoc_$uns(in w, n: uword);
begin
  if (w = 0) or (w > uid(n)) then
    exec_$mark(1)
  else
    exec_$mark(0);
end;
