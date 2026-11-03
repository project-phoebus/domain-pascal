{ codegen-patterns 14.1 ("The DSP160, DN460 and DN660"); bead
  domain-pascal-mhzy.  With `-cpu 460' the M$ARITH calls of a longword `*'
  and `div' are the DN460's extended instructions, emitted as `.short';
  `mod' still calls M$ARITH:

  - TIME_$GET_ADJUST (SAU4 0xF817466, byte for byte): `move.l D1,D0;
    717c 0003d090' `divs.l #250000,D0', then `M$OIS$LLL'; current_delta
    is held in D1 through the M$ARITH call that reads it last (12.4 R4(c));
  - a longword constant factor is the immediate (0xF8172EE `753c',
    `muls.l #250000,D2');
  - a word divisor and factor extended into registers (0xF81732E..
    0xF81733C `7544' `divs.l D4,D2', `7102' `muls.l D2,D0');
  - a factor in a word's range in the product's register (MT_$WAIT
    0xF87DBDC `move.l #1000,D1; 7303' `muls.l D3,D1');
  - a variable squared, loaded once (OS_$SHUTDOWN 0xF877144 `7301'
    `muls.l D1,D1');
  - unsigned: a constant divisor in a register (PMAP_$PURIFIER_R
    0xF814A9A `moveq #6,D2', `82ca' `divu.l D2,D1'), a word factor
    zero-extended (PCHIST_$CNTL 0xF8669CA `c2cb' `mulu.l D3,D1'), a
    longword in memory loaded (AST_$GET_SEG_MAP 0xF8071A2 `8ac9'
    `divu.l D1,D5'; PCHIST_$UNIX_PROFIL_ADDUPC 0xF866B16 `c4c9' `mulu.l
    D1,D2', the factor loaded into the product's register). }
module long_arith;
type
  time_$timeval_t = record
    sec: integer32;
    usec: integer32;
  end;
  card16 = 0..65535;
  card32 = 0..16#7FFFFFFF;
var
  time_$current_delta: extern integer32;
  pmap_$x: extern card32;
  pmap_$q: extern card32;
procedure time_$get_adjust(out delta: time_$timeval_t);
var
  current_delta: integer32;
begin
  disable;
  current_delta := time_$current_delta;
  enable;
  delta.sec := current_delta div 250000;
  delta.usec := (current_delta mod 250000) * 4;
end;
function time_$scale(x: integer32): integer32; options(val_param);
begin
  time_$scale := x * 250000;
end;
function time_$round(x: integer32; w: integer): integer32; options(val_param);
begin
  time_$round := (x div w) * w;
end;
function mt_$ms(t: integer32): integer32; options(val_param);
begin
  mt_$ms := t * 1000;
end;
procedure os_$square;
var
  r: record
    pad: integer32;
    x: integer32;
  end;
begin
  r.x := r.x * r.x;
  pmap_$q := r.x;
end;
procedure pmap_$purify;
begin
  pmap_$q := pmap_$x div 6;
end;
function pchist_$span(count: card16; size: card32): card32; options(val_param);
begin
  pchist_$span := count * size;
end;
function ast_$quot(a: card32; var b: card32): card32; options(val_param);
begin
  ast_$quot := a div b;
end;
function pchist_$scaled(pc: card32; var scale: card32): card32; options(val_param);
begin
  pchist_$scaled := pc * scale;
end;
