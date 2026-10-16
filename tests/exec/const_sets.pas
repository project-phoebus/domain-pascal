{ Execution test (bead domain-pascal-bitu): `x in c' for a constant set
  c of one word or one longword and a run-time element (codegen-patterns
  5.1, "`x in c', a constant register-sized set"): the constant in a
  data register, `btst Dk,Dn'.  A `0..65535' element of an integer
  constructor zero-extended, the constant a longword (DISK_ AS_IO_SETUP
  0xE6B75C `clr.l D0; move.l #0x7fe,D1; move.w (A0),D0; btst D0,D1':
  `vol_idx in [1..10]'); an `integer' element and an enumeration of 16
  values with a constant `moveq' loads (TTY_$K_SET_DELAY 0xE67A1A
  `moveq #0x1f,D1').  In a condition and as a
  value (`sne' in the set's register).  The elements are within 0..31
  here; one above 31 is tested modulo 32, as by the original (crates/
  dpc-m68k/tests/sets.rs, `constant_sets_take_the_element_mod_32').

  call: CONST_SETS_$RUN w:0 w:1 w:10 w:11
  call: CONST_SETS_$RUN w:15 w:16 w:31 w:5
  call: CONST_SETS_$ENUM w:0 w:1 w:4 w:6
  call: CONST_SETS_$ENUM w:7 w:2 w:15 w:3
  expect:
    false
    false
    true
    true
    false
    true
    false
    false
    false
    false
    false
    false
    false
    false
    true
    true
    false
    true
    true
    true
    false
    false
    false
    true
}
module const_sets;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;
  e16 = (e0, e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15);
var
  b: boolean;

function int_in(k: integer): boolean; options(val_param);
begin
  int_in := k in [1..6];
end;

function uns_in(var k: pinteger): boolean;
begin
  uns_in := false;
  if k in [1..10] then uns_in := true;
end;

procedure enum_in(e: e16); options(val_param);
begin
  if e in [e1, e3..e6] then b := true else b := false;
  exec_$log_bool(b);
end;

procedure const_sets_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
var
  u: pinteger;
begin
  exec_$log_bool(int_in(i));
  u := i;
  exec_$log_bool(uns_in(u));
  exec_$log_bool(int_in(j));
  u := j;
  exec_$log_bool(uns_in(u));
  exec_$log_bool(int_in(k));
  u := k;
  exec_$log_bool(uns_in(u));
  exec_$log_bool(int_in(l));
  u := l;
  exec_$log_bool(uns_in(u));
end;

procedure const_sets_$enum(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  enum_in(e16(i));
  enum_in(e16(j));
  enum_in(e16(k));
  enum_in(e16(l));
end;
