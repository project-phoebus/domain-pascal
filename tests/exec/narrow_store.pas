{ Execution test: assignments that narrow an array element (at a
  constant and at a run-time index), a pointer's target, a `with`
  record's field, a function's result and a longword value parameter into a
  byte (bead domain-pascal-d59a; codegen-patterns 8, "Narrowing and
  widening").  The target keeps the source's low bytes (reference-
  clarifications 41), read in place at the higher address or taken from
  the result register at the narrow width.  Boundary values: low words
  0x5678, 0xFFFF (-1 / 65535), 0x8000; low bytes 0x78, 0xFF, 0x80.

  call: NARROW_STORE_$WORDS l:305419896 w:2
  call: NARROW_STORE_$WORDS l:-1 w:3
  call: NARROW_STORE_$WORDS l:98304 w:1
  call: NARROW_STORE_$BYTES l:305419896 w:2
  call: NARROW_STORE_$BYTES l:-1 w:1
  call: NARROW_STORE_$BYTES l:-2147450752 w:2
  expect:
    22136
    22136
    22136
    22136
    22136
    22136
    -1
    -1
    65535
    -1
    -1
    -1
    -32768
    -32768
    32768
    -32768
    -32768
    -32768
    120
    120
    120
    120
    255
    255
    255
    255
    128
    128
    128
    128
}
module narrow_store;
%include 'exec.ins.pas';

type
  uword = 0..65535;
  ubyte = [byte] 0..255;
  ubyte_ptr_t = ^ubyte;
  larr_t = array[1..4] of integer32;
  warr_t = array[1..3] of integer;
  lptr_t = ^integer32;
  rec_t = record
    pad: integer;
    l: integer32;
  end;
var
  gr: rec_t;

procedure narrow_store_$words(l: integer32; i: integer); options(val_param);
var
  a: larr_t;
  p: lptr_t;
  x: integer;
  u: uword;
begin
  a[1] := l;
  a[2] := l;
  a[3] := l;
  x := a[2];                        { a constant index }
  exec_$log_w(x);
  x := a[i];                        { a run-time index }
  exec_$log_w(x);
  u := a[i];
  exec_$log_uw(u);
  p := addr(a[i]);
  x := p^;                          { a pointer's target }
  exec_$log_w(x);
  gr.l := l;
  with gr do
    x := l;                         { a `with` record's field }
  exec_$log_w(x);
  x := exec_$id_l(l);               { a longword result }
  exec_$log_w(x);
end;

procedure narrow_store_$lbyte(p: ubyte_ptr_t; l: integer32); options(val_param);
begin
  p^ := l;                          { a value parameter's low byte, (d+3,A6) }
end;

procedure narrow_store_$bytes(l: integer32; i: integer); options(val_param);
var
  wa: warr_t;
  b: ubyte;
begin
  wa[1] := l;
  wa[2] := l;
  b := wa[i];                       { a word element's low byte }
  exec_$logv_w(b);
  b := exec_$id_w(wa[1]);           { a word result }
  exec_$logv_w(b);
  wa[3] := l;
  b := wa[3];
  exec_$logv_w(b);
  narrow_store_$lbyte(addr(b), l);
  exec_$logv_w(b);
end;
