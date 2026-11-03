{ Execution test (beads domain-pascal-zif8, domain-pascal-35ph): set
  constructors with run-time ranges (ref 3.9.3; "Set Operations", p.
  4-173) on sets of more than a longword, built inline as SAU8 MST_$INIT
  0x3C33F0E builds them (the top clamped at the set's last member, an
  empty range setting nothing, the members set downward with `bset'), in
  `[r1, r2]', `s + [r]', `s - [r]', with constant and run-time single
  members, on 16- and 32-byte sets; and a 16-byte set widened into a
  32-byte one (its bytes the larger's last ones, the leading ones
  cleared; MST_$FREE_ASID 0x3C7F664), alone and as the left operand of
  `-'.  Ranges reach past the set's end, are empty (top below bottom),
  one member, the whole set.  The expected output is worked out from the
  set semantics and the bit layout of Figure 3-6, not from dpc.

  call: SET_RANGES_$CONS w:3 w:10 w:120 w:20
  call: SET_RANGES_$CONS w:5 w:0 w:0 w:128
  call: SET_RANGES_$CONS w:126 w:1 w:64 w:1
  call: SET_RANGES_$ADD w:8 w:15
  call: SET_RANGES_$ADD w:127 w:127
  call: SET_RANGES_$SUB w:10 w:200
  call: SET_RANGES_$SUB w:0 w:0
  call: SET_RANGES_$MIXED w:100 w:40 w:42
  call: SET_RANGES_$MIXED w:1 w:7 w:8
  call: SET_RANGES_$WIDEN
  call: SET_RANGES_$BR w:250 w:300
  call: SET_RANGES_$BR w:0 w:7
  call: SET_RANGES_$BR w:9 w:8
  expect:
    mem ff 00 00 00 00 00 00 00 00 00 00 00 00 00 1f f8
    mem ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
    mem 40 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff 01
    mem 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 03 ff
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff fe
    mem 00 00 00 10 00 00 00 00 00 00 07 00 00 00 00 02
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 82
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 20
    mem fc 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff
    mem 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
}
module set_ranges;
%include 'exec.ins.pas';
var
  a: set of 0..127;
  b: set of 0..255;
define set_ranges_$cons, set_ranges_$add, set_ranges_$sub, set_ranges_$mixed,
  set_ranges_$widen, set_ranges_$br;

procedure set_ranges_$cons(lo1, n1, lo2, n2: integer); options(val_param);
begin
  a := [lo1 .. lo1 + n1 - 1, lo2 .. lo2 + n2 - 1];
  exec_$log_mem(a, 16);
end;

procedure set_ranges_$add(lo, hi: integer); options(val_param);
begin
  a := [0];
  a := a + [lo .. hi];
  exec_$log_mem(a, 16);
end;

procedure set_ranges_$sub(lo, hi: integer); options(val_param);
begin
  a := [0 .. 127];
  b := a - [lo .. hi];
  exec_$log_mem(b, 32);
end;

procedure set_ranges_$mixed(k, lo, hi: integer); options(val_param);
begin
  a := [1, k, lo .. hi];
  exec_$log_mem(a, 16);
end;

procedure set_ranges_$widen;
begin
  a := [5, 127];
  b := [0 .. 255];
  b := a;
  exec_$log_mem(b, 32);
end;

procedure set_ranges_$br(lo, hi: integer); options(val_param);
begin
  b := [lo .. hi];
  exec_$log_mem(b, 32);
end;
