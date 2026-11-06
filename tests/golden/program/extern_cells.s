.text
P:
link.w %fp,#-0x10
movea.l (-4,%a5),%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
movea.l (-8,%a5),%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
pea (8,%fp)
move.l (-0xc,%a5),-(%sp)
jsr (USE).l
unlk %fp
rts
.globl CELLS_T
CELLS_T:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #3,-(%sp)
bsr.s P
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.CELLS_T","aw",@progbits
.balign 4
.long NIL_UID
.long DIR_ACL
.long FILE_ACL
.Ldata:
