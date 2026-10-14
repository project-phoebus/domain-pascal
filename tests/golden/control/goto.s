.text
GT_$FORWARD:
link.w %fp,#-4
clr.w (W).l
tst.b (B).l
bmi.s 1f
move.w #1,(R).l
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXT_$P).l
1:
move.w #3,(R).l
unlk %fp
rts
GT_$BACKWARD:
link.w %fp,#-4
clr.w (W).l
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w (W).l,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
tst.w (R).l
bne.s 1b
move.w #1,(W).l
unlk %fp
rts
GT_$ELSE_GOTO:
link.w %fp,#-4
cmpi.w #1,(W).l
bne.s 1f
tst.b (B).l
bpl.s 2f
move.w #5,(R).l
bra.s 3f
1:
move.w #6,(R).l
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXT_$P).l
3:
move.w #2,(W).l
2:
move.w #7,(R).l
unlk %fp
rts
GT_$ARMS:
link.w %fp,#-4
move.w (W).l,%d0
cmpi.w #4,%d0
bcc.s 1f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 3f-6b
.short 4f-6b
.short 5f-6b
2:
move.w #1,(R).l
bra.s 1f
3:
tst.b (B).l
bpl.s 7f
move.w #2,(R).l
bra.s 1f
4:
move.w #3,(R).l
bra.s 1f
5:
7:
move.w #4,(R).l
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXT_$P).l
1:
unlk %fp
rts
GT_$OUT_WHILE:
link.w %fp,#-4
clr.w (R).l
bra.s 2f
.balignw 4,0x2048
1:
move.w (R).l,%d0
cmp.w (W).l,%d0
beq.s 4f
addq.w #1,(W).l
2:
cmpi.w #0xa,(W).l
blt.s 1b
move.w #1,(R).l
4:
subq.l #2,%sp
move.w (R).l,-(%sp)
jsr (EXT_$P).l
unlk %fp
rts
GT_$OUT_FOR:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
moveq #9,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
tst.b (B).l
bmi.s 4f
addq.w #1,%d3
dbf %d2,2b
move.w #1,(R).l
bra.s 5f
4:
move.w #2,(R).l
5:
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
GT_$AND_THEN:
link.w %fp,#-4
cmpi.w #1,(W).l
bne.s 2f
tst.b (B).l
bmi.s 1f
2:
move.w #1,(R).l
1:
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXT_$P).l
unlk %fp
rts
GT_$RESULT:
link.w %fp,#-8
moveq #1,%d0
tst.b (B).l
bmi.s 1f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXT_$P).l
moveq #2,%d0
1:
unlk %fp
rts
.balignw 4,0x2048
