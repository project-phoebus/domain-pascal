{ Execution test: integer values of another size than the object, value
  parameter or function result they go to (bead domain-pascal-8td;
  codegen-patterns 8, "Narrowing and widening").  The reference sets no
  range check ("Currently, Domain Pascal does not support subrange
  checking", 3.8, p. 3-14; "Domain Pascal does not detect an overflow or
  underflow on integer arithmetic", Appendix D, p. D-4), so a narrowed
  value is its low bytes, read as the target's type: a longword's low
  word as an `integer` (signed) or a `0..65535` (unsigned), a word's low
  byte as a `[byte] 0..255`.  Widening extends by the value's own sign
  (ref 4.3.1, p. 4-5): an `integer` with its sign, a `0..65535` and a
  `[byte] 0..255` with zeros.

  Forms: a longword held in a register (a value parameter loaded at
  entry), in the frame, in the unit's data, a field of a record in the
  frame and of one passed by address (the low word read in place at the
  higher address), stored, passed by value and returned; `rshft` of a
  longword and a product of it, a word plus a longword (computed in 32
  bits, the low word kept); a word sum narrowed to a byte; a word `in`
  parameter by address (`move.b (1,An)`) and a value parameter's copy
  in the frame narrowed to a byte;
  words and bytes widened for longword and word value parameters.
  Boundary values: low words 0x5678, 0xFFFF (-1 / 65535), 0x8000
  (-32768 / 32768), 0x7FFF, 0; bytes 0xFF, 0x80, 0x00.
  Narrowings of elements, pointers' targets, function results and a
  longword into a byte in an assignment: tests/exec/narrow_store.pas
  (bead domain-pascal-d59a); passed by value or returned they are still
  refused by dpc.  A volatile word narrowed by `chr' (a device register, each word
  read counted) is read whole, at its own size (ref 3.15.1-3.15.3;
  review of bead domain-pascal-8td): not a byte read at the higher
  address.

  call: NARROW_$WORDS l:305419896 w:-2
  call: NARROW_$WORDS l:-1 w:32767
  call: NARROW_$WORDS l:98304 w:-32768
  call: NARROW_$WORDS l:-65536 w:1
  call: NARROW_$WORDS l:2147450879 w:0
  call: NARROW_$BYTES @w:511 @w:-128 @b:255
  call: NARROW_$BYTES @w:-32640 @w:383 @b:128
  call: NARROW_$BYTES @w:256 @w:0 @b:0
  call: NARROW_$RESULT l:305419896 -> w
  call: NARROW_$RESULT l:98304 -> w
  call: NARROW_$URESULT l:-1 -> uw
  call: NARROW_$VOLATILE
  expect:
    22136
    22136
    22136
    22136
    -23885
    -11952
    22134
    22136
    22136
    -2
    22136
    22136
    22136
    -2
    -30001
    22136
    -1
    -1
    -1
    -1
    -1
    -16
    32766
    -1
    -1
    32767
    -1
    65535
    65535
    32767
    -1
    -1
    -32768
    -32768
    -32768
    -32768
    3072
    1536
    0
    -32768
    -32768
    -32768
    -32768
    32768
    32768
    -32768
    12288
    -32768
    0
    0
    0
    0
    -2048
    -1024
    1
    0
    0
    1
    0
    0
    0
    1
    -8192
    0
    32767
    32767
    32767
    32767
    -1025
    -528
    32767
    32767
    32767
    0
    32767
    32767
    32767
    0
    -4097
    32767
    255
    128
    127
    255
    255
    -128
    128
    127
    255
    128
    128
    383
    0
    0
    0
    0
    0
    0
    => 22136
    => -32768
    => 65535
    0
    1
    2
}
module narrow;
%include 'exec.ins.pas';

type
  uword = 0..65535;
  ubyte = [byte] 0..255;
  rec_t = record
    pad: integer;
    l: integer32;
    w: integer;
    u: uword;
  end;
var
  gl: integer32;
  gw: integer;

procedure narrow_$fields(in r: rec_t; out o: rec_t);
begin
  o.w := r.l;                       { the low word of a field by address }
  exec_$log_w(o.w);
  exec_$logv_w(r.l);                { pushed from (d+2,An) }
end;

procedure narrow_$words(l: integer32; w: integer); options(val_param);
var
  t: integer32;
  x: integer;
  u: uword;
  r, o: rec_t;
begin
  x := l;                           { held longword, low word }
  exec_$log_w(x);
  gl := l;
  gw := gl;                         { unit data, (d+2,A5) }
  exec_$log_w(gw);
  t := exec_$id_l(l);
  x := t;                           { a local, maybe held }
  exec_$log_w(x);
  r.l := l;
  r.w := r.l;                       { a frame field, (d+2,A6) }
  exec_$log_w(r.w);
  x := rshft(l, 5);                 { computed in 32 bits }
  exec_$log_w(x);
  x := rshft(l, 10) * 16;
  exec_$log_w(x);
  x := w + l;                       { a word plus a longword }
  exec_$log_w(x);
  narrow_$fields(r, o);
  exec_$logv_w(w);
  exec_$logv_w(l);                  { by value, from the register }
  u := l;
  exec_$log_uw(u);
  exec_$logv_l(u);                  { zero-extended }
  exec_$logv_l(w);                  { sign-extended }
  exec_$logv_w(rshft(l, 3));
  exec_$logv_w(gl);                 { by value, from memory }
end;

procedure narrow_$bytes(w: integer; in v: integer; b: ubyte);
var
  c, d: ubyte;
  x: integer;
begin
  c := w;                           { a held word's low byte }
  exec_$logv_w(c);
  d := v;                           { (1,An) }
  exec_$logv_w(d);
  c := w + v;                       { a word sum }
  exec_$logv_w(c);
  exec_$logv_w(b);                  { a byte widened for a word }
  x := b;
  exec_$logv_l(x);
  exec_$logv_w(v);
end;

function narrow_$result(l: integer32): integer; options(val_param);
begin
  narrow_$result := l;
end;

function narrow_$uresult(l: integer32): uword; options(val_param);
begin
  narrow_$uresult := l;
end;

procedure narrow_$volatile;
var
  c: char;
begin
  exec_$counter := 0;
  c := chr(exec_$counter);          { read whole: move.w, not move.b }
  exec_$logv_w(ord(c));
  c := chr(exec_$counter);
  exec_$logv_w(ord(c));
  exec_$logv_w(exec_$counter);
end;
