{ Execution test: objects reached through a pointer passed by address,
  harder cases (codegen-patterns 1.5, review of bead domain-pascal-s9j;
  see also pointer_targets): two pointer locals and two parameter slots
  not held in one argument list (A0, A1), a function call evaluated after
  a push through a held slot, a nested routine reached from a sibling
  (static link in A1) passing the enclosing routine's parameter's field,
  `p^' of a pointer local (the pointer pushed) assigned twice, `p.f^'
  and a field of the same held record in one argument list.

  call: PADV_$SETUP
  call: PADV_$TWO_LOCALS
  call: PADV_$TWO_SLOTS
  call: PADV_$CALL_LATER
  call: PADV_$NESTED w:1
  call: PADV_$PLOCAL
  call: PADV_$HELD
  call: PADV_$VOL
  expect:
    11
    110
    12
    120
    17
    7
    18
    19
    121
    29
    101
    122
    102
    39
    40
}
module padv;
%include 'exec.ins.pas';
type
  ctr_t = record
    pad: integer;
    n: integer32;
  end;
  ctr_p = ^ctr_t;
  desc_t = record
    flags: integer;
    pad: integer32;
    c: ctr_t;
    other: ctr_p;
  end;
  desc_p = ^desc_t;
var
  d: desc_t;
  e: desc_t;
  extra: ctr_t := [pad := 0, n := 100];
  current: desc_p;
  vcur: [volatile] desc_p;

procedure bump(var c: ctr_t);
begin
  c.n := c.n + 1;
  exec_$log_l(c.n);
end;

procedure pair(var x: ctr_t; var y: ctr_t);
begin
  x.n := x.n + 1;
  y.n := y.n + 10;
  exec_$log_l(x.n);
  exec_$log_l(y.n);
end;

procedure mix(in v: integer32; var y: ctr_t);
begin
  y.n := y.n + v;
  exec_$log_l(y.n);
end;

procedure mixv(v: integer32; var y: ctr_t); options(val_param);
begin
  exec_$log_l(v);
  y.n := y.n + 1;
  exec_$log_l(y.n);
end;

procedure padv_$setup;
begin
  d.c.n := 10;
  e.c.n := 100;
  d.other := exec_$id_p(addr(extra));
  current := exec_$id_p(addr(d));
  vcur := exec_$id_p(addr(e));
end;

procedure padv_$two_locals;
var
  a, b: desc_p;
begin
  a := current;
  b := exec_$id_p(addr(e));
  pair(a^.c, b^.c);
end;

procedure slots(var s: desc_t; var t: desc_t);
begin
  pair(s.c, t.c);
end;

procedure padv_$two_slots;
begin
  slots(d, e);
end;

procedure later(var t: desc_t);
begin
  mix(exec_$id_l(5), t.c);
  mixv(7, t.c);
end;

procedure padv_$call_later;
begin
  later(d);
end;

procedure outer(var t: desc_t; w: integer);
  procedure inner1;
  begin
    bump(t.c);
    pair(e.c, t.c);
  end;
  procedure sib;
  begin
    inner1;
  end;
begin
  if w > 0 then
    sib;
end;

procedure padv_$nested(w: integer); options(val_param);
begin
  outer(d, w);
end;

procedure padv_$plocal;
var
  p: ctr_p;
begin
  p := d.other;
  bump(p^);
  p := addr(e.c);
  bump(p^);
end;

procedure held(in out t: desc_t);
begin
  t.flags := 0;
  pair(t.other^, t.c);
end;

procedure padv_$held;
begin
  held(d);
end;

procedure padv_$vol;
begin
  bump(current^.c);
end;
