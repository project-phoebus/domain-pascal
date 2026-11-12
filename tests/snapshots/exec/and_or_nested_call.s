.text
.globl PEEK
PEEK:
link.w %fp,#-4
move.w (8,%fp),%d0
unlk %fp
rts
.globl PX
PX:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
bne.s 1f
lea (%a5),%a0
bra.s 2f
1:
lea (2,%a5),%a0
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ONE
ONE:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
cmpi.w #5,%d2
bne.s 1f
subq.l #2,%sp
move.w %d3,-(%sp)
bsr.s PEEK
addq.w #4,%sp
cmpi.w #0x1f,%d0
sne %d0
tst.w %d3
sne %d1
or.b %d1,%d0
tst.b %d0
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
clr.w -(%sp)
5:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
cmpi.w #3,%d2
ble.s 3f
subq.l #2,%sp
move.w %d2,-(%sp)
move.l (4,%a5),-(%sp)
jsr (M$DIS$LLW).l
addq.w #8,%sp
btst #0,%d0
sne %d0
tst.b %d0
bpl.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 6f
3:
subq.l #2,%sp
clr.w -(%sp)
6:
jsr (EXEC_$LOGV_W).l
4:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl AND_OR_NESTED_CALL_$RUN
AND_OR_NESTED_CALL_$RUN:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l #0x186a5,(4,%a5)
move.w (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
move.w (0xc,%fp),-(%sp)
move.w (0xa,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
move.w (0xc,%fp),-(%sp)
move.w (8,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w PX
addq.w #4,%sp
movea.l %a0,%a1
subq.l #2,%sp
movea.l %a1,%a2
clr.w -(%sp)
bsr.w PX
addq.w #4,%sp
movea.l %a0,%a1
cmpa.l %a2,%a1
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 7f
1:
subq.l #2,%sp
clr.w -(%sp)
7:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w #1,-(%sp)
bsr.w PX
addq.w #4,%sp
movea.l %a0,%a1
subq.l #2,%sp
movea.l %a1,%a2
move.w (0xe,%fp),-(%sp)
bsr.w PX
addq.w #4,%sp
movea.l %a0,%a1
cmpa.l %a2,%a1
bne.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 8f
3:
subq.l #2,%sp
clr.w -(%sp)
8:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
4:
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w PX
addq.w #4,%sp
movea.l %a0,%a1
subq.l #2,%sp
movea.l %a1,%a2
move.w #1,-(%sp)
bsr.w PX
addq.w #4,%sp
movea.l %a0,%a1
cmpa.l %a2,%a1
beq.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
5:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$LOGV_W).l
6:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.AND_OR_NESTED_CALL","aw",@nobits
.balign 4
.Ldata:
.space 8
