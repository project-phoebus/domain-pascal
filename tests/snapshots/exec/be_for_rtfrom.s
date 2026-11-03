.text
.globl BF_$SUM
BF_$SUM:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.w (%a5)
move.w %d1,%d2
sub.w %d0,%d2
blt.s 1f
move.w %d2,%d1
move.w %d0,%d2
.balignw 4,0x2048
2:
move.w (%a5),%d0
add.w %d2,%d0
move.w %d0,(%a5)
addq.w #1,%d2
dbf %d1,2b
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BF_$COUNT
BF_$COUNT:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.l (0x44,%a5)
move.w %d1,%d2
sub.w %d0,%d2
blt.s 1f
move.w %d2,%d0
.balignw 4,0x2048
2:
addq.l #1,(0x44,%a5)
dbf %d0,2b
1:
move.l (0x44,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BF_$CALLS
BF_$CALLS:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d1,%d2
sub.w %d0,%d2
blt.s 1f
move.w %d2,%d3
move.w %d0,%d2
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,%d2
dbf %d3,2b
1:
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BF_$ARRAYS
BF_$ARRAYS:
link.w %fp,#-4
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
moveq #7,%d0
moveq #1,%d1
lea (2,%a5),%a0
lea (4,%a5),%a1
.balignw 4,0x2048
2:
move.w %d1,(6,%a0)
move.w %d1,%d4
ext.l %d4
add.l %d4,%d4
neg.l %d4
move.l %d4,(0x14,%a1)
addq.w #1,%d1
addq.l #4,%a1
addq.l #2,%a0
dbf %d0,2b
clr.w (%a5)
clr.l (4,%a5)
move.w %d3,%d0
sub.w %d2,%d0
blt.s 4f
move.w %d0,%d1
move.w %d2,%d0
.balignw 4,0x2048
5:
move.w %d0,%d4
move.w (%a5),%d5
add.w %d4,%d4
add.w (6,%a5,%d4.w),%d5
move.w %d5,(%a5)
move.w %d0,%d4
move.l (4,%a5),%d5
lsl.w #2,%d4
add.l (0x14,%a5,%d4.w),%d5
move.l %d5,(4,%a5)
addq.w #1,%d0
dbf %d1,5b
4:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d3,%d0
sub.w %d2,%d0
blt.s 7f
move.w %d0,%d1
move.w %d2,%d0
.balignw 4,0x2048
8:
move.w %d0,%d2
add.w %d2,%d2
move.w %d2,%d3
lsl.w #2,%d3
add.w %d3,%d2
move.w %d0,%d3
add.w %d3,%d3
move.w %d2,(6,%a5,%d3.w)
addq.w #1,%d0
dbf %d1,8b
7:
subq.l #2,%sp
move.w (8,%a5),%d0
add.w (0xa,%a5),%d0
add.w (0xc,%a5),%d0
add.w (0xe,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x18,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl BF_$PLUS
BF_$PLUS:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w (%a5)
moveq #5,%d1
move.w %d0,%d2
addq.w #1,%d2
sub.w %d2,%d1
blt.s 1f
move.w %d1,%d0
move.w %d2,%d1
.balignw 4,0x2048
2:
move.w (%a5),%d2
add.w %d1,%d2
move.w %d2,(%a5)
addq.w #1,%d1
dbf %d0,2b
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BF_$FN
BF_$FN:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d2
move.w %d0,(0x38,%a5)
clr.w (%a5)
move.w %d2,%d3
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
sub.w %d1,%d3
blt.s 1f
move.w %d3,%d0
move.w %d1,%d3
.balignw 4,0x2048
2:
move.w (%a5),%d1
add.w %d3,%d1
move.w %d1,(%a5)
addq.w #1,%d3
dbf %d0,2b
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
clr.w (%a5)
move.w %d2,%d0
addq.w #1,%d0
sub.w (0x38,%a5),%d0
blt.s 4f
move.w %d0,%d1
.balignw 4,0x2048
5:
addq.w #1,(%a5)
dbf %d1,5b
4:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BF_$TABLE
BF_$TABLE:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #3,(0x3a,%a5)
move.w #6,(0x3c,%a5)
move.w #2,(0x3e,%a5)
move.w #4,(0x40,%a5)
clr.w (%a5)
move.w %d0,%d1
lsl.w #2,%d1
move.w (0x38,%a5,%d1.w),%d0
subq.w #1,%d0
sub.w (0x36,%a5,%d1.w),%d0
blt.s 1f
move.w %d0,%d2
move.w (0x36,%a5,%d1.w),%d0
.balignw 4,0x2048
2:
move.w (%a5),%d1
add.w %d0,%d1
move.w %d1,(%a5)
addq.w #1,%d0
dbf %d2,2b
1:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BF_$UNSIGNED
BF_$UNSIGNED:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.l (0x44,%a5)
move.w %d1,%d2
sub.w %d0,%d2
bcs.s 1f
move.w %d2,%d0
.balignw 4,0x2048
2:
addq.l #1,(0x44,%a5)
dbf %d0,2b
1:
move.l (0x44,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BF_$VOL
BF_$VOL:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w #0x64,(EXEC_$COUNTER).l
moveq #0x67,%d0
move.w (EXEC_$COUNTER).l,%d1
sub.w %d1,%d0
blt.s 1f
move.w %d0,%d2
move.w %d1,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_FOR_RTFROM","aw",@nobits
.balign 4
.Ldata:
.space 72
