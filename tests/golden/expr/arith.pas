{ Golden: codegen-patterns 8 (integer arithmetic), 13, 3.
  - `x + c': `addq' for 1..8 (0xE00C10, 0xE68874); for 9..127 with `x' in
    memory the constant first, `moveq; add.<s> x,Dn' (0xE008A8, 0xE30BB0);
    `addi' otherwise (0xE08840);
  - `x - c': `subq' for 1..8 (0xE010D0), `subi' otherwise (0xE0B6E2);
    `c - x': the constant first, `sub.<s> x,Dn' (0xE08EE4, 0xE10114);
  - `x * c': shifts and adds for c = 2^s (2^k +- 1) (RIP_$PACKET_LENGTH
    0xE6886C, x6; 0xE0A972, x12; PEB_$GET_FP 0xE5AEF6, x28), `mulu.w #c'
    for other odd parts (BAT_$GET_BAT_STEP 0xE3BB6A, x564);
  - integer16 `div'/`mod' by a constant: `ext.l; divs.w #c' and `swap'
    for the remainder (0xE31156..0xE3115C, 0xE2F052..0xE2F058);
  - `-x', `~x': `neg', `not';
  - widening: `ext.l' (0xE02C2A), zero-extension `clr.l; move.w'
    (0xE177A6);
  - `x := x op c' updates memory in place: `addq', `subi', `andi', `ori'
    (0xE00B3E, 0xE010C4, 0xE0086E). }
module arith;
type
  pinteger = 0..65535;
var
  w: extern integer;
  l: extern integer32;
  u: extern pinteger;
function add_q(x: integer): integer; internal;
begin
  add_q := x + 2;
end;
function add_moveq(x: integer): integer; internal;
begin
  add_moveq := x + 30;
end;
function add_i(x: integer32): integer32; internal;
begin
  add_i := x + 16#1000;
end;
function sub_q(x: integer): integer; internal;
begin
  sub_q := x - 1;
end;
function sub_i(x: integer): integer; internal;
begin
  sub_i := x - 16#400;
end;
function rsub(x: integer): integer; internal;
begin
  rsub := 31 - x;
end;
function times6(x: integer): integer; internal;
begin
  times6 := x * 6 + 2;
end;
function times12(x: integer): integer; internal;
begin
  times12 := x * 12;
end;
function times28(x: integer): integer; internal;
begin
  times28 := x * 28;
end;
function times564(x: integer): integer; internal;
begin
  times564 := x * 564;
end;
function quot(x: integer): integer; internal;
begin
  quot := x div 31;
end;
function remainder(x: integer): integer; internal;
begin
  remainder := x mod 31;
end;
function negate(x: integer): integer; internal;
begin
  negate := -x;
end;
function complement(x: integer32): integer32; internal;
begin
  complement := ~x;
end;
function widen(x: integer): integer32; internal;
begin
  widen := x + l;
end;
function widen_unsigned: integer32; internal;
begin
  widen_unsigned := u + 16#400;
end;
procedure update; internal;
begin
  w := w + 1;
  l := l - 16#100;
  w := w & 16#FF;
  u := u ! 16#8000;
end;
