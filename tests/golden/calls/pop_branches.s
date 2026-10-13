.text
POPB_$IF_FOLLOWS:
link.w %fp,#-4
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
tst.w (W).l
bne.s 1f
move.w #1,(R).l
1:
unlk %fp
rts
POPB_$IN_PARTS:
link.w %fp,#-4
tst.w (W).l
bne.s 1f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXT_$P).l
bra.s 2f
1:
move.w #1,(R).l
2:
move.w #2,(R).l
unlk %fp
rts
POPB_$CALL_AFTER_JOIN:
link.w %fp,#-4
tst.w (W).l
bne.s 1f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXT_$P).l
unlk %fp
rts
POPB_$RETURN:
link.w %fp,#-4
tst.w (W).l
bne.s 1f
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXT_$P).l
bra.s 2f
1:
subq.l #2,%sp
move.w #6,-(%sp)
jsr (EXT_$P).l
move.w #1,(R).l
2:
unlk %fp
rts
POPB_$LOOP_FOLLOWS:
link.w %fp,#-8
subq.l #2,%sp
move.w #7,-(%sp)
jsr (EXT_$P).l
addq.w #4,%sp
movea.l #PAGE,%a0
moveq #0xf,%d0
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d0,1b
unlk %fp
rts
.balignw 4,0x2048
