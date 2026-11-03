.text
.globl NILP_$SHOW
NILP_$SHOW:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d2
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
tst.l %d2
beq.s 2f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
jsr (EXEC_$MARK).l
move.l (-4,%fp),%d2
unlk %fp
rts
NILP_$BOTH:
link.w %fp,#-4
move.l (0xa,%fp),%d0
bne.s 1f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.globl NILP_$RUN
NILP_$RUN:
link.w %fp,#-0xc
pea (%a5)
lea (.Ldata).l,%a5
move.w #7,(-2,%fp)
lea (%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(-8,%fp)
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
clr.l -(%sp)
bsr.w NILP_$SHOW
addq.w #8,%sp
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
move.l (-8,%fp),-(%sp)
bsr.w NILP_$SHOW
addq.w #8,%sp
move.w #9,(-2,%fp)
subq.l #2,%sp
clr.l -(%sp)
move.w (-2,%fp),-(%sp)
bsr.s NILP_$BOTH
movea.l (-0x10,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.NILP","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x2a
.space 2
