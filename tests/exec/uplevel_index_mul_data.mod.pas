{ The unit tables of uplevel_index_mul.pas reached as another unit's
  (MT_$SCB's units, sau5 MT_$ACQUIRE's nested routine 0xE75C0C),
  46-byte elements; ui_$big's scaled bounds pass 32767 (a `.l' index). }
module uplevel_index_mul_data;

type
  u46 = record a: integer; pad: array[1..21] of integer; z: integer; end;
var
  ui_$units: array[0..3] of u46;
  ui_$big: array[-800..800] of u46;
define ui_$units, ui_$big;
