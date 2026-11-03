.text
.globl FORIX_$SETUP
FORIX_$SETUP:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l %d0,(0x3c,%a5)
moveq #9,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
move.w %d1,%d2
ext.l %d2
add.l %d2,%d2
move.l %d2,%d3
lsl.l #2,%d3
add.l %d3,%d2
move.l %d2,(-6,%a0)
clr.w (-2,%a0)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,2b
moveq #3,%d0
moveq #1,%d1
lea (2,%a5),%a0
.balignw 4,0x2048
5:
move.w %d1,%d2
addq.w #8,%d2
move.w %d2,(0x3e,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,5b
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORIX_$UPTO
FORIX_$UPTO:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.l (8,%fp),%d0
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl FORIX_$ALL
FORIX_$ALL:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0x3e,%a5),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORIX_$FIND
FORIX_$FIND:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
moveq #9,%d2
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
cmp.l (-6,%a0),%d0
beq.s 1f
addq.w #1,%d1
addq.l #6,%a0
dbf %d2,2b
1:
move.w %d1,(0x48,%a5)
subq.l #2,%sp
move.w (0x48,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORIX_$TWICE
FORIX_$TWICE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #2,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
move.w %d1,(-2,%a0)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,2b
moveq #4,%d1
moveq #1,%d0
lea (6,%a5),%a0
.balignw 4,0x2048
5:
tst.w (-2,%a0)
beq.s 4f
addq.w #1,%d0
addq.l #6,%a0
dbf %d1,5b
4:
move.w %d0,(0x48,%a5)
subq.l #2,%sp
move.w (0x48,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORIX_$CLASSES
FORIX_$CLASSES:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d0
moveq #1,%d1
lea (2,%a5),%a0
.balignw 4,0x2048
2:
move.w (0x3e,%a0),%d2
add.w %d2,%d2
move.w %d1,(0x4a,%a5,%d2.w)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
subq.l #2,%sp
move.w (0x5c,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x5e,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x60,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0x62,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORIX","aw",@nobits
.balign 4
.Ldata:
.space 332
