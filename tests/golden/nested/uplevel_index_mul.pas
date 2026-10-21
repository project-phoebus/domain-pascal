{ Golden: codegen-patterns 8 ("Multiplication by a constant"), 5.2
  ("Scalings by element size"), 6 (bead domain-pascal-dctl): another
  unit's array of 46-byte records indexed by the enclosing routine's
  local, in the shape of sau5 MT_$ACQUIRE's nested routine at 0xE75C0C
  (`with mt_$scb, units[u]', ../source mt/acquire.pas).

  - 0xE75C14..0xE75C22: `movea.l (A6),A2' (the dynamic link: only its
    parent calls it), `moveq #46,D0; movea.l #MT_$SCB,A3; mulu.w
    (-0x2a,A2),D0; lea (0,A3,D0.w),A4': the constant first, then the
    array's base, then the multiply with the parent's local read through
    the link (a `.w' index: `mulu.w').  The fields then off the element
    register (0xE75C26 `bset #0,(68,A4)'; 0xE75C3C `addq.w #1,(48,A3)').
  - `big': a `.l' index (the array's scaled bounds pass 32767) of the
    same signed local: `muls.w', its 32-bit product (TPAD_$DATA
    0xE691D0's form through the link). }
module ui;
type
  unit_t = record
    flags: integer;
    count: integer;
    pad: array[1..20] of integer;
    mode: integer;
  end;
var
  units: extern array[0..1] of unit_t;
  big: extern array[-800..800] of unit_t;

procedure ui_$acquire(in unit: integer);
var
  u: integer;
  procedure init_unit;
  begin
    with units[u] do begin
      count := count + 1;
      mode := 0;
    end;
  end;
  procedure init_big;
  begin
    big[u].mode := 1;
  end;
begin
  u := unit;
  init_unit;
  init_big;
end;
