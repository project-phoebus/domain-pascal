{ Execution test (bead domain-pascal-vq0h; ref "Addr", p. 4-13): `addr' of a
  string constant and of a character constant is its cell, readable through the
  pointer.

  warns: best effort (not byte-identical): the address of a constant
  call: ADDRC_$RUN
  expect:
    104
    111
    10
}
module addrc;
%include 'exec.ins.pas';

const nl = chr(10);
type str5 = array[1..5] of char;
     pstr = ^str5;
     pchar = ^char;

procedure addrc_$run;
var p: pstr;
    b: pchar;
begin
  p := addr('hello');
  exec_$log_c(p^[1]);
  exec_$log_c(p^[5]);
  b := addr(nl);
  exec_$log_c(b^);
end;
