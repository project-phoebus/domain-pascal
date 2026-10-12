{ Golden: codegen-patterns 13, a one-statement body storing another
  object through a pointer parameter: the pointer is loaded where the
  statement needs it, the value moves memory to memory
  (PROC2_$GET_BOOT_FLAGS 0xE41B62..0xE41B66 `movea.l (8,A6),A0; move.w
  (0x1e4,A5),(A0)`; here the source is another unit's variable, absolute
  as in codegen-patterns 4). }
module mem;
type
  int_ptr = ^integer;
var
  boot_flags: extern integer;
procedure get_boot_flags(p: int_ptr); options(val_param);
begin
  p^ := boot_flags;
end;
