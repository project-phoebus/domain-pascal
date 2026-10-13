{ Golden: codegen-patterns 1.5 "Constant by address" and "Expression by
  address", 11 (constants in the code section); bead domain-pascal-5aw.9.

  - A constant passed by address is a cell in the code section, `pea
    (d,PC)', placed right after the code of the routine that uses it
    first (TERM_$SEND_KBD_STRING 0xE1AB0C `pea (0x18,PC)', cell 0xE1AB26);
    the same cell twice in a row is `move.l (SP),-(SP)' (0xE1AB10).
  - A later routine names the earlier cell of an equal constant of the
    same type (SMD_$INVERT_S 0xE6DDE4 -> SMD_$ASSOC's 0xE6D92C); one of
    another type gets a cell of its own.
  - A routine's new cells: those of 2 mod 4 bytes, then those of a
    multiple of 4 (after a 0x2048 filler in an aligning routine, LOG_$SHUTDN
    0xE17606); in each group the first used first, the rest in reverse
    order of use (TIMER_$INIT 0xE1639C).
  - An expression passed by address goes through a frame temporary below
    the locals (TONE_$ENABLE 0xE1ACFC). }
module cpool;
type
  color = (red, green, blue);
var
  x: integer32;

procedure ext_$two(a: integer; b: integer); extern;
procedure ext_$long(a: integer32; b: integer); extern;
procedure ext_$color(c: color); extern;
procedure ext_$ptr(p: univ_ptr; in q: integer); extern;

{ TIMER_$INIT's order: used 1046, 1, -1, 2, 3; laid out 1046, 3, 2, -1, 1 }
procedure cpool_$order;
begin
  ext_$two(1, 16#1046);
  ext_$two(2, -1);
  ext_$two(3, -1);
end;

{ the word first, then the longword after the filler; the same word twice
  with move.l (SP),-(SP) }
procedure cpool_$groups;
begin
  ext_$long(0, 4);
  ext_$two(7, 7);
end;

{ the earlier cells reused; an enumeration's 1 gets its own cell }
procedure cpool_$reuse;
begin
  ext_$two(1, 7);
  ext_$color(green);
end;

{ an address through a frame temporary, a constant beside it }
procedure cpool_$temp;
var s: integer;
begin
  ext_$ptr(addr(x), 5);
end;
