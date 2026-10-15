.text
.globl IS_TWO
IS_TWO:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #PROC1_$TYPE,%a0
move.w %d0,%d1
add.w %d1,%d1
cmpi.w #2,(-2,%a0,%d1.w)
sne %d1
move.b %d1,%d0
unlk %fp
rts
.globl QUIT_EC_VALUE
QUIT_EC_VALUE:
link.w %fp,#-4
move.w (PROC1_$AS_ID).l,%d0
movea.l #FIM_$QUIT_EC,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
move.l (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl BIG
BIG:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
movea.l #BIGS,%a0
add.w %d0,%d1
move.l (-0x38,%a0,%d1.w),%d0
unlk %fp
rts
.globl KTT
KTT:
link.w %fp,#-4
clr.w %d0
movea.l #SMD_$KTT,%a0
move.b (8,%fp),%d0
move.b (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl THROUGH_PTR
THROUGH_PTR:
link.w %fp,#-4
clr.w %d0
movea.l (KTT_PTR).l,%a0
move.b (8,%fp),%d0
move.b (0,%a0,%d0.w),%d0
unlk %fp
rts
.balignw 4,0x2048
