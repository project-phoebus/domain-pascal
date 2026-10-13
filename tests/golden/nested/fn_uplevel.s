.text
SUM:
link.w %fp,#-8
movea.l (%fp),%a0
move.w (8,%fp),%d0
movea.l (0xa,%a0),%a1
add.w (8,%a0),%d0
add.w (4,%a1),%d0
add.w (-2,%a0),%d0
unlk %fp
rts
.globl NFU_$UP
NFU_$UP:
link.w %fp,#-4
pea (%a2)
movea.l (0xe,%fp),%a2
move.w (%a2),(-2,%fp)
subq.l #2,%sp
move.w #1,-(%sp)
bsr.s SUM
move.w %d0,(%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
