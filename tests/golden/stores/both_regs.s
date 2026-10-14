.text
.globl SET_DELAY
SET_DELAY:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
cmpi.w #4,(%a2)
bls.s 1f
move.l #1,(%a3)
bra.s 2f
1:
pea (%a3)
move.l (8,%fp),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 2f
movea.l %d0,%a0
move.w (%a2),%d1
movea.l (0x10,%fp),%a1
add.w %d1,%d1
move.w (%a1),(0x40,%a0,%d1.w)
2:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl INQ_DELAY
INQ_DELAY:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
cmpi.w #4,(%a2)
bls.s 1f
move.l #1,(%a3)
bra.s 2f
1:
pea (%a3)
move.l (8,%fp),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 2f
movea.l %d0,%a0
move.w (%a2),%d1
movea.l (0x10,%fp),%a1
add.w %d1,%d1
move.w (0x40,%a0,%d1.w),(%a1)
2:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl TIE
TIE:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
movea.l (%a0),%a0
movea.l (%a1),%a1
move.w (0xa,%a0),(6,%a1)
unlk %fp
rts
.balignw 4,0x2048
