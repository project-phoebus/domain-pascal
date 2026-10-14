{ Execution test: an integer local assigned an `a0_return' function's
  value, held in D0 and moved to an address register inside an `if'
  (bead domain-pascal-1j61; codegen-patterns 3, "Pointer locals held in
  D0"): in the one part that refers to it (the `then' part, TTY_$K_GET
  0xE1C40A; the `else' part after a `then' part ending in `return',
  TTY_$K_SET_FUNC_CHAR 0xE67516), and as the first instruction of an
  `if' whose condition dereferences it (TTY_$K_SET_FUNC_CHAR 0xE67516).
  Each routine is called on both sides of each test; the stubs clobber
  D0, D1, A0 and A1.

  link: park_int_desc.mod.pas
  call: PARK_INT_IF_$GET l:3 @l:99! -> l
  call: PARK_INT_IF_$GET l:-1 @l:99! -> l
  call: PARK_INT_IF_$COND l:7 @l:99!
  call: PARK_INT_IF_$COND l:3 @l:99!
  call: PARK_INT_IF_$COND l:5 @l:99!
  call: PARK_INT_IF_$COND l:200 @l:99!
  call: PARK_INT_IF_$COND l:-1 @l:99!
  call: PARK_INT_IF_$ELSE l:4 @l:99!
  call: PARK_INT_IF_$SHOW
  call: PARK_INT_IF_$ELSE l:-4 @l:99!
  call: PARK_INT_IF_$SHOW
  expect:
    7
    out 0
    => 13
    out 1
    => 0
    11
    out 0
    12
    out 0
    12
    out 0
    out 2
    out 1
    out 0
    9
    out 1
    -1
}
module park_int_if;
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

{ The move in the `then' part, the first statement of which calls. }
function park_int_if_$get(k: integer32; out st: integer32): integer32;
  options(val_param);
var
  d: integer32;
  n: integer32;
begin
  n := 0;
  d := park_int_$getd(k, st);
  if st = 0 then begin
    exec_$logv_l(desc_ptr_t(d)^.a);
    n := desc_ptr_t(d)^.b + k;
  end;
  park_int_if_$get := n;
end;

{ The move first in an `if' whose condition dereferences it. }
procedure park_int_if_$cond(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then
    return;
  if k > 100 then begin
    st := 2;
    return;
  end;
  if desc_ptr_t(d)^.b < 12 then
    desc_ptr_t(d)^.b := desc_ptr_t(d)^.b + 1;
  exec_$logv_l(desc_ptr_t(d)^.b);
end;

{ The move in the `else' part, the `then' part ending in `return'. }
procedure park_int_if_$else(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then begin
    g := -1;
    return;
  end
  else
    g := desc_ptr_t(d)^.b - desc_ptr_t(d)^.a + k;
end;

procedure park_int_if_$show;
begin
  exec_$logv_l(g);
end;
