{ Execution test: bodies of one `if', `case' or loop statement that read
  by-value parameters, which are loaded at entry (codegen-patterns 12.2,
  "One-statement bodies"; bead domain-pascal-s4a5): a boolean tested on
  its load's flags (TTY_$I_ENABLE_CRASH_FUNC 0xE672A0), a parameter read
  in the condition and in the arms across calls, a `case' function on a
  char (KBD_$RCV's nested 0xE1CC64), a `for' bounded by one (DBUF_$UPDATE_VOL
  0xE3AAA2), a `while' and a `repeat' stepping by one.  Not expressed
  (refused by dpc for reasons of their own): a parameter compared with
  another or with a variable in memory, a `for' from a parameter, a
  parameter assigned to the result of a one-statement function (bead
  domain-pascal-0zo8).  Assigning to a parameter: tests/exec/param_assign.pas.

  call: ONE_STMT_PARAMS_$ENABLE @l:0 b:65 b:255
  call: ONE_STMT_PARAMS_$ENABLE @l:0 b:65 b:0
  call: ONE_STMT_PARAMS_$PICK w:-4 w:9
  call: ONE_STMT_PARAMS_$SHOW
  call: ONE_STMT_PARAMS_$PICK w:7 w:2
  call: ONE_STMT_PARAMS_$SHOW
  call: ONE_STMT_PARAMS_$PICK w:3 w:-3
  call: ONE_STMT_PARAMS_$XLATE b:145 -> c
  call: ONE_STMT_PARAMS_$XLATE b:152 -> c
  call: ONE_STMT_PARAMS_$XLATE b:66 -> c
  call: ONE_STMT_PARAMS_$COUNT w:3 w:4
  call: ONE_STMT_PARAMS_$COUNT w:5 w:0
  call: ONE_STMT_PARAMS_$GROW w:7
  call: ONE_STMT_PARAMS_$UPTO w:9
  expect:
    65
    255
    0
    0
    mark 1
    5
    7
    mark 1
    => 24
    => 9
    => 0
    3
    6
    9
    12
    7
    14
    21
    12
    3
    -6
}
module one_stmt_params;
%include 'exec.ins.pas';

type
  desc_t = record
    crash: char;
    class_of: array[char] of integer;
  end;

var
  d: desc_t;
  g: integer;

procedure one_stmt_params_$enable(in out t: integer32; ch: char;
                                  enable: boolean);
  options(val_param);
begin
  if enable then begin
    d.crash := ch;
    d.class_of[ch] := 255;
    exec_$logv_c(d.crash);
    exec_$logv_w(d.class_of[ch]);
  end
  else begin
    d.class_of[ch] := 0;
    d.crash := chr(0);
    exec_$logv_c(d.crash);
    exec_$logv_w(d.class_of[ch]);
  end;
end;

procedure one_stmt_params_$pick(a: integer; b: integer); options(val_param);
begin
  if a < 5 then begin
    g := a;
    exec_$mark(1);
    g := g + b;
  end
  else
    g := a;
end;

procedure one_stmt_params_$show;
begin
  exec_$logv_w(g);
end;

function one_stmt_params_$xlate(ch: char): char; options(val_param);
begin
  case ch of
    chr(16#91): one_stmt_params_$xlate := chr(16#18);
    chr(16#97), chr(16#98), chr(16#99): one_stmt_params_$xlate := chr(9);
    otherwise
      one_stmt_params_$xlate := chr(0);
  end;
end;

procedure one_stmt_params_$count(step: integer; hi: integer); options(val_param);
var
  i: integer;
begin
  for i := 1 to hi do
    exec_$logv_w(i * step);
end;

procedure one_stmt_params_$grow(step: integer); options(val_param);
begin
  while g < 20 do begin
    g := g + step;
    exec_$logv_w(g);
  end;
end;

procedure one_stmt_params_$upto(step: integer); options(val_param);
begin
  repeat
    g := g - step;
    exec_$logv_w(g);
  until g < 0;
end;
