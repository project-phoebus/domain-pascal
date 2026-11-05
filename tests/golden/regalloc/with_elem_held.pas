{ Golden: codegen-patterns 12.4 R4(c) for `with' records that are array
  elements (bead domain-pascal-ig8d; `tools/regalloc.py callothers', kind
  `elem'): in a body with branches, the element's address, held in an
  address register for the `with' body (5.2, "`with`"), whose last use is
  in the arguments of a call in a later statement is held through the
  call, so it takes a callee-saved register (21 of the 23 such element
  addresses; the two others are not register-held values).
  - push_in_else: the address pushed by a call in an `else' part: A2
    (EC2_$RELEASE_EC1 0xE42B96, `pea (A2)' at 0xE42BB0; PBU_$ADVANCE_EC_INT
    0xE88440, `pea (A2)' at 0xE8844E).
  - feed: a field read for a value pushed: A2 (MMAP_$INSTALL_PAGES
    0xE0CE28, `sub.l (60,A2),D0' at 0xE0CE3C; MMAP_$INSTALL_LIST
    0xE0CDC2; the image loads the compared field again at 0xE0CE38, one of
    4 such rereads against 118 reads of the register across a pad
    (docs/register-model.md, F2): `dpc' reads the register).
  - target: a routine pointer field called: A2 (IO_$DISABLE_DEVICE
    0xE1A820, `movea.l (116,A2),A0; jsr (A0)' at 0xE1A82E; IO_$RELEASE
    0xE1A706).
  Refused (driver test `with_element_last_use_in_arguments'): a call
  before the first branch, a call on every path after a join, last uses in several calls' arguments, a last
  use outside a call's arguments on another path, and a `with' record
  reached through a loaded pointer. }
module regalloc;
type
  ec_t = record
    value: integer32;
    waiters: integer;
  end;
  rec_t = record
    ec: ec_t;
    count: integer32;
    limit: integer32;
    handler: ^procedure(in arg: integer32);
  end;
var
  table: array[1..8] of rec_t;
  total: extern integer32;

procedure wake(var e: ec_t); extern;
procedure trim(n: integer32); options(val_param, extern);

procedure push_in_else(i: integer); options(val_param);
begin
  with table[i] do
    if ec.waiters = 0 then
      total := 0
    else
      wake(ec);
end;

procedure feed(i: integer); options(val_param);
begin
  with table[i] do
    if count > limit then
      trim(count - limit);
end;

procedure target(i: integer); options(val_param);
begin
  with table[i] do
    if handler <> nil then
      handler^(count);
end;
