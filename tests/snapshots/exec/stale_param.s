.text
.globl INCW
INCW:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),%d0
addi.w #0xa,%d0
move.w %d0,(%a0)
unlk %fp
rts
.globl INCL
INCL:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l (%a0),%d0
addi.l #0xa,%d0
move.l %d0,(%a0)
unlk %fp
rts
.globl STALE_PARAM_$MAIN
STALE_PARAM_$MAIN:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
pea (8,%fp)
bsr.s INCW
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
pea (0xa,%fp)
bsr.s INCL
addq.w #4,%sp
move.l (0xa,%fp),%d0
addq.l #1,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
pea (8,%fp)
bsr.s INCW
addq.w #4,%sp
cmpi.w #0x1e,(8,%fp)
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bra.s 2f
1:
jsr (EXEC_$CLOBBER).l
2:
subq.l #2,%sp
move.w (8,%fp),%d0
addq.w #1,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
SETK:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
pea (8,%a2)
bsr.w INCW
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl STALE_PARAM_$NESTED
STALE_PARAM_$NESTED:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
bsr.s SETK
subq.l #2,%sp
move.w (8,%fp),%d0
addq.w #1,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.balignw 4,0x2048
