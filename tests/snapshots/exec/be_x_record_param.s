.text
.globl MODFIRST
MODFIRST:
link.w %fp,#-4
move.w #0x63,(8,%fp)
move.l (8,%fp),(-4,%fp)
move.l (-4,%fp),%d0
unlk %fp
rts
.globl COND
COND:
link.w %fp,#-4
move.w (0xc,%fp),%d0
ble.s 1f
move.w #0x2a,(0xa,%fp)
1:
jsr (EXEC_$CLOBBER).l
move.l (8,%fp),(-4,%fp)
move.l (-4,%fp),%d0
unlk %fp
rts
.globl TWICE
TWICE:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.w (0x10,%fp),%d2
move.l %d0,(-4,%fp)
tst.w %d2
ble.s 1f
move.l %d1,(-4,%fp)
move.l %d1,%d0
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BE_X_RECORD_PARAM_$MAIN
BE_X_RECORD_PARAM_$MAIN:
link.w %fp,#-8
move.w #7,(-4,%fp)
move.w #8,(-2,%fp)
move.l (-4,%fp),-(%sp)
bsr.s MODFIRST
addq.w #4,%sp
move.l %d0,(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-6,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
move.l (-4,%fp),-(%sp)
bsr.w COND
addq.w #8,%sp
move.l %d0,(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-6,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w #5,(-8,%fp)
move.w #6,(-6,%fp)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
move.l (-8,%fp),-(%sp)
move.l (-4,%fp),-(%sp)
bsr.w TWICE
lea (0xc,%sp),%sp
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
