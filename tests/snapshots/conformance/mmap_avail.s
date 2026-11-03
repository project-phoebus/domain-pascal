.set __dpc_fragment_origin,0xe0cc64
.text
.globl MMAP_$AVAIL
MMAP_$AVAIL:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
movea.l #MMAP,%a0
move.l %d2,%d0
lsl.l #4,%d0
lea (0,%a0,%d0.l),%a2
cmpi.b #0x45,(-0x1ffc,%a2)
bls.s 1f
pea (.Lc0,%pc)
jsr (CRASH_SYSTEM).l
addq.w #4,%sp
1:
st -(%sp)
clr.w %d0
move.b (-0x1ffc,%a2),%d0
move.w %d0,-(%sp)
move.l %d2,-(%sp)
pea (-0x2000,%a2)
bsr.w MMAP_$ADD_TO_WSL
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0006,0x0004
.section ".bss.moddata.MMAP_","aw",@nobits
.balign 4
.Ldata:
.space 4
