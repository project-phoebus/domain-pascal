{ Execution test: index scalings and products by a constant done with a
  multiply (codegen-patterns 5.2 "Index scaling", 8 "Multiplication by a
  constant"; bead domain-pascal-r3yy).  A constant of 1..127 goes into a
  register first, `moveq #c,Dn; mulu.w x,Dn' for a `.w' index
  (TPAD_$SET_UNIT_MODE 0xE69824 `moveq #44,D2; mulu.w (A4),D2'), the
  variable in memory (a unit variable, a by-address parameter's object)
  or in a register (a by-value parameter, loaded first: 0xE383C8); for a
  word product, `muls.w' (0xE458D2); for a longword product of a word,
  `muls.w' (TPAD_$DATA 0xE691D0).  Constants of 128 and more: `mulu.w
  #c' on the loaded value (×268, odd part 67, three bits).  Element sizes
  44 (odd part 11), 46 (23), 84 (21), 100 (25), 92 (23) and 268, indices
  at both bounds, a negative lower bound (a `.w' index of a negative
  value: the low word of `mulu.w' is the product's), and another unit's
  array.  Element k of each array holds a value made from k, so a wrong
  scale reads or writes another element.  Review additions: longword
  products at the ends of `integer' (muls.w's 32-bit product) for a unit
  variable and a by-value parameter; a 601-element array with a negative
  lower bound whose `.w' index reaches -13200 and 13200 (`mulu.w' of a
  negative word: the low word is right while the product fits 16 bits);
  a zero-extended byte index multiplied in place (`mulu.w #44',
  RIP_$BROADCAST 0xE872F8).  Not expressed (refused): an
  index read from another unit's variable or from an array element (an
  absolute or indexed multiplicand, no site), an unsigned word product.

  link: index_mul_data.mod.pas
  call: IM_$FILL
  call: IM_$FILL_O92
  call: IM_$GET44 w:1 -> w
  call: IM_$GET44 w:8 -> w
  call: IM_$IN44 @w:5 -> w
  call: IM_$SET44 @w:8 @w:-77
  call: IM_$GET44 w:8 -> w
  call: IM_$GET44 w:7 -> w
  call: IM_$UNIT44 w:3 -> w
  call: IM_$NEG44 w:-4 -> w
  call: IM_$NEG44 w:3 -> w
  call: IM_$NEG44 w:-1 -> w
  call: IM_$GET46 w:0 -> w
  call: IM_$GET46 w:7 -> w
  call: IM_$GET84 w:1 -> w
  call: IM_$GET84 w:8 -> w
  call: IM_$GET100 w:0 -> w
  call: IM_$GET100 w:9 -> w
  call: IM_$GET268 w:0 -> w
  call: IM_$GET268 w:7 -> w
  call: IM_$OTHER92 @w:6 -> w
  call: IM_$PROD w:3 -> w
  call: IM_$PROD w:-5 -> w
  call: IM_$PROD w:700 -> w
  call: IM_$LPROD w:-1000 -> l
  call: IM_$LPROD w:1000 -> l
  call: IM_$LPROD w:-32768 -> l
  call: IM_$LPROD w:32767 -> l
  call: IM_$LPRODV w:-32768 -> l
  call: IM_$LPRODV w:745 -> l
  call: IM_$BIG44 w:-300 -> w
  call: IM_$BIG44 w:-1 -> w
  call: IM_$BIG44 w:0 -> w
  call: IM_$BIG44 w:300 -> w
  call: IM_$BYTE44 b:1 -> w
  call: IM_$BYTE44 b:7 -> w
  expect:
    => 1001
    => 1008
    => 1005
    => -77
    => 1007
    => 1003
    => -1004
    => -997
    => -1001
    => 2000
    => 2007
    => 3001
    => 3008
    => 4000
    => 4009
    => 5000
    => 5007
    => 6006
    => 184
    => -168
    => 30852
    => -44000
    => 44000
    => -1441792
    => 1441748
    => -1441792
    => 32780
    => -300
    => -1
    => 0
    => 300
    => 1001
    => 1007
}
module index_mul;

type
  c44 = record a: integer; b: integer; pad: array[1..19] of integer; z: integer; end;
  c46 = record a: integer; pad: array[1..21] of integer; z: integer; end;
  c84 = record a: integer; pad: array[1..40] of integer; z: integer; end;
  c100 = record a: integer32; pad: array[1..47] of integer; z: integer; end;
  c268 = record a: integer; pad: array[1..132] of integer; z: integer; end;
  c92 = record a: integer; pad: array[1..44] of integer; z: integer; end;
var
  a44: array[1..8] of c44;
  n44: array[-4..3] of c44;
  a46: array[0..7] of c46;
  a84: array[1..8] of c84;
  a100: array[0..9] of c100;
  a268: array[0..7] of c268;
  big44: array[-300..300] of c44;
  k: integer;
  im_$o92: extern array[1..8] of c92;

procedure im_$fill;
var
  i: integer;
begin
  i := 1;
  while i <= 8 do begin
    a44[i].z := 1000 + i;
    a84[i].z := 3000 + i;
    i := i + 1;
  end;
  i := -4;
  while i <= 3 do begin
    n44[i].z := i - 1000;
    i := i + 1;
  end;
  i := 0;
  while i <= 7 do begin
    a46[i].z := 2000 + i;
    a268[i].z := 5000 + i;
    i := i + 1;
  end;
  i := 0;
  while i <= 9 do begin
    a100[i].z := 4000 + i;
    i := i + 1;
  end;
  i := -300;
  while i <= 300 do begin
    big44[i].z := i;
    i := i + 1;
  end;
end;

{ val_param: the index loaded, then `moveq #44,Dn; mulu.w Dx,Dn'. }
function im_$get44(i: integer): integer;
  options(val_param);
begin
  im_$get44 := a44[i].z;
end;

{ A by-address parameter: `moveq #44,Dn; movea.l (8,A6),A0; mulu.w (A0),Dn'. }
function im_$in44(in i: integer): integer;
begin
  im_$in44 := a44[i].z;
end;

procedure im_$set44(in i: integer; in v: integer);
begin
  a44[i].z := v;
end;

{ A unit variable: `moveq #44,Dn; mulu.w (d,A5),Dn'. }
function im_$unit44(i: integer): integer;
  options(val_param);
begin
  k := i;
  im_$unit44 := a44[k].z;
end;

function im_$neg44(i: integer): integer;
  options(val_param);
begin
  k := i;
  im_$neg44 := n44[k].z;
end;

function im_$get46(i: integer): integer;
  options(val_param);
begin
  im_$get46 := a46[i].z;
end;

function im_$get84(i: integer): integer;
  options(val_param);
begin
  im_$get84 := a84[i].z;
end;

function im_$get100(i: integer): integer;
  options(val_param);
begin
  im_$get100 := a100[i].z;
end;

{ ×268: `mulu.w #268' on the loaded value. }
function im_$get268(i: integer): integer;
  options(val_param);
begin
  im_$get268 := a268[i].z;
end;

{ Another unit's array: `moveq #92,Dn; movea.l (8,A6),A0; movea.l
  #IM_$O92,A1; mulu.w (A0),Dn'. }
function im_$other92(in i: integer): integer;
begin
  im_$other92 := im_$o92[i].z;
end;

{ A word product, `muls.w' (module-local 0xE4519C at 0xE458CC..0xE458D4
  `moveq #44,D3; muls.w D2,D3; addi.w #52,D3'). }
function im_$prod(i: integer): integer;
  options(val_param);
begin
  im_$prod := i * 44 + 52;
end;

{ A longword product of a word: the 32-bit product of `muls.w' (ref
  4.3.1, p. 4-5), beyond 16 bits for |i| > 744. }
function im_$lprod(i: integer): integer32;
  options(val_param);
begin
  k := i;
  im_$lprod := k * 44;
end;

{ A by-value parameter: `move.w (8,A6),D0; moveq #44,D1; muls.w D0,D1'. }
function im_$lprodv(i: integer): integer32;
  options(val_param);
begin
  im_$lprodv := i * 44;
end;

{ A `.w' index of -13200..13200. }
function im_$big44(i: integer): integer;
  options(val_param);
begin
  im_$big44 := big44[i].z;
end;

{ A zero-extended byte, multiplied in place: `clr.w D0; move.b (8,A6),D0;
  mulu.w #44,D0' (RIP_$BROADCAST 0xE872F8). }
function im_$byte44(c: char): integer;
  options(val_param);
begin
  im_$byte44 := a44[ord(c)].z;
end;
