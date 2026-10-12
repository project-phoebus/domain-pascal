.text
.globl Q_INIT
Q_INIT:
link.w %fp,#0
move.b (8,%fp),%d0
move.w (0xa,%fp),%d1
movea.l (0xc,%fp),%a0
clr.l (%a0)
clr.l (4,%a0)
move.b %d0,(8,%a0)
move.w %d1,(0xa,%a0)
unlk %fp
rts
.balignw 4,0x2048
