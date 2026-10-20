{ Execution test (bead domain-pascal-5kgg): best-effort register choices
  (codegen-patterns 12.4): by-address parameters loaded once after a
  branch or a call into whatever register the colouring gives (R4(b)),
  values held through calls whose arguments are their last use in bodies
  with branches (R4(c): a register-held local, a parameter loaded at
  entry), and tests on the flags of a move (a load compared twice, a
  function's result copied and tested, codegen-patterns 7.1).  The
  runner checks that D2-D7 and A2-A6 come back.

  call: BX_REGS_$LOCAL w:10 b:255
  call: BX_REGS_$LOCAL w:10 b:0
  call: BX_REGS_$LATE @w:4 @w:0! @w:0!
  call: BX_REGS_$LATE @w:-4 @w:0! @w:0!
  call: BX_REGS_$SHOW
  call: BX_REGS_$LATE3 @w:5 @l:0! @l:0!
  call: BX_REGS_$LATE3 @w:1 @l:0! @l:0!
  call: BX_REGS_$LATE1 @w:9! @w:0! @w:3
  call: BX_REGS_$LATE1 @w:9! @w:0! @w:0
  call: BX_REGS_$LATE4 @w:0! @w:0! @w:0! @w:4
  call: BX_REGS_$LATE4 @w:7! @w:0! @w:5! @w:0
  call: BX_REGS_$TWICE @w:0
  call: BX_REGS_$TWICE @w:300
  call: BX_REGS_$TWICE @w:7
  call: BX_REGS_$CALL @w:0
  call: BX_REGS_$CALL @w:3
  warns: best effort (not byte-identical): in a body with branches, a call whose arguments are the last use of a register-held local
  warns: best effort (not byte-identical): in a body of several statements, a pointer parameter or by-address parameter used once after a branch or a call
  warns: best effort (not byte-identical): a local assigned a load and compared twice right after
  warns: best effort (not byte-identical): a function's result assigned to a local in a scratch register and tested next
  expect:
    10
    out 5
    out 6
    out -3
    out -2
    4
    -4
    mark 1
    out 5
    out 6
    out 0
    out 6
    out 3
    out 70
    out 9
    out 70
    out 4
    out 1
    out 0
    out 0
    out 1
    out 2
    1
    1
    0
    1
    0
}
module be_x_regs;
%include 'exec.ins.pas';

type
  rec = record a, b: integer32; end;

var
  g: integer;
  done: boolean;
  src: integer;
  r: rec;
  p: ^rec;
  seen, gw: integer;
  pw: ^integer;
  words: array[1..4] of integer;

procedure bx_regs_$local(v: integer; d: boolean); options(val_param);
var k: integer;
begin
  src := v;
  done := d;
  k := src;
  if done then begin
    src := k + 1;
    exec_$logv_w(k);
  end;
end;

procedure bx_regs_$late(in n: integer; out x: integer; out y: integer);
begin
  p := addr(r);
  if n > 0 then
    p^.a := n
  else
    g := n;
  x := n + 1;
  y := n + 2;
end;

procedure bx_regs_$show;
begin
  exec_$log_l(r.a);
  exec_$log_w(g);
end;

procedure bx_regs_$late3(in n: integer; out x: integer32; out y: integer32);
begin
  x := 0;
  if n > 3 then begin
    exec_$mark(1);
    x := 5;
  end;
  y := 6;
end;

procedure bx_regs_$late1(var a: integer; var b: integer; in c: integer);
begin
  words[2] := 70;
  if c > 0 then a := c;
  b := words[2];
end;

procedure bx_regs_$late4(var a, b, d: integer; in c: integer);
begin
  a := c;
  exec_$clobber;
  b := 1;
  if c = 0 then d := 2;
end;

procedure bx_regs_$twice(in v: integer);
var x: integer;
begin
  pw := addr(v);
  seen := 0;
  x := pw^;
  if (x = 0) or else (x > 255) then seen := 1;
  gw := x;
  exec_$log_w(seen);
end;

procedure bx_regs_$call(in v: integer);
var x: integer;
begin
  seen := 0;
  x := exec_$id_w(v);
  if x = 0 then seen := 1;
  gw := x;
  exec_$log_w(seen);
end;
