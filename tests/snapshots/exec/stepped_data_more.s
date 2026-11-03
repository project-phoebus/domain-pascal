.text
.globl SDM_$SETUP
SDM_$SETUP:
link.w %fp,#-0x10
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #7,%d0
moveq #-3,%d1
lea (-0xc,%a5),%a0
.balignw 4,0x2048
2:
move.w %d1,%d2
ext.l %d2
move.l %d2,%d3
add.l %d3,%d3
add.l %d3,%d2
move.l %d2,(0xc,%a0)
addq.w #1,%d1
addq.l #4,%a0
dbf %d0,2b
moveq #9,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
5:
movea.l %a0,%a1
move.w %d1,(0x1a,%a1)
move.w %d1,%d2
add.w %d2,%d2
move.w %d2,%d3
lsl.w #2,%d3
add.w %d3,%d2
move.w %d2,(0x1c,%a1)
clr.w (0x1e,%a1)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,5b
moveq #4,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
8:
movea.l %a0,%a1
clr.w (0x58,%a1)
moveq #0x64,%d2
muls.w %d1,%d2
move.w %d2,(0x56,%a1)
move.w #1,(0x5a,%a1)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,8b
moveq #4,%d0
movea.l #SDM_$X,%a0
moveq #1,%d1
addq.l #6,%a0
.balignw 4,0x2048
11:
movea.l %a0,%a1
move.w %d1,(0xc,%a1)
move.w %d1,%d2
neg.w %d2
move.w %d2,(0xe,%a1)
move.w %d1,%d2
addq.w #1,%d2
move.w %d2,(0x10,%a1)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,11b
move.w #-2,(SDM_$X).l
move.w #-1,(SDM_$X+6).l
clr.w (SDM_$X+0xc).l
move.w #-1,(SDM_$X+4).l
clr.w (SDM_$X+0xa).l
move.w #1,(SDM_$X+0x10).l
move.w #2,(SDM_$X+2).l
move.w #1,(SDM_$X+8).l
clr.w (SDM_$X+0xe).l
movem.l (-0x1c,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SDM_$NEG
SDM_$NEG:
link.w %fp,#-0xc
pea (%a5)
lea (.Ldata).l,%a5
clr.l %d1
moveq #7,%d0
lea (-0xc,%a5),%a0
.balignw 4,0x2048
2:
add.l (0xc,%a0),%d1
addq.l #4,%a0
dbf %d0,2b
subq.l #2,%sp
move.w #0x3e8,-(%sp)
move.l (%a5),-(%sp)
jsr (M$MIS$LLW).l
add.l %d1,%d0
movea.l (-0x10,%fp),%a5
unlk %fp
rts
.globl SDM_$SUM6
SDM_$SUM6:
link.w %fp,#-0xc
movem.l %d2-%d3/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d2
movea.l (8,%fp),%a0
move.w (%a0),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d3
lea (6,%a5),%a2
.balignw 4,0x2048
2:
move.w (0x1c,%a2),%d0
ext.l %d0
add.l %d0,%d2
jsr (EXEC_$CLOBBER).l
addq.l #6,%a2
dbf %d3,2b
1:
move.l %d2,%d0
movem.l (-0x1c,%fp),%d2-%d3/%a2/%a5
unlk %fp
rts
.globl SDM_$PART
SDM_$PART:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
moveq #4,%d1
lea (0x12,%a5),%a0
.balignw 4,0x2048
2:
move.w (0x1c,%a0),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,2b
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl SDM_$DOWN
SDM_$DOWN:
link.w %fp,#-8
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #9,%d2
lea (0x3c,%a5),%a2
.balignw 4,0x2048
2:
movea.l %a2,%a3
subq.l #2,%sp
move.w (0x1a,%a3),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (0x1a,%a3),(0x1e,%a3)
subq.l #6,%a2
dbf %d2,2b
movem.l (-0x18,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl SDM_$NESTWITH
SDM_$NESTWITH:
link.w %fp,#-0x10
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #4,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.w %d1,(0x58,%a1)
move.w (0x56,%a1),%d2
add.w (0x58,%a1),%d2
move.w %d2,(0x5a,%a1)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,2b
clr.l %d0
moveq #4,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
5:
move.w (0x5a,%a0),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,5b
movem.l (-0x18,%fp),%d2/%a5
unlk %fp
rts
.globl SDM_$NESTED
SDM_$NESTED:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #4,%d0
lea (6,%a5),%a0
.balignw 4,0x2048
2:
moveq #9,%d1
lea (6,%a5),%a1
.balignw 4,0x2048
5:
move.w (0x1e,%a1),%d2
add.w (0x56,%a0),%d2
move.w %d2,(0x1e,%a1)
addq.l #6,%a1
dbf %d1,5b
addq.l #6,%a0
dbf %d0,2b
clr.l %d0
moveq #9,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
8:
move.w (0x1e,%a0),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,8b
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl SDM_$EXITNEXT
SDM_$EXITNEXT:
link.w %fp,#-0xc
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
moveq #9,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
cmpi.w #3,(0x1a,%a0)
seq %d2
cmpi.w #5,(0x1a,%a0)
seq %d3
or.b %d3,%d2
bmi.s 3f
cmpi.w #7,(0x1a,%a0)
bgt.s 1f
move.w (0x1a,%a0),%d2
ext.l %d2
add.l %d2,%d0
3:
addq.l #6,%a0
dbf %d1,2b
1:
movem.l (-0x18,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SDM_$WITHLOOP
SDM_$WITHLOOP:
link.w %fp,#-0x10
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #2,%d2
lea (6,%a5),%a2
.balignw 4,0x2048
2:
movea.l %a2,%a0
clr.w (0x58,%a0)
moveq #3,%d0
lea (6,%a5),%a1
.balignw 4,0x2048
5:
move.w (0x58,%a0),%d1
add.w (0x1c,%a1),%d1
move.w %d1,(0x58,%a0)
addq.l #6,%a1
dbf %d0,5b
move.w (0x58,%a0),(0x56,%a0)
jsr (EXEC_$CLOBBER).l
addq.l #6,%a2
dbf %d2,2b
clr.l %d0
moveq #4,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
8:
move.w (0x56,%a0),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,8b
movem.l (-0x1c,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl SDM_$FAR
SDM_$FAR:
link.w %fp,#-0x10
movem.l %d2-%d3/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.w (%a2),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
movea.l %a5,%a3
moveq #1,%d3
lea (0xa,%a3),%a3
.balignw 4,0x2048
2:
movea.l %a3,%a0
move.w %d3,(0x202,%a0)
move.w %d3,%d0
add.w %d0,%d0
move.w %d0,(0x20a,%a0)
jsr (EXEC_$CLOBBER).l
addq.w #1,%d3
lea (0xa,%a3),%a3
dbf %d2,2b
1:
clr.l %d2
move.w (%a2),%d0
subq.w #1,%d0
bmi.s 4f
movea.l %a5,%a2
move.w %d0,%d3
lea (0xa,%a2),%a2
.balignw 4,0x2048
5:
move.w (0x202,%a2),%d0
move.w (0x20a,%a2),%d1
ext.l %d0
ext.l %d1
add.l %d2,%d0
add.l %d1,%d0
move.l %d0,%d2
jsr (EXEC_$CLOBBER).l
lea (0xa,%a2),%a2
dbf %d3,5b
4:
move.l %d2,%d0
movem.l (-0x24,%fp),%d2-%d3/%a2-%a3/%a5
unlk %fp
rts
.globl SDM_$FRAME
SDM_$FRAME:
link.w %fp,#-0xc4
movem.l %d2-%d3/%a2-%a3,-(%sp)
movea.l (8,%fp),%a2
moveq #5,%d0
moveq #-2,%d1
lea (-8,%fp),%a0
.balignw 4,0x2048
2:
move.w %d1,%d2
ext.l %d2
move.l %d2,(-0x10,%a0)
addq.w #1,%d1
addq.l #4,%a0
dbf %d0,2b
move.w (%a2),%d0
subq.w #1,%d0
bmi.s 4f
move.w %d0,%d2
movea.l %fp,%a3
moveq #1,%d3
addq.l #4,%a3
.balignw 4,0x2048
5:
move.w %d3,%d0
ext.l %d0
move.l %d0,%d1
neg.l %d0
lsl.l #3,%d1
add.l %d1,%d0
move.l %d0,(-0xbc,%a3)
jsr (EXEC_$CLOBBER).l
addq.w #1,%d3
addq.l #4,%a3
dbf %d2,5b
4:
clr.l %d2
moveq #5,%d1
lea (-8,%fp),%a1
.balignw 4,0x2048
8:
subq.l #2,%sp
move.w #0x3e8,-(%sp)
move.l (-0x10,%a1),-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
add.l %d0,%d2
addq.l #4,%a1
dbf %d1,8b
move.w (%a2),%d0
subq.w #1,%d0
bmi.s 10f
movea.l %fp,%a2
move.w %d0,%d3
addq.l #4,%a2
.balignw 4,0x2048
11:
pea (-0xbc,%a2)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
add.l (-0xbc,%a2),%d2
addq.l #4,%a2
dbf %d3,11b
10:
move.l %d2,%d0
movem.l (-0xd4,%fp),%d2-%d3/%a2-%a3
unlk %fp
rts
.globl SDM_$EXTDOWN
SDM_$EXTDOWN:
link.w %fp,#-0xc
movem.l %d2-%d3/%a2,-(%sp)
clr.l %d2
movea.l #SDM_$X,%a2
moveq #4,%d3
lea (0x1e,%a2),%a2
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (0x10,%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (0xc,%a2),%d0
muls.w (0xe,%a2),%d0
subq.l #6,%a2
add.l %d0,%d2
dbf %d3,2b
move.l %d2,%d0
movem.l (-0x18,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl SDM_$RESET
SDM_$RESET:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #9,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.w %d1,%d2
add.w %d2,%d2
move.w %d2,%d3
lsl.w #2,%d3
add.w %d3,%d2
move.w %d2,(0x1c,%a1)
clr.w (0x1e,%a1)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,2b
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SDM_$INNERWITH
SDM_$INNERWITH:
link.w %fp,#-0x10
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
moveq #3,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
moveq #2,%d2
moveq #1,%d3
.balignw 4,0x2048
5:
movea.l %a0,%a1
move.w (0x1e,%a1),%d4
add.w %d3,%d4
move.w %d4,(0x1e,%a1)
addq.w #1,%d3
dbf %d2,5b
move.w (0x1a,%a0),%d2
ext.l %d2
add.l %d0,%d2
move.w (0x1e,%a0),%d0
ext.l %d0
addq.l #6,%a0
add.l %d0,%d2
move.l %d2,%d0
dbf %d1,2b
movem.l (-0x20,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl SDM_$MIXED
SDM_$MIXED:
link.w %fp,#-0x10
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
moveq #9,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.w (0x1a,%a1),%d2
add.w (0x1e,%a0),%d2
move.w %d2,(0x1c,%a0)
move.w (0x1c,%a1),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,2b
movem.l (-0x18,%fp),%d2/%a5
unlk %fp
rts
.globl SDM_$FARDOWN
SDM_$FARDOWN:
link.w %fp,#-0x10
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
movea.l %a5,%a0
moveq #0x1d,%d1
lea (0x12c,%a0),%a0
.balignw 4,0x2048
2:
addq.l #1,%d0
move.w %d0,(0x206,%a0)
lea (-0xa,%a0),%a0
dbf %d1,2b
clr.l %d0
moveq #0x1d,%d1
movea.l %a5,%a0
moveq #1,%d2
lea (0xa,%a0),%a0
.balignw 4,0x2048
5:
move.w (0x206,%a0),%d3
muls.w %d2,%d3
addq.w #1,%d2
add.l %d3,%d0
lea (0xa,%a0),%a0
dbf %d1,5b
movem.l (-0x1c,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SDM_$K12
SDM_$K12:
link.w %fp,#-0xac
move.l %d2,-(%sp)
moveq #0x21,%d0
movea.l %fp,%a0
moveq #3,%d1
lea (0xc,%a0),%a0
.balignw 4,0x2048
2:
move.w %d1,%d2
ext.l %d2
move.l %d2,(-0xa4,%a0)
addq.w #1,%d1
addq.l #4,%a0
dbf %d0,2b
clr.l %d0
movea.l %fp,%a0
moveq #0x21,%d1
lea (0xc,%a0),%a0
.balignw 4,0x2048
5:
add.l (-0xa4,%a0),%d0
addq.l #4,%a0
dbf %d1,5b
move.l (-0xb0,%fp),%d2
unlk %fp
rts
.globl SDM_$RUNIDX
SDM_$RUNIDX:
link.w %fp,#-0xc
movem.l %d2-%d4/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
clr.l %d2
move.w (%a2),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d3
movea.l %a5,%a3
moveq #1,%d4
lea (0xa,%a3),%a3
.balignw 4,0x2048
2:
move.w %d4,(0x20a,%a3)
jsr (EXEC_$CLOBBER).l
addq.w #1,%d4
lea (0xa,%a3),%a3
dbf %d3,2b
1:
move.w (%a2),%d0
subq.w #1,%d0
bmi.s 4f
movea.l %a5,%a2
move.w %d0,%d3
lea (0xa,%a2),%a2
.balignw 4,0x2048
5:
move.w (0x20a,%a2),%d0
ext.l %d0
add.l %d0,%d2
jsr (EXEC_$CLOBBER).l
lea (0xa,%a2),%a2
dbf %d3,5b
4:
move.l %d2,%d0
movem.l (-0x24,%fp),%d2-%d4/%a2-%a3/%a5
unlk %fp
rts
.globl SDM_$ONE_PASS
SDM_$ONE_PASS:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
clr.w %d1
lea (0x2a,%a5),%a0
.balignw 4,0x2048
2:
move.w (0x1c,%a0),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,2b
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl SDM_$TWOWITH
SDM_$TWOWITH:
link.w %fp,#-0x14
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d0
moveq #9,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.w (0x1a,%a1),%d2
movea.l %a0,%a1
ext.l %d2
add.l %d2,%d0
move.w (0x1c,%a1),%d2
addq.l #6,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,2b
movem.l (-0x1c,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SDM","aw",@nobits
.balign 4
.Ldata:
.space 824
