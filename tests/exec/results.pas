{ Execution test: function results (codegen-patterns 3): a word result
  read from D0 whose high half is junk (the stubs leave junk above the
  result), a boolean result from D0.b, a pointer in A0, a result held
  across a later call, `f := g(...)'.

  call: RESULTS_$MAIN w:7
  call: RESULTS_$WRAP w:-3 -> w
  call: RESULTS_$BOOL w:5 -> b
  call: RESULTS_$BOOL w:-5 -> b
  call: RESULTS_$HELD w:9 -> w
  expect:
    7
    70007
    8
    true
    false
    42
    => -3
    => true
    => false
    mark 1
    => 9
}
module results;
%include 'exec.ins.pas';

type
  intp = ^integer;
var
  gw: integer;
  gl: integer32;
  gb: boolean;
  gp: intp;
  cell: integer;
  t: boolean;

procedure results_$main(a: integer); options(val_param);
begin
  gw := exec_$id_w(a);
  exec_$logv_w(gw);
  gl := 70000;
  gl := exec_$id_l(gl);
  gl := gl + gw;
  exec_$logv_l(gl);
  gl := exec_$id_w(a);
  gl := gl + 1;
  exec_$logv_l(gl);
  t := true;
  gb := exec_$id_bool(t);
  exec_$log_bool(gb);
  t := false;
  gb := exec_$id_bool(t);
  exec_$log_bool(gb);
  cell := 42;
  gp := exec_$id_p(addr(cell));
  gw := gp^;
  exec_$logv_w(gw);
end;

function results_$wrap(a: integer): integer; options(val_param);
begin
  results_$wrap := exec_$id_w(a);
end;

function positive(a: integer): boolean; options(val_param);
begin
  positive := a > 0;
end;

function results_$bool(a: integer): boolean; options(val_param);
begin
  results_$bool := positive(a);
end;

function results_$held(a: integer): integer; options(val_param);
begin
  results_$held := exec_$id_w(a);
  exec_$mark(1);
end;
