{ Execution test (bead domain-pascal-5kgg): statements on the lowering's
  best-effort path.

  - a select `if a < b then a := b` that narrows the longword `b` (here
    reached through a pointer) into the word `a` (the comparison at 32
    bits, ref 4.3.1 p. 4-5; the store of `b`'s low word, in range here);
  - a `while` loop with an empty body (ref p. 4-213: the condition
    evaluated until false; the device counter reads 0, 1, 2, ...);
  - `with` over a volatile record reached through a pointer (ref
    3.15.1, p. 4-211);
  - a `for` loop with an empty body (ref p. 4-89: the bounds evaluated
    once, the statement after it reached).

  call: BE_L_STMTS_$SELECT w:5 l:7 -> w
  call: BE_L_STMTS_$SELECT w:5 l:3 -> w
  call: BE_L_STMTS_$SELECT w:-2 l:-1 -> w
  call: BE_L_STMTS_$WHILE
  call: BE_L_STMTS_$WITH w:4
  call: BE_L_STMTS_$FOR w:3
  call: BE_L_STMTS_$FOR w:0
  warns: best effort (not byte-identical): a select `if a < b then a := b` that narrows `b` into `a`
  warns: best effort (not byte-identical): a `while` loop with an empty body
  warns: best effort (not byte-identical): a `with` statement over a volatile record reached at run time
  warns: best effort (not byte-identical): a `for` loop with an empty body
  expect:
    => 7
    => 5
    => -1
    6
    4
    5
    9
    9
}
module be_l_stmts;
%include 'exec.ins.pas';
type
  vrec = record a, b: integer; end;
  vptr = ^vrec;
var
  vr: [volatile] vrec;
  vp: ^[volatile] vrec;
  gl: integer32;

function be_l_stmts_$select(a: integer; b: integer32): integer; options(val_param);
var x: integer; p: ^integer32;
begin
  gl := b;
  p := addr(gl);
  x := a;
  if x < p^ then x := p^;
  be_l_stmts_$select := x;
end;

procedure be_l_stmts_$while;
var i: integer;
begin
  exec_$counter := 0;
  while exec_$counter < 5 do ;
  i := exec_$counter;
  exec_$log_w(i);
end;

procedure be_l_stmts_$with(k: integer); options(val_param);
var i: integer;
begin
  vp := addr(vr);
  with vp^ do begin
    a := k;
    b := k + 1;
  end;
  i := vr.a;
  exec_$log_w(i);
  i := vr.b;
  exec_$log_w(i);
end;

procedure be_l_stmts_$for(n: integer); options(val_param);
var i: integer;
begin
  for i := 1 to n do ;
  exec_$log_w(9);
end;
