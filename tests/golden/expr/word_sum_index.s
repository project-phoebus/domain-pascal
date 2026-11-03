.text
.globl P
P:
link.w %fp,#-0x80
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d1,%d2
add.w %d0,%d2
lsl.w #2,%d2
clr.l (-0x80,%fp,%d2.w)
move.b %d1,%d2
add.b %d0,%d2
subq.b #1,%d2
move.b %d2,(B).l
move.l (-0x84,%fp),%d2
unlk %fp
rts
.globl FMT
FMT:
link.w %fp,#0
move.b (VOL+3).l,%d0
lsl.b #2,%d0
subq.b #1,%d0
move.b %d0,(B).l
unlk %fp
rts
.balignw 4,0x2048
