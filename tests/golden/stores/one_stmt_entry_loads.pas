{ Golden: codegen-patterns 12.2, "One-statement bodies" (bead
  domain-pascal-s4a5): a body of one `if' loads the by-value parameters
  it reads at entry, in parameter order, and the test of the last one
  loaded branches on the load's flags (no `tst').  Mirrors
  TTY_$I_ENABLE_CRASH_FUNC 0xE67292..0xE672A4 (`movea.l (8,A6),A0;
  move.b (0xc,A6),D0; move.b (0xe,A6),D1; bpl.s'), with a cut-down
  record.  Not reproduced: the image keeps D1 (`enable', dead after the
  `bpl') and A0 out of the arms' index computations, `clr.w D2' and `lea
  (0,A0,D2.w),A1' at 0xE672AA..0xE672B0 (bead domain-pascal-d85r).
  `tst_first' tests a parameter loaded before the last one: `tst.b' of
  its register (PROC1_$SET_TYPE 0xE152FA `tst.w D2' after the load of
  D3). }
module crash;
type
  desc_t = record
    pad: array[0..81] of integer;
    class_of: array[char] of integer;
    pad2: array[0..563] of char;
    crash: char;
  end;
procedure enable_crash(in out tty: desc_t; ch: char; enable: boolean);
  options(val_param);
begin
  if enable then begin
    tty.crash := ch;
    tty.class_of[ch] := 17;
  end
  else begin
    if tty.crash <> chr(0) then
      tty.class_of[ch] := 18;
    tty.crash := chr(0);
  end;
end;
procedure tst_first(in out tty: desc_t; enable: boolean; ch: char);
  options(val_param);
begin
  if enable then
    tty.crash := ch;
end;
