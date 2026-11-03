.set __dpc_fragment_origin,0xe3f130
.text
.globl PROC2_$QUIT
PROC2_$QUIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (.Lc0,%pc)
pea (.Lc1,%pc)
move.l (8,%fp),-(%sp)
bsr.w PROC2_$SIGNAL
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc1:
.short 0x0003
.balignw 4,0x2048
.Lc0:
.short 0x0012,0x0010
.section ".bss.moddata.PROC2","aw",@nobits
.balign 4
.Ldata:
.space 4
