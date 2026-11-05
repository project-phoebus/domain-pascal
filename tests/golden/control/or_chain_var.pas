{ codegen-patterns 7.1 ("A chain of comparisons of one variable"): an
  `or else' (or `and then') chain whose every term compares one variable
  in memory with a constant loads it once into a data register (DIR_$SET_ACL
  0xE52CE0 `move.l (-20,A6),D0; cmpi.l #0xf0003,D0; beq.s; cmpi.l
  #0xe000d,D0; bne.s'; 88 such chains against 4 in memory in SAU2's
  compiled code); a first term against 0 branches on the load's flags
  (SMD_$COND_INPUT_U 0xE6FA5C `move.b (-5,A6),D0; beq.s; cmpi.b
  #15,D0'); a parameter already held in a register is compared there. }
module orchain;
type st = record all: integer32; end;
var g: integer32; r: integer;
procedure q(var s: st); extern;

procedure frame_var;
var s: st;
begin
  q(s);
  if (s.all = 983043) or else (s.all = 917517) then r := 1;
end;

procedure unit_var;
begin
  if (g <> 1) and then (g <> 5) and then (g <> 7) then r := 2;
end;

procedure getc(var c: char); extern;

procedure zero_first;
var c: char;
begin
  getc(c);
  if (c = chr(0)) or else (c = chr(15)) then r := 3;
end;
