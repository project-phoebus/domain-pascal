{ codegen-patterns 3 ("Pointer locals held in D0"), 12.4; beads
  domain-pascal-dgb, -78n2.  Under the integer reading (the descriptor
  function `integer32; options(a0_return)', ref 5.5.11, p. 5-20), an
  integer local assigned once from it, only dereferenced through a type
  transfer, and live across no call: held in D0 from the call (`move.l
  A0,D0') and moved to A0 by the first statement that dereferences it
  (`movea.l D0,A0'), D0 staying the local's up to its last dereference
  (the same Pascal with pointer locals leaves them in A0 instead,
  TTY_$I_INQ_RAW 0xE67414; integer_parked's inqraw):

  - inq: the move first in the statement (TTY_$K_INQ_INPUT_FLAGS
    0xE676D4..0xE676E0 `move.l A0,D0; tst.l (A2); bne.b; movea.l D0,A0;
    movea.l (0xc,A6),A1; move.l (0x14,A0),(A1)');
  - setf: an `if' whose two parts dereference it, the move just before
    the condition's branch and after its slot load, the set built in D1
    and D2 with D0 left alone (TTY_$K_SET_INPUT_FLAG 0xE6767E..0xE676A2);
  - put: the pointer the destination's base (TTY_$K_SET_SESSION_ID
    0xE679AA..0xE679B6). }
module pparked;
type
  status_$t = record case integer of 1: (all: integer32); end;
  flags_t = set of 0..31;
  desc_t = record
    id: integer32;
    pad: array[1..8] of integer;
    flags: flags_t;
    sid: integer;
  end;
  desc_ptr_t = ^desc_t;

function get_desc(line: integer; out status: status_$t): integer32;
  options(extern, val_param, a0_return);

procedure inq(in line: integer; out flags: flags_t; out status: status_$t);
var
  d: integer32;
begin
  d := get_desc(line, status);
  if status.all <> 0 then
    return;
  flags := desc_ptr_t(d)^.flags;
end;

procedure setf(in line: integer; in flag: integer; in value: boolean;
               out status: status_$t);
var
  d: integer32;
begin
  d := get_desc(line, status);
  if status.all <> 0 then
    return;
  if value then
    desc_ptr_t(d)^.flags := desc_ptr_t(d)^.flags + [flag]
  else
    desc_ptr_t(d)^.flags := desc_ptr_t(d)^.flags - [flag];
end;

procedure put(in line: integer; in sid: integer; out status: status_$t);
var
  d: integer32;
begin
  d := get_desc(line, status);
  if status.all <> 0 then
    return;
  desc_ptr_t(d)^.sid := sid;
end;
