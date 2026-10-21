{ Execution test (bead domain-pascal-nzs4): flags of an `add'/`sub' in
  memory and across an `if''s labels (codegen-patterns 7.1, "Flags
  across an `if''s labels").  `n := n - 1; if n < 0' branches on the
  `subq''s flags (SAU5 0xE75B9E, 0xE59682); the `else if n = 0' after
  it (SAU5 0xE75BAA) and the `if n = 0' after `if n < 0 then n := 0'
  (SAU5 0xE5968C) branch on the flags their label is entered with; so
  does an `else if' after a `tst' (SAU2 0xE3FE4A).  `addq.l' and `>='
  likewise (SAU2 0xE169C0 is an `add.l Dn' to memory: dpc adds in a
  register, `addq' only for a constant).  Values on both sides
  of 0, and -32768 (the `subq' overflows: V set, N clear, so a test of
  N, like a `tst' of the result, finds it not negative).

  call: DEC_FLAGS_$ELSE @w:0
  call: DEC_FLAGS_$ELSE @w:1
  call: DEC_FLAGS_$ELSE @w:2
  call: DEC_FLAGS_$ELSE @w:-32768
  call: DEC_FLAGS_$JOIN @w:0
  call: DEC_FLAGS_$JOIN @w:1
  call: DEC_FLAGS_$JOIN @w:2
  call: DEC_FLAGS_$JOIN @w:-5
  call: DEC_FLAGS_$FIELD @w:1
  call: DEC_FLAGS_$FIELD @w:-1
  call: DEC_FLAGS_$FIELD @w:9
  call: DEC_FLAGS_$ADDL @l:-3
  call: DEC_FLAGS_$ADDL @l:-4
  call: DEC_FLAGS_$ADDL @l:0x7ffffffd
  call: DEC_FLAGS_$TSTGT @w:3
  call: DEC_FLAGS_$TSTGT @w:0
  call: DEC_FLAGS_$TSTGT @w:-3
  call: DEC_FLAGS_$TSTEQ @w:0
  call: DEC_FLAGS_$TSTEQ @w:3
  call: DEC_FLAGS_$TSTEQ @w:-3
  call: DEC_FLAGS_$TSTEQ @w:-32768
  expect:
    0
    1
    0
    2
    1
    2
    32767
    1
    1
    2
    1
    1
    0
    0
    2
    8
    1
    0
    0
    1
    2
    3
    1
    2
    3
    3
}
module dec_flags;
%include 'exec.ins.pas';

type
  rec = record
    pad: integer32;
    n: integer;
  end;

var
  g: integer;
  gl: integer32;
  r: rec;
  p: ^rec;

{ SAU5 0xE75B9E: `subq.w; bpl.s; clr.w; bra.s; bne.s' }
procedure dec_flags_$else(in w: integer);
begin
  g := w;
  g := g - 1;
  if g < 0 then
    g := 0
  else if g = 0 then
    exec_$logv_w(1)
  else
    exec_$logv_w(2);
  exec_$logv_w(g);
end;

{ SAU5 0xE59682: `subq.w; bpl.s; clr.w; bne.s' }
procedure dec_flags_$join(in w: integer);
begin
  g := w;
  g := g - 1;
  if g < 0 then
    g := 0;
  if g = 0 then
    exec_$logv_w(1)
  else
    exec_$logv_w(2);
end;

{ The same through a pointer, a record field (SAU5 0xE75B9E's
  `(0x30,A3)'). }
procedure dec_flags_$field(in w: integer);
begin
  p := addr(r);
  with p^ do begin
    n := w;
    n := n - 1;
    if n < 0 then
      n := 0
    else if n = 0 then
      exec_$logv_w(1)
    else
      exec_$logv_w(2);
    exec_$logv_w(n);
  end;
end;

{ `addq.l' of a longword, `>=': `bmi.s' on its flags }
procedure dec_flags_$addl(in a: integer32);
begin
  gl := a;
  gl := gl + 3;
  if gl >= 0 then
    exec_$logv_w(1)
  else
    exec_$logv_w(0);
end;

{ SAU2 0xE3FE3E: `tst.w; ble.s; ...; bra.s; bne.s' }
procedure dec_flags_$tstgt(in w: integer);
begin
  g := w;
  if g > 0 then
    exec_$logv_w(1)
  else if g = 0 then
    exec_$logv_w(2)
  else
    exec_$logv_w(3);
end;

{ An ordered test at the `else' label after a `tst': `bne.s; ...; ble.s' }
procedure dec_flags_$tsteq(in w: integer);
begin
  g := w;
  if g = 0 then
    exec_$logv_w(1)
  else if g > 0 then
    exec_$logv_w(2)
  else
    exec_$logv_w(3);
end;
