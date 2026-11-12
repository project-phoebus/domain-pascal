.text
.globl HELD_SET_VALUE_$RUN
HELD_SET_VALUE_$RUN:
link.w %fp,#-8
movem.l %d2-%d6,-(%sp)
move.l (8,%fp),%d2
move.l (0xc,%fp),%d3
move.l (0x10,%fp),%d4
move.l (0x14,%fp),%d5
clr.l %d6
cmpi.l #0x1f,%d2
sle %d0
move.l %d6,%d1
btst %d2,%d1
sne %d1
and.b %d1,%d0
bpl.s 1f
move.w #1,(-6,%fp)
bra.s 2f
1:
clr.w (-6,%fp)
2:
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
clr.l %d0
bset %d2,%d0
or.l %d6,%d0
move.l %d0,%d6
cmpi.l #0x1f,%d3
sle %d0
move.l %d6,%d1
btst %d3,%d1
sne %d1
and.b %d1,%d0
bpl.s 3f
move.w #1,(-6,%fp)
bra.s 4f
3:
clr.w (-6,%fp)
4:
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
cmpi.l #0x1f,%d4
sle %d0
move.l %d6,%d1
btst %d4,%d1
sne %d1
and.b %d1,%d0
bpl.s 5f
move.w #1,(-6,%fp)
bra.s 6f
5:
clr.w (-6,%fp)
6:
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
cmpi.l #0x1f,%d5
sle %d0
move.l %d6,%d1
btst %d5,%d1
sne %d1
and.b %d1,%d0
bpl.s 7f
move.w #1,(-6,%fp)
bra.s 8f
7:
clr.w (-6,%fp)
8:
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
clr.l %d0
bset %d5,%d0
or.l %d6,%d0
move.l %d0,%d6
cmpi.l #0x1f,%d5
sle %d0
move.l %d6,%d1
btst %d5,%d1
sne %d1
and.b %d1,%d0
bpl.s 9f
move.w #1,(-6,%fp)
bra.s 10f
9:
clr.w (-6,%fp)
10:
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x1c,%fp),%d2-%d6
unlk %fp
rts
.balignw 4,0x2048
