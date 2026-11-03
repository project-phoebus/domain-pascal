.text
.globl REGHELD_$ACROSS_CALLS
REGHELD_$ACROSS_CALLS:
link.w %fp,#-0xc
movem.l %d2-%d5,-(%sp)
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
move.w %d0,%d2
move.l %d1,%d3
addq.w #1,%d2
subq.l #1,%d3
jsr (EXEC_$CLOBBER).l
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
moveq #2,%d4
moveq #1,%d5
.balignw 4,0x2048
2:
add.w %d5,%d2
jsr (EXEC_$CLOBBER).l
move.w %d2,%d0
addq.w #1,%d5
ext.l %d0
add.l %d0,%d3
dbf %d4,2b
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x1c,%fp),%d2-%d5
unlk %fp
rts
.globl REGHELD_$BRANCHES
REGHELD_$BRANCHES:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
clr.w %d3
move.w %d0,(%a5)
cmp.w (%a5),%d1
ble.s 1f
move.w %d1,%d3
sub.w %d0,%d3
jsr (EXEC_$CLOBBER).l
bra.s 2f
1:
move.w %d0,%d2
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
bra.s 4f
.balignw 4,0x2048
3:
addq.w #1,%d3
subq.w #7,%d2
4:
tst.w %d2
bgt.s 3b
move.w %d3,%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
add.w %d2,%d0
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl REGHELD_$MARITH
REGHELD_$MARITH:
link.w %fp,#-0xc
movem.l %d2-%d3,-(%sp)
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d2
subq.l #2,%sp
move.w #7,-(%sp)
move.l %d2,-(%sp)
jsr (M$DIS$LLW).l
addq.w #8,%sp
move.l %d0,%d3
move.l #0x11170,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$LLL).l
addq.w #8,%sp
move.l %d0,(-0xc,%fp)
jsr (EXEC_$CLOBBER).l
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (-0xc,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,%d0
move.l %d0,%d1
neg.l %d0
lsl.l #3,%d1
add.l %d1,%d0
move.l %d0,%d3
move.l %d2,%d0
sub.l %d3,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.REGHELD","aw",@nobits
.balign 4
.Ldata:
.space 4
