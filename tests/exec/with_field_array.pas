{ Execution test, bead domain-pascal-3m13: `with' (ref "With",
  pp. 4-211..4-214) over an element, at a run-time index, of an array
  that is a field at a nonzero offset of its record: the element's
  address formed with no displacement, the array's offset (and the lower
  bound) folded into every field's displacement (SAU5 MT_PURGE_UCB
  0xE75770, `with mt_$scb.units[u]'; codegen-patterns 5.2, "`with`").
  The record in another unit, in the unit's data, in the frame and
  through a pointer; a 1-based array; a set field updated in place; the
  element's address passed by address inside the body; the index changed
  in the body (p. 4-212: the record is evaluated once).  Expected values
  are the Pascal semantics.

  link: with_field_array_ext.mod.pas
  call: WFA_$RUN
  expect:
    7
    70001
    true
    false
    9
    70002
    5
    11
    -3
    12
    13
    4
    true
    false
}
module with_field_array;
%include 'exec.ins.pas';

type
  unit_t = record
    head: integer32;
    flags: set of 0..15;
    cnt: integer;
  end;
  scb_t = record
    pad: array[1..30] of integer;
    units: array[0..2] of unit_t;
  end;
  scb_p = ^scb_t;
  scb1_t = record
    pad: array[1..5] of integer;
    units: array[1..3] of unit_t;
  end;

var
  wfa_$scb: extern scb_t;
  m: scb1_t;
  gp: scb_p;
  k: integer;

procedure bump(var q: integer32);
begin
  q := q + 1;
end;

procedure wfa_$ext(in u: integer);
begin
  with wfa_$scb.units[u] do begin
    head := 70000;
    flags := flags + [10];
    cnt := 7;
    bump(head);
  end;
end;

procedure wfa_$run;
var
  l: scb_t;
  b: boolean;
begin
  wfa_$scb.units[1].flags := [];
  wfa_$ext(1);
  exec_$log_w(wfa_$scb.units[1].cnt);
  exec_$log_l(wfa_$scb.units[1].head);
  b := 10 in wfa_$scb.units[1].flags;
  exec_$log_bool(b);
  b := 10 in wfa_$scb.units[0].flags;
  exec_$log_bool(b);

  k := 2;
  with m.units[k] do begin
    cnt := 9;
    k := 3;
    head := 70002;
  end;
  exec_$log_w(m.units[2].cnt);
  exec_$log_l(m.units[2].head);

  gp := addr(wfa_$scb);
  k := 2;
  with gp^.units[k] do begin
    cnt := 5;
    exec_$clobber;
    head := cnt + 6;
  end;
  exec_$log_w(wfa_$scb.units[2].cnt);
  exec_$log_l(wfa_$scb.units[2].head);

  k := 0;
  with l.units[k] do begin
    cnt := -3;
    head := 12;
  end;
  exec_$log_w(l.units[0].cnt);
  exec_$log_l(l.units[0].head);

  k := 1;
  with l.units[k + 1] do begin
    head := 13;
    cnt := 4;
    flags := [3];
  end;
  exec_$log_l(l.units[2].head);
  exec_$log_w(l.units[2].cnt);
  b := 3 in l.units[2].flags;
  exec_$log_bool(b);
  b := 4 in l.units[2].flags;
  exec_$log_bool(b);
end;
