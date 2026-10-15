{ codegen-patterns 7.1 ("Flags of a load"); bead domain-pascal-1gyq.  A
  local held in a register, assigned by a `move' and tested by the next
  statement: the branch reads the move's flags, no `tst' (a `move' sets N
  and Z from the value and clears V and C, as `tst' does).
  `tools/cmpsurvey.py loadflags': of the loads from memory or copies of
  another register followed directly by a branch on the register, outside
  a join, 208 branch on the move's flags and none has a `tst' when the
  register is compared only once there; 24 copies of a call's D0 into a
  register, all on the flags.

  - a load through a pointer, `x := p^; if x = 0' (the BAT_$FREE loop
    0xE3B590 `move.l (A4),D3; bne.s');
  - its sign, `<' and `>=' (`bpl'/`bmi', 71 sites), and `>' (`ble', 8
    sites, 0xE10104);
  - a copy of another register's value, `b := a or c' formed in a
    register of its own and moved into the local's (assigned twice, so
    held in one register for the body: `isel.rs' `store_web'; 0xE072B6..
    0xE072B8 `or.b D1,D0; move.b D0,D4; bmi');
  - a function's result held across a call, `x := f(k); if x = 0'
    (XPD_$GET_EC 0xE5BDDE `move.w D0,D2; beq.s'; SEEK 0xE195DE `move.l
    D0,D2; bne.s').

  Refused (crates/dpc/tests/driver.rs, `assigned_local_tests'): a
  local compared again right after the branch (both forms seen), a
  function's result in a scratch register (the original tests D0
  first), `bgt'/`bge' after a load (not seen). }
module asflags;
var
  seen: integer;
  gw: integer;
  gl: integer32;
  gb: boolean;
  pl: ^integer32;
  pw: ^integer;
function getw(k: integer): integer; val_param; extern;
procedure use(k: integer); val_param; extern;

procedure load_eq;
var x: integer32;
begin
  x := pl^;
  if x = 0 then seen := 1;
  gl := x;
end;

procedure load_sign;
var x: integer;
begin
  x := pw^;
  if x < 0 then seen := 2;
  gw := x;
end;

procedure load_positive;
var x: integer;
begin
  x := pw^;
  if x > 0 then seen := 3;
  gw := x;
end;

procedure copy_or(a, c: boolean); val_param;
var b: boolean;
begin
  b := a or c;
  if b then seen := 4;
  gb := b;
  b := c;
  gb := b;
end;

procedure call_result(k: integer); val_param;
var x: integer;
begin
  x := getw(k);
  if x = 0 then return;
  use(k);
  use(x);
end;
