{ Golden: codegen-patterns 7 (conditions), 5 (signedness), 13.

  - Against 0 or nil: `tst.<s>' (compiled code has no `cmpi #0');
    `<' and `>=' of a signed value branch with `bmi'/`bpl' (after
    `tst.w': `bmi'/`bpl' 144 times, `blt'/`bge' 4), `<=' and `>'
    with `ble'/`bgt'.  0xE1778C `tst.l (0x14,A5); bne.s'.
  - Against another constant: `cmpi.<s> #n,<ea>' on a register
    (0xE1779A `cmpi.w #0x400,D1') or on memory (0xE4FE90 `cmpi.w
    #4,(0x12,A6)'; 0xE4BAE0 `cmpi.l #0x400,(0x10,A3)').  A byte
    immediate is written 0..255 (codegen-patterns 13).
  - Two variables: `cmp.<s> <ea>,Dn' with the left one held in Dn.
  - Signed relations branch with `blt'/`ble'/`bgt'/`bge', unsigned ones
    (`0..65535', char) with `bcs'/`bls'/`bhi'/`bcc' (0xE1779E `bcc',
    0xE177BE `bls').
  - Booleans: `tst.b' and the sign, `bpl' to skip when false, `bmi'
    when true (after `tst.b': `bpl' 919, `bmi' 425, `beq'/`bne' 37).
  - `not' swaps the branch; `and then' / `or else' test their operands
    in order, each with its own branch (0xE031A2..0xE031B6).
  Each `if' skips its `then' part with the negated condition. }
module cmp;
type
  pinteger = 0..65535;
var
  w: extern integer;
  l: extern integer32;
  u: extern pinteger;
  c: extern char;
  p: extern ^integer;
  b, b2: extern boolean;
  r: extern integer;

procedure zeros; internal;
begin
  if w = 0 then r := 1;
  if l <> 0 then r := 2;
  if w < 0 then r := 3;
  if l >= 0 then r := 4;
  if w <= 0 then r := 5;
  if w > 0 then r := 6;
  if p = nil then r := 7;
  if u = 0 then r := 8;
end;

procedure constants; internal;
begin
  if w = 5 then r := 1;
  if w < -2 then r := 2;
  if l > 16#12345 then r := 3;
  if u < 16#8000 then r := 4;
  if u >= 16#FFFF then r := 5;
  if c <= 'z' then r := 6;
  if c > chr(200) then r := 7;
end;

procedure held(x: integer; y: pinteger); internal;
begin
  if x < w then r := 1;
  if y > u then r := 2;
  if x = 7 then r := 3;
end;

procedure booleans; internal;
begin
  if b then r := 1;
  if not b then r := 2;
  if b and then b2 then r := 3;
  if b or else b2 then r := 4;
  if not (b and then (w = 1)) then r := 5;
  if (w = 1) or else not b2 then r := 6;
end;
