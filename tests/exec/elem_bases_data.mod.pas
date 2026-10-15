{ The arrays of elem_bases.pas, in a unit of their own (elem_bases.pas
  reaches them as another unit's, codegen-patterns 5.2, "Another unit's
  array"), and a routine that fills them: element k holds a value made
  from k so that a wrong index or scale reads a wrong value. }
module elem_bases_data;

type
  rec12 = record a, b, c: integer32; end;
  rec56 = record a: integer32; z: array[1..13] of integer32; end;
  bytes = array[0..15] of char;
  bytes_ptr = ^bytes;
var
  eb_$words: array[1..8] of integer;
  eb_$recs: array[0..5] of rec12;
  eb_$big: array[1..4] of rec56;
  eb_$bytes: bytes;
  eb_$ptr: bytes_ptr;
  eb_$k: integer;
  eb_$neg: array[-5..5] of integer;
  eb_$longs: array[0..9] of integer32;
define eb_$words, eb_$recs, eb_$big, eb_$bytes, eb_$ptr, eb_$k, eb_$neg, eb_$longs;

procedure eb_$fill;
var
  i: integer;
  l: integer32;
begin
  i := 1;
  while i <= 8 do begin
    eb_$words[i] := i * 64 - 1000;
    i := i + 1;
  end;
  i := 0;
  while i <= 5 do begin
    eb_$recs[i].a := i * 10 + 1;
    eb_$recs[i].b := i * 10 + 2;
    eb_$recs[i].c := i * 10 + 3;
    i := i + 1;
  end;
  i := 1;
  while i <= 4 do begin
    eb_$big[i].a := i * 1000;
    eb_$big[i].z[13] := -i;
    i := i + 1;
  end;
  i := 0;
  while i <= 15 do begin
    eb_$bytes[i] := chr(i * 16 + 15);
    i := i + 1;
  end;
  i := -5;
  while i <= 5 do begin
    eb_$neg[i] := i * 3 - 1;
    i := i + 1;
  end;
  i := 0;
  while i <= 9 do begin
    l := i * 100000;
    eb_$longs[i] := l;
    i := i + 1;
  end;
  eb_$ptr := addr(eb_$bytes);
  eb_$k := 3;
end;
