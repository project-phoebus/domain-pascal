{ tests/exec/exec.ins.pas - the execution tests' stub routines
  (tests/exec/README.md).  They are implemented in C by
  tools/exectest/runner.c; like any call, each clobbers D0, D1, A0, A1
  (except its result) and the condition codes. }

type
  exec_$bytes = array[1..256] of char;
  exec_$uword = 0..65535;

{ log a value, passed by address (the default convention, ref 5.2) }
procedure exec_$log_l(in v: integer32); extern;
procedure exec_$log_w(in v: integer); extern;
procedure exec_$log_uw(in v: exec_$uword); extern;
procedure exec_$log_c(in v: char); extern;
procedure exec_$log_bool(in v: boolean); extern;
procedure exec_$log_x(in v: integer32); extern;
{ log the first n bytes of v in hex }
procedure exec_$log_mem(in v: univ exec_$bytes; in n: integer); extern;

{ log a value passed by value (ref 5.5.7 val_param, p. 5-19) }
procedure exec_$logv_l(v: integer32); options(val_param); extern;
procedure exec_$logv_w(v: integer); options(val_param); extern;
procedure exec_$logv_c(v: char); options(val_param); extern;
procedure exec_$mark(v: integer); options(val_param); extern;

{ return the argument: D0 (junk in the bits above the result), or A0 }
function exec_$id_l(in v: integer32): integer32; extern;
function exec_$id_w(in v: integer): integer; extern;
function exec_$id_bool(in v: boolean): boolean; extern;
function exec_$id_p(in v: univ_ptr): univ_ptr; extern;

{ a call that does nothing but clobber the scratch registers }
procedure exec_$clobber; extern;

{ options(nosave) routines (ref 5.5.8, p. 5-19): they also clobber D2-D7
  and A2-A4 (not A5, A6); exec_$nosave_w logs its value parameter,
  exec_$nosave_id_l returns its argument in D0 }
procedure exec_$nosave; options(nosave); extern;
procedure exec_$nosave_w(v: integer); options(val_param, nosave); extern;
function exec_$nosave_id_l(in v: integer32): integer32; options(nosave); extern;

{ a device register: each read returns the next value (0, 1, 2, ...),
  a store sets it }
var
  exec_$counter: extern [volatile] integer;
