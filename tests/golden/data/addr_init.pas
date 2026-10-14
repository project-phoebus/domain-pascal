{ codegen-patterns 11 ("Initial values", addresses); beads
  domain-pascal-bjz, owner's ruling on 3.13.4 p. 3-51
  (docs/reference-clarifications.md "3.13.4 addr(...) as an initial
  value").  `addr(...)' initial values are relocated longs in the data
  block: a routine's symbol (an EPV as in Apollo's hardcopy
  driver_init_ex.pas), another unit's symbol plus an offset, and the
  unit's own data area plus an offset for its variables, `static' (no
  symbol) or `define'd.  The image's compiled modules keep their
  routines' entry addresses as plain longs in their `D' segments: FLP_
  0xE7AEF4 (+0x04 FLP_$SHUTDOWN 0xE3E228 .. +0x10 FLP_$DO_IO), WIN_
  0xE2B89C (+0x10 WIN_$SPIN_DOWN 0xE19BC0). }
module addri;
type
  proc_t = ^procedure(in x: integer32);
  epv_t = record
    init: proc_t;
    monitor: proc_t;
    data: ^integer32;
  end;
procedure addri_$ext(in x: integer32); extern;
procedure addri_$init(in x: integer32); forward;
var
  counter: integer32;
  table: array[1..3] of integer32;
  other: extern integer32;
  ext: extern epv_t;
  epv: epv_t := [init := addr(addri_$init), monitor := nil, data := addr(counter)];
  pp: ^integer32 := addr(table[2]);
  qq: ^integer32 := addr(other);
  rr: univ_ptr := addr(ext.data);
  ss: proc_t := addr(addri_$ext);
  dd: extern ^integer32;
  flag: integer := 7;
define dd := addr(table[3]);

procedure addri_$init;
begin
  counter := x;
end;
