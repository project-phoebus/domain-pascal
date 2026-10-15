{ Execution test: branch threading (codegen-patterns 7.1, "Branch
  threading"; bead domain-pascal-o6p).  A branch to a label that a `bra'
  follows goes to the `bra''s target, and a `bra' no longer reached is
  dropped: nested `if's inside a `then' part with an `else' after it,
  a `return' after an inner `if', an `else' part that starts with a
  `while' (its opening `bra' dropped, TTY_$K_GET 0xE1C448), and a
  `goto' to a label that a `bra' follows.  Each shape on every path.

  call: THREADING_$MAIN @w:0 @w:0
  call: THREADING_$MAIN @w:0 @w:1
  call: THREADING_$MAIN @w:1 @w:0
  call: THREADING_$MAIN @w:1 @w:1
  call: THREADING_$MAIN @w:1 @w:3
  expect:
    mark 3
    mark 13
    mark 22
    mark 30
    mark 40
    mark 3
    mark 13
    mark 22
    mark 30
    mark 40
    mark 2
    mark 12
    mark 20
    mark 31
    mark 41
    mark 1
    mark 11
    mark 21
    mark 32
    mark 40
    mark 2
    mark 12
    mark 20
    mark 31
    mark 41
}
module threading;
%include 'exec.ins.pas';

var
  a, n, b, r: integer;

{ The inner `then' part's `bra' and the outer one: both to the join. }
procedure nested_else;
begin
  if a = 1 then
    if b = 1 then exec_$mark(1) else exec_$mark(2)
  else exec_$mark(3);
end;

{ The inner test's `b<cc>' past the outer `bra'. }
procedure inner_no_else;
begin
  r := 10;
  if a = 1 then begin
    if b = 1 then r := 11;
    if r = 10 then r := 12;
  end else r := 13;
  exec_$mark(r);
end;

{ The inner test's `b<cc>' to the epilogue, past the `return''s `bra'. }
procedure return_after;
begin
  r := 20;
  if a = 1 then begin
    if b = 1 then r := 21;
    return;
  end;
  r := 22;
end;

{ An `else' part that starts with a `while': entered at its test; and
  an inner `else' part ending in `return': the outer `bra' after it is
  dropped. }
procedure else_while;
begin
  n := b;
  if a = 0 then n := 0
  else while n > 0 do n := n - 1;
  if a = 1 then
    if b = 1 then n := 32 else begin r := 31 - n; return end
  else n := 30;
  r := n;
end;

{ A `goto' to a label that the `then' part's `bra' follows. }
procedure goto_join;
label l;
begin
  n := 40;
  if a = 0 then goto l;
  if b <> 1 then begin n := 41; l: end else n := 40;
  r := n;
end;

procedure threading_$main(in x, y: integer);
begin
  a := x;
  b := y;
  nested_else;
  inner_no_else;
  return_after;
  exec_$mark(r);
  else_while;
  exec_$mark(r);
  goto_join;
  exec_$mark(r);
end;
