{ Execution test: `div' and `mod' of unsigned words (0..65535, ref 3.4):
  the dividend is not negative, so 50000 div 7 = 7142 and 50000 mod 7 =
  6 (an unsigned divide; a signed one would see -15536); a longword
  divided by a word constant; a word `div' a power of two.

  call: UDIV_$MAIN w:50000 l:-100000
  call: UDIV_$MAIN w:7 l:100000
  expect:
    7142
    6
    -400
    -100000
    12500
    0
    1
    0
    400
    100000
    1
    7
}
module udiv;
%include 'exec.ins.pas';

type
  uword = 0..65535;
var
  u: uword;
  l: integer32;
  w: integer;

procedure udiv_$main(a: uword; b: integer32); options(val_param);
begin
  u := a div 7;
  exec_$log_uw(u);
  w := a mod 7;
  exec_$logv_w(w);
  l := b div 250;
  exec_$logv_l(l);
  l := b mod 250000;
  exec_$logv_l(l);
  u := a div 4;
  exec_$log_uw(u);
  w := a mod 10;
  exec_$logv_w(w);
end;
