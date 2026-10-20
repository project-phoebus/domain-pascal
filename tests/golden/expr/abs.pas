{ Golden: `abs' of run-time integers ("Abs", p. 4-12; codegen-patterns 8,
  "abs"; bead domain-pascal-g6xa): the value formed in a new data
  register, `bpl.s' over `neg', on the flags of the instruction that
  formed it or after a `tst'; its type a non-negative subrange, unsigned.
  - pid_arg: PROC2_$WAIT 0xE3FE5C..0xE3FE64, `abs(pid)' of an `in' word
    read through its slot, passed by value after the pad: `subq.l #2,SP;
    move.w (A3),D1w; bpl.b; neg.w D1w; move.w D1w,-(SP)' (the slot held
    in A3 there).
  - adjust: TIME_$ADJUST_TIME_OF_DAY 0xE168F4..0xE16900, `abs(delta.sec)
    > 8000' of an `integer32' field: `move.l (A2),D0; bpl.b; neg.l D0;
    cmpi.l #0x1f40,D0; bls.b' (unsigned).
  - locate: LOCATE_SERVER 0xE4A440..0xE4A452, `abs(heard - clockh) >
    timeout' of `linteger's: `move.l (0x28,A5),D0; sub.l (abs).l,D0;
    bpl.b; neg.l D0; cmp.l (0x20,A5),D0; bls.b'.
  - hyst: TPAD_$DATA 0xE6924A..0xE69256, `abs(delta_x) < hysteresis' of
    a register-held `integer' against an `integer' field: `move.w
    D2w,D5w; bpl.b; neg.w D5w; cmp.w (-0x12,A2),D5w; bge.b; clr.w D2w' (a
    word compare, signed).
  - quot: 0xE69668..0xE69678, `abs(delta_x div delta_y) > 5': `divs.w
    D1w,D0; tst.w D0w; bpl.b; neg.w D0w; cmpi.w #0x5,D0w; bls.b'.
  - velocity: 0xE69624..0xE69638, `abs(delta_x * x_factor) + abs(delta_y
    * y_factor)' into a word: each on its `muls.w''s flags, `add.w'.
  - punch: 0xE69764..0xE69780, `abs(delta_x) + abs(delta_y) >=
    punch_impact': each zero-extended into a new register (`clr.l D5;
    move.w D3w,D5w'), the sum in 32 bits, the field widened, `blt'. }
module abs;
type
  linteger = 0..2147483647;
  timeval = record
    sec: integer32;
    usec: integer32;
  end;
  unit_t = record
    mode: integer;
    hysteresis: integer;
    x_factor, y_factor: integer;
    punch_impact: integer;
  end;
  unit_p = ^unit_t;
var
  heard, timeout: linteger;
  flag: boolean;
  w: integer;
  clockh: extern linteger;

procedure find(upgid: integer); options(val_param); extern;

procedure pid_arg(in pid: integer);
begin
  find(abs(pid));
end;

procedure adjust(in delta: timeval; out status: integer32);
begin
  status := 0;
  if abs(delta.sec) > 8000 then status := 16#D000C;
end;

procedure locate;
begin
  if abs(heard - clockh) > timeout then flag := false;
end;

procedure hyst(u: unit_p); options(val_param);
var
  delta_x: integer;
begin
  delta_x := w;
  if abs(delta_x) < u^.hysteresis then delta_x := 0;
  w := delta_x;
end;

procedure quot(delta_x, delta_y: integer); options(val_param);
begin
  if abs(delta_x div delta_y) > 5 then w := 0;
end;

procedure velocity(u: unit_p; delta_x, delta_y: integer); options(val_param);
begin
  w := abs(delta_x * u^.x_factor) + abs(delta_y * u^.y_factor);
end;

procedure punch(u: unit_p; delta_x, delta_y: integer); options(val_param);
begin
  if abs(delta_x) + abs(delta_y) >= u^.punch_impact then w := 0;
end;
