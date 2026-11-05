.text
.globl TWICE_PLUS_ONE
TWICE_PLUS_ONE:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),%d0
move.w %d0,%d1
add.w %d0,%d1
move.w %d1,%d0
addq.w #1,%d0
move.w %d0,(%a0)
unlk %fp
rts
.globl FORWARD_$MAIN
FORWARD_$MAIN:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
addq.w #1,(-2,%fp)
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (-2,%fp),%d2
move.l #0x10000,-(%sp)
ext.l %d2
move.l %d2,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
pea (-2,%fp)
bsr.s TWICE_PLUS_ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl FORWARD_$BOOL
FORWARD_$BOOL:
link.w %fp,#-4
move.w (8,%fp),%d0
cmpi.w #0xa,%d0
ble.s 1f
st (-2,%fp)
bra.s 2f
1:
clr.b (-2,%fp)
2:
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.b (-2,%fp)
bpl.s 3f
clr.b (-2,%fp)
bra.s 4f
3:
st (-2,%fp)
4:
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
unlk %fp
rts
.balignw 4,0x2048
