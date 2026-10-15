.text
FEV_$MID:
link.w %fp,#-4
clr.w (W).l
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
tst.b (B).l
bmi.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
bra.s 1b
2:
move.w #1,(R).l
unlk %fp
rts
FEV_$LAST_IF:
link.w %fp,#-4
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
tst.b (B).l
bpl.s 1b
move.w #2,(R).l
cmpi.w #3,(W).l
bne.s 1b
move.w #1,(R).l
unlk %fp
rts
FEV_$LAST_EXIT:
link.w %fp,#-4
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
cmpi.w #3,(W).l
beq.s 1b
move.w #2,(R).l
move.w #1,(R).l
unlk %fp
rts
FEV_$NO_WAY_OUT:
link.w %fp,#-4
clr.w (W).l
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
addq.w #1,(W).l
bra.s 1b
.balignw 4,0x2048
