{ Execution test (bead domain-pascal-338r, domain-c's gaps 38 and 39):
  a set constructor of a run-time range and of several run-time members
  in a register-sized set (a mask formed by shifts, `or'ed; codegen-
  patterns 5.1), and `odd' of a byte field (widened to a word).  Best
  effort, no site.

  call: SET_RANGE_ODD_BYTE_$RUN w:3 w:6 w:31
  expect:
    120
    0
    -2147483648
    65
    1
    0
}
module set_range_odd_byte;
%include 'exec.ins.pas';

type
  r_t = packed record b: 0..255; c: 0..255; end;
var
  s: set of 0..31;
  r: r_t;

procedure set_range_odd_byte_$run(a: integer; b: integer; c: integer);
  options(val_param);
begin
  s := [a .. b];
  exec_$logv_l(integer32(s));
  s := [b .. a];
  exec_$logv_l(integer32(s));
  s := [c .. c];
  exec_$logv_l(integer32(s));
  s := [a - 3, b, 0];
  exec_$logv_l(integer32(s));
  r.b := 7;
  r.c := 8;
  if odd(r.b) then exec_$logv_w(1) else exec_$logv_w(0);
  if odd(r.c) then exec_$logv_w(1) else exec_$logv_w(0);
end;
