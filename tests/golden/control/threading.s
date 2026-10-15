.text
.globl SET_FLAG
SET_FLAG:
link.w %fp,#0
movea.l (8,%fp),%a0
tst.b (%a0)
bpl.s 1f
bset #4,(FLAGS+1).l
move.w (HEAD).l,%d0
cmp.w (TAIL).l,%d0
beq.s 3f
subq.l #2,%sp
move.w #0x1a,-(%sp)
jsr (SIG).l
bra.s 3f
1:
bclr #4,(FLAGS+1).l
3:
unlk %fp
rts
.globl ELSE_WHILE
ELSE_WHILE:
link.w %fp,#0
tst.w (W).l
bne.s 4f
clr.w (R).l
bra.s 2f
.balignw 4,0x2048
3:
subq.w #1,(R).l
4:
tst.w (R).l
bgt.s 3b
2:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (SIG).l
unlk %fp
rts
.globl NESTED
NESTED:
link.w %fp,#0
cmpi.w #1,(W).l
bne.s 1f
cmpi.w #1,(R).l
bne.s 2f
move.w #2,(W).l
bra.s 5f
2:
move.w #3,(R).l
bra.s 4f
1:
move.w #0x1e,(R).l
5:
subq.l #2,%sp
move.w #2,-(%sp)
jsr (SIG).l
4:
unlk %fp
rts
.balignw 4,0x2048
