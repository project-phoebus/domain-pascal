.text
.globl FORUI_$A
FORUI_$A:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.l (%a5)
tst.w %d2
beq.s 1f
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,(4,%a5)
clr.l (%a5)
tst.w (4,%a5)
beq.s 4f
move.w (4,%a5),%d0
subq.w #1,%d0
move.w %d0,%d3
.balignw 4,0x2048
5:
jsr (EXEC_$CLOBBER).l
dbf %d3,5b
4:
tst.w (4,%a5)
beq.s 7f
move.w (4,%a5),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
8:
addq.l #1,(%a5)
dbf %d0,8b
7:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d0
.balignw 4,0x2048
11:
addq.l #1,(%a5)
dbf %d0,11b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
clr.l %d0
move.w %d2,%d0
subq.l #1,%d0
subq.l #1,%d0
blt.s 13f
.balignw 4,0x2048
14:
addq.l #1,(%a5)
dbf %d0,14b
clr.w %d0
subq.l #1,%d0
bcc.s 14b
13:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
clr.l %d0
move.w %d2,%d0
subq.l #1,%d0
tst.l %d0
bmi.s 16f
.balignw 4,0x2048
17:
addq.l #1,(%a5)
dbf %d0,17b
clr.w %d0
subq.l #1,%d0
bcc.s 17b
16:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d0
subq.w #1,%d0
bcs.s 19f
.balignw 4,0x2048
20:
addq.l #1,(%a5)
dbf %d0,20b
19:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d0
.balignw 4,0x2048
23:
addq.l #1,(%a5)
dbf %d0,23b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
tst.w %d2
beq.s 25f
move.w %d2,%d0
subq.w #1,%d0
moveq #1,%d1
move.w %d0,%d0
.balignw 4,0x2048
26:
clr.l %d3
move.w %d1,%d3
add.l (%a5),%d3
move.l %d3,(%a5)
addq.w #1,%d1
dbf %d0,26b
25:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d0
clr.l %d1
lsr.w #1,%d0
move.w %d0,%d1
subq.l #1,%d1
bcs.s 28f
.balignw 4,0x2048
29:
addq.l #1,(%a5)
dbf %d1,29b
clr.w %d1
subq.l #1,%d1
bcc.s 29b
28:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
cmpi.w #2,%d2
bcs.s 31f
move.w %d2,%d0
subq.w #2,%d0
move.w %d0,%d0
.balignw 4,0x2048
32:
addq.l #1,(%a5)
dbf %d0,32b
31:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
FORUI_TOUCH:
link.w %fp,#-4
movea.l (8,%fp),%a0
clr.l %d0
move.w (%a0),%d0
movea.l (8,%fp),%a0
add.l (%a5),%d0
clr.l %d1
move.w (%a0),%d1
sub.l %d1,%d0
move.l %d0,(%a5)
unlk %fp
rts
FORUI_F:
link.w %fp,#-8
move.w (8,%fp),%d0
unlk %fp
rts
FORUI_VAR:
link.w %fp,#-8
clr.l (%a5)
movea.l (8,%fp),%a0
tst.w (%a0)
beq.s 1f
move.w (%a0),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
.globl FORUI_$B
FORUI_$B:
link.w %fp,#-0xc
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
lea (%a2),%a3
clr.l (%a5)
tst.w (%a3)
beq.s 1f
move.w (%a3),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
tst.w (%a3)
beq.s 4f
move.w (%a3),%d0
subq.w #1,%d0
move.w %d0,%d2
.balignw 4,0x2048
5:
jsr (EXEC_$CLOBBER).l
addq.l #1,(%a5)
dbf %d2,5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
tst.w (%a3)
beq.s 7f
move.w (%a3),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
8:
addq.l #1,(%a5)
dbf %d0,8b
7:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w (%a2),(-0xa,%fp)
pea (-0xa,%fp)
bsr.w FORUI_VAR
movem.l (-0x1c,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
FORUI_UP:
link.w %fp,#-8
movea.l (%fp),%a0
clr.l (%a5)
tst.w (-4,%a0)
beq.s 1f
move.w (-4,%a0),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
FORUI_IDX:
link.w %fp,#-4
movea.l (%fp),%a0
tst.w (-6,%a0)
beq.s 1f
addq.l #1,(%a5)
1:
unlk %fp
rts
.globl FORUI_$C
FORUI_$C:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,(-4,%fp)
pea (-4,%fp)
bsr.w FORUI_TOUCH
addq.w #4,%sp
clr.l (%a5)
tst.w (-4,%fp)
beq.s 1f
move.w (-4,%fp),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,(0xa,%a5)
clr.l (%a5)
tst.w (0xa,%a5)
beq.s 4f
move.w (0xa,%a5),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w FORUI_F
addq.w #4,%sp
tst.w %d0
beq.s 7f
move.w %d0,%d1
subq.w #1,%d1
move.w %d1,%d0
.balignw 4,0x2048
8:
addq.l #1,(%a5)
dbf %d0,8b
7:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
bsr.w FORUI_UP
clr.l (%a5)
move.w %d2,%d3
subq.w #1,%d3
bcs.s 10f
move.w #1,(-6,%fp)
.balignw 4,0x2048
11:
bsr.w FORUI_IDX
addq.w #1,(-6,%fp)
dbf %d3,11b
10:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d0
subq.w #1,%d0
bcs.s 13f
move.w #1,(6,%a5)
.balignw 4,0x2048
14:
addq.l #1,(%a5)
addq.w #1,(6,%a5)
dbf %d0,14b
13:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d3
clr.w (6,%a5)
.balignw 4,0x2048
17:
jsr (EXEC_$CLOBBER).l
addq.w #1,(6,%a5)
dbf %d3,17b
move.w %d2,%d0
clr.w (6,%a5)
.balignw 4,0x2048
20:
addq.l #1,(%a5)
addq.w #1,(6,%a5)
dbf %d0,20b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORUI_$RT
FORUI_$RT:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
clr.l (%a5)
move.w %d3,%d0
sub.w %d2,%d0
bcs.s 1f
move.w %d0,%d1
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d1,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d3,%d0
sub.w %d2,%d0
bcs.s 4f
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORUI_$K
FORUI_$K:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.l (%a5)
move.w #-0x63c0,%d0
sub.w %d2,%d0
bcs.s 1f
move.w %d0,%d1
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d1,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w #-0x63c0,%d0
sub.w %d2,%d0
bcs.s 4f
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORUI_$I
FORUI_$I:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.l (%a5)
tst.w %d2
beq.s 1f
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
tst.w %d2
beq.s 4f
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,%d3
.balignw 4,0x2048
5:
jsr (EXEC_$CLOBBER).l
dbf %d3,5b
4:
move.w %d2,%d0
.balignw 4,0x2048
8:
addq.l #1,(%a5)
dbf %d0,8b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORUI_$ARR
FORUI_$ARR:
link.w %fp,#-0x1c
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #7,%d1
lea (-0x18,%fp),%a0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d1,1b
tst.w %d0
beq.s 2f
move.w %d0,%d1
subq.w #1,%d1
moveq #1,%d2
move.w %d1,%d1
lea (2,%fp),%a0
.balignw 4,0x2048
3:
move.w %d2,%d3
move.w %d3,%d4
add.w %d4,%d4
add.w %d4,%d3
move.w %d3,(-0x1a,%a0)
addq.w #1,%d2
addq.l #2,%a0
dbf %d1,3b
2:
lea (-0x18,%fp),%a0
clr.l (%a5)
tst.w %d0
beq.s 5f
move.w %d0,%d1
subq.w #1,%d1
lea (2,%a0),%a1
move.w %d1,%d0
.balignw 4,0x2048
6:
move.w (-2,%a1),%d1
ext.l %d1
add.l (%a5),%d1
move.l %d1,(%a5)
addq.l #2,%a1
dbf %d0,6b
5:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
moveq #7,%d0
lea (2,%fp),%a0
.balignw 4,0x2048
9:
move.w (-0x1a,%a0),%d1
ext.l %d1
add.l (%a5),%d1
move.l %d1,(%a5)
addq.l #2,%a0
dbf %d0,9b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x2c,%fp),%d2-%d4/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORUI","aw",@nobits
.balign 4
.Ldata:
.space 12
