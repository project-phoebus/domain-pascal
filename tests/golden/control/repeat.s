.text
RPT_$ONE:
link.w %fp,#0
clr.w (W).l
.balignw 4,0x2048
1:
move.w #1,(R).l
cmpi.w #3,(W).l
bne.s 1b
unlk %fp
rts
RPT_$OR_ELSE:
link.w %fp,#0
clr.w (R).l
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
cmpi.w #0xa,(W).l
bge.s 2f
tst.b (B).l
bpl.s 1b
2:
unlk %fp
rts
.balignw 4,0x2048
