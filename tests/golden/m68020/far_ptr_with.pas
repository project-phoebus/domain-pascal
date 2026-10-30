{ codegen-patterns 14 ("Symbols far from an array's first element"), the
  pointer case; bead domain-pascal-4bjx.  A `with' over an element of a
  pointer's array indexed from far beyond 16 bits (the SAU9 MMAP through
  a pointer, `array[16#10000..16#1FFFF]' of 16-byte entries) puts the
  element's offset in the temporary: the element's address off the base
  into a scratch register, copied to the record's register, the offset
  added there: SAU9 OS_$COLD_START 0x40034B6 `lea (0,A0,D1.l),A1;
  movea.l A1,A3; adda.l #-0x100000,A3; bset #6,(A3)'.  (The element
  named directly keeps `lea (0,An,Dx),Am; adda.l #d,Am', section 5.2;
  through a pointer parameter it is best effort, bead
  domain-pascal-nqg1, so it is not in this golden.) }
module far_ptr_with;
type
  mmape_t = packed record
    present: boolean;
    pageable: boolean;
    bits_5_0: 0..63;
    unused_01: char;
    unused_02: array[1..14] of char;
  end;
  mmap_ptr_t = ^array[16#10000..16#1FFFF] of mmape_t;

procedure mark_with(ppn: integer32; in mmap_pa: integer32); options(val_param);
begin
  with mmap_ptr_t(mmap_pa)^[ppn] do pageable := true;
end;

