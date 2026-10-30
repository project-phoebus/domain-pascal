{ Execution test (bead domain-pascal-5kgg): best-effort paths of control
  flow whose image form is not settled: statements after `return' and
  `exit' (not reached, compiled anyway), `exit'/`next' that branch to the
  next instruction, an `if' whose two parts end in calls with the same
  argument bytes (each part pops its own), `if v > e then v := e' (the
  value computed twice), empty procedures (a bare `rts'), an empty
  `repeat', and `for' loops with a run-time bound and no call in the body
  (the count copied wherever the colouring puts it; codegen-patterns
  7.4, bead domain-pascal-fbah).  Pascal semantics: ref 4.x (`for' runs
  from the first bound to the last, not at all when the last is smaller).

  call: BX_CTL_$RET @w:3
  call: BX_CTL_$RET @w:0
  call: BX_CTL_$EXIT @w:4
  call: BX_CTL_$EXIT @w:20
  call: BX_CTL_$SAMEPOP @w:5
  call: BX_CTL_$SAMEPOP @w:-2
  call: BX_CTL_$CLAMP @l:10 @l:3
  call: BX_CTL_$CLAMP @l:2 @l:3
  call: BX_CTL_$EMPTY1 @w:4
  call: BX_CTL_$EMPTY2
  call: BX_CTL_$SPIN
  call: BX_CTL_$NEXTEND @w:20
  call: BX_CTL_$EXITEND
  call: BX_CTL_$SUM w:10
  call: BX_CTL_$SUM w:0
  call: BX_CTL_$FILL w:5 w:100
  call: BX_CTL_$RUN w:4
  call: BX_CTL_$TWICE w:6
  call: BX_CTL_$TWICE w:-1
  warns: best effort (not byte-identical): a statement after `return` (not reached)
  warns: best effort (not byte-identical): a statement after `exit` or `next` (not reached)
  warns: best effort (not byte-identical): an `if` comparing a variable with a computed value whose `then` part assigns
  warns: best effort (not byte-identical): an empty procedure with a value parameter passed by address
  warns: best effort (not byte-identical): an empty procedure with locals
  warns: best effort (not byte-identical): an empty `repeat` loop
  warns: best effort (not byte-identical): an `exit` or `next` that would branch to the next instruction
  warns: best effort (not byte-identical): a `for` loop with a run-time bound and no call in its body
  expect:
    3
    mark 2
    4
    10
    5
    mark 7
    mark -2
    mark 7
    4
    2
    mark 5
    210
    35
    55
    0
    105
    13
    12
    0
}
module be_x_ctl;
%include 'exec.ins.pas';

var
  g: integer;
  h: integer32;
  a: array[1..50] of integer;

procedure bx_ctl_$ret(in n: integer);
begin
  if n > 0 then begin
    exec_$log_w(n);
    return;
    g := 5;
  end;
  exec_$mark(2);
end;

procedure bx_ctl_$exit(in n: integer);
var i: integer;
begin
  i := 0;
  while i < 10 do begin
    i := i + 1;
    if i = n then begin
      exit;
      g := 7;
    end;
  end;
  exec_$log_w(i);
end;

procedure bx_ctl_$samepop(in n: integer);
begin
  if n > 0 then
    exec_$log_w(n)
  else
    exec_$mark(n);
  exec_$mark(7);
end;

procedure bx_ctl_$clamp(in a, b: integer32);
begin
  h := a;
  if h > b + 1 then
    h := b + 1;
  exec_$log_l(h);
end;

procedure bx_ctl_$empty1(n: integer);
begin
end;

procedure bx_ctl_$empty2;
var i: integer;
begin
end;

procedure bx_ctl_$spin;
begin
  repeat until exec_$id_bool(true);
  exec_$mark(5);
end;

procedure bx_ctl_$nextend(in n: integer);
var i: integer;
begin
  g := 0;
  for i := 1 to n do begin
    g := g + i;
    if g > 100 then next;
  end;
  exec_$log_w(g);
end;

procedure bx_ctl_$exitend;
begin
  g := 0;
  while g < 50 do begin
    g := g + 7;
    if g > 30 then exit;
  end;
  exec_$log_w(g);
end;

procedure bx_ctl_$sum(n: integer); options(val_param);
var i: integer;
begin
  g := 0;
  for i := 1 to n do
    g := g + i;
  exec_$log_w(g);
end;

procedure bx_ctl_$fill(n, m: integer); options(val_param);
var i: integer;
begin
  g := m;
  for i := 1 to n do
    a[i] := g + i;
  exec_$log_w(a[n]);
end;

procedure bx_ctl_$run(n: integer); options(val_param);
var i, k: integer;
begin
  k := 3;
  for i := 1 to n do begin
    k := k + i;
    g := k;
  end;
  exec_$log_w(g);
end;

procedure bx_ctl_$twice(n: integer); options(val_param);
var i: integer;
begin
  g := 0;
  for i := 1 to n do
    g := g + 2;
  exec_$log_w(g);
end;
