{ Golden: codegen-patterns 8 (multiplication, division and mod), 12.4 R3,
  1.1, 1.4.  Beads domain-pascal-5aw.5, -1x1.
  - M$ARITH calls: the pad before a word argument, argument 2 pushed
    first, `jsr', `addq.w #8,SP' (0xE07FE6..0xE07FF4 `M$OIU$WLW');
    `M$DIS$LLL'/`M$OIS$LLL' by a constant beyond a word, the second call
    not popped before the epilogue (TIME_$GET_ADJUST 0xE16ABE..0xE16AE0);
    the result used in D0 in place (0xE16ADE `lsl.l #2,D0');
  - an M$ARITH result as argument 1 of another call (0xE1695C..0xE16970);
  - a value held in D1, a pointer in A1, across M$ARITH calls (12.4 R3;
    TIME_$GET_ADJUST keeps its pointer in A1);
  - words multiplied for a longword with `muls.w'/`mulu.w'
    (PROC1_$LOADAV_CALLBACK 0xE14C14; SIO_$K_TIMED_BREAK 0xE67F2A);
  - a longword quotient used as a word with `divs.w' (0xE10CAE), a
    remainder with `M$OIS$WLW' (0xE30D3E);
  - signed `div' by a power of two with `bpl; addi; asr'
    (PROC1_$LOADAV_CALLBACK 0xE14BEE; CAL_$WEEKDAY 0xE68698);
  - unsigned words: `clr.l; move.w; divu.w; swap' (0xE4CE1A). }
module marith;
type
  adjust = record sec, usec: integer32; end;
  pinteger = 0..65535;
var
  delta: extern integer32;
  w: extern integer;
  l: extern integer32;
  u: extern pinteger;
procedure get_adjust(out d: adjust);
begin
  d.sec := delta div 250000;
  d.usec := (delta mod 250000) * 4;
end;
procedure held(x: integer32; y: integer); internal;
begin
  l := (x div y) * y;
  w := y;
end;
function load_term(x: integer32; n: integer): integer32; internal;
begin
  load_term := n * 5240 + x;
end;
function ticks(ms: pinteger): integer32; internal;
begin
  ticks := ms * 250;
end;
procedure narrowed; internal;
begin
  w := l div 250;
  w := l mod 32;
end;
function per_256: integer32; internal;
begin
  per_256 := l div 256;
end;
function quarter(y: integer): integer; internal;
begin
  quarter := y div 4;
end;
function hash: integer; internal;
begin
  hash := u mod 111;
end;
