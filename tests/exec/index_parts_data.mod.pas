{ The arrays of index_parts.pas, defined here and reached there as
  another unit's.  The fill steps a pointer through each array (no
  index), so every element's value says which element it is:
  ip_$tbl[k] = 7k + 3 (0..1023), ip_$t56[k] = 1000 + k (1..56),
  ip_$w8[k] = 10k (1..8), ip_$t8l[k] = 500 + k (1..8, longwords). }
module index_parts_data;

type
  pl = ^integer32;
  pw = ^integer;
var
  ip_$tbl: array[0..1023] of integer32;
  ip_$t56: array[1..56] of integer32;
  ip_$w8: array[1..8] of integer;
  ip_$t8l: array[1..8] of integer32;
define ip_$tbl, ip_$t56, ip_$w8, ip_$t8l;

procedure ip_$fill;
var
  k: integer32;
  q: pl;
  w: pw;
begin
  q := addr(ip_$tbl);
  k := 0;
  while k <= 1023 do begin
    q^ := 7 * k + 3;
    q := univ_ptr(integer32(q) + 4);
    k := k + 1;
  end;
  q := addr(ip_$t56);
  k := 1;
  while k <= 56 do begin
    q^ := k + 1000;
    q := univ_ptr(integer32(q) + 4);
    k := k + 1;
  end;
  w := addr(ip_$w8);
  k := 1;
  while k <= 8 do begin
    w^ := 10 * k;
    w := univ_ptr(integer32(w) + 2);
    k := k + 1;
  end;
  q := addr(ip_$t8l);
  k := 1;
  while k <= 8 do begin
    q^ := k + 500;
    q := univ_ptr(integer32(q) + 4);
    k := k + 1;
  end;
end;
