{ Execution test: objects reached through a pointer passed by address
  (codegen-patterns 1.5, bead domain-pascal-s9j; golden
  calls/pointer_targets): `p.f^' with the record held in A2 (the pointer
  field pushed), a field through a slot not held, a field of the
  enclosing routine's parameter from a nested routine, through a pointer
  local in the frame (also `p^' itself) and through a pointer in the
  unit's data.  Each callee adds to the counter it is given by address
  and logs it: the right object is reached when the values follow.

  call: PTARGETS_$SETUP
  call: PTARGETS_$HELD
  call: PTARGETS_$SLOT
  call: PTARGETS_$OUTER w:1
  call: PTARGETS_$LOCAL
  call: PTARGETS_$STATIC
  expect:
    101
    11
    12
    13
    102
    14
    15
}
module ptargets;
%include 'exec.ins.pas';

type
  ctr_t = record
    pad: integer;
    n: integer32;
  end;
  ctr_p = ^ctr_t;
  flag_t = (f_wait, f_1);
  desc_t = record
    flags: set of flag_t;
    pad: integer32;
    c: ctr_t;
    other: ctr_p;
  end;
  desc_p = ^desc_t;
var
  d: desc_t;
  extra: ctr_t := [pad := 0, n := 100];
  current: desc_p;

procedure bump(var c: ctr_t);
begin
  c.n := c.n + 1;
  exec_$log_l(c.n);
end;

procedure ptargets_$setup;
begin
  d.flags := [f_wait];
  d.c.n := 10;
  d.other := exec_$id_p(addr(extra));
  current := exec_$id_p(addr(d));
end;

procedure held(in out t: desc_t);
begin
  t.flags := t.flags - [f_wait];
  bump(t.other^);
end;

procedure ptargets_$held;
begin
  held(d);
end;

procedure slot(var t: desc_t);
begin
  bump(t.c);
end;

procedure ptargets_$slot;
begin
  slot(d);
end;

procedure outer(var t: desc_t; w: integer);
  procedure two;
  begin
    bump(t.c);
    bump(t.c);
  end;
begin
  if w > 0 then
    two;
end;

procedure ptargets_$outer(w: integer); options(val_param);
begin
  outer(d, w);
end;

procedure ptargets_$local;
var
  p: ctr_p;
  q: desc_p;
begin
  p := d.other;
  bump(p^);
  q := current;
  bump(q^.c);
end;

procedure ptargets_$static;
begin
  bump(current^.c);
end;
