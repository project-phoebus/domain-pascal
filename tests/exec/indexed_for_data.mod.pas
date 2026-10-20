{ The array of indexed_for.pas's fill from an element other than the
  first (another unit's array at an offset from its symbol), in a unit of
  its own, and a routine that sets it to all ones. }
module indexed_for_data;

type
  longs = array[1..6] of integer32;
var
  ixf_$xl: longs;
define ixf_$xl;

procedure ixf_$xreset;
var
  i: integer;
begin
  i := 1;
  while i <= 6 do begin
    ixf_$xl[i] := -1;
    i := i + 1;
  end;
end;
