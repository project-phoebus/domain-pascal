{ Execution test: parameters loaded at entry whose last use on some path
  is an argument of a call, in bodies with branches, held through the
  call in callee-saved registers (codegen-patterns 12.4 R4(c); bead
  domain-pascal-9j1z; golden regalloc/args_held_branches): the call in an
  `if' part (AREA_$ASSOC 0xE096D2), a call in each part (XPD_$GET_FP_INT
  0xE5C574/0xE5C57E), the parameter also used on the other path
  (SIO_$I_INHIBIT_XMIT 0xE1C9EC), the call after a join inside an `if'
  part (SOCK_$PUT_INT 0xE161E4), the call before the first branch
  (VOLX_$GET_INFO).  The runner checks that D2-D7 and A2-A6 come back.
  Review of the bead: a parameter held through a call in a `case' arm
  (D2; TERM_$INQUIRE 0xE66D98 holds A2 so), through a call followed by
  `return', and parameters held through the first of several calls in
  an `if' part while the others are live across it (D2-D5; A2-A4, a
  different one held first in each part).

  call: ARGS_HELD_$THEN w:7 b:255
  call: ARGS_HELD_$THEN w:8 b:0
  call: ARGS_HELD_$SHOW
  call: ARGS_HELD_$PARTS @w:3 @b:255
  call: ARGS_HELD_$PARTS @w:4 @b:0
  call: ARGS_HELD_$OTHER @w:5 @b:255
  call: ARGS_HELD_$OTHER @w:6 @b:0
  call: ARGS_HELD_$JOIN @w:2
  call: ARGS_HELD_$JOIN @w:-2
  call: ARGS_HELD_$PREFIX @w:9
  call: ARGS_HELD_$SHOW
  call: ARGS_HELD_$CASE w:1 w:10 w:20
  call: ARGS_HELD_$CASE w:2 w:10 w:20
  call: ARGS_HELD_$CASE w:4 w:10 w:20
  call: ARGS_HELD_$EARLY w:6 b:255
  call: ARGS_HELD_$EARLY w:6 b:0
  call: ARGS_HELD_$FOUR w:1 w:2 w:3 w:4 w:1
  call: ARGS_HELD_$FOUR w:1 w:2 w:3 w:4 w:0
  call: ARGS_HELD_$VARS @w:1 @w:2 @w:3 w:1
  call: ARGS_HELD_$VARS @w:1 @w:2 @w:3 w:0
  expect:
    7
    8
    3
    4
    1
    6
    0
    2
    mark 1
    9
    9
    10
    20
    mark 4
    6
    mark 3
    10
    4
    1
    2
    3
    1
    2
    3
    3
    1
    2
}
module args_held;
%include 'exec.ins.pas';

var
  g: integer;

procedure args_held_$then(n: integer; on: boolean); options(val_param);
begin
  g := n;
  if on then
    exec_$logv_w(n);
end;

procedure args_held_$show;
begin
  exec_$log_w(g);
end;

procedure args_held_$parts(in n: integer; in on: boolean);
begin
  g := 0;
  if on then
    exec_$logv_w(n)
  else
    exec_$log_w(n);
end;

procedure args_held_$other(in n: integer; in on: boolean);
begin
  if on then
    g := n - 4
  else begin
    g := 0;
    exec_$log_w(n);
  end;
  exec_$log_w(g);
end;

procedure args_held_$join(in n: integer);
begin
  if n > 0 then begin
    if g > 0 then
      g := g + n;
    exec_$log_w(n);
  end
  else
    exec_$mark(1);
end;

procedure args_held_$prefix(in n: integer);
begin
  g := n + 1;
  exec_$log_w(n);
  if g > 0 then
    g := g - 1;
end;

procedure args_held_$case(k, a, b: integer); options(val_param);
begin
  case k of
    1: exec_$logv_w(a);
    2: exec_$logv_w(b);
    otherwise exec_$mark(k);
  end;
end;

procedure args_held_$early(n: integer; on: boolean); options(val_param);
begin
  g := n;
  if on then begin
    exec_$logv_w(n);
    return;
  end;
  exec_$mark(3);
end;

procedure args_held_$four(a, b, c, d, k: integer); options(val_param);
begin
  g := a + b + c + d;
  if k <> 0 then
    exec_$log_w(g)
  else begin
    exec_$logv_w(d);
    exec_$logv_w(a);
    exec_$logv_w(b);
    exec_$logv_w(c);
  end;
end;

procedure args_held_$vars(var a, b, c: integer; k: integer); options(val_param);
begin
  if k <> 0 then begin
    a := a + 0; b := b + 0; c := c + 0;
    exec_$log_w(a);
    exec_$log_w(b);
    exec_$log_w(c);
  end else begin
    c := c + 0; a := a + 0; b := b + 0;
    exec_$log_w(c);
    exec_$log_w(a);
    exec_$log_w(b);
  end;
end;
