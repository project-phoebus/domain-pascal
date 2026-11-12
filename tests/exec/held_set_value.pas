{ Execution test (bead domain-pascal-338r, domain-c's gap 30): `k in s'
  as a value, `s' a `set of 0..31' local held in a data register (an
  operand of a plain `and', guarded by `k <= 31' as the DOMAIN C
  compiler's com.pas process_instruction is, reported): the set copied,
  `btst Dk,Dn', `sne' (best effort, no site).  Elements added with
  `s := s + [k]' between the tests.

  call: HELD_SET_VALUE_$RUN l:3 l:40 l:3 l:31
  expect:
    0
    0
    1
    0
    1
}
module held_set_value;
%include 'exec.ins.pas';

procedure held_set_value_$run(a: integer32; b: integer32; c: integer32; d: integer32);
  options(val_param);
var
  s: set of 0..31;
  n: integer;
begin
  s := [];
  if (a <= 31) and (a in s) then n := 1 else n := 0;
  exec_$logv_w(n);
  s := s + [a];
  if (b <= 31) and (b in s) then n := 1 else n := 0;
  exec_$logv_w(n);
  if (c <= 31) and (c in s) then n := 1 else n := 0;
  exec_$logv_w(n);
  if (d <= 31) and (d in s) then n := 1 else n := 0;
  exec_$logv_w(n);
  s := s + [d];
  if (d <= 31) and (d in s) then n := 1 else n := 0;
  exec_$logv_w(n);
end;
