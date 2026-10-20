{ The arrays of be_u_kinds.pas, in a unit of their own (be_u_kinds.pas
  reaches them as another unit's, codegen-patterns 5.2, "Another unit's
  array"), and a routine that fills them: element k holds values made
  from k so that a wrong index, scale or displacement reads a wrong
  value.  An element is 248 bytes, so the lower bound puts it at a
  displacement beyond a byte. }
module be_u_data;

type
  blk = record a: integer32; pad: array[1..60] of integer32; z: integer32; end;
  blks = array[1..4] of blk;
  blks_ptr = ^blks;
var
  bu_$blks: blks;
  bu_$bp: blks_ptr;
define bu_$blks, bu_$bp;

procedure bu_$fill;
var
  i: integer;
begin
  i := 1;
  while i <= 4 do begin
    bu_$blks[i].a := i * 10;
    bu_$blks[i].z := i * 100 + 7;
    i := i + 1;
  end;
  bu_$bp := addr(bu_$blks);
end;
