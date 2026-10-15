{ Execution test (review of bead domain-pascal-s4a5): a by-value
  parameter tested right after the entry loads branches on its load's
  flags, with no `tst' (codegen-patterns 12.2, "One-statement bodies";
  tty_$i_set_raw_mode 0xE1BF82 `move.b (0xc,A6),D0; bpl'), also past
  `movea.l' entry loads, which leave the flags (PROC1_$RESUME
  0xE14774..0xE1477C `move.w (8,A6),D0; movea.l (0xa,A6),A0; beq.s').
  Values whose flags differ by size or sign: a word of 256 (low byte
  0), a long of 16#10000 (low word 0), the most negative byte, word and
  long, unsigned chars above 127, and zero.  A parameter loaded before
  the last one is tested with `tst' (PROC1_$SET_TYPE 0xE152FA).

  call: ENTRY_LOAD_FLAGS_$WEQ w:256 @l:0!
  call: ENTRY_LOAD_FLAGS_$WEQ w:0 @l:0!
  call: ENTRY_LOAD_FLAGS_$WEQ w:-32768 @l:0!
  call: ENTRY_LOAD_FLAGS_$WNEG w:-1
  call: ENTRY_LOAD_FLAGS_$WNEG w:-32768
  call: ENTRY_LOAD_FLAGS_$WNEG w:0
  call: ENTRY_LOAD_FLAGS_$WNEG w:32767
  call: ENTRY_LOAD_FLAGS_$WNEG w:128
  call: ENTRY_LOAD_FLAGS_$WGE w:0
  call: ENTRY_LOAD_FLAGS_$WGE w:-2
  call: ENTRY_LOAD_FLAGS_$LEQ l:0x10000
  call: ENTRY_LOAD_FLAGS_$LEQ l:0
  call: ENTRY_LOAD_FLAGS_$LEQ l:0x80000000
  call: ENTRY_LOAD_FLAGS_$LNEG l:0x80000000
  call: ENTRY_LOAD_FLAGS_$LNEG l:0x8000
  call: ENTRY_LOAD_FLAGS_$LNEG l:0
  call: ENTRY_LOAD_FLAGS_$CEQ b:0
  call: ENTRY_LOAD_FLAGS_$CEQ b:128
  call: ENTRY_LOAD_FLAGS_$CEQ b:255
  call: ENTRY_LOAD_FLAGS_$BOOLP b:255 @l:0!
  call: ENTRY_LOAD_FLAGS_$BOOLP b:0 @l:0!
  call: ENTRY_LOAD_FLAGS_$FIRST w:0 w:5
  call: ENTRY_LOAD_FLAGS_$SHOW
  call: ENTRY_LOAD_FLAGS_$FIRST w:256 w:0
  call: ENTRY_LOAD_FLAGS_$SHOW
  call: ENTRY_LOAD_FLAGS_$MULTI w:-7
  call: ENTRY_LOAD_FLAGS_$MULTI w:7
  call: ENTRY_LOAD_FLAGS_$MULTI w:0
  expect:
    out 2
    out 1
    out 2
    -1
    -1
    0
    0
    0
    1
    0
    20
    10
    20
    -1
    0
    0
    1
    0
    0
    out 3
    out 4
    10
    0
    -7
    -7
    0
    7
    0
    0
}
module entry_load_flags;
%include 'exec.ins.pas';

var
  g: integer;

{ a word against 0, a pointer loaded after it (the 0xE14774 shape) }
procedure entry_load_flags_$weq(w: integer; out st: integer32);
  options(val_param);
begin
  if w = 0 then st := 1 else st := 2;
end;

{ a word's sign: `bmi' }
procedure entry_load_flags_$wneg(w: integer); options(val_param);
begin
  if w < 0 then exec_$logv_w(-1) else exec_$logv_w(0);
end;

{ `bpl' }
procedure entry_load_flags_$wge(w: integer); options(val_param);
begin
  if w >= 0 then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure entry_load_flags_$leq(l: integer32); options(val_param);
begin
  if l = 0 then exec_$logv_l(10) else exec_$logv_l(20);
end;

procedure entry_load_flags_$lneg(l: integer32); options(val_param);
begin
  if l < 0 then exec_$logv_l(-1) else exec_$logv_l(0);
end;

procedure entry_load_flags_$ceq(ch: char); options(val_param);
begin
  if ch = chr(0) then exec_$logv_w(1) else exec_$logv_w(0);
end;

{ a boolean, a pointer loaded after it }
procedure entry_load_flags_$boolp(b: boolean; out st: integer32);
  options(val_param);
begin
  if b then st := 3 else st := 4;
end;

{ the first of two loaded parameters: `tst' }
procedure entry_load_flags_$first(a: integer; b: integer); options(val_param);
begin
  if a = 0 then g := b + 5 else g := b;
end;

procedure entry_load_flags_$show;
begin
  exec_$logv_w(g);
end;

{ a body of several statements testing its parameter first }
procedure entry_load_flags_$multi(w: integer); options(val_param);
begin
  if w < 0 then g := w else g := 0;
  exec_$logv_w(g);
  g := w;
  exec_$logv_w(g);
end;
