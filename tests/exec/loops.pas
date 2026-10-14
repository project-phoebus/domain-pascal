{ Execution test: loops (ref 4.2 "For", p. 4-89; codegen-patterns 7):
  `for' with constant and run-time bounds (a `dbf' count, guarded when
  only known at run time: no iteration when the range is empty),
  `downto', negative bounds, `while' and `repeat', `exit' and `next'
  (ref p. 4-77, 4-123).

  call: LOOPS_$UPTO w:3
  call: LOOPS_$UPTO w:0
  call: LOOPS_$UPTO w:-2
  call: LOOPS_$DOWN
  call: LOOPS_$CONST
  call: LOOPS_$EXIT_NEXT w:5
  call: LOOPS_$WHILE w:0
  call: LOOPS_$WHILE w:3
  expect:
    mark 1
    mark 2
    mark 3
    mark 0
    mark 0
    mark 0
    mark 2
    mark 1
    mark 0
    mark -1
    mark 9
    mark -2
    mark -1
    mark 0
    mark 5
    mark 1
    mark 3
    mark 5
    mark 100
    mark 0
    mark 3
}
module loops;
%include 'exec.ins.pas';

var
  total: integer;
  pad: integer32;
  n: integer;

procedure loops_$upto(k: integer); options(val_param);
var
  i: integer;
begin
  n := 0;
  for i := 1 to k do
    exec_$mark(i);
  exec_$mark(n);
end;

procedure loops_$down;
var
  i: integer;
begin
  for i := 2 downto -1 do
    exec_$mark(i);
  exec_$mark(9);
end;

procedure loops_$const;
var
  i: integer;
begin
  for i := -2 to 0 do
    exec_$mark(i);
  exec_$mark(5);
end;

procedure loops_$exit_next(k: integer); options(val_param);
var
  i, j: integer;
begin
  n := k;
  for i := 1 to 10 do begin
    j := i & 1;
    if j <> 0 then begin
      exec_$mark(i);
      if i >= n then exit;
      next;
    end;
    total := i;
  end;
  exec_$mark(100);
end;

procedure loops_$while(k: integer); options(val_param);
begin
  n := k;
  total := 0;
  while n > 0 do begin
    total := total + 1;
    n := n - 1;
  end;
  exec_$mark(total);
end;
