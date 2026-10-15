{ The array of index_mul.pas reached as another unit's (codegen-patterns
  5.2, "Another unit's array"), 92-byte elements, element k's z holding
  6000 + k. }
module index_mul_data;

type
  c92 = record a: integer; pad: array[1..44] of integer; z: integer; end;
var
  im_$o92: array[1..8] of c92;
define im_$o92;

procedure im_$fill_o92;
var
  i: integer;
begin
  i := 1;
  while i <= 8 do begin
    im_$o92[i].z := 6000 + i;
    i := i + 1;
  end;
end;
