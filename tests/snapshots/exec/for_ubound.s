.text
.globl FORUB_$PTR
FORUB_$PTR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l (4,%a5)
movea.l (8,%fp),%a0
tst.w (%a0)
beq.s 1f
move.w (%a0),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$REG
FORUB_$REG:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
tst.w %d0
beq.s 1f
move.w %d0,%d1
subq.w #1,%d1
move.w %d1,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$UNIT
FORUB_$UNIT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(8,%a5)
clr.l (4,%a5)
tst.w (8,%a5)
beq.s 1f
move.w (8,%a5),%d0
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$FROM0
FORUB_$FROM0:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$FROM2
FORUB_$FROM2:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
cmpi.w #2,%d0
bcs.s 1f
move.w %d0,%d1
subq.w #2,%d1
move.w %d1,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$MINUS
FORUB_$MINUS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
cmpi.w #2,%d0
bcs.s 1f
move.w %d0,%d1
subq.w #2,%d1
move.w %d1,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$DOWN
FORUB_$DOWN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
move.w %d0,%d1
subq.w #1,%d1
bcs.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d1,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$RTFROM
FORUB_$RTFROM:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.l (4,%a5)
move.w %d1,%d2
sub.w %d0,%d2
bcs.s 1f
move.w %d2,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORUB_$NEG
FORUB_$NEG:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
move.w %d0,%d1
addq.w #1,%d1
blt.s 1f
move.w %d1,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORUB_$VOL
FORUB_$VOL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,(EXEC_$COUNTER).l
clr.l (4,%a5)
move.w (EXEC_$COUNTER).l,%d0
tst.w %d0
beq.s 1f
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,(EXEC_$COUNTER).l
clr.l (4,%a5)
move.w (EXEC_$COUNTER).l,%d0
tst.w %d0
beq.s 4f
subq.w #1,%d0
move.w %d0,%d0
.balignw 4,0x2048
5:
addq.l #1,(4,%a5)
dbf %d0,5b
4:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORUB_$HI
FORUB_$HI:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.l (4,%a5)
clr.l %d0
move.w %d2,%d0
subq.l #1,%d0
subi.l #0x7fff,%d0
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
move.w %d2,%d0
subq.l #1,%d0
tst.l %d0
bmi.s 4f
.balignw 4,0x2048
5:
addq.l #1,(4,%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
4:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl FORUB_$AB
FORUB_$AB:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
clr.l (4,%a5)
clr.l %d0
clr.l %d1
move.w %d2,%d0
move.w %d3,%d1
subq.l #1,%d1
sub.l %d0,%d1
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d1,2b
clr.w %d1
subq.l #1,%d1
bcc.s 2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
clr.l %d1
move.w %d2,%d0
move.w %d3,%d1
subq.l #1,%d1
sub.l %d1,%d0
blt.s 4f
.balignw 4,0x2048
5:
addq.l #1,(4,%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
4:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
clr.l %d1
move.w %d3,%d0
move.w %d2,%d1
subq.l #1,%d0
sub.l %d1,%d0
blt.s 7f
.balignw 4,0x2048
8:
addq.l #1,(4,%a5)
dbf %d0,8b
clr.w %d0
subq.l #1,%d0
bcc.s 8b
7:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
clr.l %d1
move.w %d2,%d0
move.w %d3,%d1
addq.l #1,%d0
sub.l %d0,%d1
blt.s 10f
.balignw 4,0x2048
11:
addq.l #1,(4,%a5)
dbf %d1,11b
clr.w %d1
subq.l #1,%d1
bcc.s 11b
10:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORUB_$BIGC
FORUB_$BIGC:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.l (4,%a5)
clr.l %d0
move.w %d2,%d0
subi.l #0x9c3f,%d0
subq.l #1,%d0
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
move.w %d2,%d0
subi.l #0xffff,%d0
subq.l #1,%d0
blt.s 4f
.balignw 4,0x2048
5:
addq.l #1,(4,%a5)
dbf %d0,5b
clr.w %d0
subq.l #1,%d0
bcc.s 5b
4:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
move.w %d2,%d0
subi.l #0x10000,%d0
tst.l %d0
bmi.s 7f
.balignw 4,0x2048
8:
addq.l #1,(4,%a5)
dbf %d0,8b
clr.w %d0
subq.l #1,%d0
bcc.s 8b
7:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
FORUB_F:
link.w %fp,#-8
move.w (8,%fp),%d0
unlk %fp
rts
.globl FORUB_$FN
FORUB_$FN:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
clr.l (4,%a5)
clr.l %d4
subq.l #2,%sp
move.w %d2,%d4
move.w %d3,-(%sp)
bsr.s FORUB_F
addq.w #4,%sp
clr.l %d1
move.w %d0,%d1
subq.l #1,%d1
sub.l %d4,%d1
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d1,2b
clr.w %d1
subq.l #1,%d1
bcc.s 2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s FORUB_F
addq.w #4,%sp
clr.l %d1
move.w %d0,%d1
clr.l %d0
subq.l #1,%d1
move.w %d3,%d0
sub.l %d0,%d1
blt.s 4f
.balignw 4,0x2048
5:
addq.l #1,(4,%a5)
dbf %d1,5b
clr.w %d1
subq.l #1,%d1
bcc.s 5b
4:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d0
move.w %d2,%d0
andi.l #-0xffff001,(%a5)
lsl.l #8,%d0
lsl.l #4,%d0
or.l %d0,(%a5)
clr.l (4,%a5)
move.l #0xffff000,%d0
and.l (%a5),%d0
lsr.l #8,%d0
lsr.l #4,%d0
clr.l %d1
andi.l #0xffff,%d0
move.w %d3,%d1
subq.l #1,%d0
sub.l %d1,%d0
blt.s 7f
.balignw 4,0x2048
8:
addq.l #1,(4,%a5)
dbf %d0,8b
clr.w %d0
subq.l #1,%d0
bcc.s 8b
7:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl FORUB_$MIXED
FORUB_$MIXED:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d0
move.w (0xc,%fp),%d3
clr.l (4,%a5)
clr.l %d1
move.w %d0,%d4
move.w %d2,%d1
ext.l %d4
sub.l %d1,%d4
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d4,2b
clr.w %d4
subq.l #1,%d4
bcc.s 2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l (4,%a5)
clr.l %d0
move.w %d3,%d1
move.w %d2,%d0
lsr.w #1,%d1
subq.w #1,%d1
ext.l %d1
sub.l %d0,%d1
blt.s 4f
.balignw 4,0x2048
5:
addq.l #1,(4,%a5)
dbf %d1,5b
clr.w %d1
subq.l #1,%d1
bcc.s 5b
4:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl FORUB_$HUGE
FORUB_$HUGE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (4,%a5)
move.w %d0,%d1
subq.l #2,%sp
mulu.w #0x7530,%d1
move.w #0x7530,-(%sp)
move.l %d1,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
subq.l #2,%sp
move.w #0x7530,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
subq.l #2,%sp
move.w #0x7530,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
subq.l #2,%sp
move.w #0x7530,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
subq.l #1,%d0
blt.s 1f
.balignw 4,0x2048
2:
addq.l #1,(4,%a5)
dbf %d0,2b
clr.w %d0
subq.l #1,%d0
bcc.s 2b
1:
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORUB","aw",@nobits
.balign 4
.Ldata:
.space 12
