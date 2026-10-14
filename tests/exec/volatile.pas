{ Execution test: a volatile variable (ref 3.15.1, p. 3-53; codegen-
  patterns 12.4 R7) is read again at each use.  EXEC_$COUNTER is a
  "device register" whose every word read returns the next value
  (tools/exectest/runner.c), so a loop that waits for it terminates only
  if it re-reads it, and two reads give two values.

  call: VOLATILE_$WAIT -> w
  call: VOLATILE_$TWICE
  expect:
    => 6
    0
    1
    5
}
module volatile;
%include 'exec.ins.pas';

var
  n, a, b: integer;

function volatile_$wait: integer;
begin
  exec_$counter := 0;
  n := 0;
  repeat
    n := n + 1;
  until exec_$counter >= 5;
  volatile_$wait := n;
end;

procedure volatile_$twice;
begin
  exec_$counter := 0;
  a := exec_$counter;
  exec_$logv_w(a);
  b := exec_$counter;
  exec_$logv_w(b);
  a := exec_$counter + exec_$counter;
  exec_$logv_w(a);
end;
