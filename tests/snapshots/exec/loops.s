.text
.globl LOOPS_$UPTO
LOOPS_$UPTO:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w (8,%a5)
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
subq.l #2,%sp
move.w (8,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl LOOPS_$DOWN
LOOPS_$DOWN:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
moveq #3,%d2
moveq #2,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.w #1,%d3
dbf %d2,2b
subq.l #2,%sp
move.w #9,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl LOOPS_$CONST
LOOPS_$CONST:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
moveq #2,%d2
moveq #-2,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl LOOPS_$EXIT_NEXT
LOOPS_$EXIT_NEXT:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(8,%a5)
moveq #9,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
moveq #1,%d0
and.w %d3,%d0
beq.s 4f
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmp.w (8,%a5),%d3
bge.s 1f
bra.s 3f
4:
move.w %d3,(%a5)
3:
addq.w #1,%d3
dbf %d2,2b
1:
subq.l #2,%sp
move.w #0x64,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl LOOPS_$WHILE
LOOPS_$WHILE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(8,%a5)
clr.w (%a5)
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,(%a5)
subq.w #1,(8,%a5)
2:
tst.w (8,%a5)
bgt.s 1b
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.LOOPS","aw",@nobits
.balign 4
.Ldata:
.space 12
