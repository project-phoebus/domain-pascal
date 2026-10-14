{ codegen-patterns 3 ("Pointer locals held in D0"); bead
  domain-pascal-yzxr.  The descriptor function declared with a 4-byte
  integer result in A0 (`integer32; options(a0_return)', ref 5.5.11,
  p. 5-20), the reading under which TTY_$I_GET_DESC's callers hold its
  result in D0 or in an address register by the type of their local:

  - flush: an integer local, a call after the assignment: `move.l
    A0,D0' after the call, `movea.l D0,A2' as the first instruction
    after the status test, before the next call's pushes, A2 the
    status's (TTY_$K_FLUSH_INPUT 0xE1C0A8, 0xE1C0AE);
  - inq: an integer local, no call after: moved to A0 where first
    dereferenced (TTY_$K_INQ_INPUT_FLAGS 0xE676D4, 0xE676DA);
  - setraw: a pointer local assigned the transferred value, a call after:
    `movea.l A0,A2' right after the call (TTY_$I_SET_RAW 0xE673CC);
  - inqraw: a pointer local assigned the transferred value, no call
    after: left in A0, the `out'
    parameter's slot in A1 (TTY_$I_INQ_RAW 0xE67410..0xE67414). }
module iparked;
type
  status_$t = record case integer of 1: (all: integer32); end;
  flags_t = set of 0..31;
  desc_t = record
    id: integer32;
    flags: flags_t;
    raw: boolean;
  end;
  desc_ptr_t = ^desc_t;

function get_desc(line: integer; out status: status_$t): integer32;
  options(extern, val_param, a0_return);
function lock: integer; extern;
procedure unlock(t: integer); options(extern, val_param);
procedure flush_desc(in out d: desc_t); extern;
procedure set_raw_mode(in out d: desc_t; raw: boolean);
  options(extern, val_param);

procedure flush(in line: integer; out status: status_$t);
var
  tty: integer32;
  t: integer;
begin
  tty := get_desc(line, status);
  if status.all <> 0 then
    return;
  t := lock;
  flush_desc(desc_ptr_t(tty)^);
  unlock(t);
end;

procedure inq(in line: integer; out flags: flags_t; out status: status_$t);
var
  tty: integer32;
begin
  tty := get_desc(line, status);
  if status.all <> 0 then
    return;
  flags := desc_ptr_t(tty)^.flags;
end;

procedure setraw(line: integer; raw: boolean; out status: status_$t);
  options(val_param);
var
  tty: desc_ptr_t;
begin
  tty := desc_ptr_t(get_desc(line, status));
  if status.all <> 0 then
    return;
  set_raw_mode(tty^, raw);
end;

procedure inqraw(line: integer; out raw: boolean; out status: status_$t);
  options(val_param);
var
  tty: desc_ptr_t;
begin
  tty := desc_ptr_t(get_desc(line, status));
  if status.all <> 0 then
    return;
  raw := tty^.raw;
end;
