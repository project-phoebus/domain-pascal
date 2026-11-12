{ Execution test (bead domain-pascal-338r, domain-c's gap 34): string
  constants assigned to packed arrays of char, blank-padded to the
  array's size (ref p. 4-204): 9 and 7 characters copied from a cell, 4
  and 2 stored as constants (the DOMAIN C compiler's dmp.pas, reported:
  `move.l #0x4A425020,(-36,A6)' for 'JBP ').  Best effort, no kernel
  site.  Also a string constant passed by address to an `in' packed
  array of char (gap 41): a cell of its characters and blanks.

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
    65
    32
    -1
}
module string_packed;
%include 'exec.ins.pas';

type
  n4 = packed array [1..4] of char;
var
  a: packed array [1..9] of char;
  c: packed array [1..8] of char;
  b: packed array [1..4] of char;
  w: packed array [1..2] of char;

procedure show(in n: n4; k: integer32); options(val_param);
begin
  exec_$logv_w(ord(n[1]));
  exec_$logv_w(ord(n[4]));
  exec_$logv_l(k);
end;

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
  show('AP', -1);
end;
