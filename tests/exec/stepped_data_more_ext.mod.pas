{ The array of stepped_data_more.pas reached as another unit's, from a
  negative lower bound (codegen-patterns 7.4, "Forming": `movea.l #sym,Ap'
  adjusted in place). }
module stepped_data_more_ext;

type
  e6 = record a: integer; b: integer; c: integer; end;
var
  sdm_$x: array[-2..5] of e6;
define sdm_$x;
