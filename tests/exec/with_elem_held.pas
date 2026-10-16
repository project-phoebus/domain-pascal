{ Execution test: `with' records that are array elements, their address
  last used in the arguments of a call in a body with branches, held
  through the call in a callee-saved register (codegen-patterns 12.4
  R4(c); bead domain-pascal-ig8d; golden regalloc/with_elem_held): the
  address pushed in an `else' part (EC2_$RELEASE_EC1 0xE42BB0), a field
  read for a pushed value (MMAP_$INSTALL_PAGES 0xE0CE3C), a routine
  pointer field called (IO_$DISABLE_DEVICE 0xE1A82E).  The runner checks
  that D2-D7 and A2-A6 come back.

  call: WITH_ELEM_HELD_$SETUP
  call: WITH_ELEM_HELD_$PUSH w:2
  call: WITH_ELEM_HELD_$PUSH w:3
  call: WITH_ELEM_HELD_$SHOW
  call: WITH_ELEM_HELD_$FEED w:2
  call: WITH_ELEM_HELD_$SHOW
  call: WITH_ELEM_HELD_$FEED w:3
  call: WITH_ELEM_HELD_$TARGET w:2
  call: WITH_ELEM_HELD_$TARGET w:3
  expect:
    0
    30
    1
    2
    270
    3
    300
    3
}
module with_elem_held;
%include 'exec.ins.pas';

type
  rec_t = record
    waiters: integer;
    count: integer32;
    limit: integer32;
    handler: ^procedure(in v: integer32);
  end;

var
  table: array[1..4] of rec_t;
  g: integer;

procedure with_elem_held_$setup;
var
  i: integer;
begin
  for i := 1 to 4 do
    with table[i] do begin
      waiters := 0;
      count := i * 100;
      limit := i * 10;
      handler := nil;
    end;
  table[3].waiters := 1;
  table[2].count := 10;
  table[2].limit := 40;
  table[3].handler := addr(exec_$log_l);
end;

procedure with_elem_held_$show;
begin
  exec_$log_w(g);
end;

procedure with_elem_held_$push(i: integer); options(val_param);
begin
  with table[i] do
    if waiters = 0 then
      exec_$log_w(waiters)
    else
      g := 1;
  with table[i] do
    if waiters <> 0 then
      exec_$log_l(limit);
end;

procedure with_elem_held_$feed(i: integer); options(val_param);
begin
  with table[i] do
    if count > limit then
      exec_$logv_l(count - limit)
    else
      g := 2;
end;

procedure with_elem_held_$target(i: integer); options(val_param);
begin
  with table[i] do
    if handler <> nil then
      handler^(count)
    else
      g := 3;
  exec_$log_w(g);
end;
