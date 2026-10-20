{ Execution test (bead domain-pascal-5kgg): assignments of arrays whose
  copy kind the image does not settle (bead domain-pascal-4lf1:
  arrays of char byte by byte, records by longwords; codegen-patterns
  5.2, "Arrays of char"), copied on the lowering's best-effort path:
  arrays of one-byte elements other than char (`[byte] 0..255') of 4,
  6 and 10 bytes (3: refused, the longword copy of an odd size), a packed array of char, and an array of a subrange
  of char.  Every byte arrives (ref "Assignment", p. 4-25).

  call: BE_L_COPIES_$RUN
  warns: best effort (not byte-identical): assigning an array of one-byte elements other than char
  warns: best effort (not byte-identical): assigning a packed array of char
  warns: best effort (not byte-identical): assigning an array of a subrange of char
  expect:
    mem 01 02 03 04
    mem 01 02 03 04 05 06
    mem 01 02 03 04 05 06 07 08 09 0a
    mem 61 62 63 64 65 66
    mem 41 42 43 44
}
module be_l_copies;
%include 'exec.ins.pas';
type
  ub = [byte] 0..255;
  b4 = array[1..4] of ub;
  b6 = array[1..6] of ub;
  b10 = array[1..10] of ub;
  pc6 = packed array[1..6] of char;
  uc = 'A'..'Z';
  u4 = array[1..4] of uc;
var
  s4, d4: b4;
  s6, d6: b6;
  s10, d10: b10;
  sp, dp: pc6;
  su, du: u4;

procedure be_l_copies_$run;
var i: integer;
begin
  for i := 1 to 4 do s4[i] := i;
  for i := 1 to 6 do s6[i] := i;
  for i := 1 to 10 do s10[i] := i;
  sp[1] := 'a'; sp[2] := 'b'; sp[3] := 'c';
  sp[4] := 'd'; sp[5] := 'e'; sp[6] := 'f';
  su[1] := 'A'; su[2] := 'B'; su[3] := 'C'; su[4] := 'D';
  d4 := s4;
  d6 := s6;
  d10 := s10;
  dp := sp;
  du := su;
  exec_$log_mem(d4, 4);
  exec_$log_mem(d6, 6);
  exec_$log_mem(d10, 10);
  exec_$log_mem(dp, 6);
  exec_$log_mem(du, 4);
end;
