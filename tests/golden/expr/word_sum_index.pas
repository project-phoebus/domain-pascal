{ Golden: codegen-patterns 5.2 ("Computed indices"), 8 ("Narrowing and
  widening"); ../source finding 760.  An `integer' plus a `0..65535' word
  is an integer32 expression (4.3.1), but where only its low word or byte
  is used it is computed at that width, which gives the same bits:
  - as a computed index into an array whose bounds fit: `add.w', then the
    `.w' index (AST_$TOUCH_AREA SAU4 0xF803D0C `add.w D6,D4'; MST_$FORK
    SAU2 0xE73A62 `add.w D5,D0; add.w D0,D0');
  - stored into a byte: `add.b' (AST_$TOUCH_AREA SAU4 0xF803D74 `add.b
    D6,D0'). }
module word_sum_index;
type
  pinteger = 0..65535;
var
  b: extern [byte] 0..255;
procedure p(page: pinteger; n: integer);
  options(val_param);
var a: array[0..31] of integer32;
begin
  a[n + page] := 0;
  b := n + page - 1;
end;
