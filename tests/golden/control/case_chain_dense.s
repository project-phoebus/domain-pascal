.text
.globl ERROR_QUE
ERROR_QUE:
link.w %fp,#0
move.w (8,%fp),%d0
move.w %d0,%d1
cmpi.w #2,%d1
beq.s 2f
cmpi.w #3,%d1
beq.s 3f
cmpi.w #0x11,%d1
beq.s 4f
cmpi.w #0x12,%d1
beq.s 4f
cmpi.w #0x10,%d1
beq.s 4f
bra.s 1f
2:
move.w #1,(Q).l
bra.s 1f
3:
move.w #2,(Q).l
bra.s 1f
4:
move.w #3,(Q).l
1:
unlk %fp
rts
.balignw 4,0x2048
