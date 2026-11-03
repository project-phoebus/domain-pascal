.text
.globl BX_MISC_$KEPT
BX_MISC_$KEPT:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.l %d2
tst.w %d1
ble.s 1f
move.w %d0,%d1
movea.l #EB_$LONGS,%a0
lsl.w #2,%d1
move.l (4,%a0,%d1.w),%d2
1:
move.w %d0,%d1
movea.l #EB_$LONGS,%a0
lsl.w #2,%d1
move.l %d2,%d0
add.l (0,%a0,%d1.w),%d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BX_MISC_$PAD
BX_MISC_$PAD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.w (0xc,%fp),%d1
move.w %d1,(4,%a5)
move.l %d0,%d1
subq.l #2,%sp
addq.l #1,%d1
move.w (4,%a5),-(%sp)
move.l %d1,-(%sp)
jsr (M$DIS$LLW).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
INNER:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l (%fp),%a2
tst.w %d2
ble.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%a2),%d0
addq.w #1,%d0
move.w %d0,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
move.w %d2,-(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl BX_MISC_$OUTER
BX_MISC_$OUTER:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d2
move.w %d2,(-2,%fp)
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s INNER
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_MISC","aw",@nobits
.balign 4
.Ldata:
.space 8
