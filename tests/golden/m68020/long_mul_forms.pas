{ codegen-patterns 14 ("Longword multiply and divide"); bead
  domain-pascal-mhzy.  Two MC68020 multiply forms:

  - a constant factor in a word's range (the word `M$MI*$LLW' would take)
    is loaded into the product's register, the other factor the source:
    SAU6 MT_$WAIT 0x7807D67A `move.l #1000,D1; muls.l D3,D1'; SAU9
    SCSI_$C90_INT 0x3C4A5CC8 `moveq #100,D1; mulu.l D0,D1' (a longword
    constant is the immediate, `muls.l #60296,D1' at 0x3C416EC0);
  - a variable squared is loaded once: SAU9 OS_$SHUTDOWN 0x3C47422C
    `move.l (-596,A6),D1; muls.l D1,D1'. }
module long_mul_forms;
function mt_$ms(t: integer32): integer32; options(val_param);
begin
  mt_$ms := t * 1000;
end;
procedure os_$square(var y: integer32);
var
  r: record
    pad: integer32;
    x: integer32;
  end;
begin
  r.x := r.x * r.x;
  y := r.x;
end;
