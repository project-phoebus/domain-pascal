.text
.globl PARK_INT_EDGE_$LOOP
PARK_INT_EDGE_$LOOP:
link.w %fp,#-8
movem.l %d2-%d3/%a2,-(%sp)
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
moveq #0xa,%d1
movea.l %d0,%a2
move.l %d1,(4,%a2)
moveq #2,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
jsr (EXEC_$CLOBBER).l
move.w %d3,%d0
ext.l %d0
add.l (4,%a2),%d0
move.l %d0,(4,%a2)
addq.w #1,%d3
dbf %d2,2b
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PARK_INT_EDGE_$TWO
PARK_INT_EDGE_$TWO:
link.w %fp,#-0xc
movem.l %d2/%a2-%a3,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
pea (%a2)
movea.l %d0,%a3
move.l %d2,%d0
addq.l #1,%d0
move.l %d0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
movea.l %d0,%a0
move.l (4,%a0),%d1
add.l (4,%a3),%d1
move.l %d1,(4,%a3)
move.l %d1,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x18,%fp),%d2/%a2-%a3
unlk %fp
rts
.globl PARK_INT_EDGE_$WHOLE
PARK_INT_EDGE_$WHOLE:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
tst.l (-4,%fp)
beq.s 1f
move.l (4,%a0),-(%sp)
jsr (EXEC_$LOGV_L).l
1:
unlk %fp
rts
SHOW:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
movea.l (-4,%a2),%a0
move.l (%a0),%d0
subq.l #1,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PARK_INT_EDGE_$NESTED
PARK_INT_EDGE_$NESTED:
link.w %fp,#-4
pea (%a2)
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
tst.l (%a2)
bne.s 1f
bsr.s SHOW
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PARK_INT_EDGE_$VIAD0
PARK_INT_EDGE_$VIAD0:
link.w %fp,#-8
pea (%a2)
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
jsr (EXEC_$CLOBBER).l
pea (-4,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
movea.l %d0,%a2
jsr (EXEC_$CLOBBER).l
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl PARK_INT_EDGE_$A0LOOP
PARK_INT_EDGE_$A0LOOP:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
clr.l (%a5)
moveq #3,%d0
.balignw 4,0x2048
2:
move.l (%a5),%d1
add.l (4,%a0),%d1
move.l %d1,(%a5)
movea.l (0xc,%a5),%a1
move.l %d1,(%a1)
dbf %d0,2b
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PARK_INT_EDGE_$SHOW
PARK_INT_EDGE_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PARK_INT_EDGE","aw",@progbits
.balign 4
.Ldata:
.space 12
.long .Ldata+4
