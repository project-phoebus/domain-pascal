{ Execution test (bead domain-pascal-5kgg): integer narrowings the
  lowering takes on its best-effort path (`dpc-ir` narrow_best_effort):
  the value computed at its own width and its low bytes used (ref 4.3.1
  p. 4-5; reference-clarifications question 41, integer16 a subrange of
  integer32, the low bytes kept).  All values below are in the narrower
  type's range.

  - a longword expression with a right shift (no narrower form) stored
    to a one-byte variable, assigned to a one-byte function result, and
    passed by value to a one-byte parameter;
  - a call's longword result narrowed to a one-byte function result;
  - a volatile longword variable passed by value to a word parameter
    (read whole, ref 3.15.1);
  - a type transfer `integer16(x * 2)` whose product calls M$ARITH
    (computed at 32 bits, its low word);
  - a longword element, a longword `var` parameter and a volatile
    longword (read whole, ref 3.15.1) stored to a byte.

  call: BE_L_NARROW_$MAIN l:1000
  call: BE_L_NARROW_$RES l:1000 -> c
  call: BE_L_NARROW_$CALLRES l:77 -> c
  call: BE_L_NARROW_$TRANSFER l:12345 l:2 -> w
  call: BE_L_NARROW_$PARTS @l:200
  warns: best effort (not byte-identical): assigning a 4-byte value to a 1-byte object
  warns: best effort (not byte-identical): assigning a 4-byte value to a 1-byte function result
  warns: best effort (not byte-identical): passing a 4-byte value to a 1-byte parameter
  warns: best effort (not byte-identical): passing a 4-byte value to a 2-byte parameter
  warns: best effort (not byte-identical): narrowing a longword element
  warns: best effort (not byte-identical): narrowing a longword parameter passed by address
  warns: best effort (not byte-identical): narrowing a volatile
  warns: best effort (not byte-identical): a type transfer of a 4-byte integer to a 2-byte one
  expect:
    125
    125
    3000
    => 125
    => 77
    => 24690
    9
    200
    77
}
module be_l_narrow;
%include 'exec.ins.pas';
type
  sb = [byte] 0..255;
var
  gs: sb;
  gl: array[1..3] of integer32;
  gv: [volatile] integer32;

procedure takeb(v: sb); options(val_param);
var w: integer;
begin
  w := v;
  exec_$log_w(w);
end;

procedure takew(v: integer); options(val_param);
begin
  exec_$log_w(v);
end;

procedure be_l_narrow_$main(x: integer32); options(val_param);
var w: integer;
begin
  gs := rshft(x, 3);
  w := gs;
  exec_$log_w(w);
  takeb(rshft(x, 3));
  gv := x * 3;
  takew(gv);
end;

function be_l_narrow_$res(x: integer32): sb; options(val_param);
begin
  be_l_narrow_$res := rshft(x, 3);
end;

function be_l_narrow_$callres(x: integer32): sb; options(val_param);
begin
  be_l_narrow_$callres := exec_$id_l(x);
end;

function be_l_narrow_$transfer(x, y: integer32): integer; options(val_param);
begin
  be_l_narrow_$transfer := integer16(x * y);
end;

procedure be_l_narrow_$parts(var p: integer32);
var b: sb; w: integer;
begin
  gl[2] := 9;
  b := gl[2];
  w := b;
  exec_$log_w(w);
  b := p;
  w := b;
  exec_$log_w(w);
  gv := 77;
  b := gv;
  w := b;
  exec_$log_w(w);
end;
