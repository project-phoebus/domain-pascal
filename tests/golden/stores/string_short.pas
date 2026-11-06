{ Golden: codegen-patterns 5.2, "String constants": a whole array of 2, 4 or 6
  bytes in the frame or the data, by displacement: /com/lusr 0x8D48 `move.w`,
  /com/calendar 0x883C and /com/tctl 0x9E9E `move.l`, /com/tctl 0xA108
  'Seven' `move.l`, `move.w`, the text padded with blanks. }
module string_short;

type name2 = array[1..2] of char;
     name4 = array[1..4] of char;
     name6 = array[1..6] of char;
     zone = record delta: integer; name: name4; end;

var two: name2;
    four: name4;
    six: name6;

procedure p;
var z: zone;
    l: name4;
begin
  two := '  ';
  four := '1200';
  six := 'Seven';
  z.name := '    ';
  l := 'abc';
  four := 'x';
  two := 'y';
  z.delta := 0;
end;
