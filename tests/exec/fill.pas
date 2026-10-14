{ Execution test: `for' loops over another unit's arrays whose index is
  used for nothing else (codegen-patterns 7, 7.4: `movea.l #sym,A0', a
  `dbf' count, `clr.<s> (A0)+'): exactly the elements 1..n are cleared.
  (dpc does not yet compile such a loop with a run-time count.)

  link: fill_data.mod.pas
  call: FILL_$RUN
  expect:
    mem 00 00 00 00 00 00 00 00 00 00 ff ff ff ff ff ff
    mem 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff
}
module fill;
%include 'exec.ins.pas';

type
  words = array[1..8] of integer;
  longs = array[1..4] of integer32;
var
  fill_$wa: extern words;
  fill_$la: extern longs;

procedure fill_$reset; extern;

procedure fill_$run;
var
  i: integer;
begin
  fill_$reset;
  for i := 1 to 5 do
    fill_$wa[i] := 0;
  exec_$log_mem(fill_$wa, 16);
  for i := 1 to 3 do
    fill_$la[i] := 0;
  exec_$log_mem(fill_$la, 16);
end;
