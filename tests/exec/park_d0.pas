{ Execution test: `dpc --park-d0' (bead domain-pascal-bks; codegen-patterns
  3, "Pointer locals held in D0").  In the routines named below a pointer
  local assigned a pointer function's result is held in D0 (`move.l
  A0,D0') and moved to a callee-saved address register before the first
  statement that refers to it or calls a routine (TTY_$K_FLUSH_INPUT
  0xE1C0A8, 0xE1C0AE).  The value must survive the status test, a `case'
  that computes in a data register, and the calls that follow (the stubs
  clobber D0, D1, A0 and A1); the status cell must get the function's
  store.

  dpc: --park-d0=PARK_D0_$AFTER_CALL,PARK_D0_$CASE,PARK_D0_$BY_ADDR
  call: PARK_D0_$AFTER_CALL l:3 @l:99!
  call: PARK_D0_$AFTER_CALL l:-1 @l:99!
  call: PARK_D0_$CASE l:2 @l:99!
  call: PARK_D0_$BY_ADDR l:4 @l:99!
  call: PARK_D0_$BY_ADDR l:-5 @l:99!
  expect:
    3
    13
    out 0
    out 1
    2
    15
    out 0
    7
    4
    19
    out 0
    out 1
}
module park_d0;
%include 'exec.ins.pas';

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
var
  desc: desc_t := [a := 7, b := 10];
  sel: integer := 0;
  gp: desc_ptr_t := addr(desc);

function getd(in k: integer32; out st: integer32): desc_ptr_t;
begin
  if k < 0 then
    st := 1
  else
    st := 0;
  getd := gp;
end;

procedure show(in d: desc_t);
begin
  exec_$logv_l(d.b);
end;

procedure park_d0_$after_call(k: integer32; out st: integer32);
  options(val_param);
var
  d: desc_ptr_t;
begin
  d := getd(k, st);
  if st <> 0 then
    return;
  exec_$logv_l(k);
  d^.b := d^.b + k;
  exec_$logv_l(d^.b);
end;

procedure park_d0_$case(k: integer32; out st: integer32);
  options(val_param);
var
  d: desc_ptr_t;
begin
  d := getd(k, st);
  case sel of
    0: sel := 3;
    1: sel := 4;
    2: sel := 5;
    3: sel := 6;
  otherwise
    sel := 0;
  end;
  exec_$logv_l(sel - 3 + k);
  d^.b := d^.b + k;
  exec_$logv_l(d^.b);
end;

procedure park_d0_$by_addr(k: integer32; out st: integer32);
  options(val_param);
var
  d: desc_ptr_t;
begin
  d := getd(k, st);
  if st <> 0 then
    return;
  exec_$logv_l(d^.a);
  exec_$logv_l(k);
  d^.b := d^.b + k;
  show(d^);
end;
