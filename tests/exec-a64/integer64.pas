{ Execution test on aarch64 only (tools/exectest_a64.py runs
  tests/exec-a64/; docs/aarch64.md, "integer64 and PointerInt"): dpc's
  integer64 and include/pointerint.ins.pas's PointerInt.  Constants
  beyond 32 bits are integer64s; integer32 and integer operands widen to
  64 bits; `div' truncates and `mod' takes the dividend's sign (p. 4-65,
  p. 4-115), at 64 bits; shifts and bit operators work on all 64 bits;
  comparisons are signed; a pointer goes to PointerInt and back intact;
  an integer64 is passed by value under val_param and returned in x0.
  call: I64_$RUN
  expect:
    4294967296
    -4294967296
    9223372036854775807
    -9223372036854775808
    6000000000
    -2147483649
    3
    -1
    -3
    1
    1099511627776
    -2
    9223372036854775807
    1229782938247303441
    TRUE
    FALSE
    42
    TRUE
    7
    -12884901888
    8589934593
}
module i64;
%include 'exec.ins.pas';
%include 'pointerint.ins.pas';

const
  big = 4294967296;
  top = 9223372036854775807;
  bottom = -9223372036854775808;

var
  a, b: integer64;
  l: integer32;
  w: integer;
  x: integer32;
  p: ^integer32;
  q: ^integer32;
  pi: PointerInt;

function triple(x: integer64): integer64; options(val_param);
begin
  triple := x * 3;
end;

function plus(in x: integer64; in y: integer64): integer64;
begin
  plus := x + y;
end;

procedure i64_$run;
begin
  a := big;
  writeln(a:1);
  writeln(-a:1);
  writeln(top:1);
  writeln(bottom:1);
  l := 2000000000;
  a := l;
  a := a * 3;
  writeln(a:1);
  l := -2147483647;
  w := 2;
  a := l - w;
  writeln(a:1);
  a := 10; b := 3;
  writeln(a div b:1);
  a := -10;
  writeln(a mod b:1);
  writeln(a div b:1);
  a := 10; b := -3;
  writeln(a mod b:1);
  a := 1;
  a := lshft(a, 40);
  writeln(a:1);
  a := -4;
  a := arshft(a, 1);
  writeln(a:1);
  a := -1;
  a := rshft(a, 1);
  writeln(a:1);
  a := 16#1111111111111111;
  writeln(a:1);
  a := big; b := big - 1;
  writeln(a > b);
  writeln(a < b);
  x := 42;
  p := addr(x);
  pi := PointerInt(p);
  q := univ_ptr(pi);
  writeln(q^:1);
  writeln(pi = PointerInt(addr(x)));
  a := 7;
  l := a;
  writeln(l:1);
  a := -4294967296;
  writeln(triple(a):1);
  b := 1;
  writeln(plus(big * 2, b):1);
end;
