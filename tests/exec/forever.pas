{ Execution test: loops with no test, `repeat ... until false' (ref
  "Repeat/Until", p. 4-161; the `close' example, p. 4-42; codegen-patterns
  7.2),
  left by `exit' in the middle, by an `if ... then exit' that ends the
  body (the branch back on the negated condition), by an `exit' that ends
  it (no `bra' back), from inside an `if' and a `case' arm, by `return'
  and by `goto'; `next' to the head; nested.  Review of bead
  domain-pascal-pkle: functions left by `return' (the epilogue still
  emitted, the result set), a `return' in a `case' arm, a `goto' to a
  label that ends the routine, an inner loop left by `exit' and the outer
  one by `return'.

  call: FOREVER_$MID w:3
  call: FOREVER_$LAST_IF w:2
  call: FOREVER_$LAST_EXIT w:4
  call: FOREVER_$INNER w:3
  call: FOREVER_$ARMS w:5
  call: FOREVER_$RET w:2
  call: FOREVER_$GOTO w:3
  call: FOREVER_$NESTED w:2
  call: FOREVER_$FRET w:3 -> w
  call: FOREVER_$FRET_LAST w:2 -> w
  call: FOREVER_$ARM_RET w:2
  call: FOREVER_$GOTO_END w:2
  call: FOREVER_$NEST_RET w:3
  expect:
    mark 1
    mark 2
    mark 3
    mark 100
    mark 1
    mark 2
    mark 100
    mark 4
    mark 3
    mark 2
    mark 1
    mark 100
    mark 1
    mark 3
    mark 6
    mark 100
    mark 1
    mark 2
    mark 3
    mark 4
    mark 5
    mark -5
    mark 100
    mark 1
    mark 2
    mark 1
    mark 2
    mark 3
    mark 100
    mark 1
    mark 10
    mark 2
    mark 20
    mark 100
    mark 1
    mark 2
    mark 3
    => 30
    mark 1
    mark 2
    => 2
    mark 1
    mark 2
    mark -2
    mark 1
    mark 2
    mark 1
    mark 1
    mark 2
    mark 1
    mark 2
    mark 3
}
module forever;
%include 'exec.ins.pas';

var
  n: integer;
  lim: integer;
  j: integer;

{ `exit' in the middle of the body. }
procedure forever_$mid(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    if n > lim then exit;
    exec_$mark(n);
  until false;
  exec_$mark(100);
end;

{ An `if ... then exit' ends the body. }
procedure forever_$last_if(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    if n = lim then exit;
  until false;
  exec_$mark(100);
end;

{ An `exit' ends the body; `next' goes back to the head. }
procedure forever_$last_exit(k: integer); options(val_param);
begin
  n := k;
  repeat
    exec_$mark(n);
    n := n - 1;
    if n > 0 then next;
    exit;
  until false;
  exec_$mark(100);
end;

{ The `exit' inside an `if' at the end of the body. }
procedure forever_$inner(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    if odd(n) then begin
      exec_$mark(n);
      if n >= lim then exit;
    end;
  until false;
  exec_$mark(n * 2);
  exec_$mark(100);
end;

{ The `exit's in `case' arms, the last one ending the body. }
procedure forever_$arms(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    j := n - lim;
    case j of
      0: begin
           exec_$mark(-n);
           exit;
         end;
      1: exit;
    end;
  until false;
  exec_$mark(100);
end;

{ Left by `return': no code after the loop is reached. }
procedure forever_$ret(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    if n = lim then return;
  until false;
end;

{ Left by `goto'. }
procedure forever_$goto(k: integer); options(val_param);
label
  9;
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    if n = lim then goto 9;
  until false;
9:
  exec_$mark(100);
end;

{ A loop with no test inside another. }
procedure forever_$nested(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    j := 0;
    repeat
      j := j + 1;
      if j >= n then exit;
    until false;
    exec_$mark(j * 10);
    if n = lim then exit;
  until false;
  exec_$mark(100);
end;

{ A function left by `return' from the loop. }
function forever_$fret(k: integer): integer; options(val_param);
var
  i: integer;
begin
  lim := k;
  i := 0;
  repeat
    i := i + 1;
    exec_$mark(i);
    if i = lim then begin
      forever_$fret := i * 10;
      return;
    end;
  until false;
end;

{ An `if ... then return' ends the body of a function. }
function forever_$fret_last(k: integer): integer; options(val_param);
var
  i: integer;
begin
  lim := k;
  i := 0;
  repeat
    i := i + 1;
    exec_$mark(i);
    forever_$fret_last := i;
    if i = lim then return;
  until false;
end;

{ `return' from a `case' arm, `next' from another. }
procedure forever_$arm_ret(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    j := n - lim;
    case j of
      0: begin
           exec_$mark(-n);
           return;
         end;
      -1: next;
    end;
    exec_$mark(1000);
  until false;
end;

{ `goto' a label that ends the routine. }
procedure forever_$goto_end(k: integer); options(val_param);
label
  9;
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    exec_$mark(n);
    if n = lim then goto 9;
  until false;
9:
end;

{ The inner loop left by `exit', the outer by `return'. }
procedure forever_$nest_ret(k: integer); options(val_param);
begin
  lim := k;
  n := 0;
  repeat
    n := n + 1;
    j := 0;
    repeat
      j := j + 1;
      exec_$mark(j);
      if j >= n then exit;
    until false;
    if n = lim then return;
  until false;
end;
