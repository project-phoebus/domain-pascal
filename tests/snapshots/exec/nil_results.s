.text
.globl NONE
NONE:
link.w %fp,#-4
movea.l #0,%a0
unlk %fp
rts
.globl AOP
AOP:
link.w %fp,#-8
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.w (0x18,%a5),%d1
lsl.w #2,%d1
movea.l (0,%a5,%d1.w),%a0
move.w (0x18,%a5),%d1
move.w (0x1a,%a5),%d2
ext.l %d1
ext.l %d2
addq.l #1,%d1
cmp.l %d2,%d1
slt %d2
tst.w (%a0)
sne %d1
and.b %d1,%d2
bpl.s 1f
addq.w #1,(0x18,%a5)
movea.l %a0,%a2
bra.s 2f
1:
tst.b %d0
bmi.s 3f
suba.l %a2,%a2
bra.s 4f
3:
subq.l #2,%sp
move.w #-1,-(%sp)
jsr (EXEC_$LOGV_W).l
4:
2:
movea.l %a2,%a0
movem.l (-0x14,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl NIL_RESULTS_$RUN
NIL_RESULTS_$RUN:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #3,-(%sp)
bsr.w NONE
addq.w #4,%sp
cmpa.w #0,%a0
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
1:
clr.w (0x10,%a5)
move.w #7,(0x12,%a5)
clr.w (0x14,%a5)
move.w #9,(0x16,%a5)
lea (0x10,%a5),%a0
move.l %a0,(%a5)
lea (0x12,%a5),%a0
move.l %a0,(4,%a5)
lea (0x14,%a5),%a0
move.l %a0,(8,%a5)
lea (0x16,%a5),%a0
move.l %a0,(0xc,%a5)
move.w #4,(0x1a,%a5)
move.w #1,(0x18,%a5)
subq.l #2,%sp
clr.w -(%sp)
bsr.w AOP
addq.w #4,%sp
movea.l %a0,%a2
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w -(%sp)
bsr.w AOP
addq.w #4,%sp
movea.l %a0,%a2
cmpa.w #0,%a2
bne.s 2f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
move.w #3,(0x18,%a5)
move.w #5,(0x1a,%a5)
subq.l #2,%sp
st -(%sp)
bsr.w AOP
addq.w #4,%sp
movea.l %a0,%a2
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NIL_RESULTS","aw",@nobits
.balign 4
.Ldata:
.space 28
