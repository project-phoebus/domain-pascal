{ codegen-patterns 7.4 ("Counted loops", a longword count) under the
  MC68020: the count is a longword counted down with `subq.l #1; bcc',
  no `dbf' (SAU6 PBU2_$WIRE 0x78061C3E `subq.l #1,D3; bcc.s', and
  0x78061C2C; PBU2_$UNWIRE 0x780618BA; the SAU5's same routines have
  `dbf; clr.w; subq.l #1; bcc'; ../source finding 935). }
module long_for;
define p;
var k: integer32;
procedure p(n: integer32); options(val_param);
var i: integer32;
begin
  for i := 1 to 100000 do k := k + i;
end;
