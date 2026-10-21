{ Execution test (bead domain-pascal-gfnj): value parameters passed by
  address (ref 5.2 Table 5-1, p. 5-5) whose copy is held in a data
  register rather than in the frame (codegen-patterns 1.2, "Held in a
  register instead"): read only, not every read pushed, no read in a
  loop, not a pointer.  The value is loaded through the argument slot at
  entry; the caller's cells are not changed.

  - HELD_$REFCNT: FILE_$SET_REFCNT's shape (0xE5E3F4): a longword tested
    twice and its low word stored;
  - HELD_$ACROSS: a word and a byte read after calls that clobber the
    scratch registers (PROC2_$SET_SERVER 0xE4147C: held in D2);
  - HELD_$STORE: a word stored into the unit's data and a byte passed on
    by value in an expression (SMD_$SET_TP_REPORTING 0xE6E4C0).

  call: HELD_$REFCNT @l:-1! @w:7!
  call: HELD_$REFCNT @l:22136! @w:7!
  call: HELD_$REFCNT @l:-5! @w:7!
  call: HELD_$REFCNT @l:65525! @w:7!
  call: HELD_$ACROSS @w:-300! @b:255!
  call: HELD_$ACROSS @w:12! @b:0!
  call: HELD_$STORE @w:-2! @b:200!
  expect:
    out -1
    out 0
    out 22136
    out 22136
    out -5
    out -5
    out 65525
    out -11
    -300
    -299
    out -300
    out 255
    0
    out 12
    out 0
    -2
    201
    out -2
    out 200
}
module held_copies;
%include 'exec.ins.pas';
type
  ub = [byte] 0..255;
var
  mode_v: integer;

procedure held_$refcnt(refcnt: integer32; out w: integer);
begin
  if refcnt = -1 then
    w := 0
  else if refcnt < 16#FFF5 then
    w := refcnt
  else
    w := -11;
end;

procedure held_$across(n: integer; flag: boolean);
begin
  exec_$clobber;
  if flag then begin
    exec_$logv_w(n);
    exec_$clobber;
    exec_$logv_w(n + 1);
  end
  else
    exec_$logv_w(0);
end;

procedure held_$store(mode: integer; b: ub);
begin
  mode_v := mode;
  exec_$log_w(mode_v);
  exec_$logv_w(b + 1);
end;
