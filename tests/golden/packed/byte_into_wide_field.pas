{ codegen-patterns 5.3 ("Wider field, value"); bead domain-pascal-nvqu.
  A `[byte] 0..255' stored into a 31-bit packed field (bits 30..0 of a
  longword after a boolean) is widened like a conversion, `clr.l Dn;
  move.b x,Dn', then merged: MST_$LOOKUP_OBJECT 0xE43D52 `clr.l D1;
  move.b (-12,A6),D1; andi.l #0x80000000,(A3); or.l D1,(A3)' (../source
  mst/lookup_object.pas).  A byte that fills an 8-bit field takes
  `move.b; andi.l #255' instead (0xE0086E).  The 31-bit subrange after
  the boolean takes its 31 bits, so the record is 4 bytes (SAU9
  ast_$load_aote 0x3C403ADA `bfextu (A0)' at offset 1, width 31). }
module byte_into_wide_field;
type
  loc_t = record
    unused_00: array[0..27] of char;
    rights_byte: [byte] 0..255;
    flags_byte: [byte] 0..255;
    unused_1e: integer;
  end;
  local_t = packed record
    remote: boolean;
    low31: 0..16#7FFFFFFF;
  end;
  location_t = packed record case integer of
    1: (all: integer32);
    2: (local: local_t);
  end;

procedure t(out location: location_t);
var loc: loc_t; flags: set of 0..15;
begin
  location.all := 0;
  if not (7 in flags) then
    location.local.low31 := loc.rights_byte;
end;
