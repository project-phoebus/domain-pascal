{ Golden: plain `and' / `or' and `odd' (ref "And", p. 4-18; "Or",
  p. 4-134; "Odd", p. 4-128; codegen-patterns 7.1, "Plain `and' and
  `or'"; 8, "odd").

  - two comparisons: `s<cc>' each, `and.b'/`or.b', the branch on its
    sign (TTY_$I_ERR 0xE1BE48..0xE1BE60 `cmpi.l; seq D1b; ...; sne D3b;
    and.b D3b,D1b; bmi.b'; 0xE4BC2A..0xE4BC4A, a chain of `or');
  - a boolean variable on the right: the `and.b'/`or.b' operand as it is
    (0xE39EDA `and.b (20,A6),D6'; 0xE0F554 `or.b (844,A5),D0'); on the
    left: loaded (0xE6E3D8 `move.b (-28,A6),D4'), `not' with `not.b'
    (0xE1009C);
  - `not (a or b)': `not.b' and the sign (0xE0AED6..0xE0AEDA);
  - stored: `move.b Dn,<mem>' (0xE38630..0xE38632);
  - `odd' of a parameter held in a register: `btst #0', the branch on Z
    (FIM_$BUILD_DF 0xE0A6F6, 0xE0A8EE); in a plain `or', `sne'. }
module and_or;

var
  x: boolean;
  i: integer;
  y: boolean;
  j: integer;
  r: boolean;
  l: integer32;

procedure ao_$both;
begin
  if (i = 1) and (j > 2) then
    i := 0;
end;

procedure ao_$chain;
begin
  if (l = 16#e0002b) or (l = 16#e00020) or (l = 16#e00033) then
    i := 0;
end;

procedure ao_$vars;
begin
  if x and y then
    i := 0;
  if not x or (j < 0) then
    j := 0;
end;

procedure ao_$not;
begin
  if not (x or (j <> 3)) then
    i := 0;
end;

procedure ao_$store;
begin
  r := (i <> 0) and not y;
end;

procedure ao_$odd(v: integer32; w: integer); options(val_param);
begin
  if odd(v) then
    i := 0;
  if odd(w) or x then
    j := 0;
end;
