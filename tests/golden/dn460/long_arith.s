.text
.globl TIME_$GET_ADJUST
TIME_$GET_ADJUST:
link.w %fp,#-4
movea.l (8,%fp),%a1
ori.w #0x700,%sr
move.l (TIME_$CURRENT_DELTA).l,%d1
andi.w #0xf8ff,%sr
move.l %d1,%d0
.short 0x717c
.long 0x3d090
move.l %d0,(%a1)
move.l #0x3d090,-(%sp)
move.l %d1,-(%sp)
jsr (M$OIS$LLL).l
lsl.l #2,%d0
move.l %d0,(4,%a1)
unlk %fp
rts
.globl TIME_$SCALE
TIME_$SCALE:
link.w %fp,#-4
move.l (8,%fp),%d0
.short 0x713c
.long 0x3d090
unlk %fp
rts
.globl TIME_$ROUND
TIME_$ROUND:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (0xc,%fp),%d0
move.l (8,%fp),%d1
ext.l %d0
move.w (0xc,%fp),%d2
ext.l %d2
.short 0x7342
.short 0x7101
move.l (-8,%fp),%d2
unlk %fp
rts
.globl MT_$MS
MT_$MS:
link.w %fp,#-4
move.l #0x3e8,%d0
move.l (8,%fp),%d1
.short 0x7101
unlk %fp
rts
.globl OS_$SQUARE
OS_$SQUARE:
link.w %fp,#-8
move.l (-4,%fp),%d0
.short 0x7100
move.l %d0,(-4,%fp)
move.l (-4,%fp),(PMAP_$Q).l
unlk %fp
rts
.globl PMAP_$PURIFY
PMAP_$PURIFY:
link.w %fp,#0
move.l (PMAP_$X).l,%d0
moveq #6,%d1
.short 0x80c9
move.l %d0,(PMAP_$Q).l
unlk %fp
rts
.globl PCHIST_$SPAN
PCHIST_$SPAN:
link.w %fp,#-4
clr.l %d0
move.l (0xa,%fp),%d1
move.w (8,%fp),%d0
.short 0xc0c9
unlk %fp
rts
.globl AST_$QUOT
AST_$QUOT:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l (0xc,%fp),%a0
move.l (%a0),%d1
.short 0x80c9
unlk %fp
rts
.globl PCHIST_$SCALED
PCHIST_$SCALED:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
move.l (8,%fp),%d1
move.l (%a0),%d0
.short 0xc0c9
unlk %fp
rts
.balignw 4,0x2048
