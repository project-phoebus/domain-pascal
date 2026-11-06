.text
.globl P
P:
link.w %fp,#-8
move.l %d2,-(%sp)
moveq #1,%d2
pea (-8,%fp)
jsr (GET_CLOCK).l
addq.w #1,%d2
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl Q
Q:
link.w %fp,#-0xc
move.l %d2,-(%sp)
moveq #1,%d2
pea (-0xa,%fp)
jsr (GET_WORDS).l
addq.l #1,%d2
move.l (-0x10,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
