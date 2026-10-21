{ Execution test (bead domain-pascal-791y): one constant member added to
  or removed from a `[device]' set with `ori'/`andi' of the mask at the
  set's own width (codegen-patterns 5.5; COLOR2_$ENABLE_GPU_INT 0xE7355C
  `andi.w #-5', 0xE73564 `ori.w #0x80' on KOLOR_CTL's ctl, sau5): byte,
  word and longword sets, members in each byte (big-endian, ref 3.9.3),
  with the neighbouring bits and fields set (or clear) so that a wrong
  mask or width shows; and a set that is not a device, `bset'/`bclr'.

  call: DEVICE_SET_MEMBERS_$RUN
  expect:
    mem 5a 5a ff 7f a5 a5
    mem 5a 5a fe fe a5 a5
    mem 5a 5a 00 01 a5 a5
    mem 5a 5a 01 81 a5 a5
    mem a5 f7 5a 5a
    mem a5 09 5a 5a
    mem 3c ef ef ff ff fe c3 c3
    mem 3c ef 00 01 80 01 c3 c3
    mem 80 01
}
module device_set_members;
%include 'exec.ins.pas';
type
  w_t = [device] record
    lo: integer;
    s: set of 0..15;
    hi: integer;
  end;
  b_t = [device] record
    lo: [byte] 0..255;
    b: [byte] set of 0..7;
    hi: integer;
  end;
  l_t = [device] record
    lo: integer;
    l: set of 0..31;
    hi: integer;
  end;
  p_t = record
    s: set of 0..15;
  end;

var
  w: w_t;
  b: b_t;
  l: l_t;
  p: p_t;

procedure device_set_members_$run;
begin
  { a word set between two words }
  w.lo := 16#5A5A;
  w.hi := -16#5A5B;
  w.s := [0..15];
  w.s := w.s - [7];
  exec_$log_mem(w, 6);
  w.s := w.s + [7];
  w.s := w.s - [8];
  w.s := w.s - [0];
  exec_$log_mem(w, 6);
  w.s := [];
  w.s := w.s + [15];
  w.s := w.s + [0];
  w.s := w.s - [15];
  exec_$log_mem(w, 6);
  w.s := w.s + [8];
  w.s := w.s + [7];
  exec_$log_mem(w, 6);

  { a byte set }
  b.lo := 16#A5;
  b.hi := 16#5A5A;
  b.b := [0..7];
  b.b := b.b - [3];
  exec_$log_mem(b, 4);
  b.b := [];
  b.b := b.b + [3];
  b.b := b.b + [0];
  exec_$log_mem(b, 4);

  { a longword set }
  l.lo := 16#3CEF;
  l.hi := -16#3C3D;
  l.l := [0..31];
  l.l := l.l - [31];
  l.l := l.l - [0];
  l.l := l.l + [0];
  l.l := l.l - [28];
  l.l := l.l - [31];
  l.l := l.l + [31];
  l.l := l.l - [0];
  exec_$log_mem(l, 8);
  l.l := [];
  l.l := l.l + [0];
  l.l := l.l + [31];
  l.l := l.l - [31];
  l.l := l.l + [16];
  l.l := l.l + [15];
  exec_$log_mem(l, 8);

  { not a device }
  p.s := [];
  p.s := p.s + [15];
  p.s := p.s + [0];
  exec_$log_mem(p, 2);
end;
