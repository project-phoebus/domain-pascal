{ Golden: stores whose destination and value both need a register
  (codegen-patterns 12.4, "Stores whose destination and value both need a
  register"; bead domain-pascal-k1sv; tools/storesurvey.py): the side set
  up by more instructions first, the value on a tie, then the scheduler
  (R11) interleaves them.  TTY_$K_SET_DELAY 0xE67A40 `movea.l D0,A0;
  move.w (A2),D1; movea.l (0x10,A6),A1; add.w D1,D1; move.w
  (A1),(0x40,A0,D1.w)' (the element first); TTY_$K_INQ_DELAY 0xE67A96
  (the element as the value, first).  The descriptor comes from an
  `a0_return' integer function held in D0, as ../source's reading of
  TTY_$I_GET_DESC (tty/tty.ins.pas). }
module both_regs;
type
  delay_t = (d0, d1, d2, d3, d4);
  desc_t = record
    pad: array[0..31] of integer;
    delay: array[delay_t] of integer;
  end;
  desc_ptr_t = ^desc_t;
function get_desc(in line: integer; out st: integer32): integer32;
  options(extern, a0_return);
procedure set_delay(in line: integer; in delay: delay_t; in val: integer;
                    out st: integer32);
var
  t: integer32;
begin
  if not in_range(delay) then begin
    st := 1;
    return;
  end;
  t := get_desc(line, st);
  if st <> 0 then
    return;
  desc_ptr_t(t)^.delay[delay] := val;
end;
procedure inq_delay(in line: integer; in delay: delay_t; out val: integer;
                    out st: integer32);
var
  t: integer32;
begin
  if not in_range(delay) then begin
    st := 1;
    return;
  end;
  t := get_desc(line, st);
  if st <> 0 then
    return;
  val := desc_ptr_t(t)^.delay[delay];
end;
{ A tie (each side one load of a by-address pointer parameter's slot and
  one of the pointer): the value first (71 of the 84 ties of the survey;
  TTY_$K_GET 0xE1C4D6 `lea (0,A2,D2.w),A0; movea.l (-4,A6),A1; move.b
  (0x2d1,A0),(A1)'). }
procedure tie(in a: desc_ptr_t; in b: desc_ptr_t);
begin
  a^.pad[3] := b^.pad[5];
end;
