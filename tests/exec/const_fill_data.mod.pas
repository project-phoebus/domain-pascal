{ The arrays of const_fill.pas's fills of another unit's arrays, in a
  unit of their own: a 1-based longword array between guard words in a
  record (filled from an element other than its first), and a 0-based
  word array that is a whole variable (filled from its first byte) with
  a guard variable on each side. }
module const_fill_data;

type
  cfl_$xrec = record
    g0: integer;
    l: array[1..6] of integer32;
    g1: integer;
  end;
var
  cfl_$x: cfl_$xrec;
  cfl_$xg0: integer;
  cfl_$xw: array[0..4] of integer;
  cfl_$xg1: integer;
define cfl_$x, cfl_$xg0, cfl_$xw, cfl_$xg1;
