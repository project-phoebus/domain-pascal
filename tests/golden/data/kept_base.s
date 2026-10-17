.text
.globl CLEAR_TWO
CLEAR_TWO:
link.w %fp,#0
move.w (AS_ONE).l,%d0
movea.l #FIM_$TRACE_STS,%a0
lsl.w #2,%d0
clr.l (0,%a0,%d0.w)
move.w (AS_TWO).l,%d0
lsl.w #2,%d0
clr.l (0,%a0,%d0.w)
unlk %fp
rts
.globl HANDLER
HANDLER:
link.w %fp,#0
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
movea.l #DTTE,%a0
add.w %d0,%d1
tst.l (0x28,%a0,%d1.w)
bne.s 1f
clr.w (COUNT).l
bra.s 2f
1:
move.l (0x28,%a0,%d1.w),(LAST).l
2:
unlk %fp
rts
.globl ACROSS_IF
ACROSS_IF:
link.w %fp,#0
move.w (8,%fp),%d0
movea.l #FIM_$QUIT_INH,%a0
tst.b (0,%a0,%d0.w)
bmi.s 1f
addq.w #1,(COUNT).l
1:
st (0,%a0,%d0.w)
unlk %fp
rts
.balignw 4,0x2048
