{ Execution test (bead domain-pascal-5kgg, best effort): packed-field
  accesses whose image form is not catalogued (codegen-patterns 5.3): an
  8-bit field of a word type on a byte boundary read and written (a
  constant, a variable), a constant into a one-bit field that is not a
  boolean, a field copied into another.  The neighbours must be kept
  (ref 3.10.7).

  warns: best effort (not byte-identical): a packed field of 8 bits on a byte boundary
  warns: best effort (not byte-identical): a constant stored into a one-bit packed field
  warns: best effort (not byte-identical): copying a packed field that is not a boolean
  call: BE_PACKED_$FILL w:200
  call: BE_PACKED_$ONEBIT
  call: BE_PACKED_$COPY
  expect:
    mem 11 05
    200
    mem 05 c8
    mem 05 ff
    mem ea
    mem 6a
    mem 3a
}
module be_packed;
%include 'exec.ins.pas';

type
  fill_t = packed record
    a: 0..255;
    b: 0..255;
  end;
  bits_t = packed record
    f: 0..1;
    g: 0..127;
  end;
  nib_t = packed record
    hi: 0..15;
    lo: 0..15;
  end;
var
  x: fill_t;
  y: bits_t;
  p, q: nib_t;
  w: integer;

procedure be_packed_$fill(v: integer); options(val_param);
begin
  x.a := 16#11;
  x.b := 5;
  exec_$log_mem(x, 2);
  w := v;
  x.b := w;
  w := 0;
  w := x.b;
  exec_$log_w(w);
  x.a := 5;
  exec_$log_mem(x, 2);
  x.b := 255;
  exec_$log_mem(x, 2);
end;

procedure be_packed_$onebit;
begin
  y.g := 16#6a;
  y.f := 1;
  exec_$log_mem(y, 1);
  y.f := 0;
  exec_$log_mem(y, 1);
end;

procedure be_packed_$copy;
begin
  p.hi := 16#a;
  p.lo := 16#3;
  q.hi := 16#c;
  q.lo := 16#b;
  q.hi := p.lo;
  q.lo := p.hi;
  exec_$log_mem(q, 1);
end;
