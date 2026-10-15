{ Golden: codegen-patterns 1.5, 1.3 (bead domain-pascal-g5xv): byte
  constants passed by value.  Other than `true' (`st -(SP)') and 0
  (`clr.w -(SP)', 0xE1AFD4 for chr(0)), a byte constant is pushed as a
  word with the byte in the high half of its slot, `move.w #n<<8,-(SP)':
  TTY 0xE1AFC0 `move.w #-0x200,-(SP)' for chr(254), before `pea' of the
  buffer pushed first (0xE1AFBC); SIO_$I_TSTART 0xE1C7D6 `move.w
  #0x1300'.  A char next to `true' is a separate push (SIO_$INIT
  0xE32E60 `st -(SP)', 0xE32E62 `move.w #0x1b00,-(SP)'). }
module byte_consts;
type
  ubyte = [byte] 0..255;
  sbyte = [byte] -128..127;
var
  last: extern char;
  lastu: extern ubyte;
  lasts: extern sbyte;
  flag: extern boolean;
  buf: extern integer32;

procedure put(c: char; var b: integer32); internal;
begin
  last := c;
  b := 0;
end;

procedure put2(c: char; f: boolean); internal;
begin
  last := c;
  flag := f;
end;

procedure putu(u: ubyte; var b: integer32); internal;
begin
  lastu := u;
  b := 0;
end;

procedure puts(s: sbyte; var b: integer32); internal;
begin
  lasts := s;
  b := 0;
end;

procedure byte_consts_$test;
begin
  put(chr(254), buf);
  put(chr(0), buf);
  put('A', buf);
  put2(chr(27), true);
  putu(128, buf);
  puts(-2, buf);
  puts(127, buf);
end;
