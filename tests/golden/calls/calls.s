.text
WORKER:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXT_$LONG).l
unlk %fp
rts
.globl CALLS_$NEAR
CALLS_$NEAR:
link.w %fp,#0
move.l (8,%fp),-(%sp)
bsr.s WORKER
unlk %fp
rts
.globl CALLS_$CONSTS
CALLS_$CONSTS:
link.w %fp,#0
clr.l -(%sp)
jsr (EXT_$LONG).l
addq.w #4,%sp
pea (0x22).w
jsr (EXT_$LONG).l
addq.w #4,%sp
pea (-2).w
jsr (EXT_$LONG).l
addq.w #4,%sp
move.l #0x12345,-(%sp)
jsr (EXT_$LONG).l
addq.w #4,%sp
subq.l #2,%sp
clr.w -(%sp)
jsr (EXT_$WORD).l
addq.w #4,%sp
subq.l #2,%sp
move.w #-5,-(%sp)
jsr (EXT_$WORD).l
addq.w #4,%sp
move.l (8,%fp),-(%sp)
move.l #0x10020,-(%sp)
jsr (EXT_$PAIR).l
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
st -(%sp)
jsr (EXT_$FLAG).l
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
clr.w -(%sp)
jsr (EXT_$FLAG).l
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
move.b (0xe,%fp),-(%sp)
jsr (EXT_$FLAG).l
addq.w #8,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
jsr (EXT_$WORD).l
addq.w #4,%sp
move.l #UID_$NIL,-(%sp)
jsr (EXT_$UID).l
addq.w #4,%sp
move.l (8,%fp),-(%sp)
move.l (8,%fp),-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXT_$THREE).l
lea (0xc,%sp),%sp
move.l (8,%fp),-(%sp)
bsr.w WORKER
unlk %fp
rts
.globl CALLS_$TAIL
CALLS_$TAIL:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXT_$LONG).l
move.w #3,(COUNT).l
unlk %fp
rts
.balignw 4,0x2048
