{ Execution test: nested routines (ref 2.4; codegen-patterns 6): uplevel
  reads and writes of the enclosing routine's parameters and locals, a
  nested routine called by a sibling, recursion with the static link kept, a nested function's
  result.  (dpc does not yet nest a routine in a nested routine.)

  call: NESTED_$UP w:5 @w:7!
  call: NESTED_$TWO w:2
  call: NESTED_$REC w:4 -> w
  expect:
    13
    105
    out 113
    mark 3
    mark 4
    mark 7
    mark 8
    mark 4
    mark 3
    mark 2
    mark 1
    mark 0
    4
    => 10
}
module nested;
%include 'exec.ins.pas';

var
  gj: integer;

procedure nested_$up(v: integer; var cnt: integer); options(val_param);
var
  k: integer;

  function sum(x: integer): integer;
  begin
    sum := x + v + k;
  end;

  procedure put;
  begin
    k := 100;
  end;

begin
  k := cnt;
  gj := sum(1);
  exec_$logv_w(gj);
  put;
  k := sum(0);
  exec_$logv_w(k);
  cnt := sum(3);
end;

procedure nested_$two(v: integer); options(val_param);
var
  a: integer;

  procedure mid;
  var
    b: integer;
  begin
    b := 1;
    a := a + b;
    exec_$mark(a);
    exec_$mark(b + a);
  end;

  procedure sib;
  begin
    a := a * 2;
    mid;
  end;

begin
  a := v;
  mid;
  sib;
end;

function nested_$rec(n: integer): integer; options(val_param);
var
  total: integer;
  depth: integer;

  procedure walk(i: integer);
  begin
    exec_$mark(i);
    if i > 0 then begin
      total := total + i;
      walk(i - 1);
      depth := depth + 1;
    end;
  end;

begin
  total := 0;
  exec_$clobber;
  depth := 0;
  walk(n);
  exec_$logv_w(depth);
  nested_$rec := total;
end;
