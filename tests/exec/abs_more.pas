{ Execution test (bead domain-pascal-g6xa; tests/exec/abs.pas has the
  image's forms): `abs' ("Abs", p. 4-12) in the uses no image site
  shows, best effort (codegen-patterns 8, "abs").  Narrowing
  (review of g6xa): `abs' of a longword sum stored into a word keeps the
  low word of the longword's `abs' (abs(-1 + 40000) is 39999, also as an
  `integer', 39999 - 65536), not `abs' of the low word; a word product by
  a constant takes its sign from the 32-bit product (abs(-50 * 1000) is
  50000 in a `0..65535' word, the image's `muls.w' form, 0xE69626).  And
  the other best-effort uses: a `case' selector, an unsigned `max', `>
  0', `in', `abs' of a function's result.

  call: ABS_MORE_$NARROW_U w:-1 w:40000 -> uw
  call: ABS_MORE_$NARROW_U w:-30000 w:0 -> uw
  call: ABS_MORE_$NARROW_W w:-1 w:40000 -> w
  call: ABS_MORE_$MULC w:-50 -> uw
  call: ABS_MORE_$MULC w:50 -> uw
  call: ABS_MORE_$MULC2 w:-3 -> w
  call: ABS_MORE_$CASE w:-3 -> w
  call: ABS_MORE_$CASE w:5 -> w
  call: ABS_MORE_$CASE w:-4 -> w
  call: ABS_MORE_$MAX w:-7 w:3 -> uw
  call: ABS_MORE_$MAX w:2 w:-9 -> uw
  call: ABS_MORE_$MAX w:-32768 w:5 -> uw
  call: ABS_MORE_$POS w:0 -> w
  call: ABS_MORE_$POS w:-1 -> w
  call: ABS_MORE_$POS w:-32768 -> w
  call: ABS_MORE_$INSET w:-3 -> w
  call: ABS_MORE_$INSET w:2 -> w
  call: ABS_MORE_$INSET w:-5 -> w
  call: ABS_MORE_$FUNC w:-4 -> w
  call: ABS_MORE_$FUNC w:0 -> w
  expect:
    => 39999
    => 30000
    => -25537
    => 50000
    => 50000
    => 31000
    => 30
    => 50
    => 99
    => 7
    => 9
    => 32768
    => 0
    => 1
    => 1
    => 1
    => 0
    => 1
    => 4
    => 0
}
module abs_more;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;

var
  g: integer;
  gu: pinteger;

function abs_more_$narrow_u(w: integer; v: pinteger): pinteger; options(val_param);
begin
  gu := abs(w + v);
  abs_more_$narrow_u := gu;
end;

function abs_more_$narrow_w(w: integer; v: pinteger): integer; options(val_param);
begin
  g := abs(w + v);
  abs_more_$narrow_w := g;
end;

function abs_more_$mulc(w: integer): pinteger; options(val_param);
begin
  gu := abs(w * 1000);
  abs_more_$mulc := gu;
end;

function abs_more_$mulc2(w: integer): integer; options(val_param);
begin
  g := abs(w * 11000) - 2000;
  abs_more_$mulc2 := g;
end;

function abs_more_$case(i: integer): integer; options(val_param);
begin
  case abs(i) of
  0: abs_more_$case := 0;
  1: abs_more_$case := 10;
  2: abs_more_$case := 20;
  3: abs_more_$case := 30;
  5: abs_more_$case := 50;
  otherwise abs_more_$case := 99;
  end;
end;

function abs_more_$max(x, y: integer): pinteger; options(val_param);
begin
  abs_more_$max := max(abs(x), abs(y));
end;

function abs_more_$pos(x: integer): integer; options(val_param);
begin
  if abs(x) > 0 then abs_more_$pos := 1 else abs_more_$pos := 0;
end;

function abs_more_$inset(i: integer): integer; options(val_param);
begin
  if abs(i) in [1, 3, 5] then abs_more_$inset := 1 else abs_more_$inset := 0;
end;

function abs_more_$func(x: integer): integer; options(val_param);
begin
  abs_more_$func := abs(exec_$id_w(x));
end;
