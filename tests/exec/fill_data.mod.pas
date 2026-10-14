{ The arrays of fill.pas, in a unit of their own (fill.pas reaches them
  as another unit's, codegen-patterns 7.4), and a routine that sets
  them to all ones. }
module fill_data;

type
  words = array[1..8] of integer;
  longs = array[1..4] of integer32;
var
  fill_$wa: words;
  fill_$la: longs;
define fill_$wa;
define fill_$la;

procedure fill_$reset;
var
  i: integer;
begin
  i := 1;
  while i <= 8 do begin
    fill_$wa[i] := -1;
    i := i + 1;
  end;
  i := 1;
  while i <= 4 do begin
    fill_$la[i] := -1;
    i := i + 1;
  end;
end;
