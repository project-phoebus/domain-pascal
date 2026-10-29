{ codegen-patterns 14 ("Unsigned arithmetic and byte constants"); bead
  domain-pascal-p1pd; ../source findings 305 and 306.  With `-cpu
  mathchip':
  - an unsigned `mod' by a power of two is the mask, `moveq #7,D0; and.w
    (P).l,D0' (SAU9 WIN_SCSI_$DINIT 0x3C41BAB2 `moveq #7,D0; and.w D4,D0';
    WIN_SCSI_$CINIT 0x3C4361E8 `moveq #7,D4; and.w (6,A3),D4'); neither
    image divides by a power of two, and the MC68010 build keeps its
    best-effort `divu.w';
  - `div' and `*' by 1024 of a 0..16#7FFFFFFF are the shifts of both
    images, `lsr.l #8; lsr.l #2' and `lsl.l #8; lsl.l #2' (0x3C41BE18,
    0x3C41C0EE);
  - a byte constant 255 is stored with `st' (WIN_SCSI_$DINIT 0x3C41BE5A
    `st (-12,A6)' for `chr(255)'; 60 `st' on memory and no `move.b #-1'
    in the SAU9 image, against three `move.b #-1' in the SAU2 image:
    DMA_$CHECK 0xE0A3C6, 0xE0A432, RING_$INT 0xE75840). }
module unsigned_forms;
type pinteger = 0..65535; linteger = 0..16#7FFFFFFF;
var p: extern pinteger; l: extern linteger; x: extern pinteger; y, z: extern linteger; c: extern char;
procedure q;
begin
  x := p mod 8;
  y := l div 1024;
  z := l * 1024;
  c := chr(255);
end;
type pchar = ^char;
procedure r(pc: pchar); options(val_param);
var lc: char;
begin
  pc^ := chr(255);
end;
