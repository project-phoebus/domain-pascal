.text
.globl FORHM_$SHOW
FORHM_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FORHM_$SHOWL
FORHM_$SHOWL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FORHM_$SHOWC
FORHM_$SHOWC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.b (8,%a5),-(%sp)
jsr (EXEC_$LOGV_C).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FORHM_$GLOBAL
FORHM_$GLOBAL:
link.w %fp,#0
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d2,%d0
move.w %d3,%d4
sub.w %d0,%d4
blt.s 1f
move.w %d0,(%a5)
.balignw 4,0x2048
2:
bsr.s FORHM_$SHOW
addq.w #1,(%a5)
dbf %d4,2b
1:
move.w %d3,%d0
move.w %d0,%d4
sub.w %d2,%d4
blt.s 4f
move.w %d0,(%a5)
.balignw 4,0x2048
5:
bsr.w FORHM_$SHOW
subq.w #1,(%a5)
dbf %d4,5b
4:
bsr.w FORHM_$SHOW
cmp.w %d3,%d2
bgt.s 7f
subq.l #2,%sp
move.w (%a5),-(%sp)
bra.s 9f
7:
subq.l #2,%sp
move.w #0x63,-(%sp)
9:
jsr (EXEC_$MARK).l
8:
movem.l (-0x10,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl FORHM_$LONG
FORHM_$LONG:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l %d0,%d2
move.l %d1,%d3
sub.l %d2,%d3
blt.s 1f
move.l %d2,(4,%a5)
.balignw 4,0x2048
2:
bsr.w FORHM_$SHOWL
addq.l #1,(4,%a5)
dbf %d3,2b
clr.w %d3
subq.l #1,%d3
bcc.s 2b
1:
bsr.w FORHM_$SHOWL
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORHM_$CHARS
FORHM_$CHARS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l (0xc,%a5)
move.w #0xff,%d0
clr.b (8,%a5)
.balignw 4,0x2048
2:
addq.l #1,(0xc,%a5)
addq.b #1,(8,%a5)
dbf %d0,2b
cmpi.l #0x100,(0xc,%a5)
bne.s 4f
pea (3).w
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
4:
moveq #2,%d2
move.b #0x78,(8,%a5)
.balignw 4,0x2048
6:
bsr.w FORHM_$SHOWC
addq.b #1,(8,%a5)
dbf %d2,6b
bsr.w FORHM_$SHOWC
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
INNER:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
movea.l (%fp),%a2
move.w (8,%a2),%d2
subq.w #1,%d2
blt.s 1f
move.w #1,(-2,%a2)
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (-2,%a2),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(-2,%a2)
dbf %d2,2b
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl FORHM_$NESTED
FORHM_$NESTED:
link.w %fp,#-4
move.w #5,(-2,%fp)
bsr.s INNER
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
SHOW:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
subq.l #2,%sp
move.w (-2,%a2),%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl FORHM_$PARENT
FORHM_$PARENT:
link.w %fp,#-4
move.l %d2,-(%sp)
moveq #2,%d2
move.w #1,(-2,%fp)
.balignw 4,0x2048
2:
bsr.s SHOW
addq.w #1,(-2,%fp)
dbf %d2,2b
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl FORHM_$VARPARAM
FORHM_$VARPARAM:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
moveq #2,%d2
move.w #5,(%a2)
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.w #1,(%a2)
dbf %d2,2b
moveq #9,%d0
move.w #1,(%a2)
.balignw 4,0x2048
5:
cmpi.w #2,(%a2)
beq.s 4f
addq.w #1,(%a2)
dbf %d0,5b
4:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl FORHM_$SKIP
FORHM_$SKIP:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d2
move.w #1,(%a5)
.balignw 4,0x2048
2:
cmpi.w #3,(%a5)
beq.s 3f
bsr.w FORHM_$SHOW
3:
addq.w #1,(%a5)
dbf %d2,2b
moveq #5,%d2
move.w #6,(%a5)
.balignw 4,0x2048
5:
cmpi.w #5,(%a5)
beq.s 4f
bsr.w FORHM_$SHOW
subq.w #1,(%a5)
dbf %d2,5b
4:
moveq #1,%d2
move.w #1,(%a5)
.balignw 4,0x2048
8:
bsr.w FORHM_$SHOW
addq.w #1,(%a5)
dbf %d2,8b
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl FORHM_$TOHOME
FORHM_$TOHOME:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w #0xa,(%a5)
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
move.w (%a5),%d2
sub.w %d1,%d2
blt.s 1f
move.w %d1,(%a5)
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(%a5)
dbf %d2,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl FORHM_$DOWNHOME
FORHM_$DOWNHOME:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
move.w (%a5),%d2
pea (.Lc0,%pc)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d2,%d3
sub.w %d0,%d3
blt.s 1f
move.w %d2,(%a5)
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.w #1,(%a5)
dbf %d3,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc0:
.short 0x0005
.globl FORHM_$GRID
FORHM_$GRID:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d2
move.w #1,(%a5)
.balignw 4,0x2048
2:
moveq #2,%d3
move.w #1,(0x10,%a5)
.balignw 4,0x2048
5:
subq.l #2,%sp
move.w (%a5),%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
add.w (0x10,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(0x10,%a5)
dbf %d3,5b
addq.w #1,(%a5)
dbf %d2,2b
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x10,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
GRID:
link.w %fp,#-4
movem.l %d2-%d3/%a2,-(%sp)
movea.l (%fp),%a2
move.w (8,%a2),%d2
subq.w #1,%d2
blt.s 1f
move.w #1,(-2,%a2)
.balignw 4,0x2048
2:
move.w (0xa,%a2),%d3
subq.w #1,%d3
blt.s 4f
move.w #1,(-4,%a2)
.balignw 4,0x2048
5:
subq.l #2,%sp
move.w (-2,%a2),%d0
add.w %d0,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
add.w (-4,%a2),%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,(-4,%a2)
dbf %d3,5b
4:
addq.w #1,(-2,%a2)
dbf %d2,2b
1:
movem.l (-0x10,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl FORHM_$NESTGRID
FORHM_$NESTGRID:
link.w %fp,#-4
bsr.s GRID
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-4,%fp),-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORHM","aw",@nobits
.balign 4
.Ldata:
.space 20
