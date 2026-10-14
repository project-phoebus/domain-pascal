{ Execution test: `addr(...)' as an initial value (owner's ruling on
  3.13.4, p. 3-51; docs/reference-clarifications.md "3.13.4 addr(...)
  as an initial value"; bead domain-pascal-bjz; codegen-patterns 11
  "Initial values").  Calls through routine-pointer fields of an
  initialised EPV record (as in Apollo's hardcopy driver_init_ex.pas),
  and reads and writes through pointers initialised with the address of
  a unit variable, of an element at a constant index, of a routine's
  `static' variable, of another unit's array element (fill_data's
  `define'd fill_$la, an extern here), and of a whole longword field of
  a packed record (ref 3.10.7).

  link: fill_data.mod.pas
  call: ADDRI_$RUN
  expect:
    1007
    -5
    mark 3
    42
    42
    20
    9
    11
    77
}
module addri;
%include 'exec.ins.pas';

type
  setter_t = ^procedure(in v: integer32);
  marker_t = ^procedure(v: integer); options(val_param);
  epv_t = record
    setter: setter_t;
    mark: marker_t;
    target: ^integer32;
  end;
  longs = array[1..4] of integer32;
  pk_t = packed record b: 0..7; c: char; d: integer32; end;

procedure addri_$set(in v: integer32); forward;

var
  counter: integer32;
  table: array[1..3] of integer32 := [7, 8, 9];
  fill_$la: extern longs;
  epv: epv_t := [setter := addr(addri_$set), mark := addr(exec_$mark),
                 target := addr(counter)];
  third: ^integer32 := addr(table[3]);
  far: ^integer32 := addr(fill_$la[2]);
  pkr: pk_t;
  pd: ^integer32 := addr(pkr.d);   { a whole longword field of a packed record }

procedure addri_$set;
begin
  counter := v + 1000;
end;

procedure addri_$run;
var
  keep: static integer32 := 42;
  pk: static ^integer32 := addr(keep);
begin
  epv.setter^(7);
  exec_$log_l(counter);
  epv.target^ := -5;
  exec_$log_l(counter);
  epv.mark^(3);
  exec_$logv_l(pk^);
  exec_$log_l(keep);
  far^ := 20;
  exec_$logv_l(fill_$la[2]);
  exec_$logv_l(third^);
  third^ := 11;
  exec_$log_l(table[3]);
  pd^ := 77;
  exec_$log_l(pkr.d);
end;
