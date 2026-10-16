{ Execution test: one-byte integers, `[byte]' subranges (ref 3.15.5,
  pp. 3-60..3-62: "[byte] 0..255 corresponds to the unsigned char data
  type in Domain/C"; a `[byte] -128..127' is the signed byte).  Fields at
  odd offsets; values above 127 of the unsigned byte stay positive and
  values of the signed one negative, through widening for arithmetic
  and comparisons (ref 4.3.1, p. 4-5); type transfers between the two
  see the same bits (p. 4-197: "the type transfer function does not
  change any bits"); bytes passed by value in the high half of a word
  slot (codegen-patterns 1.3) and by address; a byte function result;
  `&', `lshft', `rshft' of bytes; byte arrays (3.11.4.1: elements times
  their size).  BYTE_INTS_$EDGE takes the boundary values 0, 127, 128,
  255 and 0, 127, -128, -1 through negation, `div`, `mod`, `~`, `!`, a
  product, comparisons with constants (an unsigned byte against -1),
  `lshft` stored into an `integer16` (its low word; owner's ruling,
  reference-clarifications "Lshft of a 16-bit operand"; a byte's `lshft`
  used as a longword, e.g. compared, is refused: bead
  domain-pascal-4rfc), `rshft` by a variable count, a `case` on a byte, the bytes
  stored at odd offsets.  Not expressible yet (refused by dpc): two byte operands
  compared with each other, a widened byte compared with 0 (`tst`).  A
  word variable assigned to a byte: tests/exec/narrow.pas.

  call: BYTE_INTS_$MAIN b:200 b:253
  call: BYTE_INTS_$MAIN b:5 b:127
  call: BYTE_INTS_$EDGE b:0 b:0
  call: BYTE_INTS_$EDGE b:127 b:127
  call: BYTE_INTS_$EDGE b:128 b:-128
  call: BYTE_INTS_$EDGE b:255 b:-1
  expect:
    200
    -3
    197
    -30
    -56
    253
    mark 1
    mark 2
    mark 3
    201
    -4
    12
    1
    mem c9 fc 05 ff
    16340
    53
    -4
    201
    -4
    mark 5
    5
    127
    132
    1270
    5
    127
    mark 4
    6
    126
    0
    2
    mem 06 7e 05 ff
    16340
    53
    126
    6
    126
    mark 6
    0
    0
    0
    0
    0
    0
    0
    0
    -1
    0
    0
    0
    256
    0
    mark 9
    mark 13
    mark 20
    mem 00 00 00 aa
    0
    0
    0
    127
    127
    -127
    -127
    32512
    42
    42
    1
    -128
    16129
    25400
    254
    383
    63
    mark 9
    mark 12
    mark 13
    mark 23
    mem 00 7f 7f aa
    127
    127
    127
    128
    -128
    -128
    128
    -32768
    42
    -42
    2
    -129
    -16384
    -25600
    0
    384
    64
    mark 9
    mark 10
    mark 13
    mark 21
    mem 00 80 80 aa
    -128
    128
    -128
    255
    -1
    -255
    1
    -256
    85
    0
    3
    -256
    -255
    -200
    254
    511
    127
    mark 9
    mark 11
    mark 22
    mem 00 ff ff aa
    -1
    255
    -1
}
module byte_ints;
%include 'exec.ins.pas';

type
  ubyte = [byte] 0..255;
  sbyte = [byte] -128..127;
  rec = record
    pad: char;
    u: ubyte;
    s: sbyte;
    pad2: char;
  end;
  bytes4 = array[1..4] of ubyte;
var
  r: rec;
  w: integer;
  a: bytes4;
  k: integer;

function same_s(v: sbyte): sbyte; options(val_param);
begin
  same_s := v;
end;

procedure take(u: ubyte; s: sbyte); options(val_param);
begin
  w := u;
  exec_$logv_w(w);
  w := s;
  exec_$logv_w(w);
end;

procedure bump(var u: ubyte; var s: sbyte);
begin
  u := u + 1;
  exec_$clobber;
  s := s - 1;
end;

procedure byte_ints_$main(u: ubyte; s: sbyte); options(val_param);
begin
  r.u := u;
  exec_$clobber;
  r.s := s;
  w := r.u;
  exec_$logv_w(w);
  w := r.s;
  exec_$logv_w(w);
  w := r.u + r.s;
  exec_$logv_w(w);
  w := r.s * 10;
  exec_$logv_w(w);
  w := sbyte(r.u);
  exec_$logv_w(w);
  w := ubyte(r.s);
  exec_$logv_w(w);
  if r.s < -1 then exec_$mark(1);
  if r.u > 199 then exec_$mark(2);
  if r.u = 200 then exec_$mark(3);
  if r.s = 127 then exec_$mark(4);
  bump(r.u, r.s);
  w := r.u;
  exec_$logv_w(w);
  w := r.s;
  exec_$logv_w(w);
  w := rshft(r.u & 16#F0, 4);
  exec_$logv_w(w);
  w := r.u & 16#03;
  exec_$logv_w(w);
  a[1] := r.u;
  exec_$clobber;
  a[2] := r.s;
  exec_$clobber;
  a[3] := 5;
  exec_$clobber;
  a[4] := 255;
  exec_$log_mem(a, 4);
  w := lshft(a[4], 6) + a[3] * 7;
  w := w - 15;
  exec_$logv_w(w);
  k := 3;
  w := a[k] * 3 + a[k] * 12 - 22;
  exec_$logv_w(w);
  w := same_s(r.s);
  exec_$logv_w(w);
  take(r.u, r.s);
  if r.u <> 6 then exec_$mark(5) else exec_$mark(6);
end;

procedure byte_ints_$edge(u: ubyte; s: sbyte); options(val_param);
begin
  w := u;
  exec_$logv_w(w);
  w := s;
  exec_$logv_w(w);
  w := -u;
  exec_$logv_w(w);
  w := -s;
  exec_$logv_w(w);
  w := lshft(u, 8);
  exec_$logv_w(w);
  w := u div 3;
  exec_$logv_w(w);
  w := s div 3;
  exec_$logv_w(w);
  w := u mod 7;
  exec_$logv_w(w);
  w := ~u;
  exec_$logv_w(w);
  w := u * s;
  exec_$logv_w(w);
  k := 200;
  w := s * k;
  exec_$logv_w(w);
  w := u + s;
  exec_$logv_w(w);
  w := u ! 16#100;
  exec_$logv_w(w);
  k := 1;
  w := rshft(u, k);
  exec_$logv_w(w);
  if u > -1 then exec_$mark(9);
  if s < -127 then exec_$mark(10);
  if u = 255 then exec_$mark(11);
  if s >= 127 then exec_$mark(12);
  if u - 128 < 0 + 1 then exec_$mark(13);
  case u of
    0: k := 20;
    128: k := 21;
    255: k := 22;
    otherwise k := 23;
  end;
  exec_$mark(k);
  r.pad := chr(0);
  exec_$clobber;
  r.u := u;
  exec_$clobber;
  r.s := s;
  exec_$clobber;
  r.pad2 := chr(16#AA);
  exec_$log_mem(r, 4);
  w := same_s(r.s);
  exec_$logv_w(w);
  take(r.u, r.s);
end;
