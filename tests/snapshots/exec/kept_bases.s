.text
.globl KB_$TWO
KB_$TWO:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
movea.l #EB_$WORDS,%a0
add.w %d2,%d2
move.w (-2,%a0,%d2.w),%d1
move.w %d0,%d2
move.w %d1,%d0
add.w %d2,%d2
add.w (-2,%a0,%d2.w),%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl KB_$WORD
KB_$WORD:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d0
unlk %fp
rts
.globl KB_$SET2
KB_$SET2:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
movea.l #EB_$WORDS,%a0
add.w %d2,%d2
move.w #5,(-2,%a0,%d2.w)
move.w %d1,%d0
add.w %d0,%d0
move.w #6,(-2,%a0,%d0.w)
move.l (-4,%fp),%d2
unlk %fp
rts
.globl KB_$COND
KB_$COND:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
cmpi.l #0x14,(0,%a0,%d1.w)
ble.s 1f
moveq #1,%d0
bra.s 2f
1:
moveq #2,%d0
2:
add.l (4,%a0,%d1.w),%d0
unlk %fp
rts
.globl KB_$BIG
KB_$BIG:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d1
clr.l %d0
move.w %d1,%d2
lsl.w #3,%d2
move.w %d2,%d1
neg.w %d2
lsl.w #3,%d1
movea.l #EB_$BIG,%a0
add.w %d1,%d2
tst.l (-0x38,%a0,%d2.w)
beq.s 1f
move.l (-4,%a0,%d2.w),%d0
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
