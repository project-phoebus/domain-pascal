.arch 68020
.text
.globl TIME_$GET_ADJUST
TIME_$GET_ADJUST:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l (TIME_$CURRENT_DELTA).l,%d0
move.l %d0,%d1
divs.l #0x3d090,%d1
move.l %d1,(%a0)
move.l %d0,%d1
divsl.l #0x3d090,%d0:%d1
lsl.l #2,%d0
move.l %d0,(4,%a0)
unlk %fp
rts
.globl PCHIST_$SPAN
PCHIST_$SPAN:
link.w %fp,#-4
move.w (8,%fp),%d0
ext.l %d0
muls.l (0xa,%fp),%d0
unlk %fp
rts
.globl BAT_$OFFSET
BAT_$OFFSET:
link.w %fp,#-4
move.w (8,%fp),%d0
ext.l %d0
muls.l (0xa,%fp),%d0
unlk %fp
rts
.globl AREA_$HASH
AREA_$HASH:
link.w %fp,#-4
move.l (8,%fp),%d0
divsl.l #0xb,%d1:%d0
move.w %d1,%d0
unlk %fp
rts
.balignw 4,0x2048
