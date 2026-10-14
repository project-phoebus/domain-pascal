{ codegen-patterns 7.1 ("A test right after a store"); bead
  domain-pascal-dgb.  A local stored from a data register and tested by
  the next instruction: the branch reads the store's own flags (20 of the
  23 such stores into the frame, `tools/resultsurvey.py storetest`):

  - a call's result, `x := f(k); if x = 0 then return' (0xE19BE6
    `move.l D0,(-4,A6); bne.s'; 0xE55480 `move.w D0,(-24,A6); bne.s');
  - a boolean, `b := ready(n); if b then', on the sign (0xE6B180 `seq
    D1; move.b D1,(-60,A6); bmi.s');
  - a signed `< 0' (0xE4F09C `move.w D1,(-66,A6); bmi.s').

  Not on the store's flags: a test after a join, where the other path
  did not store (`tst' of the slot); a test of a variable of the unit's
  data (other memory: tested again, 0xE02806). }
module stflags;
var
  seen: integer;
  gl: integer;
function getw(k: integer): integer; val_param; extern;
function getl(k: integer): integer32; val_param; extern;
function ready(k: integer): boolean; val_param; extern;
procedure use(k: integer); val_param; extern;
procedure usel(k: integer32); val_param; extern;
procedure usb(b: boolean); val_param; extern;

procedure call_result(k: integer); val_param;
var x: integer32;
begin
  x := getl(k);
  if x = 0 then return;
  usel(x);
end;

procedure boolean_flag(n: integer); val_param;
var b: boolean;
begin
  b := ready(n);
  if b then seen := 1;
  usb(b);
end;

procedure negative(k: integer); val_param;
var x: integer;
begin
  x := getw(k);
  if x < 0 then seen := 0;
  use(x);
end;

procedure after_join;
var x: integer;
begin
  x := 0;
  if seen > 3 then x := getw(seen);
  if x <> 0 then seen := 2;
  use(x);
end;

procedure unit_data(k: integer); val_param;
begin
  gl := getw(k);
  if gl = 0 then seen := 3;
  use(gl);
end;
