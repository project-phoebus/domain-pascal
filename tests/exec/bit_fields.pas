{ Execution test (bead domain-pascal-eoc6): packed record fields that are
  not a whole byte, word or longword of their type (ref 3.10.7 Table 3-3,
  p. 3-36: subranges, enumerations and sets bit aligned, one bit per set
  element, member k at bit k of the field's value; Figure 3-17, p. 3-37,
  a signed subrange in two's complement), codegen-patterns 5.3.

  The record (bit 0 the most significant bit of byte 0):
    neg   -8..7          bits 0..3     (signed, sign-extended on a load)
    s4    set of 0..3    bits 4..7
    flags set of 0..7    byte 1        (a one-byte set, ../source's XPD,
                                        PKT, AREA, AST fields)
    big   0..255         byte 2        (`[byte] 0..255', status_$t.modc)
    sb    -128..127      byte 3        (a signed byte)
    e     (e0..e3)       bits 32..33
    s5    set of 0..5    bits 34..39
    w12   -2048..2047    bits 40..51   (bytes 5..6: a longword unit)
    one   0..1           bit 52
  Stores keep the neighbouring bits (the record's bytes are logged after
  each group); loads, comparisons and `in' with constant and run-time
  members; field to field copies; set fields updated with `+', `-' and
  `*'; bit fields passed to value and `in' parameters (ref 3.10.3 forbids
  it; accepted, best effort).  BIT_FIELDS_$WIDE: run-time members
  outside a word set field (the register cleared above the value, after
  a call leaves junk in the scratch registers), a set field of 21
  elements in a longword unit tested in a condition, and static initial
  values of set fields (member k at bit k; a one-byte one as a `[byte]
  set of 0..7').

  call: BIT_FIELDS_$RUN w:-5 w:3
  call: BIT_FIELDS_$WIDE w:20 w:17
  warns: best effort (not byte-identical): `in` on a one-byte set with a run-time element
  warns: best effort (not byte-identical): a constant stored into a one-bit packed field that is not a boolean
  warns: best effort (not byte-identical): a packed field of a subrange with negative values
  warns: best effort (not byte-identical): a packed field of type set of 0..3 that is not a whole byte, word or longword
  warns: best effort (not byte-identical): a packed field of type set of 0..20 that is not a whole byte, word or longword
  warns: best effort (not byte-identical): passing a field of a packed record that is not a whole byte, word or longword
  warns: best effort (not byte-identical): storing a computed value narrower than the field's unit
  warns: best effort (not byte-identical): initialising a set field of a packed record that is not whole bytes
  expect:
    mem 00 00 00 00 00 00 00 00
    mem ba 00 00 00 00 00 00 00
    -5
    true
    false
    true
    false
    mem ba a4 c8 80 00 00 00 00
    true
    true
    true
    false
    200
    -128
    mem ba a4 c8 80 95 00 00 00
    2
    true
    true
    false
    mem ba a4 c8 80 95 fe d0 00
    -19
    -19
    mem ba a4 c8 80 95 fe d8 00
    1
    mem ba a4 c8 80 95 fe d0 00
    0
    mark 1
    mem bc 8c 05 80 75 fb 60 00
    true
    false
    true
    5
    -5
    true
    -5
    -74
    0
    mem bf 8c 05 80 64 fb 60 00
    mem b6 8c 05 80 64 fb 60 00
    mem c9 89 c8 00
    false
    false
    true
    true
    mark 4
    mark 5
    mark 6
    mem 00 04 00 10
}
module bit_fields;
%include 'exec.ins.pas';

type
  e4_t = (e0, e1, e2, e3);
  pr_t = packed record
    neg: -8..7;
    s4: set of 0..3;
    flags: set of 0..7;
    big: 0..255;
    sb: -128..127;
    e: e4_t;
    s5: set of 0..5;
    w12: -2048..2047;
    one: 0..1;
  end;
  s8_t = set of 0..7;
  wr_t = packed record
    pad: 0..1;
    s9: set of 0..8;          { bits 1..9: a word unit }
    s21: set of 0..20;        { bits 10..30: a longword unit }
  end;
  ir_t = packed record
    pad: 0..1;
    s3: set of 0..3;          { bits 1..4 }
    n3: 0..7;                 { bits 5..7 }
    fl: set of 0..7;          { byte 1 }
    ub: 0..255;               { byte 2 }
  end;

var
  r: pr_t;
  w: integer;
  k: integer;
  b: boolean;
  s: s8_t;
  wr: wr_t;
  ir: ir_t := [pad := 1, s3 := [0, 3], n3 := 1, fl := [0, 3, 7], ub := 200];
  ll: integer32;

procedure dump; internal;
begin
  exec_$log_mem(r, 8);
end;

procedure log_member(m: integer); internal;
begin
  b := m in r.s4;
  exec_$log_bool(b);
end;

procedure bit_fields_$run(a: integer; n: integer); options(val_param);
begin
  r.flags := [];
  r.neg := 0;
  r.big := 0;
  r.s4 := [];
  r.sb := 0;
  r.e := e0;
  r.s5 := [];
  r.w12 := 0;
  r.one := 0;
  dump;

  { a signed 4-bit field and a 4-bit set }
  r.neg := a;
  r.s4 := [1, 3];
  dump;
  w := r.neg;
  exec_$log_w(w);
  log_member(1);
  log_member(2);
  log_member(n);
  log_member(n - 1);

  { the one-byte fields }
  r.flags := [2, 5];
  w := 0;
  r.big := 200;
  w := 1;
  r.sb := -128;
  r.big := r.big - n + 3;
  r.flags := r.flags + [7];
  dump;
  b := 5 in r.flags;
  exec_$log_bool(b);
  b := 7 in r.flags;
  exec_$log_bool(b);
  k := 2;
  b := k in r.flags;
  exec_$log_bool(b);
  k := n;
  b := k in r.flags;
  exec_$log_bool(b);
  w := r.big;
  exec_$log_w(w);
  w := r.sb;
  exec_$log_w(w);

  { an enumeration and a set across a byte, a copy between fields }
  r.e := e2;
  r.s5 := [0, 2, 4];
  r.neg := r.neg;
  dump;
  w := ord(r.e);
  exec_$log_w(w);
  if r.e = e2 then
    exec_$log_bool(true);
  if 4 in r.s5 then
    exec_$log_bool(true);
  b := n in r.s5;
  exec_$log_bool(b);

  { a signed field in a longword unit }
  r.w12 := -19;
  dump;
  w := r.w12;
  exec_$log_w(w);
  if r.w12 < 0 then
    exec_$log_w(r.w12 + 0);
  r.one := 1;
  dump;
  w := r.one;
  exec_$log_w(w);
  r.one := 0;
  dump;
  w := r.one;
  exec_$log_w(w);
  exec_$mark(1);

  { a set field and a set variable, assigned both ways; update in place }
  s := r.flags;
  s := s - [5];
  s := s + [3];
  r.flags := s;
  r.big := 5;
  r.s4 := r.s4 - [1];
  r.s4 := r.s4 + [2];
  r.w12 := r.neg;
  r.e := e1;
  r.s5 := r.s5 + [5];
  r.w12 := r.w12 + r.neg - 64;
  dump;
  b := 3 in r.flags;
  exec_$log_bool(b);
  b := 1 in r.s4;
  exec_$log_bool(b);
  b := 2 in r.s4;
  exec_$log_bool(b);
  w := r.big;
  exec_$log_w(w);
  w := r.neg;
  exec_$log_w(w);
  b := r.neg = a;
  exec_$log_bool(b);

  { bit fields as value and `in' arguments: their values }
  exec_$log_w(r.neg);
  exec_$logv_w(r.w12);
  exec_$log_w(r.one);

  { several constant members at once }
  r.s5 := r.s5 - [0, 4];
  r.s4 := r.s4 + [0, 1];
  dump;
  r.s4 := r.s4 * [1, 2];
  dump;
end;

procedure bit_fields_$wide(d: integer; e: integer); options(val_param);
begin
  exec_$log_mem(ir, 4);
  wr.pad := 0;
  wr.s9 := [1];
  wr.s21 := [0, 17, 20];
  ll := exec_$id_l(-1);
  b := d in wr.s9;
  exec_$log_bool(b);
  ll := exec_$id_l(-1);
  b := d - 4 in ir.s3;
  exec_$log_bool(b);
  b := e in wr.s21;
  exec_$log_bool(b);
  b := 3 in ir.fl;
  exec_$log_bool(b);
  if d in wr.s21 then
    exec_$mark(4);
  if e in wr.s21 then
    exec_$mark(5);
  if 1 in wr.s9 then
    exec_$mark(6);
  if 2 in wr.s9 then
    exec_$mark(7);
  wr.s9 := [];
  wr.s21 := wr.s21 - [0, 20];
  wr.s21 := wr.s21 + [3];
  exec_$log_mem(wr, 4);
end;
