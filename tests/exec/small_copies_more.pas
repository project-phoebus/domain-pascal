{ Execution test: more 6- and 8-byte record assignments (codegen-patterns
  5.2, "6 and 8 bytes"; bead domain-pascal-fbg; added in its review).
  Operands the first test (small_copies.pas) does not reach: both sides
  through the same `in out' slot; two `var' slots; pointer locals from
  calls (A0 and a callee-saved register); pointers held across a call;
  `val_param' pointer parameters; a constant element of a 1-based array;
  a `with' record; a nested routine storing into its parent's frame;
  value parameters copied at entry; 6-byte array elements.  Expected
  values from ref "Assignment" (p. 4-25).

  The overlapping copy (`p^ := q^' with p four bytes above q) logs
  1 1 1: the two `move.l (Am)+' read the second longword after the first
  store wrote it, as every 8-byte copy in the image does (855 runs,
  codegen-patterns 5.2).  The reference says nothing about overlapping
  operands; this records the original's form, not a language rule.

  call: SM_$MAIN w:5
  expect:
    mem 00 00 00 05 00 00 00 06 00 00 00 05 00 00 00 06
    mem 00 00 00 01 00 00 00 02 00 00 00 01 00 00 00 02
    mem 00 00 00 07 00 00 00 08
    mem 00 00 00 07 00 00 00 08
    mem 00 00 00 09 00 0a 00 00 00 09 00 0a
    mem 00 00 00 05 00 00 00 06
    mem 00 00 00 05 00 00 00 06
    mem 00 00 00 05 00 00 00 06
    mem 00 00 00 05 00 00 00 06
    mem 00 00 00 05 00 00 00 06
    mem 00 00 00 05 00 00 00 06
    mem 00 00 00 01 00 00 00 01 00 00 00 01
    mem 00 00 00 05 00 04
    mem 00 00 00 05 00 04
    mem 00 00 00 05 00 00 00 04
    9
    mem 00 00 00 05 00 00 00 04
    mem 00 00 00 05 00 04 00 00 00 00
}
module sm;
%include 'exec.ins.pas';

type
  uid_$t = record
    high, low: integer32;
  end;
  clk = record
    high: integer32;
    low: integer;
  end;
  pair = record
    a, b: uid_$t;
  end;
  pairp = ^pair;
  uidp = ^uid_$t;
  clkp = ^clk;
var
  g: pair;
  arr: array[1..3] of uid_$t;
  carr: array[1..2] of clk;
  h, i2: uid_$t;
  a3: array[0..2] of integer32;
  gc, hc: clk;
  gu, hu: uid_$t;
  cc: array[0..1] of clk;

procedure same_slot(in out p: pair);
begin
  p.b := p.a;
end;

procedure two_vars(var x: uid_$t; var y: uid_$t);
begin
  x := y;
end;

procedure via_ptr_locals;
var
  p, q: uidp;
begin
  p := exec_$id_p(addr(arr[3]));
  q := exec_$id_p(addr(arr[1]));
  p^ := q^;
end;

procedure keep_ptr(in out r: pair; var s: uid_$t);
begin
  r.a := s;
  exec_$clobber;
  r.b := s;
end;

procedure one(q: uidp); options(val_param);
begin
  q^ := g.a;
end;

procedure two(q: uidp; r: uidp); options(val_param);
begin
  q^ := r^;
  exec_$clobber;
  r^ := q^;
end;

procedure three(q: clkp; r: clkp); options(val_param);
begin
  q^ := r^;
end;

procedure overlap;
var
  p, q: uidp;
begin
  p := exec_$id_p(addr(a3[1]));
  q := exec_$id_p(addr(a3[0]));
  p^ := q^;
end;

procedure p1(c: clk);
begin
  gc := c;
end;

procedure p2(c: clk); options(val_param);
begin
  hc := c;
end;

procedure p3(u: uid_$t);
begin
  gu := u;
end;

procedure p4(u: uid_$t; k: integer); options(val_param);
begin
  hu := u;
  exec_$logv_w(k);
end;

procedure sm_$main(k: integer); options(val_param);
var
  t, t2: uid_$t;
  c: clk;
  pp: pairp;

  procedure inner;
  begin
    t2 := g.b;
  end;

begin
  g.a.high := k;
  g.a.low := k + 1;
  same_slot(g);
  exec_$log_mem(g, 16);
  arr[1].high := 1;
  arr[1].low := 2;
  arr[2] := arr[1];
  exec_$log_mem(arr, 16);
  t.high := 7;
  t.low := 8;
  two_vars(arr[1], t);
  exec_$log_mem(arr[1], 8);
  via_ptr_locals;
  exec_$log_mem(arr[3], 8);
  c.high := 9;
  c.low := 10;
  carr[1] := c;
  carr[2] := carr[1];
  exec_$log_mem(carr, 12);
  pp := exec_$id_p(addr(g));
  with pp^ do
    b := a;
  exec_$log_mem(pp^.b, 8);
  keep_ptr(g, g.a);
  exec_$log_mem(g.b, 8);
  one(addr(h));
  exec_$log_mem(h, 8);
  two(addr(i2), addr(h));
  exec_$log_mem(i2, 8);
  exec_$log_mem(h, 8);
  inner;
  exec_$log_mem(t2, 8);
  a3[0] := 1;
  a3[1] := 2;
  a3[2] := 0;
  overlap;
  exec_$log_mem(a3, 12);
  c.high := k;
  c.low := 4;
  p1(c);
  exec_$log_mem(gc, 6);
  p2(c);
  exec_$log_mem(hc, 6);
  t.high := k;
  t.low := 4;
  p3(t);
  exec_$log_mem(gu, 8);
  p4(t, 9);
  exec_$log_mem(hu, 8);
  three(addr(cc[0]), addr(gc));
  exec_$log_mem(cc, 10);
end;
