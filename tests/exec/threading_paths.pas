{ Execution test: branch threading on more paths (codegen-patterns 7.1,
  "Branch threading"; bead domain-pascal-o6p).  `next', `exit',
  `return' and `goto' in an inner `else' part whose `bra' is dropped as
  no longer reached; `while' loops starting `else' parts, in loops and
  nested, and a `while' whose body is a `while'; a backward `goto'
  threaded; a function result through a dropped `bra'; `case' tables
  and chains ending `then' parts, with a `while' arm or `otherwise' part
  and a `return' arm; nested `if's with `or else'/`and then'; a call
  ending a `then' part left unpopped before the epilogue; threading in
  a nested routine.  The expected marks follow the Pascal semantics
  (checked against a Python model of the routines).

  call: THREADING_PATHS_$MAIN @w:0 @w:0 @w:0 @w:0
  call: THREADING_PATHS_$MAIN @w:0 @w:1 @w:1 @w:1
  call: THREADING_PATHS_$MAIN @w:1 @w:0 @w:0 @w:2
  call: THREADING_PATHS_$MAIN @w:1 @w:1 @w:1 @w:3
  call: THREADING_PATHS_$MAIN @w:1 @w:1 @w:0 @w:4
  call: THREADING_PATHS_$MAIN @w:1 @w:0 @w:1 @w:70
  call: THREADING_PATHS_$MAIN @w:0 @w:0 @w:3 @w:7
  call: THREADING_PATHS_$MAIN @w:1 @w:1 @w:3 @w:100
  call: THREADING_PATHS_$MAIN @w:0 @w:1 @w:0 @w:70
  call: THREADING_PATHS_$MAIN @w:1 @w:0 @w:3 @w:0
  call: THREADING_PATHS_$MAIN @w:0 @w:1 @w:3 @w:3
  call: THREADING_PATHS_$MAIN @w:1 @w:1 @w:1 @w:7
  expect:
    mark 39
    mark 39
    mark 0
    mark 13
    mark 26
    mark 39
    mark 0
    mark 103
    mark 1103
    mark 0
    mark 0
    mark 0
    mark 1
    mark 0
    mark 80
    mark 0
    mark 1
    mark 1
    mark 0
    mark 111
    mark 121
    mark 131
    mark 141
    mark 149
    mark 164
    mark 172
    mark 177
    mark 179
    mark 182
    mark 192
    mark 205
    mark 211
    mark 220
    mark 36
    mark 36
    mark 0
    mark 12
    mark 24
    mark 36
    mark 0
    mark 103
    mark 1103
    mark 1
    mark 6
    mark 0
    mark 60
    mark 0
    mark 80
    mark 1
    mark 93
    mark 1
    mark 0
    mark 112
    mark 120
    mark 130
    mark 141
    mark 149
    mark 150
    mark 164
    mark 172
    mark 177
    mark 179
    mark 182
    mark 192
    mark 202
    mark 212
    mark 220
    mark 39
    mark 39
    mark 0
    mark 13
    mark 26
    mark 39
    mark 0
    mark 1100
    mark 0
    mark 0
    mark 50
    mark 1
    mark 70
    mark 80
    mark 0
    mark 1
    mark 1
    mark 0
    mark 119
    mark 123
    mark 130
    mark 140
    mark 149
    mark 150
    mark 163
    mark 170
    mark 175
    mark 179
    mark 181
    mark 205
    mark 211
    mark 223
    mark 36
    mark 36
    mark 0
    mark 12
    mark 24
    mark 36
    mark 0
    mark 102
    mark 1102
    mark 1
    mark 6
    mark 50
    mark 60
    mark 70
    mark 81
    mark 1
    mark 93
    mark 2
    mark 1
    mark 119
    mark 123
    mark 130
    mark 141
    mark 149
    mark 150
    mark 161
    mark 171
    mark 176
    mark 179
    mark 202
    mark 212
    mark 221
    mark 0
    mark 0
    mark 2
    mark 1100
    mark 1
    mark 5
    mark 50
    mark 60
    mark 70
    mark 81
    mark 0
    mark 93
    mark 2
    mark 0
    mark 119
    mark 123
    mark 130
    mark 140
    mark 149
    mark 150
    mark 162
    mark 171
    mark 175
    mark 179
    mark 191
    mark 202
    mark 211
    mark 221
    mark 39
    mark 39
    mark 0
    mark 13
    mark 26
    mark 39
    mark 0
    mark 102
    mark 1102
    mark 0
    mark 0
    mark 50
    mark 1
    mark 70
    mark 80
    mark 0
    mark 1
    mark 1
    mark 1
    mark 119
    mark 123
    mark 132
    mark 140
    mark 149
    mark 150
    mark 163
    mark 171
    mark 175
    mark 179
    mark 181
    mark 191
    mark 205
    mark 212
    mark 223
    mark 39
    mark 39
    mark 0
    mark 13
    mark 26
    mark 39
    mark 0
    mark 103
    mark 1103
    mark 0
    mark 0
    mark 0
    mark 1
    mark 0
    mark 80
    mark 3
    mark 1
    mark 1
    mark 0
    mark 110
    mark 120
    mark 130
    mark 143
    mark 149
    mark 151
    mark 164
    mark 172
    mark 177
    mark 179
    mark 182
    mark 192
    mark 205
    mark 214
    mark 220
    mark 0
    mark 0
    mark 2
    mark 1100
    mark 1
    mark 8
    mark 50
    mark 60
    mark 70
    mark 81
    mark 3
    mark 93
    mark 2
    mark 3
    mark 119
    mark 123
    mark 130
    mark 143
    mark 149
    mark 152
    mark 162
    mark 171
    mark 175
    mark 179
    mark 191
    mark 202
    mark 214
    mark 221
    mark 0
    mark 0
    mark 2
    mark 103
    mark 1103
    mark 1
    mark 5
    mark 0
    mark 60
    mark 0
    mark 80
    mark 0
    mark 93
    mark 1
    mark 0
    mark 110
    mark 122
    mark 132
    mark 140
    mark 149
    mark 150
    mark 164
    mark 172
    mark 177
    mark 179
    mark 182
    mark 192
    mark 202
    mark 211
    mark 220
    mark 39
    mark 39
    mark 0
    mark 13
    mark 26
    mark 39
    mark 0
    mark 1100
    mark 0
    mark 0
    mark 50
    mark 1
    mark 70
    mark 80
    mark 0
    mark 1
    mark 1
    mark 3
    mark 119
    mark 123
    mark 131
    mark 141
    mark 149
    mark 163
    mark 170
    mark 175
    mark 179
    mark 181
    mark 191
    mark 205
    mark 214
    mark 223
    mark 0
    mark 0
    mark 2
    mark 103
    mark 1103
    mark 1
    mark 8
    mark 0
    mark 60
    mark 0
    mark 80
    mark 3
    mark 93
    mark 1
    mark 0
    mark 115
    mark 120
    mark 130
    mark 143
    mark 149
    mark 150
    mark 164
    mark 172
    mark 177
    mark 179
    mark 182
    mark 192
    mark 202
    mark 214
    mark 220
    mark 36
    mark 36
    mark 0
    mark 12
    mark 24
    mark 36
    mark 0
    mark 102
    mark 1102
    mark 1
    mark 6
    mark 50
    mark 60
    mark 70
    mark 81
    mark 1
    mark 93
    mark 2
    mark 1
    mark 119
    mark 123
    mark 130
    mark 141
    mark 149
    mark 151
    mark 161
    mark 171
    mark 176
    mark 179
    mark 202
    mark 212
    mark 221
}
module threading_paths;
%include 'exec.ins.pas';

