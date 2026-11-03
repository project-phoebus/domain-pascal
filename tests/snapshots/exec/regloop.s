.text
.globl REGLOOP_$MAIN
REGLOOP_$MAIN:
link.w %fp,#-8
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
moveq #1,%d3
clr.w %d4
move.w #3,(%a5)
move.w (%a5),(-8,%fp)
.balignw 4,0x2048
1:
move.l %d3,%d0
move.l %d0,%d1
add.l %d1,%d1
move.l %d2,-(%sp)
add.l %d1,%d0
move.l %d0,%d3
move.l %d3,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d3
addq.w #1,%d4
addq.w #1,(%a5)
jsr (EXEC_$CLOBBER).l
cmpi.w #4,%d4
blt.s 1b
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (%a5),(-8,%fp)
subq.l #2,%sp
move.w (-8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x18,%fp),%d2-%d4/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.REGLOOP","aw",@nobits
.balign 4
.Ldata:
.space 4
