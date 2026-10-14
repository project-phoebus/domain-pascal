{ Execution test: attribute lists on parameter types (owner ruling,
  docs/reference-clarifications.md "5.1 Attribute lists on parameter
  types"; bead domain-pascal-lmw).  A `[volatile]` parameter passed by
  address is read again at each use (ref 3.15.1, p. 3-53): EXEC_$COUNTER
  (a "device register" whose every word read returns the next value,
  tools/exectest/runner.c) passed to it makes a wait loop end and two
  reads give two values.  An `out` parameter of Apollo's rws.ins.pas
  form `[aligned(1)] status_$t` is a 4-byte record stored through its
  address, and a `univ [aligned(0)]` one (error.ins.pas) is passed on by
  address.

  call: PARAM_ATTRS_$WAIT -> w
  call: PARAM_ATTRS_$TWICE
  call: PARAM_ATTRS_$STATUS
  expect:
    => 6
    0
    1
    5
    mem 00 0b 00 2a
    42
}
module param_attrs;
%include 'exec.ins.pas';

type
  status_$t = record all: integer32 end;

var
  n, a, b: integer;

function wait(var x: [volatile] integer): integer;
begin
  x := 0;
  n := 0;
  repeat
    n := n + 1;
  until x >= 5;
  wait := n;
end;

function param_attrs_$wait: integer;
begin
  param_attrs_$wait := wait(exec_$counter);
end;

procedure twice(var x: [volatile] integer);
begin
  x := 0;
  a := x;
  exec_$logv_w(a);
  b := x;
  exec_$logv_w(b);
  a := x + x;
  exec_$logv_w(a);
end;

procedure param_attrs_$twice;
begin
  twice(exec_$counter);
end;

procedure set_status(out status: [aligned(1)] status_$t);
begin
  status.all := 16#000b002a;
end;

procedure pass_on(in v: univ [aligned(0)] integer32);
begin
  exec_$log_mem(v, 4);
end;

procedure param_attrs_$status;
var
  st: status_$t;
  w: integer;
begin
  set_status(st);
  pass_on(st);
  w := 42;
  exec_$log_w(w);
end;
