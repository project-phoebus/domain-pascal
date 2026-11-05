.text
.globl FORTL_$ROW
FORTL_$ROW:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
moveq #3,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d2,%d3
ext.l %d3
add.l %d3,%d3
move.l %d3,%d4
lsl.l #2,%d4
add.l %d4,%d3
move.w %d1,%d4
ext.l %d4
add.l %d4,%d3
move.w %d2,%d4
lsl.w #4,%d4
lea (-0x10,%a5,%d4.w),%a0
move.w %d1,%d4
lsl.w #2,%d4
move.l %d3,(-4,%a0,%d4.w)
addq.w #1,%d1
dbf %d0,2b
moveq #3,%d3
moveq #1,%d4
.balignw 4,0x2048
5:
move.w %d2,%d0
lsl.w #4,%d0
lea (-0x10,%a5,%d0.w),%a0
move.w %d4,%d0
lsl.w #2,%d0
move.l (-4,%a0,%d0.w),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
addq.w #1,%d4
dbf %d3,5b
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl FORTL_$FROW
FORTL_$FROW:
link.w %fp,#-0x34
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d1
moveq #2,%d0
lea (0x10,%fp),%a0
.balignw 4,0x2048
2:
moveq #3,%d2
lea (-0x40,%a0),%a1
.balignw 4,0x2048
4:
clr.l (%a1)+
dbf %d2,4b
lea (0x10,%a0),%a0
dbf %d0,2b
moveq #3,%d2
moveq #1,%d3
.balignw 4,0x2048
6:
move.w %d3,%d0
subq.l #2,%sp
ext.l %d0
move.w %d1,-(%sp)
subq.l #1,%d0
move.l %d0,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
move.w %d1,%d4
lsl.w #4,%d4
lea (-0x40,%fp,%d4.w),%a0
move.w %d3,%d4
lsl.w #2,%d4
move.l %d0,(-4,%a0,%d4.w)
addq.w #1,%d3
dbf %d2,6b
pea (.Lc0,%pc)
pea (-0x30,%fp)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x40,%fp),%d2-%d4
unlk %fp
rts
.Lc0:
.short 0x0030
FILL:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
movea.l (%fp),%a0
moveq #4,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d1,%d2
move.w %d1,%d3
muls.w (8,%a0),%d2
lsl.w #2,%d3
move.l %d2,(-0x1c,%a0,%d3.w)
addq.w #1,%d1
dbf %d0,2b
moveq #9,%d0
clr.w %d1
.balignw 4,0x2048
5:
move.w %d1,%d2
movea.l (-0x1c,%a0),%a1
add.w %d2,%d2
move.w %d1,%d3
add.w (8,%a0),%d3
move.w %d3,(0,%a1,%d2.w)
addq.w #1,%d1
dbf %d0,5b
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
SHOW:
link.w %fp,#-8
movem.l %d2-%d3/%a2,-(%sp)
movea.l (%fp),%a2
moveq #4,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,%d0
lsl.w #2,%d0
move.l (-0x1c,%a2,%d0.w),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
movem.l (-0x14,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl FORTL_$ENCL
FORTL_$ENCL:
link.w %fp,#-0x30
lea (-0x30,%fp),%a0
move.l %a0,(-0x1c,%fp)
bsr.w FILL
bsr.s SHOW
pea (.Lc1,%pc)
pea (-0x30,%fp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc1:
.short 0x0014
.globl FORTL_$DOWN
FORTL_$DOWN:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d2
move.w %d2,%d3
subq.w #1,%d3
blt.s 1f
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.w #1,%d2
dbf %d3,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl FORTL_$DOWNTO
FORTL_$DOWNTO:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
moveq #5,%d2
sub.w %d0,%d2
blt.s 1f
moveq #5,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.w #1,%d3
dbf %d2,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl FORTL_$DOWNBIG
FORTL_$DOWNBIG:
link.w %fp,#-0xc
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.l %d2
clr.l %d3
move.w %d0,%d4
move.w %d4,%d0
sub.w %d1,%d0
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,%d2
move.w %d4,%d1
subq.w #1,%d4
ext.l %d1
add.l %d1,%d3
dbf %d0,2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x18,%fp),%d2-%d4
unlk %fp
rts
.globl FORTL_$PTR
FORTL_$PTR:
link.w %fp,#-0xc
movem.l %d2-%d4/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,(0x32,%a5)
lea (0x30,%a5),%a2
move.w (2,%a2),%d0
tst.w %d0
bmi.s 1f
move.w %d0,%d3
clr.w %d4
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,%d4
dbf %d3,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
moveq #9,%d0
lea (0x34,%a5),%a0
.balignw 4,0x2048
4:
clr.w (%a0)+
dbf %d0,4b
lea (0x34,%a5),%a0
move.w (2,%a2),%d0
tst.w %d0
bmi.s 5f
move.w %d0,%d0
clr.w %d1
lea (%a0),%a1
.balignw 4,0x2048
6:
move.w %d1,%d3
add.w %d3,%d3
move.w %d3,(0,%a1)
addq.w #1,%d1
addq.l #2,%a1
dbf %d0,6b
5:
cmpi.w #3,%d2
bne.s 8f
pea (.Lc1,%pc)
pea (0x34,%a5)
jsr (EXEC_$LOG_MEM).l
8:
movem.l (-0x20,%fp),%d2-%d4/%a2/%a5
unlk %fp
rts
.globl FORTL_$PTRBIG
FORTL_$PTRBIG:
link.w %fp,#-0x10
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(0x32,%a5)
lea (0x30,%a5),%a0
clr.l %d2
clr.l %d3
move.w (2,%a0),%d0
tst.w %d0
bmi.s 1f
move.w %d0,%d0
clr.w %d1
.balignw 4,0x2048
2:
addq.l #1,%d2
move.w %d1,%d4
addq.w #1,%d1
ext.l %d4
add.l %d4,%d3
dbf %d0,2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x20,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl FORTL_$FROM13
FORTL_$FROM13:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d1
subi.w #0xd,%d1
blt.s 1f
move.w %d1,%d2
moveq #0xd,%d3
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
.globl FORTL_$FROM2
FORTL_$FROM2:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #9,%d1
clr.w %d2
lea (%a5),%a0
.balignw 4,0x2048
2:
move.w %d2,(0x34,%a0)
addq.w #1,%d2
addq.l #2,%a0
dbf %d1,2b
move.w %d0,%d1
subq.w #3,%d1
blt.s 4f
move.w %d1,%d0
lea (4,%a5),%a0
.balignw 4,0x2048
5:
move.w (0x36,%a0),%d1
add.w %d1,%d1
move.w %d1,%d2
lsl.w #2,%d2
add.w %d2,%d1
move.w %d1,(0x34,%a0)
addq.l #2,%a0
dbf %d0,5b
4:
pea (.Lc1,%pc)
pea (0x34,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORTL_$FROMNEG
FORTL_$FROMNEG:
link.w %fp,#-0xc
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d0
clr.l %d2
clr.l %d3
move.w %d0,%d1
addq.w #1,%d1
blt.s 1f
move.w %d1,%d0
moveq #-1,%d1
.balignw 4,0x2048
2:
addq.l #1,%d2
move.w %d1,%d4
addq.w #1,%d1
ext.l %d4
add.l %d4,%d3
dbf %d0,2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x18,%fp),%d2-%d4
unlk %fp
rts
.globl FORTL_$FROM100
FORTL_$FROM100:
link.w %fp,#-0xc
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d0
clr.l %d2
clr.l %d3
move.w %d0,%d1
subi.w #-0x64,%d1
blt.s 1f
move.w %d1,%d0
moveq #-0x64,%d1
.balignw 4,0x2048
2:
addq.l #1,%d2
move.w %d1,%d4
addq.w #1,%d1
ext.l %d4
add.l %d4,%d3
dbf %d0,2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x18,%fp),%d2-%d4
unlk %fp
rts
.globl FORTL_$FROMFOLD
FORTL_$FROMFOLD:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
clr.l %d2
tst.w %d0
bmi.s 1f
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,%d2
dbf %d0,2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl FORTL_$WIDE
FORTL_$WIDE:
link.w %fp,#-8
move.l %d2,-(%sp)
move.l (8,%fp),%d0
clr.l %d2
move.l %d0,%d1
subq.l #1,%d1
tst.l %d1
bmi.s 1f
.balignw 4,0x2048
2:
addq.l #1,%d2
dbf %d1,2b
clr.w %d1
subq.l #1,%d1
bcc.s 2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d2
pea (8,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d1
subq.l #1,%d1
blt.s 4f
.balignw 4,0x2048
5:
addq.l #1,%d2
dbf %d1,5b
clr.w %d1
subq.l #1,%d1
bcc.s 5b
4:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl FORTL_$REHOME
FORTL_$REHOME:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
moveq #2,%d3
move.w #1,(-2,%fp)
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(-2,%fp)
dbf %d3,2b
move.w %d2,(-2,%fp)
moveq #1,%d2
.balignw 4,0x2048
5:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(-2,%fp)
dbf %d2,5b
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
FORTL_BUMP:
link.w %fp,#-4
movea.l (8,%fp),%a0
addi.w #0x64,(%a0)
unlk %fp
rts
.globl FORTL_$ADDR
FORTL_$ADDR:
link.w %fp,#-4
move.l %d2,-(%sp)
moveq #1,%d2
move.w #1,(-2,%fp)
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(-2,%fp)
dbf %d2,2b
pea (-2,%fp)
bsr.s FORTL_BUMP
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #5,(-2,%fp)
move.w (-2,%fp),%d0
moveq #6,%d2
sub.w %d0,%d2
blt.s 4f
move.w %d0,(-2,%fp)
.balignw 4,0x2048
5:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(-2,%fp)
dbf %d2,5b
4:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl FORTL_$FIND
FORTL_$FIND:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
moveq #9,%d0
clr.w (-2,%fp)
.balignw 4,0x2048
2:
move.w (-2,%fp),%d1
move.w %d1,%d3
add.w %d3,%d3
add.w %d3,%d1
move.w (-2,%fp),%d3
add.w %d3,%d3
move.w %d1,(0x34,%a5,%d3.w)
addq.w #1,(-2,%fp)
dbf %d0,2b
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
beq.s 4f
moveq #8,%d0
move.w #1,(-2,%fp)
.balignw 4,0x2048
6:
move.w (-2,%fp),%d1
add.w %d1,%d1
cmp.w (0x34,%a5,%d1.w),%d2
beq.s 5f
addq.w #1,(-2,%fp)
dbf %d0,6b
5:
4:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORTL_$AFTER
FORTL_$AFTER:
link.w %fp,#-4
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
moveq #9,%d2
clr.w %d3
lea (%a5),%a0
.balignw 4,0x2048
2:
move.w %d3,%d4
move.w %d4,%d5
add.w %d5,%d5
add.w %d5,%d4
move.w %d4,(0x34,%a0)
addq.w #1,%d3
addq.l #2,%a0
dbf %d2,2b
move.w %d0,(0x48,%a5)
clr.w %d2
move.w (0x48,%a5),%d0
tst.w %d0
bmi.s 4f
move.w %d0,%d0
lea (%a5),%a0
.balignw 4,0x2048
5:
cmp.w (0x34,%a0),%d1
bne.s 7f
subq.l #2,%sp
moveq #0x64,%d0
add.w %d2,%d0
move.w %d0,-(%sp)
bra.s 11f
7:
addq.w #1,%d2
addq.l #2,%a0
dbf %d0,5b
4:
cmpi.w #4,%d2
bge.s 9f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 12f
9:
subq.l #2,%sp
move.w #2,-(%sp)
12:
jsr (EXEC_$MARK).l
addq.w #4,%sp
10:
subq.l #2,%sp
move.w %d2,-(%sp)
11:
jsr (EXEC_$MARK).l
8:
movem.l (-0x18,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl FORTL_$AFTERSKIP
FORTL_$AFTERSKIP:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #1,%d1
move.w %d0,%d2
subq.w #1,%d2
bmi.s 1f
move.w %d2,%d0
lea (2,%a5),%a0
.balignw 4,0x2048
2:
clr.w (0x34,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
1:
move.w %d1,%d0
move.w %d0,(0x34,%a5)
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORTL_$AFTERL
FORTL_$AFTERL:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),%d0
move.l #0x1116e,%d2
move.l %d0,%d1
subi.l #0x1116e,%d1
blt.s 1f
.balignw 4,0x2048
2:
cmpi.l #0x11171,%d2
beq.s 1f
addq.l #1,%d2
dbf %d1,2b
clr.w %d1
subq.l #1,%d1
bcc.s 2b
1:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl FORTL_$UNDEF
FORTL_$UNDEF:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d2
st %d0
.balignw 4,0x2048
1:
tst.b %d0
bmi.s 2f
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,(-2,%fp)
clr.b %d0
subq.w #1,%d2
bpl.s 1b
move.l (-8,%fp),%d2
unlk %fp
rts
.globl FORTL_$BITS
FORTL_$BITS:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
moveq #0,%d3
tst.w %d2
ble.s 1f
move.w %d3,%d0
ori.w #1,%d0
move.w %d0,%d3
1:
cmpi.w #1,%d2
ble.s 2f
move.w %d3,%d0
ori.w #8,%d0
move.w %d0,%d3
2:
btst #0,%d3
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
btst #3,%d3
beq.s 4f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
4:
movea.l (0xa,%fp),%a0
move.w %d2,(%a0)
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl FORTL_$DOWNARR
FORTL_$DOWNARR:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #9,%d1
lea (0x34,%a5),%a0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d1,1b
move.w %d0,%d1
move.w %d1,%d0
subq.w #1,%d0
blt.s 2f
.balignw 4,0x2048
3:
move.w %d1,%d2
add.w %d2,%d2
move.w #7,(0x34,%a5,%d2.w)
subq.w #1,%d1
dbf %d0,3b
2:
pea (.Lc1,%pc)
pea (0x34,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORTL_$UPARR
FORTL_$UPARR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #9,%d1
lea (0x34,%a5),%a0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d1,1b
tst.w %d0
beq.s 2f
move.w %d0,%d1
subq.w #1,%d1
lea (2,%a5),%a0
move.w %d1,%d0
.balignw 4,0x2048
3:
move.w #7,(0x34,%a0)
addq.l #2,%a0
dbf %d0,3b
2:
pea (.Lc1,%pc)
pea (0x34,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORTL","aw",@nobits
.balign 4
.Ldata:
.space 76
