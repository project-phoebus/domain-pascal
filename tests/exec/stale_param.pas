{ Execution test: a by-value parameter whose address is passed to a
  callee must be re-read after the call (bead domain-pascal-pe9: `h := x;
  ext_$set(x); g := x' stored a stale register copy).  (dpc does not yet
  store such a parameter to a variable, so its uses here are arguments
  and comparisons.)

  call: STALE_PARAM_$MAIN w:5 l:70000
  call: STALE_PARAM_$MAIN w:15 l:-1
  call: STALE_PARAM_$NESTED w:3
  expect:
    5
    15
    70011
    26
    15
    25
    10
    mark 1
    36
    3
    14
}
module stale_param;
%include 'exec.ins.pas';

procedure incw(var v: integer);
var
  t: integer;
begin
  t := v;
  t := t + 10;
  v := t;
end;

procedure incl(var v: integer32);
var
  t: integer32;
begin
  t := v;
  t := t + 10;
  v := t;
end;

procedure stale_param_$main(a: integer; b: integer32); options(val_param);
begin
  exec_$logv_w(a);
  incw(a);
  exec_$logv_w(a);
  incl(b);
  exec_$logv_l(b + 1);
  incw(a);
  if a > 30 then exec_$mark(1) else exec_$clobber;
  exec_$logv_w(a + 1);
end;

procedure stale_param_$nested(a: integer); options(val_param);

  procedure setk;
  begin
    incw(a);
  end;

begin
  exec_$logv_w(a);
  setk;
  exec_$logv_w(a + 1);
end;
