{ Execution test: integer locals assigned an `a0_return' function's value
  and dereferenced through a type transfer, where a call follows (beads
  domain-pascal-yzxr, -78n2; codegen-patterns 3, "Pointer locals held in
  D0").  Formerly park_d0.pas, the same routines with pointer locals and
  `dpc --park-d0' (bead domain-pascal-bks), retired with the integer
  reading (bead domain-pascal-78n2): the Pascal now has the form that
  gives the D0 code without options.  Such a local is held in D0
  (`move.l A0,D0') and moved to a callee-saved address register before
  the first statement that refers to it or calls a routine
  (TTY_$K_FLUSH_INPUT 0xE1C0A8, 0xE1C0AE).  The value must survive the
  status test, a `case' that computes in a data register, and the calls
  that follow (the stubs clobber D0, D1, A0 and A1); the status cell must
  get the function's store.  The descriptor and its function are
  park_int.pas's (park_int_desc.mod.pas).

  link: park_int_desc.mod.pas
  call: PARK_INT_CALLS_$AFTER_CALL l:3 @l:99!
  call: PARK_INT_CALLS_$AFTER_CALL l:-1 @l:99!
  call: PARK_INT_CALLS_$CASE l:2 @l:99!
  call: PARK_INT_CALLS_$BY_ADDR l:4 @l:99!
  call: PARK_INT_CALLS_$BY_ADDR l:-5 @l:99!
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
module park_int_calls;
%include 'exec.ins.pas';

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
var
  sel: integer := 0;

function park_int_$getd(in k: integer32; out st: integer32): integer32;
  options(extern, a0_return);

procedure show(in d: desc_t);
begin
  exec_$logv_l(d.b);
end;

procedure park_int_calls_$after_call(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then
    return;
  exec_$logv_l(k);
  desc_ptr_t(d)^.b := desc_ptr_t(d)^.b + k;
  exec_$logv_l(desc_ptr_t(d)^.b);
end;

procedure park_int_calls_$case(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  case sel of
    0: sel := 3;
    1: sel := 4;
    2: sel := 5;
    3: sel := 6;
  otherwise
    sel := 0;
  end;
  exec_$logv_l(sel - 3 + k);
  desc_ptr_t(d)^.b := desc_ptr_t(d)^.b + k;
  exec_$logv_l(desc_ptr_t(d)^.b);
end;

procedure park_int_calls_$by_addr(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then
    return;
  exec_$logv_l(desc_ptr_t(d)^.a);
  exec_$logv_l(k);
  desc_ptr_t(d)^.b := desc_ptr_t(d)^.b + k;
  show(desc_ptr_t(d)^);
end;
