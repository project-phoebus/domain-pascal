{ codegen-patterns 3 ("Record results over 4 bytes"); bead
  domain-pascal-ytu3.  A function whose result is a record of more than 4
  bytes returns it through a hidden first argument: the address of a
  place for it, which the caller pushes last and the callee stores
  through; no register carries it back.

  Callees:
  - get_root: an 8-byte record from the unit's data, `movea.l (8,A6),A1;
    lea (d,A5),A0; move.l (A0)+,(A1); move.l (A0)+,(4,A1)', `link #0'
    (no result slot; NAME_$GET_ROOT_UID 0xE589BC);
  - from_local: an `internal' function of a word value parameter, which
    is at (0xc,A6) behind the hidden address (the module-local 0xE4232A
    `move.w (0xc,A6),D0'), the result copied from a local
    (0xE42344..0xE4234E);
  - clk: a 6-byte record, `movea.l (8,A6),A1; move.l (-0xc,A6),(A1);
    move.w (-0x8,A6),(0x4,A1)' (the module-local 0xE58C02, 0xE58C28);
  - pick: assignments on several paths, each a copy through the hidden
    address (ACL_$DEFAULT_ACL 0xE4787E).

  Callers:
  - assign_local: `pea temp; bsr; addq.w #4,SP' then the record copied
    from the temporary (the module-local 0xE4A060 0xE4A0D4..0xE4A0E4);
  - assign_field: into a field through a parameter's pointer
    (ASKNODE_$INTERNET_INFO 0xE648AC..0xE648C4); the pop skipped before
    the copy and the epilogue;
  - assign_self: the argument is the destination, still through the
    temporary (TIME_$SET_ITIMER 0xE58ED2..0xE58EE4);
  - word_arg: a word value argument and no pad (PROC2_$UPGID_TO_UID
    0xE41030..0xE4103C, `addq.w #6,SP');
  - pass_on: the result passed by address to another call, the inner
    call made among the outer call's pushes and popped at once
    (dir_$old_create_obj 0xE54698..0xE546BC);
  - pass_two: two of them, each in its own temporary, -8 and -0x10
    (TIME_$SET_ITIMER 0xE58E9E..0xE58ECA). }
module hres;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  time_$clock_t = record
    high: integer32;
    low: 0..65535;
  end;
  holder_t = record
    flags: integer32;
    uid: uid_$t;
  end;
  holder_ptr_t = ^holder_t;
var
  root: uid_$t;
  other: uid_$t;
  uid_$nil: extern uid_$t;
  acl_$fndwrx: extern uid_$t;

function get_root: uid_$t;
begin
  get_root := root;
end;

function from_local(upgid: integer): uid_$t; internal;
var
  u: uid_$t;
begin
  u := uid_$nil;
  u.low := upgid;
  from_local := u;
end;

function clk(in c: time_$clock_t): time_$clock_t; internal;
var
  x: time_$clock_t;
begin
  x.high := c.high + c.high;
  x.low := c.low;
  clk := x;
end;

function pick(in kind: integer): uid_$t;
begin
  if kind = 0 then
    pick := acl_$fndwrx
  else if kind = 3 then
    pick := uid_$nil;
end;

function default_acl(in kind: integer): uid_$t; extern;
function to_clock(in c: time_$clock_t): time_$clock_t; extern;
procedure set_acl(in u: uid_$t; in obj: integer32; out st: integer32); extern;
procedure set_both(in which: integer; in a: time_$clock_t; in b: time_$clock_t); extern;

procedure assign_local;
var
  u: uid_$t;
begin
  u := get_root;
  other := u;
end;

procedure assign_field(h: holder_ptr_t);
begin
  h^.uid := get_root;
end;

procedure assign_self(in out c: time_$clock_t);
begin
  c := clk(c);
end;

procedure word_arg(in upgid: integer);
var
  u: uid_$t;
  w: integer;
begin
  w := upgid;
  u := from_local(w);
  other := u;
end;

procedure pass_on(in obj: integer32);
var
  st: integer32;
begin
  set_acl(default_acl(3), obj, st);
end;

procedure pass_two(in a: time_$clock_t; in b: time_$clock_t);
begin
  set_both(1, to_clock(a), to_clock(b));
end;
