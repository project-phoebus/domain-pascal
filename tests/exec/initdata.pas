{ Execution test: initialised static data (ref 3.2.2, 3.11.1;
  codegen-patterns 11 "Initial values"): words, repeat counts, a variant
  by field name, a string shorter than its array, a boolean, an
  enumeration, `nil', a routine's `static' variable keeping its value
  between calls (the calls of one test share memory).

  call: INITDATA_$SHOW
  call: INITDATA_$COUNT -> w
  call: INITDATA_$COUNT -> w
  call: INITDATA_$COUNT -> w
  expect:
    mem 00 07 00 09 00 08 00 08
    mem 00 12 00 10 00 0b 00 0e
    mem 74 74 79 00 00 00 00 00
    true
    2
    0
    -1
    => 101
    => 102
    => 103
}
module initdata;
%include 'exec.ins.pas';

type
  st = record case integer of 1: (all: integer32); 2: (code: integer) end;
  colour = (red, green, blue);
var
  classes: array[0..3] of integer := [7, 9, 2 of 8];
  status: array[0..1] of st := [[all := 16#120010], [all := 16#B000E]];
  name: array[1..8] of char := 'tty';
  ok: boolean := true;
  hue: colour := blue;
  p: ^integer := nil;
  late: integer32 := -1;
  w: integer;
  l: integer32;

procedure initdata_$show;
begin
  exec_$log_mem(classes, 8);
  exec_$log_mem(status, 8);
  exec_$log_mem(name, 8);
  exec_$log_bool(ok);
  w := ord(hue);
  exec_$logv_w(w);
  if p = nil then w := 0 else w := 1;
  exec_$logv_w(w);
  exec_$log_l(late);
end;

function initdata_$count: integer;
var
  n: static integer := 100;
begin
  n := n + 1;
  initdata_$count := n;
end;
