{ Execution test: `nil' passed by value (codegen-patterns 1.5, "Zero":
  `clr.l -(SP)', NET_IO_$PUT_IN_SOCK 0xE0E4CC; bead domain-pascal-73sx)
  to a `val_param' routine and to an `internal' one, beside non-constant
  arguments, and a non-nil pointer for contrast; the callee tests the
  pointer it received and marks the word argument beside it.

  call: NILP_$RUN
  expect:
    mark 1
    mark 7
    mark 3
    mark 7
    mark 2
    mark 9
}
module nilp;
%include 'exec.ins.pas';

type
  int_p = ^integer;
var
  cell: integer := 42;

procedure nilp_$show(p: int_p; w: integer); options(val_param);
begin
  if p = nil then
    exec_$mark(1);
  if p <> nil then
    exec_$mark(3);
  exec_$mark(w);
end;

procedure nilp_$both(w: integer; p: int_p); internal;
begin
  if p = nil then
    exec_$mark(2);
  exec_$mark(w);
end;

procedure nilp_$run;
var
  k: integer;
  q: int_p;
begin
  k := 7;
  q := exec_$id_p(addr(cell));
  nilp_$show(nil, k);
  nilp_$show(q, k);
  k := 9;
  nilp_$both(k, nil);
end;
