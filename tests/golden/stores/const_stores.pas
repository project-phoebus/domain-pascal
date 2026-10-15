{ Golden: codegen-patterns 13, constant stores through a pointer: clr for
  zero (TIME_$Q_INIT_QUEUE 0xE16C6E), st for boolean true
  (SMD_$ENABLE_TRACKING 0xE6E468), move #n otherwise (module-local
  0xE0A328 at 0xE0A334 move.b #0x10; 0xE758D8 move.w #0x200;
  MMAP_$ALLOC_CONTIG 0xE0D906 move.l #imm).  A byte immediate is written 0..255
  (0xE0A33E `10bc 00ff`).  12.2: the pointer is loaded at entry, once.
  No two stores are adjacent (ref 3.15.3 p. 3-56 store merging; w2 and l2,
  a word and a longword above it, are kept apart: codegen-patterns 13). }
module consts;
type
  r = record
    flag: boolean;      { 0 }
    w: integer;         { 2 }
    l: integer32;       { 4 }
    c: char;            { 8 }
    b2: boolean;        { 9 }
    w2: integer;        { 10 }
    l2: integer32;      { 12 }
  end;
  rp = ^r;
procedure p(q: rp); options(val_param);
begin
  q^.flag := true;
  q^.l := 0;
  q^.c := chr(255);
  q^.w2 := -1;
  q^.w := 512;
  q^.l2 := 16#6000e;
end;
