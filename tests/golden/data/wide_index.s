.text
.globl BY_AREA_ID
BY_AREA_ID:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
seq %d1
cmp.w (N_AREAS).l,%d0
shi %d2
or.b %d2,%d1
bpl.s 1f
clr.l %d0
bra.s 2f
1:
clr.l %d1
movea.l #AREAS,%a0
move.w %d0,%d1
lsl.l #4,%d1
move.l %d1,%d0
add.l %d0,%d0
add.l %d0,%d1
move.l (-0x30,%a0,%d1.l),%d0
2:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BY_LOCAL
BY_LOCAL:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
clr.l %d0
movea.l #AREAS,%a0
move.w %d1,%d0
lsl.l #4,%d0
move.l %d0,%d1
add.l %d1,%d1
add.l %d1,%d0
move.l (-0x30,%a0,%d0.l),%d0
unlk %fp
rts
.globl BY_ID
BY_ID:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #AREAS,%a0
ext.l %d0
lsl.l #4,%d0
move.l %d0,%d1
add.l %d1,%d1
add.l %d1,%d0
move.l (-0x30,%a0,%d0.l),%d0
unlk %fp
rts
.globl BY_228
BY_228:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #T228,%a0
move.w %d0,%d1
muls.w #0xe4,%d1
lea (0,%a0,%d1.l),%a1
move.l (-0xe4,%a1),%d0
unlk %fp
rts
.globl BY_150
BY_150:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #T150,%a0
move.w %d0,%d1
mulu.w #0x96,%d1
move.w (0,%a0,%d1.l),%d0
unlk %fp
rts
.globl BY_CHAR
BY_CHAR:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
clr.l %d0
movea.l #CHARS,%a0
move.w %d1,%d0
move.b (-1,%a0,%d0.l),%d0
unlk %fp
rts
.balignw 4,0x2048
