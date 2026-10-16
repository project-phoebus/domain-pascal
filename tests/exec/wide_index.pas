{ Execution test: word indices used as `.l' indices (codegen-patterns 5.2,
  "Word indices used as `.l'"; bead domain-pascal-wkg).  Into an array
  whose bounds times the element size do not fit in 16 bits, a variable
  index whose type's range does not fit either is widened by its sign: a
  `0..65535' word held in a register zero-extended (`clr.l Dn; move.w
  Dm,Dn', AREA_$DELETE 0xE07C4A; `mulu.w #c', 0xE54D7C), an `integer' sign-extended (`ext.l',
  0xE076DE; `muls.w #c', 0xE3EBBA; `moveq #44,Dn; muls.w x,Dn', TPAD_$DATA
  0xE691CA), then scaled at a longword's width and used as `(d,An,Dn.l)'.
  A `.w' index would sign-extend a scaled value beyond 32767 and read
  another element (or below the array).  The arrays are another unit's
  (wide_index_data.mod.pas), each element holding a value made from its
  index: 228-byte elements (`mulu.w'/`muls.w #228') at indices whose
  offsets pass 32767 (150: 34200; 300: 68400), 48-byte ones (shifts) up
  to 16#540 (64512), 24-byte ones from -1000 to 1400 (33600), 44-byte
  ones to 1000 (44000; the constant first), and chars to 40000 (no
  scaling).  A store through a `.l' index is read back.
  The constants stored into elements lie outside the range of `moveq': a
  smaller longword is stored from a register (codegen-patterns 13), set
  up before or after the element's base: not seen, refused (bead
  domain-pascal-53xi).

  link: wide_index_data.mod.pas
  call: WI_$FILL
  call: WI_$U228 w:1 -> l
  call: WI_$U228 w:150 -> l
  call: WI_$U228 w:300 -> l
  call: WI_$S228 w:200 -> l
  call: WI_$S228 w:300 -> l
  call: WI_$U48 w:1 -> l
  call: WI_$U48 w:700 -> l
  call: WI_$U48 w:1344 -> l
  call: WI_$S24 w:-1000 -> l
  call: WI_$S24 w:-1 -> l
  call: WI_$S24 w:0 -> l
  call: WI_$S24 w:1400 -> l
  call: WI_$S44 @w:1 -> l
  call: WI_$S44 @w:745 -> l
  call: WI_$S44 @w:1000 -> l
  call: WI_$CHAR w:0 -> w
  call: WI_$CHAR w:32767 -> w
  call: WI_$CHAR w:32768 -> w
  call: WI_$CHAR w:40000 -> w
  call: WI_$SET48 w:1000
  call: WI_$U48 w:1000 -> l
  call: WI_$U48 w:999 -> l
  expect:
    => 100001
    => 100150
    => 100300
    => 100200
    => 100300
    => 200001
    => 200700
    => 201344
    => 299000
    => 299999
    => 300000
    => 301400
    => 400001
    => 400745
    => 401000
    => 0
    => 137
    => 138
    => 91
    => -700
    => 200999
}
module wide_index;

type
  pinteger = 0..65535;
  r228 = record z: integer32; pad: array[1..112] of integer; end;
  r48 = record z: integer32; pad: array[1..22] of integer; end;
  r24 = record z: integer32; pad: array[1..10] of integer; end;
  r44 = record z: integer32; pad: array[1..20] of integer; end;
var
  wi_$r228: extern array[1..300] of r228;
  wi_$r48: extern array[1..16#540] of r48;
  wi_$r24: extern array[-1000..1400] of r24;
  wi_$r44: extern array[1..1000] of r44;
  wi_$chars: extern array[0..40000] of char;

{ `mulu.w #228' of the loaded word, `.l'. }
function wi_$u228(i: pinteger): integer32;
  options(val_param);
begin
  wi_$u228 := wi_$r228[i].z;
end;

{ `muls.w #228', `.l'. }
function wi_$s228(i: integer): integer32;
  options(val_param);
begin
  wi_$s228 := wi_$r228[i].z;
end;

{ A word held in a register zero-extended, `clr.l Dn; movea.l #sym,An;
  move.w Dm,Dn', ×48 with shifts at a longword's width (the parameter
  tested first, so loaded at entry: AREA_$DELETE 0xE07C34..0xE07C5E;
  from memory refused, the base's place not settled). }
function wi_$u48(i: pinteger): integer32;
  options(val_param);
begin
  if i = 0 then
    wi_$u48 := 0
  else
    wi_$u48 := wi_$r48[i].z;
end;

procedure wi_$set48(i: pinteger);
  options(val_param);
begin
  if i <> 0 then
    wi_$r48[i].z := -700;
end;

{ `ext.l', ×24 with shifts. }
function wi_$s24(i: integer): integer32;
  options(val_param);
begin
  wi_$s24 := wi_$r24[i].z;
end;

{ The constant first: `moveq #44,Dn; muls.w (A0),Dn'. }
function wi_$s44(in i: integer): integer32;
begin
  wi_$s44 := wi_$r44[i].z;
end;

{ No scaling: the zero-extended word itself, `.l'. }
function wi_$char(i: pinteger): integer;
  options(val_param);
begin
  if i = 65535 then
    wi_$char := -1
  else
    wi_$char := ord(wi_$chars[i]);
end;
