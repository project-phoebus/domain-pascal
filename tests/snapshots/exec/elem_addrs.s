.text
.globl BUMP
BUMP:
link.w %fp,#0
movea.l (8,%fp),%a0
addi.l #0xa,(%a0)
unlk %fp
rts
.globl EA_$SUM
EA_$SUM:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a0
move.l (%a0),%d0
add.l (4,%a0),%d0
add.l (8,%a0),%d0
unlk %fp
rts
.globl EA_$BUMP
EA_$BUMP:
link.w %fp,#-8
pea (%a2)
move.w (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a2
pea (%a2)
bsr.s BUMP
addq.w #4,%sp
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (4,%a2),%d0
add.l (8,%a2),%d0
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl EA_$BIG
EA_$BIG:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
movea.l #EB_$BIG,%a0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a0
move.l (-0x38,%a0),%d0
add.l (-4,%a0),%d0
unlk %fp
rts
.globl EA_$LONG
EA_$LONG:
link.w %fp,#-8
move.l (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.l %d0,%d1
lsl.l #2,%d1
move.l %d1,%d0
add.l %d0,%d0
add.l %d0,%d1
lea (0,%a0,%d1.l),%a0
move.l (%a0),%d0
add.l (4,%a0),%d0
unlk %fp
rts
.globl EA_$NEXT
EA_$NEXT:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a0
move.l (0xc,%a0),%d0
add.l (0x10,%a0),%d0
unlk %fp
rts
.globl EA_$NEG
EA_$NEG:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l #EB_$NEGR,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a0
move.l (0x24,%a0),%d0
add.l (0x28,%a0),%d0
add.l (0x2c,%a0),%d0
unlk %fp
rts
.globl EA_$MOVED
EA_$MOVED:
link.w %fp,#-8
move.w (EB_$K).l,%d0
movea.l #EB_$RECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (0,%a0,%d0.w),%a0
addq.w #1,(EB_$K).l
move.l (%a0),%d0
add.l (4,%a0),%d0
unlk %fp
rts
.globl BUMPK
BUMPK:
link.w %fp,#0
addq.w #1,(EB_$K).l
unlk %fp
rts
.globl EA_$KCALL
EA_$KCALL:
link.w %fp,#-8
pea (%a2)
move.w (EB_$K).l,%d0
movea.l #EB_$RECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (0,%a0,%d0.w),%a2
bsr.s BUMPK
move.l (%a2),%d0
add.l (8,%a2),%d0
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl EA_$VIDX
EA_$VIDX:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),%d0
lsl.l #3,%d0
move.l %d0,%d1
neg.l %d0
lsl.l #3,%d1
movea.l #EB_$BIG,%a0
add.l %d1,%d0
lea (0,%a0,%d0.l),%a0
move.l (-0x38,%a0),%d0
add.l (-4,%a0),%d0
unlk %fp
rts
.globl EA_$BRANCH
EA_$BRANCH:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a0
cmpi.l #0x1e,(%a0)
ble.s 1f
move.l (4,%a0),%d0
bra.s 2f
1:
move.l (8,%a0),%d0
2:
unlk %fp
rts
.globl EA_$NEGCALL
EA_$NEGCALL:
link.w %fp,#-0xc
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d0
movea.l #EB_$NEGR,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a2
move.l (0x24,%a2),%d2
jsr (EXEC_$CLOBBER).l
move.l %d2,%d0
add.l (0x2c,%a2),%d0
movem.l (-0x14,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
