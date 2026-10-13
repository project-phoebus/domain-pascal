.text
.globl BODY_$FIRST
BODY_$FIRST:
link.w %fp,#0
bsr.s WORKER
bsr.s BODY_$LATE
jsr (BODY_$ELSEWHERE).l
unlk %fp
rts
.globl BODY_$LATE
BODY_$LATE:
link.w %fp,#0
move.w #1,(X).l
unlk %fp
rts
WORKER:
link.w %fp,#-4
move.w #2,(X).l
unlk %fp
rts
.balignw 4,0x2048
