{ The record of with_field_array.pas reached as another unit's object
  (`movea.l #sym,An', SAU5 MT_PURGE_UCB 0xE7577E). }
module with_field_array_ext;

type
  unit_t = record
    head: integer32;
    flags: set of 0..15;
    cnt: integer;
  end;
  scb_t = record
    pad: array[1..30] of integer;
    units: array[0..2] of unit_t;
  end;
var
  wfa_$scb: scb_t;
define wfa_$scb;
