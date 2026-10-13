{ codegen-patterns 4 (which routines load A5; options in a5_modes.args:
  --a5-always-from=m_$from --a5-always=worker,m_$back
  --a5-absolute=m_$absolute), 2, 9, 13.

  - m_$uses: used mode, uses the data: loads A5 (PROC1_$GET_LOADAV
    0xE14BBE);
  - m_$not: used mode, does not use it: no A5 (PROC1_$TST_LOCK 0xE148CA,
    in a unit with data);
  - worker: an internal routine in --a5-always loads A5 itself (unnamed
    0xE14B12, RING 0xE76048), so a call to it is not a use and m_$calls
    loads nothing (MST_$FIND 0xE0E11E calls such a routine);
  - m_$absolute: --a5-absolute, `lea <area>.l,A0' at the first static
    operand, then `(d,A0)' for both statements (REM_NAME_$REGISTER_SERVER
    0xE4A4B2..0xE4A4C0);
  - m_$empty: a bare `rts' never loads A5 (SMD_$PUTC 0xE1D89C);
  - m_$from: always mode from here on: loads A5 without using the data
    (SMD_$BUSY_WAIT 0xE1D8A2), and so does m_$later, but m_$empty2 stays
    a bare `rts';
  - m_$back: in used mode, but in --a5-always: loads A5 without using
    the data (the switch-backs of PKT 0xE1202C, XPD 0xE5AF38). }
module m;
type
  status_$t = record
    all: integer32;
  end;
var
  x: integer;
  y: integer32;
  flag: boolean;
  clock: extern integer32;

procedure worker; internal;
begin
  x := 0;
end;

procedure m_$uses(out v: integer);
begin
  v := x;
end;

procedure m_$not(out v: integer);
begin
  v := 0;
end;

procedure m_$back(out v: integer);
begin
  v := 1;
end;

procedure m_$calls;
begin
  worker;
end;

procedure m_$absolute;
begin
  y := clock;
  flag := true;
end;

procedure m_$empty;
begin
end;

procedure m_$from(in p: integer32; out status: status_$t);
begin
  status.all := 0;
end;

procedure m_$empty2(in p: integer32);
begin
end;

procedure m_$later(out status: status_$t);
begin
  status.all := 16#3A0001;
end;
