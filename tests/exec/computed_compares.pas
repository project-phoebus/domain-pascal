{ Execution test: comparisons of two computed values (codegen-patterns
  7.1, "Two computed values"; bead domain-pascal-o1v0): the left operand
  computed first, `cmp.<s> Dr,Dl', the relation as written; a variable
  held in a register against a computed value; the same long mask on both
  sides (the constant copied).  Each true test sets a bit of the logged
  mask.

  cc_$w: 1 a + b > c - 1, 2 a - b = c + 1, 4 (a & 15) = (b & 15),
  8 k < b - c (k a longword local := a), 16 c - 1 >= a + b,
  32 (a & 16#FF0) <> (b & 16#FF0).
  cc_$l: 1 (x & 16#FFFFF) = (y & 16#FFFFF), 2 x + 1 > y - 1,
  4 x - y < y + 5, 8 (x & 16#FFFFF) <> (y & 16#FFFFF).

  call: CC_$W w:5 w:3 w:7
  call: CC_$W w:20 w:4 w:17
  call: CC_$W w:-3 w:10 w:-14
  call: CC_$W w:1 w:2 w:10
  call: CC_$L l:1048581 l:2097157
  call: CC_$L l:10 l:3
  call: CC_$L l:-1 l:1048575
  call: CC_$L l:100 l:20
  expect:
    1
    37
    43
    16
    5
    14
    5
    10
}
module comp_cmp;
%include 'exec.ins.pas';

var
  r: integer;

procedure cc_$w(a, b, c: integer); options(val_param);
var k: integer32;
begin
  r := 0;
  k := a;
  if a + b > c - 1 then r := r + 1;
  if a - b = c + 1 then r := r + 2;
  if (a & 15) = (b & 15) then r := r + 4;
  if k < b - c then r := r + 8;
  if c - 1 >= a + b then r := r + 16;
  if (a & 16#FF0) <> (b & 16#FF0) then r := r + 32;
  exec_$log_w(r);
end;

procedure cc_$l(x, y: integer32); options(val_param);
begin
  r := 0;
  if (x & 16#FFFFF) = (y & 16#FFFFF) then r := r + 1;
  if x + 1 > y - 1 then r := r + 2;
  if x - y < y + 5 then r := r + 4;
  if (x & 16#FFFFF) <> (y & 16#FFFFF) then r := r + 8;
  exec_$log_w(r);
end;
