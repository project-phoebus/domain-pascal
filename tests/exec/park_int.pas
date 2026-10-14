{ Execution test: integer locals assigned an `a0_return' function's value
  and dereferenced through a type transfer (bead domain-pascal-yzxr;
  codegen-patterns 3, "Pointer locals held in D0").  Such a local is held
  in D0 (`move.l A0,D0') and moved to an address register before the
  first statement that refers to it or calls a routine, where a call
  follows (callee-saved, TTY_$K_FLUSH_INPUT 0xE1C0A8 / 0xE1C0AE) or not
  (A0, TTY_$K_INQ_INPUT_FLAGS 0xE676DA); a pointer local assigned the
  transferred value that no call follows stays in A0 with
  `--no-park-pointer-locals' (TTY_$I_INQ_RAW 0xE67414).  The value must
  survive the status test and the calls that follow (the stubs clobber
  D0, D1, A0 and A1).

  dpc: --no-park-pointer-locals
  link: park_int_desc.mod.pas
  call: PARK_INT_$AFTER_CALL l:3 @l:99!
  call: PARK_INT_$AFTER_CALL l:-1 @l:99!
  call: PARK_INT_$NO_CALL l:2 @l:99!
  call: PARK_INT_$SHOW
  call: PARK_INT_$IN_A0 l:5 @l:99!
  call: PARK_INT_$SHOW
  call: PARK_INT_$IN_A0 l:-5 @l:99!
  call: PARK_INT_$SHOW
  expect:
    3
    13
    out 0
    out 1
    out 0
    15
    out 0
    20
    out 1
    20
}
module park_int;
%include 'exec.ins.pas';

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
var
  g: integer32 := 0;

function park_int_$getd(in k: integer32; out st: integer32): integer32;
  options(extern, a0_return);

procedure park_int_$after_call(k: integer32; out st: integer32);
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

procedure park_int_$no_call(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then
    return;
  desc_ptr_t(d)^.b := desc_ptr_t(d)^.b + k;
  g := desc_ptr_t(d)^.b;
end;

procedure park_int_$in_a0(k: integer32; out st: integer32);
  options(val_param);
var
  p: desc_ptr_t;
begin
  p := desc_ptr_t(park_int_$getd(k, st));
  if st <> 0 then
    return;
  g := p^.b + p^.a - 2;
end;

procedure park_int_$show;
begin
  exec_$logv_l(g);
end;
