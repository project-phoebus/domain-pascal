{ Execution test (bead domain-pascal-5kgg): arguments on the lowering's
  best-effort path, passed by address (the default convention, ref 5.2):

  - a variable of another size to an `in` parameter: a converted copy
    in a frame temporary (ref 5.3.2 p. 5-7, assignment compatible;
    widened by its sign, or narrowed to its low bytes, reference-
    clarifications question 41);
  - an expression of another size, 2 to 4 bytes and 4 to 2 bytes, into
    a frame temporary of the parameter's size;
  - a field of a type transfer passed by address, in place (ref p. 4-197:
    the variable's bytes seen as the other type).

  call: BE_L_ARGS_$VARS w:-7 l:70000
  call: BE_L_ARGS_$EXPRS w:-7 l:100
  call: BE_L_ARGS_$FIELD l:0x00030004
  warns: best effort (not byte-identical): passing a variable of another size to a value or `in` parameter
  warns: best effort (not byte-identical): passing a 2-byte expression by address to a 4-byte parameter
  warns: best effort (not byte-identical): passing a 4-byte expression by address to a 2-byte parameter
  warns: best effort (not byte-identical): a field or element of a type transfer passed by address
  expect:
    -7
    4464
    -6
    -114
    3
    4
}
module be_l_args;
%include 'exec.ins.pas';
type
  pair = record hi, lo: integer; end;

procedure be_l_args_$vars(x: integer; y: integer32); options(val_param);
var i: integer; l: integer32;
begin
  i := x;
  l := y;
  exec_$log_l(i);
  exec_$log_w(l);
end;

procedure be_l_args_$exprs(x: integer; y: integer32); options(val_param);
begin
  exec_$log_l(x + 1);
  exec_$log_w(rshft(y, 1) * 2 - 214);
end;

procedure be_l_args_$field(v: integer32); options(val_param);
var l: integer32;
begin
  l := v;
  exec_$log_w(pair(l).hi);
  exec_$log_w(pair(l).lo);
end;
