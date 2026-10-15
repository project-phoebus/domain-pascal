{ Execution test (bead domain-pascal-1gyq): a register-held local tested
  right after the `move' of its assignment branches on the move's flags,
  with no `tst' (codegen-patterns 7.1, "Flags of a load"; the BAT_$FREE
  loop 0xE3B590 `move.l (A4),D3; bne.s'; XPD_$GET_EC 0xE5BDDE `move.w
  D0,D2; beq.s'; 0xE072B8 `move.b D0,D4; bmi').  Values whose flags
  differ by size or sign: a word of 256 (low byte 0), a long of 16#10000
  (low word 0), the most negative word and long, 1, -1 and 0.  The
  function results come back with junk above the word (exec_$id_w), so a
  test of the wrong size shows.

  call: ASSIGN_FLAGS_$WEQ @w:256
  call: ASSIGN_FLAGS_$WEQ @w:0
  call: ASSIGN_FLAGS_$WEQ @w:-32768
  call: ASSIGN_FLAGS_$WNEG @w:-1
  call: ASSIGN_FLAGS_$WNEG @w:-32768
  call: ASSIGN_FLAGS_$WNEG @w:0
  call: ASSIGN_FLAGS_$WNEG @w:32767
  call: ASSIGN_FLAGS_$WPOS @w:1
  call: ASSIGN_FLAGS_$WPOS @w:0
  call: ASSIGN_FLAGS_$WPOS @w:-32768
  call: ASSIGN_FLAGS_$LEQ @l:0x10000
  call: ASSIGN_FLAGS_$LEQ @l:0
  call: ASSIGN_FLAGS_$LNEG @l:0x80000000
  call: ASSIGN_FLAGS_$LNEG @l:0x8000
  call: ASSIGN_FLAGS_$BOR b:0 b:0
  call: ASSIGN_FLAGS_$BOR b:255 b:0
  call: ASSIGN_FLAGS_$BOR b:0 b:255
  call: ASSIGN_FLAGS_$CRES @w:256
  call: ASSIGN_FLAGS_$CRES @w:0
  call: ASSIGN_FLAGS_$CRESL @l:0x10000
  call: ASSIGN_FLAGS_$CRESL @l:0
  call: ASSIGN_FLAGS_$SHOW
  expect:
    0
    1
    0
    1
    1
    0
    0
    1
    0
    0
    0
    1
    1
    0
    0
    1
    1
    0
    1
    0
    1
    0
}
module assign_flags;
%include 'exec.ins.pas';

var
  g: integer;
  gl: integer32;
  gb: boolean;

procedure assign_flags_$weq(in w: integer);
var x: integer;
begin
  x := w;
  if x = 0 then exec_$logv_w(1) else exec_$logv_w(0);
  g := x;
end;

{ `bpl' }
procedure assign_flags_$wneg(in w: integer);
var x: integer;
begin
  x := w;
  if x < 0 then exec_$logv_w(1) else exec_$logv_w(0);
  g := x;
end;

{ `ble' }
procedure assign_flags_$wpos(in w: integer);
var x: integer;
begin
  x := w;
  if x > 0 then exec_$logv_w(1) else exec_$logv_w(0);
  g := x;
end;

procedure assign_flags_$leq(in l: integer32);
var x: integer32;
begin
  x := l;
  if x = 0 then exec_$logv_w(1) else exec_$logv_w(0);
  gl := x;
end;

procedure assign_flags_$lneg(in l: integer32);
var x: integer32;
begin
  x := l;
  if x < 0 then exec_$logv_w(1) else exec_$logv_w(0);
  gl := x;
end;

{ a copy of another register: `or.b' then `move.b' into the local's }
procedure assign_flags_$bor(a, c: boolean); options(val_param);
var b: boolean;
begin
  b := a or c;
  if b then exec_$logv_w(1) else exec_$logv_w(0);
  gb := b;
  b := c;
  gb := b;
end;

{ a function's result, held across a call: `move.w D0,D2; beq' }
procedure assign_flags_$cres(in w: integer);
var x: integer;
begin
  x := exec_$id_w(w);
  if x = 0 then exec_$logv_w(1) else exec_$logv_w(0);
  g := x;
end;

procedure assign_flags_$cresl(in l: integer32);
var x: integer32;
begin
  x := exec_$id_l(l);
  if x = 0 then exec_$logv_w(1) else exec_$logv_w(0);
  gl := x;
end;

procedure assign_flags_$show;
begin
  exec_$logv_w(g);
end;
