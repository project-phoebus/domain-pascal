.text
STORE_RESULT:
link.w %fp,#0
move.l (L).l,-(%sp)
jsr (G).l
move.l %d0,(L).l
unlk %fp
rts
ARITH_ON_RESULT:
link.w %fp,#0
subq.l #2,%sp
move.w (R).l,-(%sp)
jsr (F).l
move.w %d0,%d1
addq.w #1,%d1
move.w %d1,(W).l
unlk %fp
rts
TEST_RESULT:
link.w %fp,#0
subq.l #2,%sp
move.w (W).l,-(%sp)
jsr (B).l
addq.w #4,%sp
tst.b %d0
bpl.s 1f
clr.w (R).l
1:
move.w #1,(R).l
unlk %fp
rts
WRAPPER:
link.w %fp,#-4
move.l (8,%fp),-(%sp)
jsr (G).l
unlk %fp
rts
BIT13:
link.w %fp,#-4
movea.l (RP).l,%a0
move.w (0x16,%a0),%d0
btst #13,%d0
sne %d0
unlk %fp
rts
BIT4:
link.w %fp,#0
movea.l (RP).l,%a0
move.w (0x16,%a0),%d0
btst #4,%d0
beq.s 1f
clr.w (R).l
1:
move.w #1,(R).l
unlk %fp
rts
POSITIVE:
link.w %fp,#-4
tst.w (W).l
sgt %d0
unlk %fp
rts
NOT_TWO:
link.w %fp,#-4
cmpi.w #2,(W).l
sne %d0
unlk %fp
rts
.balignw 4,0x2048
