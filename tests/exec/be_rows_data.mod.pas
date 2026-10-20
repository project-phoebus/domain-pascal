{ The other unit of be_rows.pas: a two-dimensional array of records that
  be_rows.pas reaches as another unit's object (as AST_PMAPS for the
  PMAP_ routines, FILE_$LOCK_TABLE for the FILE_ ones). }
module be_rows_data;

type
  ent_t = record
    a: integer32;
    flags: set of 0..15;
  end;
var
  rows_$tbl: array[1..5] of array[0..3] of ent_t;
  rows_$mode: array[0..2, 0..3] of integer;
define rows_$tbl;
define rows_$mode;
