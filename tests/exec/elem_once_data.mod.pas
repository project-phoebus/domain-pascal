{ The arrays of elem_once.pas, in a unit of their own (elem_once.pas
  reaches them as another unit's, codegen-patterns 5.2, "Another unit's
  array"), and a routine that fills them: element k holds values made
  from k so that a wrong index, scale or register reads a wrong value. }
module elem_once_data;

type
  rec12 = record a, b, c: integer32; end;
  rec288 = record a: integer32; z: integer32; pad: array[1..70] of integer32; end;
var
  eo_$recs: array[1..6] of rec12;
  eo_$quits: array[1..6] of rec12;
  eo_$big: array[1..4] of rec288;
define eo_$recs, eo_$quits, eo_$big;

procedure eo_$fill;
var
  i: integer;
begin
  i := 1;
  while i <= 6 do begin
    eo_$recs[i].a := i * 10 + 1;
    eo_$recs[i].b := i * 10 + 2;
    eo_$recs[i].c := i * 10 + 3;
    eo_$quits[i].a := i * 100 + 1;
    eo_$quits[i].b := i * 100 + 2;
    eo_$quits[i].c := i * 100 + 3;
    i := i + 1;
  end;
  i := 1;
  while i <= 4 do begin
    eo_$big[i].a := i * 1000;
    eo_$big[i].z := -i;
    eo_$big[i].pad[70] := i;
    i := i + 1;
  end;
end;
