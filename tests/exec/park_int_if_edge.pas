{ Execution test: edge cases of bead domain-pascal-1j61's placement of
  a parked integer local's `movea.l D0,An' inside an `if'
  (codegen-patterns 3, "Placement inside an `if`"): D0 held over code
  that wants data registers before the move (a condition that is not
  one test, statements of the part before its first reference, a loop,
  a nested `if'), an out parameter read after the `if' (its register
  must not be the one the local moves to), and a condition reading the
  local with arithmetic.  The descriptor is a = 7, b = 10; getd sets st to 1
  for k < 0.

  link: park_int_desc.mod.pas
  call: PIE_$RES l:3 @l:99! -> l
  call: PIE_$RES l:-3 @l:99! -> l
  call: PIE_$PRE l:3 @l:99! -> l
  call: PIE_$PRE l:-3 @l:99! -> l
  call: PIE_$CMPX l:3 @l:99! -> l
  call: PIE_$CMPX l:0 @l:99! -> l
  call: PIE_$CMPX l:-3 @l:99! -> l
  call: PIE_$NEST l:3 @l:99! -> l
  call: PIE_$NEST l:20 @l:99! -> l
  call: PIE_$NEST l:-3 @l:99! -> l
  call: PIE_$LOOP l:3 @l:99! -> l
  call: PIE_$LOOP l:-3 @l:99! -> l
  call: PIE_$STAT l:3 @l:99!
  call: PIE_$STAT l:-3 @l:99!
  call: PIE_$ARITH l:3 @l:99! -> l
  call: PIE_$ARITH l:2 @l:99! -> l
  call: PIE_$ARITH l:-3 @l:99! -> l
  expect:
    out 0
    => 13
    out 1
    => 0
    out 0
    => 27
    out 1
    => -1
    out 0
    => 20
    out 0
    => 5
    out 1
    => 5
    out 0
    => 13
    out 0
    => 1
    out 1
    => 2
    out 0
    => 15
    out 1
    => -1
    out 100
    out 101
    out 0
    => 1
    out 0
    => 2
    out 1
    => 3
}
module pie;
%include 'exec.ins.pas';

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;

function park_int_$getd(in k: integer32; out st: integer32): integer32;
  options(extern, a0_return);

{ The result set in the then part only; the else part is empty. }
function pie_$res(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
begin
  pie_$res := 0;
  d := park_int_$getd(k, st);
  if st = 0 then
    pie_$res := desc_ptr_t(d)^.b + k;
end;

{ Statements of the part before its first reference, wanting data
  registers. }
function pie_$pre(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
  x, y: integer32;
begin
  x := -1;
  d := park_int_$getd(k, st);
  if st = 0 then begin
    y := k + 4;
    x := y + y + k;
    x := x + desc_ptr_t(d)^.b + desc_ptr_t(d)^.a - k - 4;
  end;
  pie_$pre := x;
end;

{ A condition that is not one test and does not read the local. }
function pie_$cmpx(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
  x: integer32;
begin
  x := 5;
  d := park_int_$getd(k, st);
  if (st = 0) and (k <> 0) then
    x := desc_ptr_t(d)^.b + desc_ptr_t(d)^.b;
  pie_$cmpx := x;
end;

{ An `if' inside the part: one part refers, the other empty. }
function pie_$nest(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
  x: integer32;
begin
  x := 2;
  d := park_int_$getd(k, st);
  if st = 0 then begin
    x := 1;
    if k < 10 then
      x := desc_ptr_t(d)^.b + k;
  end;
  pie_$nest := x;
end;

{ A loop before the first reference, inside the part. }
function pie_$loop(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
  x: integer32;
begin
  x := -1;
  d := park_int_$getd(k, st);
  if st = 0 then begin
    x := k;
    while x < 5 do
      x := x + 1;
    x := x + desc_ptr_t(d)^.b;
  end;
  pie_$loop := x;
end;

{ The out parameter written after the `if': its register must not be
  the one the local moves to. }
procedure pie_$stat(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st = 0 then
    desc_ptr_t(d)^.a := desc_ptr_t(d)^.a + 0;
  st := st + 100;
end;

{ A condition reading the local with arithmetic. }
function pie_$arith(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then begin
    pie_$arith := 3;
    return;
  end;
  if desc_ptr_t(d)^.b + k > 12 then
    pie_$arith := 1
  else
    pie_$arith := 2;
end;

