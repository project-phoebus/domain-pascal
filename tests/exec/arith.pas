{ Execution test: word and longword arithmetic (codegen-patterns 8):
  multiplications by constants done with shifts and adds, `muls.w' of
  two words (a 32-bit product kept when the target is a longword),
  negation, 16-bit wraparound, M$ARITH for 32-bit `*' (signed operands
  of both signs), `div' and `mod'.  Word operands are expanded to 32 bits
  before the operation (ref 4.3.1, p. 4-5), so a sum stored to a
  longword does not wrap at 16 bits; stored to a word it does.  (Word
  sums compared: wide_compare.pas.)

  call: ARITH_$CONSTS w:-7
  call: ARITH_$CONSTS w:1000
  call: ARITH_$WORDS w:300 w:-200
  call: ARITH_$WORDS w:32767 w:2
  call: ARITH_$LONG l:-123456 l:789
  call: ARITH_$LONG l:65536 l:65536
  call: ARITH_$WIDE w:32767 w:1
  call: ARITH_$WIDE w:-32768 w:0
  expect:
    -42
    -49
    -1750
    7
    6000
    7000
    250000
    -1000
    -60000
    100
    -60000
    5536
    65534
    -32767
    65534
    -2
    -97406784
    -156
    -123456
    -372
    0
    1
    65536
    0
    32768
    -32768
}
module arith;
%include 'exec.ins.pas';

var
  w: integer;
  l: integer32;

procedure arith_$consts(a: integer); options(val_param);
begin
  w := a * 6;
  exec_$logv_w(w);
  w := a * 7;
  exec_$logv_w(w);
  l := a;
  l := l * 250;
  exec_$logv_l(l);
  w := -a;
  exec_$logv_w(w);
end;

procedure arith_$words(a: integer; b: integer); options(val_param);
begin
  l := a * b;
  exec_$logv_l(l);
  w := a + b;
  exec_$logv_w(w);
  l := a;
  l := l * b;
  exec_$logv_l(l);
  w := a * b;
  exec_$logv_w(w);
end;

procedure arith_$long(a: integer32; b: integer32); options(val_param);
begin
  l := a * b;
  exec_$logv_l(l);
  l := a div b;
  exec_$logv_l(l);
  l := a * 1;
  exec_$logv_l(l);
  l := a mod b;
  exec_$logv_l(l);
end;

procedure arith_$wide(a: integer; b: integer); options(val_param);
begin
  l := a + b;
  exec_$logv_l(l);
end;
