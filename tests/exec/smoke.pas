{ Execution test: the harness itself.  Logging stubs of every kind, a
  by-value and a by-address entry argument, a word result, an out
  parameter.

  call: SMOKE_$MAIN
  call: SMOKE_$ADD w:3 l:-5 -> l
  call: SMOKE_$SET @w:1! w:7
  expect:
    1
    -2
    65535
    65
    true
    0x12345678
    mark 9
    100000
    -7
    200
    => -2
    out 7
}
module smoke;
%include 'exec.ins.pas';

procedure smoke_$main;
var
  b: boolean;
  c: char;
begin
  exec_$log_l(1);
  exec_$log_w(-2);
  exec_$log_uw(65535);
  exec_$log_c('A');
  b := true;
  exec_$log_bool(b);
  exec_$log_x(16#12345678);
  exec_$mark(9);
  exec_$logv_l(100000);
  exec_$logv_w(-7);
  c := chr(200);
  exec_$logv_c(c);
end;

function smoke_$add(a: integer; in b: integer32): integer32; options(val_param);
begin
  smoke_$add := a + b;
end;

procedure smoke_$set(out x: integer; v: integer); options(val_param);
var
  t: integer;
begin
  t := v;
  x := t;
end;
