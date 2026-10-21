{ Execution test (bead domain-pascal-jgtd): a string constant passed by
  address to an `in' (or value) parameter that is an array of char, not
  `univ', is the value an assignment to a variable of the parameter's
  type gives: its characters, then blanks to the array's size (ref
  "Variable-Length String Operations", p. 4-204, and "Passing
  Variable-Length Parameters", p. 4-207).  SAU5 TAPE_$BOOT's
  tape_$check_block 0xE33F12 passes an 80-character message to a
  200-character `in' formal as the cell 0xE33FA0..0xE34068, the text and
  120 blanks; TAPE_$BOOT passes its 16-character name constant to a
  16-character `in' formal three times as one cell, 0xE346DE.  The
  callees log the parameter's bytes, and the length the constant gives
  them; a 5-character array (an odd size) gets 'abc' and two blanks.

  call: STR_CONST_ARG_$RUN
  expect:
    mem 62 73 63 6f 6d 2f 72 62 61 6b 5f 73 68 65 6c 6c
    16
    mem 62 73 63 6f 6d 2f 72 62 61 6b 5f 73 68 65 6c 6c
    16
    mem 73 68 6f 72 74 20 20 20 20 20 20 20 20 20 20 20
    5
    mem 61 62 63 20 20
    3
    mem 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20
    195
    mem 78 79 20 20 20
    2
    mem 61 62 63 64 65
    5
}
module str_const_arg;
%include 'exec.ins.pas';

type
  name_t = array[1..16] of char;
  odd_t = array[1..5] of char;
  long_t = array[1..200] of char;

const
  shell_name = 'bscom/rbak_shell';

{ The number of characters before the trailing blanks. }
function used(in s: univ exec_$bytes; n: integer): integer;
  options(val_param);
begin
  while (n > 0) and (s[n] = ' ') do
    n := n - 1;
  used := n;
end;

procedure take_name(in s: name_t; len: integer); options(val_param);
begin
  exec_$log_mem(s, len);
  exec_$logv_w(used(s, 16));
end;

procedure take_long(in s: long_t);
begin
  exec_$log_mem(s, 16);
  exec_$logv_w(used(s, 200));
end;

procedure take_odd(in s: odd_t);
begin
  exec_$log_mem(s, 5);
  exec_$logv_w(used(s, 5));
end;

procedure take_tail(in s: long_t);
var t: exec_$bytes;
    k: integer;
begin
  for k := 1 to 16 do
    t[k] := s[184 + k];
  exec_$log_mem(t, 16);
  exec_$logv_w(200 - used(s, 200));
end;

procedure str_const_arg_$run;
begin
  take_name(shell_name, 16);
  take_name(shell_name, 16);
  take_long('short');
  take_odd('abc');
  take_tail('short');
  take_odd('xy');
  take_odd('abcde');
end;
