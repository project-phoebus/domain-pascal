.text
.globl LSP_$INQ_PGROUP
LSP_$INQ_PGROUP:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (LSP_$GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a3
lea (0x4c,%a0),%a1
move.l (%a1)+,(%a3)
move.l (%a1)+,(4,%a3)
1:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl LSP_$AFTER_TEST
LSP_$AFTER_TEST:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a0
move.w #1,(%a0)
movea.l (0xc,%fp),%a1
move.w #2,(%a1)
cmpi.w #3,(W).l
bne.s 1f
movea.l (0x10,%fp),%a2
clr.w (%a2)
1:
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
