{ Execution test: a stored local's value reused from a register
  (stored-local forwarding, codegen-patterns 12.4) must be the stored one,
  and not survive a call that may change the local (its address passed).

  call: FORWARD_$MAIN w:4
  call: FORWARD_$MAIN w:-2
  call: FORWARD_$BOOL w:11
  call: FORWARD_$BOOL w:10
  expect:
    12
    13
    851968
    27
    27
    -6
    -5
    -327680
    -9
    -9
    true
    false
    false
    true
}
module forward;
%include 'exec.ins.pas';

procedure twice_plus_one(var v: integer);
var
  t: integer;
begin
  t := v;
  t := t + t;
  t := t + 1;
  v := t;
end;

procedure forward_$main(a: integer); options(val_param);
var
  x: integer;
  y: integer32;
begin
  x := a;
  x := x * 3;
  exec_$log_w(x);
  x := x + 1;
  exec_$logv_w(x);
  y := x;
  y := y * 65536;
  exec_$logv_l(y);
  twice_plus_one(x);
  exec_$logv_w(x);
  exec_$log_w(x);
end;

procedure forward_$bool(a: integer); options(val_param);
var
  b: boolean;
begin
  if a > 10 then b := true else b := false;
  exec_$log_bool(b);
  if b then b := false else b := true;
  exec_$log_bool(b);
end;
