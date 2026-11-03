.text
.globl GETP
GETP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
tst.l (%a1)
bpl.s 1f
moveq #1,%d0
move.l %d0,(%a0)
bra.s 2f
1:
clr.l (%a0)
2:
move.l (4,%a5),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CALL_RESULT_FIRST_$DIES
CALL_RESULT_FIRST_$DIES:
link.w %fp,#-4
movem.l %d2/%a2-%a3,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a3
pea (%a3)
pea (8,%fp)
bsr.s GETP
addq.w #8,%sp
movea.l %a0,%a2
tst.l (%a3)
bne.s 1f
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (%a2),%d0
add.l %d2,%d0
move.l %d0,(%a2)
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
1:
movem.l (-0x10,%fp),%d2/%a2-%a3
unlk %fp
rts
.globl CALL_RESULT_FIRST_$KEPT
CALL_RESULT_FIRST_$KEPT:
link.w %fp,#-4
movem.l %d2/%a2-%a3,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
bsr.w GETP
addq.w #8,%sp
movea.l %a0,%a3
tst.l (%a2)
bne.s 1f
move.l (%a3),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (%a3),%d0
add.l %d2,%d0
move.l %d0,(%a3)
move.l (%a3),-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (%a3),%d0
addq.l #1,%d0
move.l %d0,(%a2)
1:
movem.l (-0x10,%fp),%d2/%a2-%a3
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.CALL_RESULT_FIRST","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x00,0x05
.long .Ldata
