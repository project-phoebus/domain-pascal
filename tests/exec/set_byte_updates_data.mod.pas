{ The other unit of set_byte_updates.pas: a record with a word set that
  set_byte_updates.pas reaches as another unit's object (absolute
  operands, beads domain-pascal-fozq, domain-pascal-wrzd). }
module sbu_data;

type
  scb_t = record
    lo: integer;
    flags: set of 0..15;
    hi: integer;
  end;
var
  sbu_$scb: scb_t;
define sbu_$scb;
