{ Execution test (bead domain-pascal-jyba): assignments to by-value
  parameters (codegen-patterns 12.2, "Assigned parameters").  A parameter
  loaded at entry is assigned in its register: in place for `x := x op
  y`, a constant into it, a computation from itself made in it
  (area_$internal_create 0xE077FE `addi.l #0x7fff,D2; andi.l
  #-0x8000,D2`), `min` as a select (MSG_$$RCV_INTERNAL 0xE5961C).  One
  in its slot is assigned there: read by a nested routine (area_$resize
  0xE0883C..0xE0884C), or only pushed (AST_$GET_SEG_MAP 0xE06BC6).  A
  value parameter passed by address (the default convention, ref 5.2
  Table 5-1, p. 5-5) is assigned in its copy.  The caller's variable is
  unchanged after each call (ref 5.3.1, p. 5-6: "whatever happens to y
  in addc has no effect on the value of x").

  call: PARAM_ASSIGN_$ROUND l:0x12345 l:0
  call: PARAM_ASSIGN_$ROUND l:-5 l:0
  call: PARAM_ASSIGN_$COUNT w:10
  call: PARAM_ASSIGN_$COUNT w:-1
  call: PARAM_ASSIGN_$CLAMP w:30 w:5
  call: PARAM_ASSIGN_$CLAMP w:3 w:5
  call: PARAM_ASSIGN_$STRIP b:200 b:255
  call: PARAM_ASSIGN_$STRIP b:200 b:0
  call: PARAM_ASSIGN_$SHIFT l:0x10000
  call: PARAM_ASSIGN_$UP l:0x8001
  call: PARAM_ASSIGN_$PUSHED w:7 b:255
  call: PARAM_ASSIGN_$PUSHED w:7 b:0
  call: PARAM_ASSIGN_$CALLERS
  expect:
    mark 0
    98304
    98304
    mark 0
    0
    0
    mark 1
    10
    7
    4
    1
    -2
    mark 1
    -1
    mark 2
    5
    5
    mark 2
    3
    3
    mark 3
    72
    mark 3
    200
    mark 4
    8192
    32768
    mark 5
    65536
    65636
    mark 6
    0
    mark 6
    7
    mark 7
    mark 0
    32768
    32768
    5
    12
    5
    mark 1
    5
    2
    -1
    5
}
module param_assign;
%include 'exec.ins.pas';

{ `x := (x + c) & m` in the register of the entry load; read again
  after }
procedure param_assign_$round(x: integer32; y: integer32); options(val_param);
begin
  exec_$mark(0);
  x := (x + 16#7FFF) & (-16#8000);
  exec_$logv_l(x);
  y := x;
  exec_$logv_l(y);
end;

{ `n := n - 3` in a loop: the register of the entry load, updated in
  place (NETBUF_$RTN_PKT 0xE0F10A `subi.w #0x400,D2`) }
procedure param_assign_$count(n: integer); options(val_param);
begin
  exec_$mark(1);
  while n > 0 do begin
    exec_$logv_w(n);
    n := n - 3;
  end;
  exec_$logv_w(n);
end;

{ a constant in an `if` (RIP_$UPDATE_INT 0xE15944 `moveq #0x11,D0`),
  then `min` (MSG_$$RCV_INTERNAL 0xE5961C) }
procedure param_assign_$clamp(h: integer; m: integer); options(val_param);
begin
  exec_$mark(2);
  if h > 17 then
    h := 5;
  exec_$logv_w(h);
  h := min(h, m);
  exec_$logv_w(h);
end;

{ a character masked in its register (tty_$i_rcv 0xE1B966 `andi.b
  #0x7f,D3`) }
procedure param_assign_$strip(ch: char; strip: boolean); options(val_param);
begin
  exec_$mark(3);
  if strip then
    ch := chr(ord(ch) & 16#7F);
  exec_$logv_w(ord(ch));
end;

{ `rshft` in its register (pacct_$compress 0xE5A9E4 `lsr.l #3,D0`), then
  `+ 1` (0xE5A9F2 `addq.l #1,D0`) }
procedure param_assign_$shift(v: integer32); options(val_param);
begin
  exec_$mark(4);
  v := rshft(v, 3);
  exec_$logv_l(v);
  v := v + 1;
  v := v * 4 - 4;
  exec_$logv_l(v);
end;

{ in its slot: a nested routine reads it }
procedure param_assign_$up(x: integer32); options(val_param);
  procedure show;
  begin
    exec_$logv_l(x);
  end;
begin
  exec_$mark(5);
  x := (x + 16#7FFF) & (-16#8000);
  show;
  x := x + 100;
  show;
end;

{ in its slot: only pushed }
procedure param_assign_$pushed(x: integer; b: boolean); options(val_param);
begin
  exec_$mark(6);
  if b then
    x := 0;
  exec_$logv_w(x);
end;

{ the default convention: passed by address, assigned in the callee's
  copy }
procedure param_assign_$copy(x: integer);
begin
  x := x + 7;
  exec_$logv_w(x);
end;

procedure param_assign_$callers;
var
  a: integer32;
  w: integer;
begin
  exec_$mark(7);
  a := 5;
  param_assign_$round(a, 0);
  exec_$logv_l(a);
  w := 5;
  param_assign_$copy(w);
  exec_$logv_w(w);
  param_assign_$count(w);
  exec_$logv_w(w);
end;
