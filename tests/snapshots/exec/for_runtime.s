.text
.globl FORRT_$UPTO
FORRT_$UPTO:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(0x10,%a5)
tst.w %d0
bmi.s 1f
move.w %d0,%d2
clr.w %d3
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
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORRT_$SUM
FORRT_$SUM:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(0x10,%a5)
clr.w (%a5)
move.w #-1,(8,%a5)
tst.w %d0
bmi.s 1f
move.w %d0,%d0
clr.w %d1
.balignw 4,0x2048
2:
move.w (%a5),%d2
add.w %d1,%d2
move.w %d2,(%a5)
move.w %d1,(8,%a5)
addq.w #1,%d1
dbf %d0,2b
1:
subq.l #2,%sp
move.w (8,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORRT_$BELOW
FORRT_$BELOW:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d2
clr.w %d3
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
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl FORRT_$INNER
FORRT_$INNER:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d1
subq.w #2,%d1
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
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl FORRT_$FAR
FORRT_$FAR:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d1
subi.w #0xd,%d1
bmi.s 1f
move.w %d1,%d2
clr.w %d3
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
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORRT","aw",@nobits
.balign 4
.Ldata:
.space 20
