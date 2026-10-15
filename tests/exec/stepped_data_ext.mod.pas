{ The array of stepped_data.pas reached as another unit's (codegen-
  patterns 7.4, "Forming": `movea.l #sym,Ap' adjusted in place). }
module stepped_data_ext;

type
  sdx_$pair = record
    key: integer32;
    val: integer;
  end;
var
  sdx_$ext: array[1..12] of sdx_$pair;
define sdx_$ext;
