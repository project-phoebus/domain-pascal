.text
CUR:
link.w %fp,#-8
movea.l (%fp),%a0
move.l (-4,%a0),(-8,%fp)
movea.l (-8,%fp),%a0
unlk %fp
rts
.globl NFP_$PTR
NFP_$PTR:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d2
move.l (0xa,%fp),%d0
move.l %d0,(-4,%fp)
bsr.s CUR
move.w %d2,(%a0)
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
