{ Execution test (bead domain-pascal-vq0h; codegen-patterns 4, "Main
  programs"): a main program's body runs as an exported routine with its
  procedures inheriting A5.  They take a small value parameter by value and a
  large one by address, as nested routines do; its variables are the data
  area's, and another unit's are reached through cells below the data base.

  link: program_main.mod.pas
  call: PROGRAM_MAIN
  call: PM_$COUNT -> l
  call: PM_$TARGET_B -> l
  call: PM_$LOG -> l
  call: PM_$NOTED -> l
  expect:
    => 47
    => 22
    => 8
    => 37
}
program program_main;

type pair = record a, b: integer32; end;
     four = record a, b, c, d: integer32; end;

var total: integer32 := 40;
    local: pair;
    copy4: four;
    count: extern integer32;
    source: extern pair;
    target: extern pair;
    block: extern four;
    log: extern integer32;

procedure pm_$add(in p: pair; in n: integer32); extern;

procedure bump(by: integer);
begin
  total := total + by;
end;

procedure keep(p: pair; times: integer);
begin
  local := p;
  log := copy4.d * 2 + times - 2;
  pm_$add(source, 4);
end;

begin
  bump(2);
  copy4 := block;
  keep(source, 2);
  target := local;
  count := total + count;
end.
