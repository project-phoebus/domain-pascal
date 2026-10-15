{ Execution test: `nil' stored (ref 3.13.4 p. 3-51; codegen-patterns 13,
  "Constants stored": `clr.l <ea>', bead domain-pascal-j4ip) into a
  pointer of the unit, a field reached through a pointer, a routine
  pointer, a local held in a data register (`clr.l D0'; IO_$GET_DCTE
  0xE1A466 `clr.l D2'), locals in the frame, one passed by address; each
  one non-nil before, tested after.

  call: NILS_$SETUP
  call: NILS_$CLEAR
  call: NILS_$COUNT
  call: NILS_$FIND
  call: NILS_$PICK
  call: NILS_$EMPTY
  call: NILS_$FIND
  call: NILS_$PICK
  call: NILS_$FRAME
  expect:
    mark 1
    mark 2
    mark 3
    mark 20
    mark 30
    mark 0
    mark 6
    mark 5
}
module nils;
%include 'exec.ins.pas';

type
  node_p = ^node;
  node = record
    val: integer;
    next: node_p;
  end;
  proc_t = ^procedure;
var
  n1: node := [val := 10];
  n2: node := [val := 20];
  n3: node := [val := 30];
  head: node_p;
  spare: node_p;
  callback: proc_t;
  total: integer;

procedure hello;
begin
  exec_$mark(99);
end;

procedure nils_$setup;
begin
  n1.next := exec_$id_p(addr(n3));
  n2.next := exec_$id_p(addr(n1));
  n3.next := exec_$id_p(addr(n2));
  head := exec_$id_p(addr(n3));
  spare := exec_$id_p(addr(n2));
  callback := addr(hello);
end;

{ the list cut after n1 (n1.next := nil), the spare and the callback
  cleared }
procedure nils_$clear;
begin
  n1.next := nil;
  spare := nil;
  callback := nil;
  if spare = nil then
    exec_$mark(1);
  if callback = nil then
    exec_$mark(2);
end;

procedure nils_$count;
var
  p: node_p;
begin
  total := 0;
  p := head;
  while p <> nil do begin
    total := total + 1;
    p := p^.next;
  end;
  exec_$mark(total);
end;

{ the head, or nil: a local held in a data register (`clr.l D0') }
procedure nils_$pick;
var
  p: node_p;
begin
  p := nil;
  if head <> nil then
    p := head;
  spare := p;
  if spare = nil then
    exec_$mark(6)
  else
    exec_$mark(spare^.val);
end;

{ the head pointer cleared }
procedure nils_$empty;
begin
  head := nil;
end;

{ the last node whose value is 20, or nil: the local starts at nil }
procedure nils_$find;
var
  found: node_p;
  p: node_p;
begin
  found := nil;
  p := head;
  while p <> nil do begin
    if p^.val = 20 then
      found := p;
    p := p^.next;
  end;
  if found = nil then
    exec_$mark(0)
  else
    exec_$mark(found^.val);
end;

procedure set_ptr(var q: node_p);
begin
  q := head;
end;

{ a local in the frame (its address is taken) set to nil after a call
  made it non-nil }
procedure nils_$frame;
var
  q: node_p;
begin
  set_ptr(q);
  q := nil;
  if q = nil then
    exec_$mark(5);
end;
