{ Execution test: `for' loops from 0 or 1 to a run-time upper bound (ref
  4.2 "For", p. 4-89: no iteration when the range is empty; codegen-
  patterns 7.4, "Run-time bounds"; bead domain-pascal-jnsb): from 0 to a
  variable (`tst.w; bmi', the count copied, with and without a call in
  the body), from 0 to `k - 1' and from 1 to `k - 1' (the constant folded
  into the count's `subq.w'), from 0 to `k - 13' (`subi.w #13'); each
  for an empty range, one iteration and several.

  call: FORRT_$UPTO w:-1
  call: FORRT_$UPTO w:0
  call: FORRT_$UPTO w:2
  call: FORRT_$SUM w:-1
  call: FORRT_$SUM w:0
  call: FORRT_$SUM w:3
  call: FORRT_$BELOW w:0
  call: FORRT_$BELOW w:1
  call: FORRT_$BELOW w:3
  call: FORRT_$INNER w:1
  call: FORRT_$INNER w:2
  call: FORRT_$INNER w:4
  call: FORRT_$FAR w:12
  call: FORRT_$FAR w:13
  call: FORRT_$FAR w:15
  expect:
    mark 99
    mark 0
    mark 99
    mark 0
    mark 1
    mark 2
    mark 99
    mark -1
    mark 0
    mark 0
    mark 0
    mark 3
    mark 6
    mark 99
    mark 0
    mark 99
    mark 0
    mark 1
    mark 2
    mark 99
    mark 99
    mark 1
    mark 99
    mark 1
    mark 2
    mark 3
    mark 99
    mark 99
    mark 0
    mark 99
    mark 0
    mark 1
    mark 2
    mark 99
}
module forrt;
%include 'exec.ins.pas';

var
  total: integer;
  pad1: integer32;
  last: integer;
  pad2: integer32;
  n: integer;

{ from 0 to a variable of the unit, a call in the body }
procedure forrt_$upto(k: integer); options(val_param);
var
  i: integer;
begin
  n := k;
  for i := 0 to n do
    exec_$mark(i);
  exec_$mark(99);
end;

{ from 0 to a variable, no call in the body: the count copied into the
  bound's own register }
procedure forrt_$sum(k: integer); options(val_param);
var
  i: integer;
begin
  n := k;
  total := 0;
  last := -1;
  for i := 0 to n do begin
    total := total + i;
    last := i;
  end;
  exec_$mark(last);
  exec_$mark(total);
end;

{ from 0 to `k - 1' }
procedure forrt_$below(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 0 to k - 1 do
    exec_$mark(i);
  exec_$mark(99);
end;

{ from 1 to `k - 1' }
procedure forrt_$inner(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 1 to k - 1 do
    exec_$mark(i);
  exec_$mark(99);
end;

{ from 0 to `k - 13' }
procedure forrt_$far(k: integer); options(val_param);
var
  i: integer;
begin
  for i := 0 to k - 13 do
    exec_$mark(i);
  exec_$mark(99);
end;
