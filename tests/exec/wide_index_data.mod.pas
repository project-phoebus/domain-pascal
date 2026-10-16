{ The arrays of wide_index.pas, each larger than 32K bytes: defined here,
  reached there as another unit's (codegen-patterns 5.2, "Word indices
  used as `.l'").  The fill steps a pointer through each array (no
  index), so every element's value says which element it is (a char: its
  index mod 251).  Only the
  first array is within 32K of the data area's start (dpc refuses a
  static beyond it): the pointer runs on from each array into the next,
  as dpc lays them out in declaration order, each a multiple of 4 bytes
  but the last. }
module wide_index_data;

type
  r228 = record z: integer32; pad: array[1..112] of integer; end;
  r48 = record z: integer32; pad: array[1..22] of integer; end;
  r24 = record z: integer32; pad: array[1..10] of integer; end;
  r44 = record z: integer32; pad: array[1..20] of integer; end;
  p228 = ^r228;
  p48 = ^r48;
  p24 = ^r24;
  p44 = ^r44;
  pc = ^char;
var
  wi_$r228: array[1..300] of r228;
  wi_$r48: array[1..16#540] of r48;
  wi_$r24: array[-1000..1400] of r24;
  wi_$r44: array[1..1000] of r44;
  wi_$chars: array[0..40000] of char;
define wi_$r228, wi_$r48, wi_$r24, wi_$r44, wi_$chars;

procedure wi_$fill;
var
  k: integer32;
  q228: p228;
  q48: p48;
  q24: p24;
  q44: p44;
  qc: pc;
  c: integer;
begin
  q228 := addr(wi_$r228);
  k := 1;
  while k <= 300 do begin
    q228^.z := k + 100000;
    q228 := univ_ptr(integer32(q228) + 228);
    k := k + 1;
  end;
  q48 := univ_ptr(q228);
  k := 1;
  while k <= 16#540 do begin
    q48^.z := k + 200000;
    q48 := univ_ptr(integer32(q48) + 48);
    k := k + 1;
  end;
  q24 := univ_ptr(q48);
  k := -1000;
  while k <= 1400 do begin
    q24^.z := k + 300000;
    q24 := univ_ptr(integer32(q24) + 24);
    k := k + 1;
  end;
  q44 := univ_ptr(q24);
  k := 1;
  while k <= 1000 do begin
    q44^.z := k + 400000;
    q44 := univ_ptr(integer32(q44) + 44);
    k := k + 1;
  end;
  qc := univ_ptr(q44);
  k := 0;
  c := 0;
  while k <= 40000 do begin
    qc^ := chr(c);
    qc := univ_ptr(integer32(qc) + 1);
    k := k + 1;
    c := c + 1;
    if c = 251 then
      c := 0;
  end;
end;
