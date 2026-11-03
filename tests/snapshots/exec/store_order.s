.text
.globl STORE_ORDER_$SET
STORE_ORDER_$SET:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
cmpi.w #4,(%a2)
bls.s 1f
moveq #2,%d0
move.l %d0,(%a3)
bra.s 2f
1:
pea (%a3)
move.l (8,%fp),-(%sp)
jsr (STORE_ORDER_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 2f
movea.l %d0,%a0
move.w (%a2),%d1
movea.l (0x10,%fp),%a1
add.w %d1,%d1
move.w (%a1),(4,%a0,%d1.w)
2:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl STORE_ORDER_$GET
STORE_ORDER_$GET:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
cmpi.w #4,(%a2)
bls.s 1f
moveq #2,%d0
move.l %d0,(%a3)
bra.s 2f
1:
pea (%a3)
move.l (8,%fp),-(%sp)
jsr (STORE_ORDER_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 2f
movea.l %d0,%a0
move.w (%a2),%d1
movea.l (0x10,%fp),%a1
add.w %d1,%d1
move.w (4,%a0,%d1.w),(%a1)
2:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.balignw 4,0x2048
