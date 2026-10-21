{ Execution test: pointer constants other than `nil' (codegen-patterns
  13, "Pointer constants"; bead domain-pascal-gw19), formed in an
  address register with `movea.l #n,An' where they need a register:
  stored in a `dbf' loop into a frame record's handlers (SAU5
  PBU_$ACQUIRE_INT 0xE588D4), into the unit's array and variables and
  through a pointer; stored directly outside `moveq''s range
  (`move.l #0x7000'); assigned to a loop's pointer local held in an
  address register (PEB_$GET_STATUS 0xE5ADD6), with `nil' as `suba.l';
  compared with `cmpi.l' in memory and `cmpa.l' in a register, also a
  pointer function's result in A0.  The constants are the type
  transfers of ref p. 4-197 (owner's rulings, docs/
  reference-clarifications.md: `p(-1)' is 16#FFFFFFFF).

  warns: best effort (not byte-identical): comparing a pointer held in an address register with a constant in a word's range
  warns: best effort (not byte-identical): comparing a pointer function's result with a pointer constant
  call: PCK_$SETUP
  call: PCK_$FILL
  call: PCK_$SHOW
  call: PCK_$WALK @l:100 -> l
  call: PCK_$WALK @l:15 -> l
  call: PCK_$WALK_NIL -> l
  call: PCK_$IS_NONE @l:-1 -> b
  call: PCK_$IS_NONE @l:5 -> b
  expect:
    15
    0x00000000
    0x00000005
    0x00007000
    0xffffffff
    => 60
    => 30
    => 60
    => true
    => false
}
module pck;
%include 'exec.ins.pas';

type
  node_p = ^node;
  node = record
    next: node_p;
    v: integer32;
  end;
  hold_t = record
    unit: integer;
    stack: univ_ptr;
    h: array[0..15] of univ_ptr;
  end;
  hold_p = ^hold_t;
const
  no_handler = -1;
var
  n1: node := [v := 10];
  n2: node := [v := 20];
  n3: node := [v := 30];
  head: node_p;
  tab: array[0..15] of univ_ptr;
  hold: hold_t;
  hp: hold_p;

procedure pck_$setup;
begin
  n1.next := exec_$id_p(addr(n2));
  n2.next := exec_$id_p(addr(n3));
  n3.next := node_p(no_handler);
  head := exec_$id_p(addr(n1));
  hp := exec_$id_p(addr(hold));
end;

procedure pck_$fill;
var
  i: integer;
  local: hold_t;
begin
  for i := 0 to 15 do
    local.h[i] := univ_ptr(no_handler);
  local.h[4] := nil;
  for i := 0 to 15 do
    tab[i] := local.h[i];
  hp^.h[7] := univ_ptr(5);
end;

procedure pck_$show;
var
  i, n: integer;
begin
  hp^.stack := univ_ptr(16#7000);
  n := 0;
  for i := 0 to 15 do
    if tab[i] = univ_ptr(no_handler) then
      n := n + 1;
  exec_$log_w(n);
  exec_$log_x(integer32(tab[4]));
  exec_$log_x(integer32(hp^.h[7]));
  exec_$log_x(integer32(hold.stack));
  exec_$log_x(integer32(n3.next));
end;

function pck_$walk(in lim: integer32): integer32;
var
  r: node_p;
  s: integer32;
begin
  s := 0;
  r := head;
  while r <> node_p(no_handler) do begin
    s := s + r^.v;
    r := r^.next;
    if s > lim then
      r := node_p(no_handler);
  end;
  pck_$walk := s;
end;

function pck_$walk_nil: integer32;
var
  r: node_p;
  s: integer32;
begin
  s := 0;
  r := head;
  while r <> nil do begin
    s := s + r^.v;
    if r^.next = node_p(no_handler) then
      r := nil
    else
      r := r^.next;
  end;
  pck_$walk_nil := s;
end;

function pck_$is_none(in x: integer32): boolean;
begin
  pck_$is_none := exec_$id_p(univ_ptr(x)) = univ_ptr(no_handler);
end;
