{ Execution test: div and mod signs (ref 4.2 "Div", p. 4-65: truncation
  toward zero; "Mod", p. 4-115: with a negative dividend, -(|d1| mod
  |d2|)), 16-bit inline (divs.w) and 32-bit through M$ARITH, a word
  operand widened to a longword; division by a power of two of a
  negative dividend (not just an arithmetic shift).

  call: DIVMOD_$W w:13 w:3
  call: DIVMOD_$W w:-13 w:3
  call: DIVMOD_$W w:13 w:-3
  call: DIVMOD_$W w:-13 w:-3
  call: DIVMOD_$W w:-32768 w:7
  call: DIVMOD_$L l:1000000 l:7
  call: DIVMOD_$L l:-1000000 l:7
  call: DIVMOD_$L l:1000000 l:-7
  call: DIVMOD_$L l:-1000000 l:-70000
  call: DIVMOD_$POW2 w:-7 l:-7
  call: DIVMOD_$POW2 w:7 l:7
  call: DIVMOD_$POW2 w:-8 l:-1048577
  call: DIVMOD_$WIDEN w:300 w:300
  call: DIVMOD_$WIDEN w:-300 w:300
  call: DIVMOD_$WIDEN w:32767 w:-32768
  expect:
    4
    1
    -4
    -1
    -4
    1
    4
    -1
    -4681
    -1
    142857
    1
    -142857
    -1
    -142857
    1
    14
    -20000
    -1
    -3
    -1
    1
    3
    1
    -2
    0
    -262144
    90000
    -90000
    -1073709056
}
module divmod;
%include 'exec.ins.pas';

var
  q, r: integer;
  ql, rl: integer32;
  wl: integer32;

procedure divmod_$w(a: integer; b: integer); options(val_param);
begin
  q := a div b;
  exec_$logv_w(q);
  r := a mod b;
  exec_$logv_w(r);
end;

procedure divmod_$l(a: integer32; b: integer32); options(val_param);
begin
  ql := a div b;
  exec_$logv_l(ql);
  rl := a mod b;
  exec_$logv_l(rl);
end;

procedure divmod_$pow2(a: integer; b: integer32); options(val_param);
begin
  q := a div 4;
  exec_$logv_w(q);
  r := a mod 4;
  exec_$logv_w(r);
  ql := b div 4;
  exec_$logv_l(ql);
end;

procedure divmod_$widen(a: integer; b: integer); options(val_param);
var
  x: integer32;
begin
  x := a;
  wl := x * b;
  exec_$logv_l(wl);
end;
