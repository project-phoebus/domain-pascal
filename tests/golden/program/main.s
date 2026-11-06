.text
P:
link.w %fp,#-0x10
movea.l (8,%fp),%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
move.w (0xc,%fp),%d0
ext.l %d0
add.l (-0xc,%fp),%d0
move.l %d0,(%a5)
unlk %fp
rts
.globl MAIN_T
MAIN_T:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #3,-(%sp)
pea (4,%a5)
bsr.s P
addq.w #8,%sp
jsr (PGM_$EXIT).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.MAIN_T","aw",@nobits
.balign 4
.Ldata:
.space 12
