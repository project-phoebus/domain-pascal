{ Execution test (bead domain-pascal-338r, domain-c's gap 8): `k in c'
  for a constant set c of more than a longword and a word element
  (codegen-patterns 5.1; best effort, no site).  The element is
  zero-extended to a longword and the range guard is a longword
  subtraction, so an `integer' below 0 and a `0..65535' above 32767 are
  not members (the byte element's word guard would read them as
  members).  In a condition and as a value.

  call: IN_WIDE_CONST_SET_$RUN w:-1 w:0 w:76 w:77
  call: IN_WIDE_CONST_SET_$RUN w:87 w:88 w:101 w:200
  call: IN_WIDE_CONST_SET_$RUN w:199 w:207 w:208 w:32767
  call: IN_WIDE_CONST_SET_$UNS w:65535 w:40000 w:200 w:100
  expect:
    false
    false
    false
    false
    false
    false
    true
    true
    true
    true
    false
    false
    true
    true
    true
    true
    false
    false
    false
    true
    false
    false
    false
    false
    false
    false
    true
    false
}
module in_wide_const_set;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;

function int_in(k: integer): boolean; options(val_param);
begin
  int_in := k in [77..87, 101, 200];
end;

function uns_in(var k: pinteger): boolean;
begin
  uns_in := false;
  if k in [77..87, 101, 200..207] then uns_in := true;
end;

procedure both(k: integer); options(val_param);
var
  u: pinteger;
begin
  exec_$log_bool(int_in(k));
  u := k;
  exec_$log_bool(uns_in(u));
end;

procedure in_wide_const_set_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  both(i);
  both(j);
  both(k);
  both(l);
end;

procedure in_wide_const_set_$uns(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
var
  u: pinteger;
begin
  u := i;
  exec_$log_bool(uns_in(u));
  u := j;
  exec_$log_bool(uns_in(u));
  u := k;
  exec_$log_bool(uns_in(u));
  u := l;
  exec_$log_bool(uns_in(u));
end;
