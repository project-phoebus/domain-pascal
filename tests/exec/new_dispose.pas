{ Execution test (bead domain-pascal-338r, domain-c's gap 3): `new(p)'
  and `dispose(p)' ("New", p. 4-117; "Dispose", p. 4-64) as calls of
  the Pascal run time's PAS_$NEW and PAS_$DISPOSE with p by address and
  the size of p^ by value (the headings of ../source's
  lib/ftnlib/new_dispose.pas; best effort, no kernel site).  The module
  supplies a small run time: a bump allocator that logs each size, and
  a dispose that logs the size and sets p to nil, as the reference
  says dispose does.

  call: NEW_DISPOSE_$RUN
  expect:
    6
    12
    12
    300
    1
    6
    1
}
module new_dispose;
%include 'exec.ins.pas';

type
  r_t = record a, b: integer; c: char; end;
  r_p = ^r_t;
  big_t = record x: array [1..3] of integer32; end;
  big_p = ^big_t;

var
  heap: array [1..64] of integer32;
  top: integer;

procedure pas_$new(out p: univ_ptr; size: integer32); options(val_param);
begin
  exec_$logv_l(size);
  p := addr(heap[top]);
  top := top + (size + 3) div 4;
end;

procedure pas_$dispose(in out p: univ_ptr; size: integer32); options(val_param);
begin
  exec_$logv_l(size);
  p := nil;
end;

procedure new_dispose_$run;
var
  p: r_p;
  q: big_p;
begin
  top := 1;
  new(p);
  new(q);
  p^.a := 100;
  p^.b := 200;
  q^.x[1] := 1;
  q^.x[3] := 3;
  exec_$logv_l(q^.x[1] + q^.x[3] + 8);
  exec_$logv_l(p^.a + p^.b);
  if p <> nil then exec_$logv_l(1);
  dispose(p);
  if p = nil then exec_$logv_l(1);
end;
