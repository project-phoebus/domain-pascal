{ Execution test (bead domain-pascal-338r, domain-c's gap 48): `in` of an
  enumeration of 38 values (its sets are 6 bytes) against a constant set
  whose members all lie in 0..31: the element checked against 31, then
  tested in the members' longword mask (reported, not checked here: the
  DOMAIN C compiler's print_top_node, cc.m68k 0x083462, `move.l
  #0x05F84006,D1; btst`).  An element beyond 31 is not a member (ref
  3.9.3), with or without the program's own `k <= e31`.  Best effort.
  call: LCE_$RUN w:1
  call: LCE_$RUN w:3
  call: LCE_$RUN w:26
  call: LCE_$RUN w:31
  call: LCE_$RUN w:37
  warns: best effort (not byte-identical): `in` on a constant set of members 0..31
  expect:
    true
    true
    false
    false
    true
    true
    false
    false
    false
    false
}
module lce;
%include 'exec.ins.pas';
type e_t = (e0, e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12,
            e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23,
            e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34,
            e35, e36, e37);
var k: e_t; b: boolean;
procedure lce_$run(n: integer); options(val_param);
begin
  k := e_t(n);
  b := (k <= e31) and (k in [e1..e2, e14, e19..e24, e26]);
  exec_$log_bool(b);
  b := k in [e1..e2, e14, e19..e24, e26];
  exec_$log_bool(b);
end;
