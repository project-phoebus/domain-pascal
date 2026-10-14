{ Execution test: sets (ref 3.9.3; codegen-patterns 5.1): `+', `-', `*',
  `in', `<> []' on a set held in a word (set of 0..15), and `in' on a
  multi-word one (set of 0..127, stored as a big-endian bit string: bit 0
  is the low bit of its last byte), initialised (codegen-patterns 11).
  (dpc does not yet assign sets of more than a longword, or build a set
  from several run-time members.)

  call: SETS_$SMALL w:3 w:9
  call: SETS_$SMALL w:9 w:9
  call: SETS_$LARGE w:5 w:100 w:6
  call: SETS_$LARGE w:127 w:0 w:64
  expect:
    mem 02 0a
    mem 02 02
    mem 00 02
    mem 00 00
    true
    false
    true
    mem 02 02
    mem 00 02
    mem 00 02
    mem 00 00
    false
    false
    true
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 21
    true
    false
    false
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 21
    true
    true
    false
}
module sets;
%include 'exec.ins.pas';

type
  small = set of 0..15;
  large = set of 0..127;
  idx = 0..127;
var
  s: small;
  sep: integer32;
  t: small;
  u: large := [0, 5, 127];
  b: boolean;

procedure sets_$small(i: integer; j: integer); options(val_param);
begin
  s := [];
  s := s + [i];
  s := s + [j];
  s := s + [1];
  t := s - [i];
  exec_$log_mem(s, 2);
  t := t + [1];
  exec_$log_mem(t, 2);
  t := s * [1, 2];
  exec_$log_mem(t, 2);
  t := t - [1];
  exec_$log_mem(t, 2);
  if 3 in s then b := true else b := false;
  exec_$log_bool(b);
  if 4 in s then b := true else b := false;
  exec_$log_bool(b);
  if s <> [] then b := true else b := false;
  exec_$log_bool(b);
end;

procedure sets_$large(i: idx; j: idx; k: idx); options(val_param);
begin
  exec_$log_mem(u, 16);
  if i in u then b := true else b := false;
  exec_$log_bool(b);
  if j in u then b := true else b := false;
  exec_$log_bool(b);
  if k in u then b := true else b := false;
  exec_$log_bool(b);
end;
