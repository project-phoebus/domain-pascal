.text
PUT:
link.w %fp,#-4
move.b (8,%fp),%d0
move.b %d0,(LAST).l
movea.l (0xa,%fp),%a0
clr.l (%a0)
unlk %fp
rts
PUT2:
link.w %fp,#-4
move.b (8,%fp),%d0
move.b (0xa,%fp),%d1
move.b %d0,(LAST).l
move.b %d1,(FLAG).l
unlk %fp
rts
PUTU:
link.w %fp,#-4
move.b (8,%fp),%d0
move.b %d0,(LASTU).l
movea.l (0xa,%fp),%a0
clr.l (%a0)
unlk %fp
rts
PUTS:
link.w %fp,#-4
move.b (8,%fp),%d0
move.b %d0,(LASTS).l
movea.l (0xa,%fp),%a0
clr.l (%a0)
unlk %fp
rts
.globl BYTE_CONSTS_$TEST
BYTE_CONSTS_$TEST:
link.w %fp,#0
subq.l #2,%sp
move.l #BUF,-(%sp)
move.w #-0x200,-(%sp)
bsr.s PUT
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
clr.w -(%sp)
bsr.w PUT
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
move.w #0x4100,-(%sp)
bsr.w PUT
addq.w #8,%sp
st -(%sp)
move.w #0x1b00,-(%sp)
bsr.w PUT2
addq.w #4,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
move.w #-0x8000,-(%sp)
bsr.s PUTU
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
move.w #-0x200,-(%sp)
bsr.s PUTS
addq.w #8,%sp
subq.l #2,%sp
move.l #BUF,-(%sp)
move.w #0x7f00,-(%sp)
bsr.w PUTS
unlk %fp
rts
.balignw 4,0x2048
