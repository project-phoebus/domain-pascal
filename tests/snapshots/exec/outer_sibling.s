.text
BUMP:
link.w %fp,#-4
movea.l %a1,%a0
move.w (-2,%a0),%d0
add.w (8,%fp),%d0
move.w %d0,(-2,%a0)
unlk %fp
rts
INNER:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (%fp),%a2
move.w #1,-(%sp)
movea.l (-4,%a2),%a1
bsr.s BUMP
addq.w #4,%sp
movea.l (-4,%a2),%a0
pea (-2,%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%a2),-(%sp)
movea.l (-4,%a2),%a1
bsr.s BUMP
movea.l (-8,%fp),%a2
unlk %fp
rts
MID:
link.w %fp,#-8
movea.l (%fp),%a0
lea (0,%a0),%a1
move.l %a1,(-4,%fp)
move.w #0xa,(-6,%fp)
bsr.s INNER
unlk %fp
rts
.globl OUTER_SIBLING_$MAIN
OUTER_SIBLING_$MAIN:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
bsr.s MID
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
