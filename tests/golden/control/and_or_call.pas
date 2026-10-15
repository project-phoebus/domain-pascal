{ Golden: a plain `or' / `and' whose right operand compares a function's
  result (ref "Or", p. 4-134..4-135, "And", p. 4-18: no order of
  evaluation guaranteed; codegen-patterns 7.1, "A plain `or'/`and' whose
  right operand calls a function"; beads domain-pascal-i1by, -1rwo).

  - the left operand jumping code, as `or else' / `and then': `tst.w
    (A2); ble' to the `then' part for `or', to the end for `and';
  - the right one: the call, its result compared in D0 (`cmp.w (A2),D0':
    the variable read after the call, the relation reversed when the
    call is on the right of the source), `s<cc>' into the lowest
    callee-saved data register free, `tst.b' of it, `bpl'/`bmi'
    (TPAD_$SET_UNIT_MODE 0xE6980A..0xE6981A `slt D2'; TPAD_$SET_UNIT
    0xE699EE..0xE699FE `sge D2', the `and');
  - against 0: `tst.w D0' (FILE_$SET_PROT_INT 0xE5DE5A `tst.w D0; sne
    D2; tst.b D2'); against a constant: `cmpi.w #c,D0' (EC2_$WAIT
    0xE4262A `cmpi.w #2,D0; seq D2; tst.b D2');
  - a register-held local compared, D2 taken: `cmp.l D2,D0; sne D3'
    (0xE5E590 `cmp.l D3,D0; sne D5', D2..D4 live). }
module and_or_call;

var
  u: integer;
  lg: integer32;

function n_devices: integer; extern;
function count(in k: integer32): integer32; extern;

procedure aoc_$set_unit(in unitnum: integer);
begin
  if (unitnum > 0) and (unitnum <= n_devices) then
    u := unitnum;
end;

procedure aoc_$check(in unitp: integer; out st: integer32);
begin
  if (unitp <= 0) or (unitp > n_devices) then begin
    st := 16#130001;
    return;
  end;
  u := unitp;
  st := 0;
end;

procedure aoc_$call_left(in unitp: integer);
begin
  if (unitp < 0) or (n_devices = unitp) then
    u := 0;
end;

procedure aoc_$zero(in unitp: integer);
begin
  if (unitp <> 1) and (n_devices <> 0) then
    u := 1;
end;

procedure aoc_$const(in unitp: integer);
begin
  if (unitp = 3) or (n_devices = 2) then
    u := 2;
end;

procedure aoc_$held(in k: integer32);
var
  m: integer32;
begin
  m := k + 1;
  if (k > 0) or (count(k) <> m) then
    lg := k;
end;
