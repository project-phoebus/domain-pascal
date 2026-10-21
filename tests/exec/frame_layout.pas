{ Execution test (beads domain-pascal-1edl, domain-pascal-67c): frame
  layout with objects of 8 bytes or more at multiples of 8 and the
  compiler's objects (value parameters' copies, the function result
  variable) in front of the first smaller local (codegen-patterns 2,
  "Objects of 8 bytes or more at multiples of 8", "Where the compiler's
  objects go").  Every object keeps its own bytes: each is filled, the
  others are filled after it, and each is read back.

  - fl_$copies: a 14-byte copy, two 8-byte ones, a word and a byte
    copy around word, longword and 0x34-byte locals (the copies split
    between the top, the middle and the bottom of the frame, the larger
    first); a copy written does not change the caller's variable;
  - fl_$result: a 4-byte result variable kept in the frame (its every
    assignment last, read back through a nested routine) above a word
    local, below a longword one;
  - fl_$nested: an internal routine with the reserved slot, a 12-byte
    record (8-aligned below the slot), a 6-byte clock and an 8-byte
    record reached by a nested routine.

  call: FL_$RUN
  expect:
    287454020
    1432778632
    4660
    90
    1
    2
    3
    -7
    77
    mem 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e
    1048576
    mem 11 22 33 44 55 66 77 88
    99
    21
    12
    5
    6
    7
    8
    9
    12
    287454020
    4660
}
module frame_layout;
%include 'exec.ins.pas';
type
  uid_$t = record high, low: integer32; end;
  b14 = record a, b, c: integer32; d: integer; end;
  big_t = record
    case integer of
      1: (raw: array[1..16#34] of char);
      2: (first: integer32; rest: array[1..12] of integer32);
  end;
  ec_t = record value: integer32; awaiters: integer32; tick: integer32; end;
  clock_t = record high: integer32; low: integer; end;
  b8 = array[1..8] of char;
var
  gu, gv: uid_$t;
  gw: integer;
  gc: char;
  g14: b14;
  gb8: b8;

procedure fl_$copies(s: b14; u: uid_$t; w: integer; c: char; v: uid_$t);
var
  word1: integer;
  long1: integer32;
  big: big_t;
  i: integer;
begin
  word1 := -7;
  long1 := 77;
  big.first := 16#100000;
  for i := 1 to 12 do big.rest[i] := i;
  u.high := 1;
  v.low := 3;
  exec_$log_l(gu.high);
  exec_$log_l(gu.low);
  exec_$log_w(w);
  exec_$log_c(c);
  exec_$log_l(u.high);
  exec_$log_l(v.high);
  exec_$log_l(v.low);
  exec_$log_w(word1);
  exec_$log_l(long1);
  exec_$log_mem(s, 14);
  exec_$log_l(big.first);
end;

function fl_$result(k: integer32): integer32;
var
  long2: integer32;
  word2: integer;
  pad: b8;

  procedure fl_$show;
  begin
    exec_$log_mem(pad, 8);
  end;

begin
  long2 := k;
  word2 := 21;
  pad := gb8;
  fl_$show;
  exec_$log_l(long2);
  exec_$log_w(word2);
  fl_$result := long2 - 87;
end;

function fl_$nested(n: integer32): integer32; internal;
var
  ec: ec_t;
  clk: clock_t;
  w: integer;
  u: uid_$t;

  procedure fl_$fill;
  begin
    u.high := gv.high + 6;
    u.low := gv.high + 7;
  end;

begin
  ec.value := 5;
  ec.awaiters := n;
  ec.tick := 7;
  clk.high := 0;
  clk.low := 6;
  w := 0;
  fl_$fill;
  exec_$log_l(ec.value);
  exec_$log_w(clk.low);
  exec_$log_l(ec.tick);
  exec_$log_l(u.high);
  exec_$log_l(u.low);
  fl_$nested := ec.awaiters + w;
end;

procedure fl_$run;
var
  i: integer;
begin
  gu.high := 16#11223344;
  gu.low := 16#55667788;
  gv.high := 2;
  gv.low := 0;
  gw := 16#1234;
  gc := 'Z';
  g14.a := 16#01020304; g14.b := 16#05060708;
  g14.c := 16#090a0b0c; g14.d := 16#0d0e;
  for i := 1 to 8 do gb8[i] := chr(i * 17);
  fl_$copies(g14, gu, gw, gc, gv);
  exec_$log_l(fl_$result(99));
  exec_$log_l(fl_$nested(12));
  exec_$log_l(gu.high);
  exec_$log_w(gw);
end;
