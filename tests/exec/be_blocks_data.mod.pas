{ The objects of be_blocks.pas in a unit of their own (another unit's
  variable and arrays for it), and a routine that fills them: each
  longword holds a value made from its place, so a wrong address reads
  another value. }
module be_blocks_data;

type
  rec = record a, b: integer32; end;
  r12 = record a, b, c: integer32; end;
  outer = record x: integer32; r: rec; t: r12; end;

var
  bb_$orec: outer;
  bb_$oarr: array[1..4] of rec;
  bb_$oarr12: array[1..4] of r12;
define bb_$orec, bb_$oarr, bb_$oarr12;

procedure bb_$fill;
var
  i: integer;
begin
  bb_$orec.x := 1000;
  bb_$orec.r.a := 1001;
  bb_$orec.r.b := 1002;
  bb_$orec.t.a := 1003;
  bb_$orec.t.b := 1004;
  bb_$orec.t.c := 1005;
  i := 1;
  while i <= 4 do begin
    bb_$oarr[i].a := i * 100 + 1;
    bb_$oarr[i].b := i * 100 + 2;
    bb_$oarr12[i].a := i * 100 + 11;
    bb_$oarr12[i].b := i * 100 + 12;
    bb_$oarr12[i].c := i * 100 + 13;
    i := i + 1;
  end;
end;
