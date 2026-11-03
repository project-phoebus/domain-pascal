.text
.globl ULONG_OPS_$SET
ULONG_OPS_$SET:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l (0x10,%fp),%d2
move.w (0x14,%fp),%d3
move.l %d0,(%a5)
move.l %d1,(4,%a5)
move.l %d2,(8,%a5)
move.w %d3,(0xc,%a5)
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BIG
BIG:
link.w %fp,#-4
move.l (8,%fp),%d0
unlk %fp
rts
INNER:
link.w %fp,#-4
movea.l (%fp),%a0
cmpi.l #0x3e8,(-4,%a0)
bls.s 1f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
1:
unlk %fp
rts
.globl NEST
NEST:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (4,%a5),(-4,%fp)
bsr.s INNER
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ULONG_OPS_$RUN
ULONG_OPS_$RUN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #1,(%a5)
cmpi.l #0x3e8,(%a5)
bls.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
move.l (%a5),(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
addq.l #1,(%a5)
move.l (%a5),%d0
cmp.l (4,%a5),%d0
bcc.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.l (4,%a5),%d0
lsr.l #1,%d0
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%a5),-(%sp)
move.l (4,%a5),-(%sp)
jsr (M$DIU$LLW).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l (%a5),(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
clr.l %d0
move.w (0xc,%a5),%d0
move.l %d0,(%a5)
addq.l #2,(%a5)
move.l (%a5),-(%sp)
move.l (%sp),-(%sp)
jsr (M$MIU$LLL).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l (%a5),(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
move.l (4,%a5),(%a5)
move.l (8,%a5),-(%sp)
move.l (%a5),-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,(%a5)
move.l (%a5),(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
move.l (8,%a5),-(%sp)
move.l (4,%a5),-(%sp)
jsr (M$DIS$LLL).l
addq.w #8,%sp
move.l %d0,(-4,%fp)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a5),%d0
cmpi.l #1,%d0
beq.s 4f
cmpi.l #0x64,%d0
beq.s 5f
bra.s 6f
4:
move.w #7,(0xe,%a5)
bra.s 3f
5:
move.w #8,(0xe,%a5)
bra.s 3f
6:
move.w #9,(0xe,%a5)
3:
subq.l #2,%sp
move.w (0xe,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.l (4,%a5),-(%sp)
bsr.w BIG
addq.w #4,%sp
cmpi.l #0x3e8,%d0
bls.s 7f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
7:
bsr.w NEST
move.l (4,%a5),%d0
cmp.l (8,%a5),%d0
ble.s 8f
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
8:
clr.l (%a5)
bra.s 10f
.balignw 4,0x2048
9:
addq.l #1,(%a5)
10:
cmpi.l #3,(%a5)
bcs.s 9b
move.l (%a5),(-4,%fp)
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (%a5),%d0
add.l %d0,%d0
move.w #7,(0x10,%a5,%d0.l)
move.w (0x16,%a5),(0xe,%a5)
subq.l #2,%sp
move.w (0xe,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ULONG_OPS","aw",@nobits
.balign 4
.Ldata:
.space 24
