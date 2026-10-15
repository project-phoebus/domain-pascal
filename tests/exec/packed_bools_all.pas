{ Execution test: packed arrays of booleans and of bit and byte elements,
  review of bead domain-pascal-51e (ref 3.11.4.2 Table 3-5 p. 3-47;
  3.15.5 p. 3-62: elements from the most significant bit; codegen-patterns
  5.2, "Packed arrays").  Expected values come from those rules, not from
  dpc's output.

  - all: for each of the 64 elements of a 1-based array, cleared
    alone among set ones and set alone among clear ones, every byte (read
    through a variant overlay as `packed array[1..8] of 0..255') and every
    element (by tests) checked; then all set one by one and cleared one by
    one, every element tested after each store.  Logs the mismatch counts.
  - big, lo8: a 256-element array from 1 and one from 8 (`subq #8'), its
    first and last elements: the test's byte arithmetic wraps the index's
    low byte (256 -> 0 - 1 = 0xFF; 263 -> 7 - 8 = 0xFF).
  - inits: initial values of 1-bit (9 elements: 2 bytes), 2-bit (0..3),
    3-bit padded to 4 (0..7) and 4-bit (0..15) elements, signed byte
    elements; module variables are word aligned, so a 1-byte array is
    followed by a pad byte.
  - bytes: byte elements read back, signed (-128..127) and unsigned
    (0..255).
  - w, rp, at: elements as `while', `repeat ... until' and `and then' /
    `or else' conditions.

  call: PACKED_BOOLS_ALL_$ALL
  call: PACKED_BOOLS_ALL_$BIG w:1
  call: PACKED_BOOLS_ALL_$BIG w:8
  call: PACKED_BOOLS_ALL_$BIG w:9
  call: PACKED_BOOLS_ALL_$BIG w:256
  call: PACKED_BOOLS_ALL_$LO8 w:8
  call: PACKED_BOOLS_ALL_$LO8 w:15
  call: PACKED_BOOLS_ALL_$LO8 w:16
  call: PACKED_BOOLS_ALL_$LO8 w:263
  call: PACKED_BOOLS_ALL_$INITS
  call: PACKED_BOOLS_ALL_$BYTES
  call: PACKED_BOOLS_ALL_$SETUP
  call: PACKED_BOOLS_ALL_$W w:3
  call: PACKED_BOOLS_ALL_$W w:9
  call: PACKED_BOOLS_ALL_$RP w:1
  call: PACKED_BOOLS_ALL_$RP w:5
  call: PACKED_BOOLS_ALL_$AT w:3 w:4
  call: PACKED_BOOLS_ALL_$AT w:3 w:9
  call: PACKED_BOOLS_ALL_$AT w:9 w:3
  call: PACKED_BOOLS_ALL_$AT w:9 w:10
  expect:
    0
    0
    0
    0
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 00 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 00 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    true
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    true
    mem ff 80 55 aa
    mem 12 34 56 00
    mem e4 00
    mem 07 41 40 00
    mem fb 00 c8 7f 80 00
    -5
    200
    127
    -128
    0
    8
    9
    3
    8
    6
    9
    1
    3
    0
    2
    0
    0
    0
    2
}
module packed_bools_all;
%include 'exec.ins.pas';

type
  ov_t = record case integer of
    1: (b: packed array[1..64] of boolean);
    2: (y: packed array[1..8] of 0..255);
  end;

var
  ov: ov_t;
  big: packed array[1..256] of boolean;
  lo8: packed array[8..263] of boolean;
  { Kept apart: dpc refuses stores to adjacent variables in successive
    statements (they may be merged) and comparisons of two register-held
    values, so the loop's helpers are module variables with gaps. }
  m: integer; p1: integer32;
  bad: integer; p2: integer32;
  e: integer; p3: integer32;
  jb: integer; p4: integer32;
  kk: integer;
  init9: packed array[1..9] of boolean := [* of true];
  initx: packed array[1..16] of boolean := [
    false, true, false, true, false, true, false, true,
    true, false, true, false, true, false, true, false];
  pad1: char := chr(16#55);
  pad2: char := chr(16#aa);
  nib: packed array[1..6] of 0..15 := [1, 2, 3, 4, 5, 6];
  pad3: char := chr(0);
  pad4: char := chr(0);
  two: packed array[1..4] of 0..3 := [3, 2, 1, 0];
  three: packed array[1..5] of 0..7 := [0, 7, 4, 1, 4];
  sb: packed array[1..4] of -128..127 := [-5, 0, -56, 127];
  ub: packed array[1..2] of 0..255 := [128, 0];
  r: integer;
  fl: packed array[1..64] of boolean;
  n: integer;

{ For every k: clear all, set k; check every byte and every element.
  Then set all, clear k; check again.  Logs mismatch counts. }
procedure packed_bools_all_$all;
var k, i, j: integer;
begin
  bad := 0;
  m := 128; jb := 1;
  for k := 1 to 64 do begin
    kk := k;
    for i := 1 to 64 do ov.b[i] := false;
    ov.b[k] := true;
    for j := 1 to 8 do begin
      if j = jb then e := m else e := 0;
      if ov.y[j] <> e then bad := bad + 1;
    end;
    for i := 1 to 64 do begin
      if ov.b[i] then begin if i <> kk then bad := bad + 1 end
      else if i = kk then bad := bad + 1;
    end;
    m := m div 2;
    if m = 0 then begin m := 128; jb := jb + 1 end;
  end;
  exec_$log_w(bad);
  bad := 0;
  m := 128; jb := 1;
  for k := 1 to 64 do begin
    kk := k;
    for i := 1 to 64 do ov.b[i] := true;
    ov.b[k] := false;
    for j := 1 to 8 do begin
      if j = jb then e := 255 - m else e := 255;
      if ov.y[j] <> e then bad := bad + 1;
    end;
    for i := 1 to 64 do begin
      if not ov.b[i] then begin if i <> kk then bad := bad + 1 end
      else if i = kk then bad := bad + 1;
    end;
    m := m div 2;
    if m = 0 then begin m := 128; jb := jb + 1 end;
  end;
  exec_$log_w(bad);
  { set/clear accumulate: set every element one at a time }
  bad := 0;
  for i := 1 to 64 do ov.b[i] := false;
  for k := 1 to 64 do begin
    kk := k;
    ov.b[k] := true;
    for i := 1 to 64 do
      if ov.b[i] then begin if i > kk then bad := bad + 1 end
      else if i <= kk then bad := bad + 1;
  end;
  exec_$log_w(bad);
  bad := 0;
  for k := 1 to 64 do begin
    kk := k;
    ov.b[k] := false;
    for i := 1 to 64 do
      if ov.b[i] then begin if i <= kk then bad := bad + 1 end
      else if i > kk then bad := bad + 1;
  end;
  exec_$log_w(bad);
end;

function packed_bools_all_$bigt(in k: integer): boolean;
begin
  packed_bools_all_$bigt := big[k];
end;

procedure packed_bools_all_$big(v: integer); options(val_param);
var i: integer;
begin
  for i := 1 to 256 do big[i] := false;
  big[v] := true;
  exec_$log_mem(big, 32);
  exec_$log_bool(packed_bools_all_$bigt(v));
end;

function packed_bools_all_$lo8t(in k: integer): boolean;
begin
  packed_bools_all_$lo8t := lo8[k];
end;

procedure packed_bools_all_$lo8(v: integer); options(val_param);
var i: integer;
begin
  for i := 8 to 263 do lo8[i] := false;
  lo8[v] := true;
  exec_$log_mem(lo8, 32);
  exec_$log_bool(packed_bools_all_$lo8t(v));
end;

procedure packed_bools_all_$inits;
begin
  exec_$log_mem(init9, 4);
  exec_$log_mem(nib, 4);
  exec_$log_mem(two, 2);
  exec_$log_mem(three, 4);
  exec_$log_mem(sb, 6);
end;

procedure packed_bools_all_$bytes;
begin
  r := sb[1];
  exec_$log_w(r);
  r := ub[1] + 72;
  exec_$log_w(r);
  r := sb[4];
  exec_$log_w(r);
  sb[2] := -128;
  r := sb[2];
  exec_$log_w(r);
  r := ub[2];
  exec_$log_w(r);
end;

procedure packed_bools_all_$setup;
var i: integer;
begin
  for i := 1 to 64 do fl[i] := false;
  for i := 3 to 7 do fl[i] := true;
end;
procedure packed_bools_all_$w(k: integer); options(val_param);
var i: integer;
begin
  i := k;
  while fl[i] do i := i + 1;
  r := i;
  exec_$log_w(r);
end;
procedure packed_bools_all_$rp(k: integer); options(val_param);
var i: integer;
begin
  i := k;
  repeat i := i + 1 until fl[i];
  r := i;
  exec_$log_w(r);
  i := k + 3;
  repeat i := i + 1 until not fl[i];
  r := i;
  exec_$log_w(r);
end;
procedure packed_bools_all_$at(k, j: integer); options(val_param);
begin
  n := 0;
  if fl[k] and then fl[j] then n := 1;
  exec_$log_w(n);
  if fl[k] or else not fl[j] then n := n + 2;
  exec_$log_w(n);
end;
