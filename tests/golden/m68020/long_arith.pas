{ codegen-patterns 14 ("Longword multiply and divide"); bead
  domain-pascal-vzzh.  With `-cpu mathchip' the M$ARITH calls of the
  MC68010 code are MC68020 instructions:

  - `div' by a longword constant: `move.l D0,D1; divs.l #250000,D1',
    the quotient in the dividend's register, which is copied as the
    dividend is read again (SAU9 TIME_$GET_ADJUST 0x3C418B1E);
  - `mod': `divsl.l #250000,Dr:Dq', the remainder in a new register
    (0x3C418B2A `divsl.l #250000,D2:D0'; the image reuses the dying
    dividend's register and takes D2, dpc copies and takes D1: open, bead
    domain-pascal-tbiu);
  - a longword times a word: the word extended into the register, the
    longword the source, `clr.l D1; move.w D2,D1; mulu.l D3,D1' (SAU9
    PCHIST_$CNTL 0x3C463E9A) and `move.w D1,D2; ext.l D2; muls.l
    (-528,A4),D2' (SAU9 BAT_$ALLOCATE 0x3C443092);
  - a `mod' by a word constant whose result is a word: the remainder's
    low word (SAU9 AREA_$CREATE_FROM 0x3C40905C `divul.l #11,D4:D5', of an
    unsigned dividend). }
module long_arith;
type
  time_$timeval_t = record
    sec: integer32;
    usec: integer32;
  end;
var
  time_$current_delta: extern integer32;
procedure time_$get_adjust(out delta: time_$timeval_t);
var
  current_delta: integer32;
begin
  current_delta := time_$current_delta;
  delta.sec := current_delta div 250000;
  delta.usec := (current_delta mod 250000) * 4;
end;
function pchist_$span(count: integer; size: integer32): integer32; options(val_param);
begin
  pchist_$span := count * size;
end;
function bat_$offset(step: integer; size: integer32): integer32; options(val_param);
begin
  bat_$offset := step * size;
end;
function area_$hash(uid: integer32): integer; options(val_param);
begin
  area_$hash := uid mod 11;
end;
