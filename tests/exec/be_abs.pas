{ Execution test (bead domain-pascal-5kgg): best-effort `abs' shapes
  (ref "Abs", p. 4-12; codegen-patterns 8, "abs"):

  - `abs' of an integer16 assigned to an integer32 (a plain path, as a
    check);
  - `abs' stored into an element through a pointer, a destination that
    needs a register (the value's branch kept before it).

  call: BE_ABS_$WIDE @w:-5 -> l
  call: BE_ABS_$WIDE @w:7 -> l
  call: BE_ABS_$ELEM @w:2 @w:-9
  call: BE_ABS_$ELEM @w:3 @w:4
  warns: best effort (not byte-identical): a store whose destination needs a register of an `abs` value
  expect:
    => 5
    => 7
    0
    9
    0
    0
    9
    4
}
module be_abs;
%include 'exec.ins.pas';
type
  arr_t = array[1..3] of integer;
  arr_p = ^arr_t;
var
  a: arr_t;
  ap: arr_p;

function be_abs_$wide(in x: integer): integer32;
var l: integer32;
begin
  l := abs(x);
  be_abs_$wide := l;
end;

procedure be_abs_$elem(in i: integer; in x: integer);
var k: integer;
begin
  ap := addr(a);
  ap^[i] := abs(x);
  for k := 1 to 3 do exec_$log_w(a[k]);
end;
