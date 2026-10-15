{ Execution test: byte-sized module variables (codegen-patterns 11,
  "Byte-sized module variables"; bead domain-pascal-yvko).  Each char
  or boolean static takes the high byte of a word of its own (SIO_IO's
  WIRED_DATA 0xE2DEB8+0x48: `70 00 60 00'), initialised or not, unit
  level or a routine's `static'; stores to one never touch its
  neighbours, and the pad bytes stay zero.

  call: BYTE_STATICS_$MAIN b:65 b:255
  call: BYTE_STATICS_$MAIN b:66 b:0
  expect:
    mem 70 00 60 00 00 0e 00 00
    mem 41 00 ff 00 00 0e 00 00
    1
    65
    mem 70 00 60 00 00 0e 00 01
    mem 42 00 00 00 00 0e 00 00
    2
    66
}
module byte_statics;
%include 'exec.ins.pas';

var
  stop: char := chr(16#70);
  start: char := chr(16#60);
  w: integer := 14;
  last: integer;
  c1: char;
  f1: boolean;
  w2: integer := 14;
  l2: integer;

function count: integer;
var
  n: static char;
begin
  n := chr(ord(n) + 1);
  count := ord(n);
end;

procedure byte_statics_$main(c: char; f: boolean); options(val_param);
begin
  exec_$log_mem(stop, 8);
  c1 := c;
  f1 := f;
  exec_$log_mem(c1, 8);
  last := count;
  exec_$logv_w(last);
  exec_$logv_c(c1);
end;
