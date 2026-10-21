{ Execution test (bead domain-pascal-59st): an element's address formed
  with `lea' for an `if''s test and held for the call through a routine
  pointer in the same element that begins the `then' part
  (IO_$ENABLE_DEVICE 0xE1A876..0xE1A888 `lea (0,A5,D0.w),A2; tst.l
  (0x70,A2); beq.s; move.l (0x118,A3),-(SP); movea.l (0x70,A2),A0; jsr
  (A0)'; codegen-patterns 1.6, "Held across an `if''s test"), held only
  in the image's shape (the index a field of an enclosing `with' over an
  element, no `else' part, no loop: ENABLE, RELEASE): a nil pointer
  skips the call; the next statement forms the address again
  (IO_$RELEASE 0xE1A724).  Formed twice: the test of another field and
  an `else' part, without a `with'; a function in the arguments that
  changes the index, where
  the address is not held (the pointer is read after the arguments,
  from the element the changed index selects); likewise a function in
  the test.

  call: ELEM_HELD_TEST_$SETUP
  call: ELEM_HELD_TEST_$ENABLE w:0
  call: ELEM_HELD_TEST_$ENABLE w:1
  call: ELEM_HELD_TEST_$ENABLE w:2
  call: ELEM_HELD_TEST_$RELEASE w:0
  call: ELEM_HELD_TEST_$RELEASE w:2
  call: ELEM_HELD_TEST_$OTHER_FIELD w:0
  call: ELEM_HELD_TEST_$OTHER_FIELD w:1
  call: ELEM_HELD_TEST_$WITH_ELSE w:1
  call: ELEM_HELD_TEST_$WITH_ELSE w:3
  call: ELEM_HELD_TEST_$CHANGING
  call: ELEM_HELD_TEST_$CHANGING_TEST
  call: ELEM_HELD_TEST_$TAGS
  expect:
    100
    mark 1
    101
    mark 1
    300
    mark 1
    100
    101
    300
    301
    0
    7
    mark 3
    mark 4
    5
    mark 4
    mark 0
    mark 0
    0
    1
    0
    1
}
module elem_held_test;
%include 'exec.ins.pas';

type
  int_proc_t = ^procedure(v: integer32); options(val_param);
  vec_t = record
    init: univ_ptr;
    log: int_proc_t;
    other: int_proc_t;
    pad: integer32;
  end;
  slot_t = record
    id: integer32;
    tag: integer;
    bus: integer;
  end;

var
  front: array[0..5] of integer32;
  table: array[0..3] of vec_t;
  slots: array[0..3] of slot_t;
  cur: integer;

procedure mark0(v: integer32); options(val_param);
begin
  exec_$mark(0);
end;

procedure elem_held_test_$setup;
var
  i: integer;
begin
  for i := 0 to 3 do
    with table[i] do begin
      log := addr(exec_$logv_l);
      other := addr(exec_$logv_l);
    end;
  table[2].log := nil;
  table[3].log := nil;
  table[1].other := nil;
  for i := 0 to 3 do
    with slots[i] do begin
      id := 100 + i;
      tag := 1;
      bus := i;
    end;
  slots[2].bus := 0;
  slots[2].id := 300;
end;

{ IO_$ENABLE_DEVICE's shape: the `with' element's bus selects the vector. }
procedure elem_held_test_$enable(u: integer); options(val_param);
begin
  with slots[u] do
    if table[bus].log <> nil then
      table[bus].log^(id);
  exec_$mark(1);
end;

{ IO_$RELEASE's shape: the next statement forms the address again. }
procedure elem_held_test_$release(u: integer); options(val_param);
begin
  with slots[u] do begin
    if table[bus].log <> nil then
      table[bus].log^(id);
    table[bus].other^(id + 1);
    tag := 0;
  end;
end;

{ The test of one field, the call through another. }
procedure elem_held_test_$other_field(i: integer); options(val_param);
begin
  if table[i].other <> nil then
    table[i].log^(i);
  if table[i].other = nil then
    exec_$logv_l(7);
end;

procedure elem_held_test_$with_else(i: integer); options(val_param);
begin
  if table[i].other <> nil then
    table[i].other^(i + 2)
  else
    exec_$mark(3);
  exec_$mark(4);
end;

function bump: integer32;
begin
  cur := cur + 1;
  bump := cur;
end;

{ Not held: the argument's function changes the index first, and the
  pointer is read from table[1] (mark0 there). }
procedure elem_held_test_$changing;
begin
  cur := 0;
  table[1].log := addr(mark0);
  if table[cur].log <> nil then
    table[cur].log^(bump);
end;

function bump_nil: int_proc_t;
begin
  cur := cur + 1;
  bump_nil := nil;
end;

{ Not held: the test's function changes the index; the call is through
  table[1] (mark0 there). }
procedure elem_held_test_$changing_test;
begin
  cur := 0;
  table[1].log := addr(mark0);
  if table[cur].log <> bump_nil then
    table[cur].log^(5);
end;

procedure elem_held_test_$tags;
var
  i: integer;
begin
  for i := 0 to 3 do
    exec_$logv_l(slots[i].tag);
end;
