.text
.globl EVEN
EVEN:
link.w %fp,#-4
move.w (8,%fp),%d0
btst #0,%d0
seq %d0
unlk %fp
rts
.globl FOUR
FOUR:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d2
clr.w (-2,%fp)
cmpi.w #1,%d2
beq.s 2f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s EVEN
addq.w #4,%sp
tst.b %d0
bpl.s 1f
2:
move.w #1,(-2,%fp)
1:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (-2,%fp)
cmpi.w #2,%d2
ble.s 3f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s EVEN
addq.w #4,%sp
tst.b %d0
bpl.s 3f
move.w #1,(-2,%fp)
3:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (-2,%fp)
cmpi.w #2,%d2
ble.s 4f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s EVEN
addq.w #4,%sp
tst.b %d0
bmi.s 4f
move.w #1,(-2,%fp)
4:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
clr.w (-2,%fp)
cmpi.w #3,%d2
beq.s 6f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w EVEN
addq.w #4,%sp
tst.b %d0
bmi.s 5f
6:
move.w #1,(-2,%fp)
5:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl AND_OR_BOOL_CALL_$RUN
AND_OR_BOOL_CALL_$RUN:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w FOUR
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
bsr.w FOUR
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bsr.w FOUR
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w FOUR
unlk %fp
rts
.balignw 4,0x2048
