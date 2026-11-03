.text
.globl ULONG_$MAIN
ULONG_$MAIN:
link.w %fp,#0
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l (0xc,%fp),%d0
move.l (0x10,%fp),%d3
move.w (0x14,%fp),%d4
move.l %d0,(%a5)
cmpi.l #0x3e8,%d2
bls.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
cmp.l (%a5),%d2
bls.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
cmp.l (%a5),%d2
bcc.s 3f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
move.l %d2,%d0
lsr.l #4,%d0
move.l %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x3e8,-(%sp)
move.l %d2,-(%sp)
jsr (M$DIU$LLW).l
addq.w #8,%sp
move.l %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x3e8,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIU$WLW).l
addq.w #8,%sp
clr.l %d1
move.w %d0,%d1
move.l %d1,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (%a5),-(%sp)
move.l %d2,-(%sp)
jsr (M$DIU$LLL).l
addq.w #8,%sp
move.l %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
move.l %d3,(8,%a5)
move.l (8,%a5),%d0
cmp.l (%a5),%d0
bge.s 4f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
move.w %d4,(0xc,%a5)
move.w (0xc,%a5),%d0
ext.l %d0
cmp.l (%a5),%d0
bge.s 5f
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
5:
movem.l (-0x10,%fp),%d2-%d4/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ULONG","aw",@nobits
.balign 4
.Ldata:
.space 16
