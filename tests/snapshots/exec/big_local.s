.text
.globl FILL
FILL:
link.w %fp,#-8
movem.l %d2-%d4,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w #0x270f,%d1
move.l (%a1),%d0
clr.w %d2
.balignw 4,0x2048
2:
move.w %d2,%d3
move.w %d2,%d4
ext.l %d3
ext.l %d4
lsl.l #2,%d3
move.l %d4,(0,%a0,%d3.l)
addq.w #1,%d2
dbf %d1,2b
move.l %d0,(%a0)
movem.l (-0x14,%fp),%d2-%d4
unlk %fp
rts
POKE:
link.w %fp,#-4
movea.l (%fp),%a0
move.w (8,%fp),%d0
movea.l (-4,%a0),%a0
ext.l %d0
lsl.l #2,%d0
addi.l #0x3e8,(0,%a0,%d0.l)
unlk %fp
rts
.globl BIG_LOCAL_$RUN
BIG_LOCAL_$RUN:
link.w %fp,#-0xc
suba.l #0x9c40,%sp
movem.l %d2-%d3,-(%sp)
lea (8,%sp),%a0
move.l %a0,(-4,%fp)
move.w (8,%fp),%d2
pea (.Lc0,%pc)
move.l (-4,%fp),-(%sp)
bsr.s FILL
addq.w #8,%sp
movea.l (-4,%fp),%a0
move.l (%a0),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
movea.l (-4,%fp),%a0
move.l #0x9c3c,%d0
move.l (0,%a0,%d0.l),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,%d0
movea.l (-4,%fp),%a0
ext.l %d0
lsl.l #2,%d0
move.l (0,%a0,%d0.l),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
movea.l (-4,%fp),%a0
clr.l (%a0)
clr.l %d2
move.w #0x270f,%d0
clr.w %d1
.balignw 4,0x2048
2:
move.w %d1,%d3
movea.l (-4,%fp),%a0
ext.l %d3
addq.w #1,%d1
lsl.l #2,%d3
add.l (0,%a0,%d3.l),%d2
dbf %d0,2b
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x270e,-(%sp)
bsr.w POKE
addq.w #4,%sp
movea.l (-4,%fp),%a0
move.l #0x9c38,%d0
move.l (0,%a0,%d0.l),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l %fp,%a1
adda.l #-0x9c54,%a1
movem.l (%a1),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0007
.balignw 4,0x2048
