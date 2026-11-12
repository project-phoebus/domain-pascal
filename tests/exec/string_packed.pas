{ Execution test (bead domain-pascal-338r, domain-c's gap 34): string
  constants assigned to packed arrays of char, blank-padded to the
  array's size (ref p. 4-204): 9 and 7 characters copied from a cell, 4
  and 2 stored as constants (the DOMAIN C compiler's dmp.pas, reported:
  `move.l #0x4A425020,(-36,A6)' for 'JBP ').  Best effort, no kernel
  site.

  call: STRING_PACKED_$RUN
  expect:
    101
    102
    97
    32
    74
    32
    111
    32
}
module string_packed;
%include 'exec.ins.pas';

var
  a: packed array [1..9] of char;
  c: packed array [1..8] of char;
  b: packed array [1..4] of char;
  w: packed array [1..2] of char;

procedure string_packed_$run;
begin
  a := 'externdef';
  exec_$logv_w(ord(a[1]));
  exec_$logv_w(ord(a[9]));
  c := 'abc';
  exec_$logv_w(ord(c[1]));
  exec_$logv_w(ord(c[8]));
  b := 'JBP';
  exec_$logv_w(ord(b[1]));
  exec_$logv_w(ord(b[4]));
  w := 'o';
  exec_$logv_w(ord(w[1]));
  exec_$logv_w(ord(w[2]));
end;
