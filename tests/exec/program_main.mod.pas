{ The other unit of tests/exec/program_main.pas: the external variables a
  main program reaches through cells, and calls to read them back. }
module program_main_data;

type pair = record a, b: integer32; end;
     four = record a, b, c, d: integer32; end;

var count: define integer32 := 5;
    source: define pair := [a := 11, b := 22];
    target: define pair;
    block: define four := [a := 1, b := 2, c := 3, d := 4];
    log: define integer32;
    noted: define integer32 := 0;

procedure pm_$add(in p: pair; in n: integer32);
begin
  noted := p.a + p.b + n;
end;

function pm_$count: integer32;
begin
  pm_$count := count;
end;

function pm_$target_b: integer32;
begin
  pm_$target_b := target.b;
end;

function pm_$log: integer32;
begin
  pm_$log := log;
end;

function pm_$noted: integer32;
begin
  pm_$noted := noted;
end;
