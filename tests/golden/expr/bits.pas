{ Golden: codegen-patterns 8 (bit operators, ref "Bit Operators" p. 4-33,
  "Lshft" p. 4-109, "Rshft" p. 4-171, "Arshft" p. 4-29, "Xor" p. 4-221).
  - `x & c' with `x' in memory or held in a register: the constant into
    the result register first, then `and.<s> x,Dn' (0xE177A0 `moveq
    #-2,D2; and.w D1,D2', the parameter held in D1; 0xE60F4A; 0xE02D44);
    on a computed value `andi' in place;
  - `!' and `xor' with a constant on a computed value: `ori', `eori' in
    place (0xE03498);
  - shifts: `lsl'/`lsr'/`asr' by a constant, split above 8 (0xE02AA6
    `lsl.l #8; lsl.l #2'), or by a register (0xE02D52 `lsl.w D1,D3'). }
module bits;
type
  pinteger = 0..65535;
var
  w: extern integer;
  l: extern integer32;
  r: extern integer;
  n: extern integer;
function mask(x: integer): integer; internal;
begin
  mask := x & 16#FFFE;
end;
procedure held(x: integer; y: integer); internal;
begin
  w := y;
  r := x & 16#FFFE;
end;
function mask_or(x: integer): integer; internal;
begin
  mask_or := (x & 16#FF) ! 16#100;
end;
function mask_xor(x: integer): integer; internal;
begin
  mask_xor := xor(lshft(x, 2), 16#55);
end;
function shl10(x: integer32): integer32; internal;
begin
  shl10 := lshft(x, 10);
end;
function shr3(x: integer32): integer32; internal;
begin
  shr3 := rshft(x, 3);
end;
function sar2(x: integer): integer; internal;
begin
  sar2 := arshft(x, 2);
end;
function shl_var(x: integer): integer; internal;
begin
  shl_var := lshft(x, n);
end;
