{ Golden: an `if' with an empty `then' part and no `else' (ref
  "Statements", p. 4-188: the empty statement "causes no action"): no test, no branch; the condition's operands
  evaluated for nothing.  SAU8 PBU2_$ASSOC 0x3C6169E `bfextu (0x10,A2)
  at offset 5, width 3, into D5', the unit's bus extracted and never tested (D5 the
  next loop's index, 0x3C616B0): `if dcte^.unit_info.bus <> io_$bus_vme
  then ;' with its Multibus arm configured out (../source
  pascal-translation-notes findings 700, 702).  A variable in memory
  leaves no instruction; a function in the condition is called.
}
module empty_then;
type
  info_t = packed record
    unit: 0..31;
    bus: 0..7;
    rest: 0..255;
  end;
  dcte_t = record
    pad: array[0..3] of integer32;
    unit_info: info_t;
  end;
var
  w: extern integer;
  dcte: extern ^dcte_t;
function ready: boolean; extern;
define assoc;
procedure assoc;
begin
  if dcte^.unit_info.bus <> 1 then
    ;
  if w <> 1 then begin end;
  if ready then ;
  w := 3;
end;
