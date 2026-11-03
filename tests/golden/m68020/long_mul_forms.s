.arch 68020
.text
.globl MT_$MS
MT_$MS:
link.w %fp,#-4
move.l #0x3e8,%d0
muls.l (8,%fp),%d0
unlk %fp
rts
.globl OS_$SQUARE
OS_$SQUARE:
link.w %fp,#-8
move.l (-4,%fp),%d0
muls.l %d0,%d0
move.l %d0,(-4,%fp)
movea.l (8,%fp),%a0
move.l (-4,%fp),(%a0)
unlk %fp
rts
.balignw 4,0x2048