var
  a, n, b, r, c, pad, w: integer;

{ `next' in an inner `else' part: the inner test's branch goes to the
  loop's test, and the `next''s own `bra', no longer reached, and the
  outer `bra' after it are dropped. }
procedure next_inner_else;
begin
  n := 3;
  r := 0;
  while n > 0 do begin
    n := n - 1;
    if b = 1 then begin if c = 1 then r := r + 2 else next end
    else r := r + 3;
    r := r + 10;
  end;
  exec_$mark(r);
end;

{ The same with `exit' and with `return'. }
procedure exit_inner_else;
begin
  n := 3;
  r := 0;
  while n > 0 do begin
    n := n - 1;
    if b = 1 then begin if c = 1 then r := r + 2 else exit end
    else r := r + 3;
    r := r + 10;
  end;
  exec_$mark(r);
  exec_$mark(n);
end;

procedure return_inner_else;
begin
  n := 3;
  r := 0;
  while n > 0 do begin
    n := n - 1;
    if b = 1 then begin if c = 1 then r := r + 2 else return end
    else r := r + 3;
    r := r + 10;
    exec_$mark(r);
  end;
  exec_$mark(n);
end;

{ ... and with a `goto' past the next statement. }
procedure goto_inner_else;
label l;
begin
  r := 100;
  if a = 1 then begin if c = 1 then r := 102 else goto l end
  else r := 103;
  exec_$mark(r);
l:
  exec_$mark(r + 1000);
end;

{ `exit' as a `then' part, a `while' as the `else' part, in a loop. }
procedure exit_else_while;
begin
  n := 2;
  r := c + 5;
  while n > 0 do begin
    n := n - 1;
    if b = 1 then exit else while r > 0 do r := r - 1;
  end;
  exec_$mark(n);
  exec_$mark(r);
end;

{ Two `else' parts that start with a `while'; a `while' whose body is
  a `while'. }
procedure else_whiles;
begin
  r := c;
  n := c + 2;
  if a = 1 then r := 50 else while r > 0 do r := r - 1;
  if b = 1 then n := 60 else while n > 1 do n := n - 1;
  exec_$mark(r);
  exec_$mark(n);
  r := c;
  if a = 1 then r := 70 else while r > 0 do while r > 0 do r := r - 1;
  exec_$mark(r);
end;

{ A `goto' to a label that the `then' part's `bra' follows, the `else'
  part a `while'. }
procedure goto_else_while;
label l;
begin
  r := c;
  n := 80;
  if a = 0 then goto l;
  if b = 1 then begin n := 81; l: end else while r > 0 do r := r - 1;
  exec_$mark(n);
  exec_$mark(r);
end;

{ A backward `goto' in an inner `if'. }
procedure back_goto;
label l;
begin
  n := 3;
  r := 0;
l:
  n := n - 1;
  r := r + 1;
  if n > 0 then begin if b = 1 then goto l end else r := r + 90;
  exec_$mark(r);
end;

{ A function result held in D0 through a threaded branch and a dropped
  `bra'. }
function fres: integer;
begin
  fres := 1;
  r := c;
  if a = 1 then begin if b = 1 then fres := 2 else return end
  else while r > 0 do r := r - 1;
end;

{ `case' statements: a table and a compare chain ending a `then' part,
  a `while' as an arm and as the `otherwise' part, an arm of `return'
  in a chain. }
procedure cases;
begin
  r := 0;
  if a = 0 then case w of 0: r := 1; 1: r := 2; 2: r := 3; 3: r := 5 end
  else r := 9;
  exec_$mark(r + 110);
  r := 0;
  if a = 0 then case w of 0: r := 1; 70: r := 2; end else r := 3;
  exec_$mark(r + 120);
  r := c;
  case w of 0: r := 1; 70: r := 2; otherwise while r > 0 do r := r - 1 end;
  exec_$mark(r + 130);
  r := c;
  case w of 0: r := 1; 70: while r > 0 do r := r - 1; end;
  exec_$mark(r + 140);
end;

procedure case_return;
begin
  exec_$mark(149);
  r := 150;
  case w of 0: return; 7: r := 151; 100: r := 152; end;
  exec_$mark(r);
end;

{ Three nested `if's; `or else' and `and then' ending a `then' part; an
  inner `then' part of `return'. }
procedure ifs;
begin
  if a = 1 then if b = 1 then if c = 1 then r := 1 else r := 2 else r := 3
  else r := 4;
  exec_$mark(r + 160);
  r := 170;
  if a = 1 then begin if (b = 1) or else (c = 1) then r := 171 end
  else r := 172;
  exec_$mark(r);
  r := 175;
  if a = 1 then begin if (b = 1) and then (c = 1) then r := 176 end
  else r := 177;
  exec_$mark(r);
end;

procedure if_return;
begin
  exec_$mark(179);
  r := 180;
  if a = 1 then if b = 1 then return else r := 181 else r := 182;
  exec_$mark(r);
end;

{ A call ending a `then' part before the epilogue: not popped. }
procedure unpopped;
begin
  if a = 1 then begin
    r := 190;
    if b <> c then exec_$mark(191);
  end
  else exec_$mark(192);
end;

{ An `else' part that starts with a `while' inside a loop. }
procedure loop_else_while;
begin
  n := 2;
  r := 0;
  while n > 0 do begin
    n := n - 1;
    if b = 1 then r := r + 1 else while r < 5 do r := r + 1;
  end;
  exec_$mark(r + 200);
end;

{ A `while' whose body is a `while': the outer opening `bra' goes to
  the outer test, the outer back edge to the inner test, and the inner
  `bra', no longer reached, is dropped. }
procedure while_while;
begin
  n := c + 1;
  r := 0;
  while n > 0 do while n > 0 do begin n := n - 1; r := r + 1 end;
  exec_$mark(r + 210);
end;

{ Threading inside a nested routine, which reaches its parent's local. }
procedure outer;
var loc: integer;
  procedure inner;
  begin
    if a = 1 then begin if b = 1 then loc := 221 end
    else while loc > 220 do loc := loc - 1;
  end;
begin
  loc := 223;
  inner;
  exec_$mark(loc);
end;

procedure threading_paths_$main(in x, y, z, v: integer);
begin
  a := x;
  b := y;
  c := z;
  w := v;
  next_inner_else;
  exit_inner_else;
  return_inner_else;
  goto_inner_else;
  exit_else_while;
  else_whiles;
  goto_else_while;
  back_goto;
  pad := fres;
  exec_$mark(pad);
  exec_$mark(r);
  cases;
  case_return;
  ifs;
  if_return;
  unpopped;
  loop_else_while;
  while_while;
  outer;
end;
