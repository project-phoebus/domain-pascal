{ codegen-patterns 2: empty procedures compile to a bare `rts', without
  `link', with or without parameters (TIME_$Q_INIT 0xE16C5C, IIC_$INIT
  0xE70A54; MT_$RELEASE 0xE70A4C with three by-address parameters;
  SMD_$PUTC 0xE1D89C), in a unit with data too (they never load A5,
  SMD 0xE1D89C); the module end is padded with 0x2048 (MT 0xE70A4E). }
module m;
var
  x: integer;

procedure m_$init;
begin
end;

procedure m_$release(in unit: integer; in mode: integer; out status: integer32);
begin
end;

procedure m_$putc(c: integer); options(val_param);
begin
end;

procedure m_$set;
begin
  x := 1;
end;

procedure m_$last;
begin
end;
