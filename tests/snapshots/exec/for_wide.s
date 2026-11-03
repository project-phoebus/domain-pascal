.text
.globl FORWD_$LCONST
FORWD_$LCONST:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l (4,%a5)
moveq #4,%d0
clr.l %d1
.balignw 4,0x2048
2:
move.l (4,%a5),%d2
add.l %d1,%d2
move.l %d2,(4,%a5)
addq.l #1,%d1
dbf %d0,2b
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
moveq #5,%d2
moveq #3,%d3
.balignw 4,0x2048
5:
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #1,%d3
dbf %d2,5b
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORWD_$LBIG
FORWD_$LBIG:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
move.l #0x1116f,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORWD_$LRUN
FORWD_$LRUN:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l (0xc,%fp),%d3
clr.l (%a5)
clr.l (8,%a5)
clr.l (0xc,%a5)
move.l %d2,%d0
move.l %d3,%d1
sub.l %d0,%d1
blt.s 1f
.balignw 4,0x2048
2:
tst.l (%a5)
bne.s 4f
move.l %d0,(8,%a5)
4:
addq.l #1,(%a5)
move.l %d0,(0xc,%a5)
addq.l #1,%d0
dbf %d1,2b
clr.w %d1
subq.l #1,%d1
bcc.s 2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (8,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (0xc,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d3,%d0
sub.l %d2,%d0
blt.s 5f
.balignw 4,0x2048
6:
addq.l #1,(%a5)
dbf %d0,6b
clr.w %d0
subq.l #1,%d0
bcc.s 6b
5:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORWD_$LUPTO
FORWD_$LUPTO:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
clr.l (%a5)
move.l %d2,%d0
subq.l #1,%d0
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
tst.l %d0
bmi.s 4f
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
subi.l #0x14,%d0
blt.s 7f
moveq #0x14,%d1
.balignw 4,0x2048
8:
move.l (%a5),%d3
add.l %d1,%d3
move.l %d3,(%a5)
addq.l #1,%d1
dbf %d0,8b
clr.w %d0
subq.l #1,%d0
bcc.s 8b
7:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
move.l %d0,%d1
subi.l #0x14,%d1
blt.s 10f
.balignw 4,0x2048
11:
move.l (%a5),%d3
add.l %d0,%d3
move.l %d3,(%a5)
subq.l #1,%d0
dbf %d1,11b
clr.w %d1
subq.l #1,%d1
bcc.s 11b
10:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
tst.l %d0
bmi.s 13f
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
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORWD_$UUPTO
FORWD_$UUPTO:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
clr.l (%a5)
move.l %d2,%d0
subq.l #1,%d0
bcs.s 1f
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
move.l %d0,%d1
subq.l #1,%d1
bcs.s 7f
.balignw 4,0x2048
8:
move.l (%a5),%d2
add.l %d0,%d2
move.l %d2,(%a5)
subq.l #1,%d0
dbf %d1,8b
clr.w %d1
subq.l #1,%d1
bcc.s 8b
7:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORWD_$WIRE
FORWD_$WIRE:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l %d1,%d2
move.l %d0,%d1
lsr.l #8,%d2
lsr.l #8,%d1
lsr.l #2,%d2
lsr.l #2,%d1
sub.l %d1,%d2
addq.l #1,%d2
move.l %d2,%d3
subq.l #1,%d3
bcs.s 1f
moveq #1,%d2
.balignw 4,0x2048
2:
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
addq.l #1,%d2
dbf %d3,2b
clr.w %d3
subq.l #1,%d3
bcc.s 2b
1:
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.globl FORWD_$ONCE
FORWD_$ONCE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #3,%d0
move.l %d0,(0x10,%a5)
clr.l (%a5)
move.l (0x10,%a5),%d0
subq.l #1,%d0
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(0x10,%a5)
addq.l #1,(%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORWD_$BYTES
FORWD_$BYTES:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l (%a5)
clr.w (0x14,%a5)
move.w #0xff,%d0
clr.b %d1
.balignw 4,0x2048
2:
addq.l #1,(%a5)
clr.w %d2
move.b %d1,%d2
add.w (0x14,%a5),%d2
move.w %d2,(0x14,%a5)
addq.b #1,%d1
dbf %d0,2b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x14,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
clr.l (%a5)
move.w #0xff,%d0
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
clr.w (0x14,%a5)
move.w #0xff,%d0
moveq #-0x80,%d1
.balignw 4,0x2048
8:
addq.l #1,(%a5)
move.b %d1,%d2
ext.w %d2
add.w (0x14,%a5),%d2
move.w %d2,(0x14,%a5)
addq.b #1,%d1
dbf %d0,8b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x14,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
clr.l (%a5)
move.w #0xff,%d0
clr.b %d1
.balignw 4,0x2048
11:
clr.w %d2
move.b %d1,%d2
ext.l %d2
add.l (%a5),%d2
move.l %d2,(%a5)
addq.b #1,%d1
dbf %d0,11b
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl FORWD_$CRUN
FORWD_$CRUN:
link.w %fp,#-4
movem.l %d2-%d5,-(%sp)
move.b (8,%fp),%d2
move.b (0xa,%fp),%d3
move.b %d2,%d4
move.b %d3,%d5
sub.b %d4,%d5
bcs.s 1f
andi.w #0xff,%d5
.balignw 4,0x2048
2:
subq.l #2,%sp
move.b %d4,-(%sp)
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
addq.b #1,%d4
dbf %d5,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.b %d3,%d4
move.b %d4,%d3
sub.b %d2,%d3
bcs.s 4f
andi.w #0xff,%d3
.balignw 4,0x2048
5:
subq.l #2,%sp
move.b %d4,-(%sp)
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
subq.b #1,%d4
dbf %d3,5b
4:
subq.l #2,%sp
move.w #0x62,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x14,%fp),%d2-%d5
unlk %fp
rts
.globl FORWD_$URUN
FORWD_$URUN:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d2
move.b (0xa,%fp),%d3
clr.l (%a5)
move.b %d3,%d0
sub.b %d2,%d0
bcs.s 1f
andi.w #0xff,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.b %d3,%d0
sub.b %d2,%d0
bcs.s 4f
andi.w #0xff,%d0
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
.globl FORWD_$SRUN
FORWD_$SRUN:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.b (0xa,%fp),%d1
clr.l (%a5)
clr.w (0x14,%a5)
move.b %d0,%d2
move.b %d1,%d0
sub.b %d2,%d0
blt.s 1f
andi.w #0xff,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
move.b %d2,%d1
ext.w %d1
add.w (0x14,%a5),%d1
move.w %d1,(0x14,%a5)
addq.b #1,%d2
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x14,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORWD_$AFTER
FORWD_$AFTER:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #9,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
cmpi.l #4,%d1
beq.s 1f
addq.l #1,%d1
dbf %d0,2b
1:
move.l %d1,(0xc,%a5)
move.l (0xc,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
moveq #1,%d2
moveq #2,%d3
.balignw 4,0x2048
5:
move.l %d3,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
addq.l #1,%d3
dbf %d2,5b
moveq #0x19,%d0
moveq #0x61,%d1
.balignw 4,0x2048
8:
cmpi.b #0x71,%d1
beq.s 7f
addq.b #1,%d1
dbf %d0,8b
7:
clr.w %d0
move.b %d1,%d0
ext.l %d0
move.l %d0,(0xc,%a5)
move.l (0xc,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORWD_$NEXT
FORWD_$NEXT:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l (%a5)
moveq #3,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
cmpi.l #3,%d1
beq.s 3f
move.l (%a5),%d2
add.l %d1,%d2
move.l %d2,(%a5)
3:
addq.l #1,%d1
dbf %d0,2b
move.l (%a5),%d0
subq.l #2,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.w (0x14,%a5)
moveq #4,%d0
moveq #0x61,%d1
.balignw 4,0x2048
5:
cmpi.b #0x63,%d1
beq.s 6f
addq.w #1,(0x14,%a5)
6:
addq.b #1,%d1
dbf %d0,5b
subq.l #2,%sp
move.w (0x14,%a5),%d0
addq.w #3,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl FORWD_$BELOW
FORWD_$BELOW:
link.w %fp,#-0xc
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d3
clr.l (%a5)
move.w %d2,%d0
subq.w #1,%d0
tst.w %d0
bmi.s 1f
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d3,%d0
subq.l #1,%d0
tst.l %d0
bmi.s 4f
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.w %d2,%d0
move.l %d3,%d1
ext.l %d0
addq.l #1,%d1
sub.l %d1,%d0
blt.s 7f
.balignw 4,0x2048
8:
addq.l #1,(%a5)
dbf %d0,8b
clr.w %d0
subq.l #1,%d0
bcc.s 8b
7:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x18,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORWD_$UWORD
FORWD_$UWORD:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d2
clr.l (%a5)
tst.w %d0
beq.s 1f
move.w %d0,%d1
subq.w #1,%d1
move.w %d1,%d0
.balignw 4,0x2048
2:
addq.l #1,(%a5)
dbf %d0,2b
1:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
move.l %d2,%d0
subq.l #1,%d0
blt.s 4f
.balignw 4,0x2048
5:
addq.l #1,(%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
4:
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (%a5)
clr.l (0xc,%a5)
move.l %d2,%d0
subq.l #1,%d0
tst.l %d0
bmi.s 7f
clr.w %d1
.balignw 4,0x2048
8:
clr.l %d2
move.w %d1,%d2
move.l %d2,(0xc,%a5)
addq.w #1,%d1
dbf %d0,8b
clr.w %d0
subq.l #1,%d0
bcc.s 8b
7:
move.l (%a5),%d0
add.l (0xc,%a5),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORWD","aw",@nobits
.balign 4
.Ldata:
.space 24
