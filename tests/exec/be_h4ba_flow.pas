{ Execution test (bead domain-pascal-h4ba): tests and dereferences of
  register-held locals in the link test's long tail (codegen-patterns
  7.1, 12.5).

  - A local assigned and tested next with a branch the image does not
    show on a `move''s flags: a function's boolean result (`bpl'/`bmi'
    after `move.b D0,Dn', FILE_$SET_PROT_INT, SMD_$DM_COND_EVENT_WAIT), a
    function's word result against 0 ordered, and a loaded word against
    0 with `>=' and `>' (on the move's flags or after a `tst'; a `move'
    and a `tst' leave the same flags; best effort).
  - A local held in D0 (an integer32 an `a0_return' function returned,
    used as a pointer) first dereferenced inside a loop (SIO_$K_SIGNAL_WAIT's
    `repeat'): moved to an address register before the loop (best
    effort).
  Edge values: zero, negative, the maximum.

  link: park_int_desc.mod.pas
  call: BH_F_$BOOL b:255
  call: BH_F_$BOOL b:0
  call: BH_F_$CALLW w:0
  call: BH_F_$CALLW w:-1
  call: BH_F_$CALLW w:32767
  call: BH_F_$LOADW w:0
  call: BH_F_$LOADW w:-32768
  call: BH_F_$LOADW w:5
  call: BH_F_$DEREF l:3
  call: BH_F_$DEREF l:-4
  call: BH_F_$DEREF l:0
  warns: best effort (not byte-identical): this test of a local right after its assignment
  warns: best effort (not byte-identical): the first dereference of a local held in D0
  expect:
    mark 1
    mark 2
    0
    mark 2
    0
    -1
    mark 2
    -1
    mark 1
    32767
    32767
    mark 1
    0
    mark 2
    0
    -32768
    mark 2
    -32768
    mark 1
    5
    5
    14
    10
    14
    10
}
module be_h4ba_flow;
%include 'exec.ins.pas';
type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
var
  gw: integer;

function park_int_$getd(in k: integer32; out st: integer32): integer32;
  options(extern, a0_return);

procedure bh_f_$bool(v: boolean); options(val_param);
var b: boolean;
begin
  b := exec_$id_bool(v);
  if b then exec_$mark(1);
  b := exec_$id_bool(v);
  if not b then exec_$mark(2);
end;

procedure bh_f_$callw(v: integer); options(val_param);
var k: integer;
begin
  k := exec_$id_w(v);
  if k > 0 then exec_$mark(1);
  exec_$logv_w(k);
  k := exec_$id_w(v);
  if k <= 0 then exec_$mark(2);
  exec_$logv_w(k);
end;

procedure bh_f_$loadw(v: integer); options(val_param);
var k: integer;
begin
  gw := v;
  exec_$clobber;
  k := gw;
  if k >= 0 then exec_$mark(1);
  exec_$logv_w(k);
  k := gw;
  if k <= 0 then exec_$mark(2);
  exec_$logv_w(k);
end;

{ The function returns its record's address in A0; `a' is 7, `b' 10
  (park_int_desc.mod.pas). }
procedure bh_f_$deref(k: integer32); options(val_param);
var d: integer32; st: integer32; s: integer32;
begin
  d := park_int_$getd(k, st);
  if st <> 0 then
    return;
  s := 0;
  repeat
    s := s + desc_ptr_t(d)^.a;
  until s > 10;
  exec_$logv_l(s);
  exec_$logv_l(desc_ptr_t(d)^.b);
end;
