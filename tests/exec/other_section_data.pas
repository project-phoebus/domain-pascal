{ Execution test: a routine of one data section writing and reading the
  module's block of another (codegen-patterns 11, "Data per section";
  bead domain-pascal-a2g): `movea.l #block,An' and `(d,An)', the block
  loaded again after each call (the stubs clobber A0/A1), `addr' of a
  variable of the block (`lea (d,An),Am'), the block's first variable
  passed by address (`move.l #block,-(SP)'); a routine of the block's own
  section reads the values back through A5.

  call: OSD_$INIT
  call: OSD_$SHOW
  expect:
    7
    300
    -2
    300
    7
    300
    -2
    true
}
module osd, wired_proc, wired_data;
%include 'exec.ins.pas';

type
  pair = record
    a: integer;
    b: integer32;
  end;
var
  first: integer32;             { WIRED_DATA +0 }
  pr: pair;                     { +4 }
  last: integer;                { +0xC }
  ptr: ^pair;                   { +0x10 }

[section(init_proc, init_data)]
procedure osd_$init;
begin
  pr.a := 300;
  first := 7;
  pr.b := -2;                   { not next to pr.a (codegen-patterns 13,
                                  "Store merging") }
  exec_$log_l(first);
  exec_$log_w(pr.a);
  exec_$log_l(pr.b);
  last := 1;
  ptr := addr(pr);
end;

procedure osd_$show;
begin
  exec_$logv_w(ptr^.a);
  exec_$logv_l(first);
  exec_$logv_w(pr.a);
  exec_$logv_l(pr.b);
  exec_$log_bool(last = 1);
end;
