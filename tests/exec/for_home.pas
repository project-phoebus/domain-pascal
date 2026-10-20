{ Execution test: `for' indices kept in memory (ref "For", p. 4-89:
  "Domain Pascal permits the index_variable to be declared in a scope
  other than the scope of the routine immediately containing the for
  loop"; codegen-patterns 7.4, "An index kept in memory"; bead
  domain-pascal-1mtr): a unit variable (word, longword, char), the
  enclosing routine's local looped over by a nested routine (RINGLOG_$CNTL's
  0xE721CC), a local that a nested routine called in the body reads, a
  `var' parameter.  The body's calls see the index's current value; after
  the loop the variable holds one step past the last value (or the value
  at an `exit'); `next' still steps the index; the bounds are evaluated
  before the variable is set (`for g := f(lo) to g'); two nested loops
  over unit variables, and over the parent's i and j from a nested
  routine; empty ranges run no pass
  and leave the variable as it was (the store after the guard, as
  RINGLOG_$CNTL's nested 0xE721EC `bmi' before 0xE721F0 `move.w
  #1,(-2,A2)'; ISO 7185 6.8.3.9 assigns it only when the loop runs).

  call: FORHM_$GLOBAL w:1 w:3
  call: FORHM_$GLOBAL w:4 w:3
  call: FORHM_$LONG l:69998 l:70001
  call: FORHM_$CHARS
  call: FORHM_$NESTED w:3
  call: FORHM_$NESTED w:0
  call: FORHM_$PARENT
  call: FORHM_$VARPARAM @w:0!
  call: FORHM_$SKIP
  call: FORHM_$TOHOME w:3
  call: FORHM_$TOHOME w:11
  call: FORHM_$DOWNHOME w:7
  call: FORHM_$GRID
  call: FORHM_$NESTGRID w:2 w:3
  expect:
    mark 1
    mark 2
    mark 3
    mark 3
    mark 2
    mark 1
    mark 0
    mark 0
    mark 0
    mark 99
    69998
    69999
    70000
    70001
    70002
    3
    120
    121
    122
    123
    mark 1
    mark 2
    mark 3
    mark 4
    mark 5
    mark 10
    mark 20
    mark 30
    mark 4
    mark 5
    mark 4
    mark 3
    out 2
    mark 1
    mark 2
    mark 4
    mark 6
    mark 1
    mark 2
    mark 3
    mark 4
    mark 5
    mark 6
    mark 7
    mark 8
    mark 9
    mark 10
    mark 99
    mark 99
    mark 7
    mark 6
    mark 5
    mark 99
    mark 11
    mark 12
    mark 13
    mark 21
    mark 22
    mark 23
    mark 3
    mark 4
    mark 11
    mark 12
    mark 13
    mark 21
    mark 22
    mark 23
    mark 3
    mark 4
}
module forhm;
%include 'exec.ins.pas';

var
  g: integer;
  gl: integer32;
  gc: char;
  cnt: integer32;

procedure forhm_$show;
begin
  exec_$mark(g);
end;

procedure forhm_$showl;
begin
  exec_$logv_l(gl);
end;

procedure forhm_$showc;
begin
  exec_$logv_c(gc);
end;

{ a unit variable, run-time bounds both ways, read by a call in the body }
procedure forhm_$global(lo, hi: integer); options(val_param);
begin
  for g := lo to hi do
    forhm_$show;
  for g := hi downto lo do
    forhm_$show;
  forhm_$show;
  if lo <= hi then
    exec_$mark(g)
  else
    exec_$mark(99);
end;

{ a longword unit variable }
procedure forhm_$long(lo, hi: integer32); options(val_param);
begin
  for gl := lo to hi do
    forhm_$showl;
  forhm_$showl;
end;

{ a char unit variable to its last value, then constant bounds }
procedure forhm_$chars;
begin
  cnt := 0;
  for gc := chr(0) to chr(255) do
    cnt := cnt + 1;
  if cnt = 256 then
    exec_$logv_l(3);
  for gc := 'x' to 'z' do
    forhm_$showc;
  forhm_$showc;
end;

{ a nested routine's loop over the enclosing routine's local }
procedure forhm_$nested(n: integer); options(val_param);
var
  i: integer;

  procedure inner;
  begin
    for i := 1 to n do
      exec_$mark(i);
  end;

begin
  i := 5;
  inner;
  exec_$mark(i);
end;

{ a local read by a nested routine called in the loop }
procedure forhm_$parent;
var
  i: integer;

  procedure show;
  begin
    exec_$mark(i * 10);
  end;

begin
  for i := 1 to 3 do
    show;
  exec_$mark(i);
end;

{ a `var' parameter as the index; `exit' leaves the value }
procedure forhm_$varparam(var k: integer);
begin
  for k := 5 downto 3 do
    exec_$mark(k);
  for k := 1 to 10 do
    if k = 2 then
      exit;
end;

{ `next' in the body still steps the index }
procedure forhm_$skip;
begin
  for g := 1 to 4 do begin
    if g = 3 then
      next;
    forhm_$show;
  end;
  for g := 6 downto 1 do begin
    if g = 5 then
      exit;
    forhm_$show;
  end;
  for g := 1 to 2 do
    forhm_$show;
end;

{ the bounds evaluated before the index is set: `to' reads g's old value }
procedure forhm_$tohome(lo: integer); options(val_param);
begin
  g := 10;
  for g := exec_$id_w(lo) to g do
    exec_$mark(g);
  exec_$mark(99);
end;

{ the same downwards: `from' reads g's old value, `to' a call }
procedure forhm_$downhome(lo: integer); options(val_param);
begin
  g := lo;
  for g := g downto exec_$id_w(5) do
    exec_$mark(g);
  exec_$mark(99);
end;

var
  h: integer;

{ two nested loops over unit variables; h and g read after them }
procedure forhm_$grid;
begin
  for g := 1 to 2 do
    for h := 1 to 3 do
      exec_$mark(g * 10 + h);
  exec_$mark(g);
  exec_$mark(h);
end;

{ a nested routine's two nested loops over the parent's i and j }
procedure forhm_$nestgrid(n, m: integer); options(val_param);
var
  i, j: integer;

  procedure grid;
  begin
    for i := 1 to n do
      for j := 1 to m do
        exec_$mark(i * 10 + j);
  end;

begin
  grid;
  exec_$mark(i);
  exec_$mark(j);
end;
