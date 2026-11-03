.section ".text.SOF.INIT_PROC","ax",@progbits
.globl SOF_$INIT
SOF_$INIT:
link.w %fp,#0
movea.l #.Ldata.WIRED_DATA,%a0
moveq #5,%d0
move.l %d0,(%a0)
move.w #0x4d,(0x50,%a0)
unlk %fp
rts
.globl SOF_$FILLW
SOF_$FILLW:
link.w %fp,#-4
move.l %d2,-(%sp)
moveq #9,%d0
clr.w %d1
movea.l #.Ldata.WIRED_DATA,%a0
.balignw 4,0x2048
2:
moveq #0xb,%d2
muls.w %d1,%d2
move.w %d2,(0xa,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
move.l (-8,%fp),%d2
unlk %fp
rts
.globl SOF_$FILL8
SOF_$FILL8:
link.w %fp,#-8
move.l %d2,-(%sp)
moveq #5,%d0
movea.l #.Ldata.WIRED_DATA,%a0
moveq #1,%d1
addq.l #8,%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.w %d1,(0x18,%a1)
move.w #-1,(0x1a,%a1)
move.w %d1,%d2
muls.w #0x3e8,%d2
move.l %d2,(0x1c,%a1)
addq.w #1,%d1
addq.l #8,%a0
dbf %d0,2b
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl SOF_$BLOCKLOG
SOF_$BLOCKLOG:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
movea.l #.Ldata.WIRED_DATA,%a2
moveq #5,%d2
addq.l #8,%a2
.balignw 4,0x2048
2:
cmpi.w #2,(0x18,%a2)
bne.s 4f
subq.l #2,%sp
move.w (0x18,%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
bra.s 5f
4:
cmpi.w #4,(0x18,%a2)
bne.s 6f
subq.l #2,%sp
move.w (0x18,%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
bra.s 7f
6:
jsr (EXEC_$CLOBBER).l
7:
5:
addq.l #8,%a2
dbf %d2,2b
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".text.SOF.WIRED_PROC","ax",@progbits
.globl SOF_$SHOW
SOF_$SHOW:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
move.l (%a5),%d0
moveq #5,%d1
lea (8,%a5),%a0
.balignw 4,0x2048
2:
move.w (0x18,%a0),%d2
ext.l %d2
add.l %d0,%d2
move.w (0x1a,%a0),%d0
add.l (0x1c,%a0),%d2
ext.l %d0
addq.l #8,%a0
add.l %d0,%d2
move.l %d2,%d0
dbf %d1,2b
moveq #8,%d1
lea (2,%a5),%a0
.balignw 4,0x2048
5:
move.w (0xa,%a0),%d2
addq.l #2,%a0
ext.l %d2
add.l %d2,%d0
dbf %d1,5b
move.w (0xa,%a5),%d1
muls.w #0x3e8,%d1
add.l %d0,%d1
moveq #0x64,%d0
muls.w (0x50,%a5),%d0
add.l %d0,%d1
move.l %d1,%d0
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl SOF_$SETUP
SOF_$SETUP:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
move.w #1,(0x5a,%a5)
moveq #9,%d0
move.l %d0,(0x54,%a5)
move.w #3,(0x5e,%a5)
move.w #5,(0x58,%a5)
move.w #5,(0x62,%a5)
move.w #2,(0x5c,%a5)
move.w #4,(0x60,%a5)
moveq #4,%d0
moveq #1,%d1
lea (6,%a5),%a0
.balignw 4,0x2048
2:
move.w %d1,(0x5e,%a0)
move.w %d1,%d2
add.w %d2,%d2
move.w %d2,%d3
lsl.w #2,%d3
add.w %d3,%d2
move.w %d2,(0x60,%a0)
moveq #0x64,%d2
muls.w %d1,%d2
move.w %d2,(0x62,%a0)
addq.w #1,%d1
addq.l #6,%a0
dbf %d0,2b
moveq #3,%d0
movea.l %a5,%a0
moveq #1,%d1
addq.l #4,%a0
.balignw 4,0x2048
5:
move.w %d1,%d2
ext.l %d2
lsl.l #3,%d2
move.l %d2,%d3
lsl.l #2,%d3
add.l %d3,%d2
move.l %d2,(0x7e,%a0)
addq.w #1,%d1
addq.l #4,%a0
dbf %d0,5b
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SOF_$SUMREC_IN
SOF_$SUMREC_IN:
link.w %fp,#-0xc
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
clr.l %d0
moveq #4,%d1
lea (%a0),%a1
.balignw 4,0x2048
2:
move.w (6,%a1),%d2
addq.l #2,%a1
ext.l %d2
add.l %d2,%d0
dbf %d1,2b
moveq #4,%d1
lea (6,%a0),%a1
.balignw 4,0x2048
5:
move.w (0xc,%a1),%d2
addq.l #6,%a1
ext.l %d2
add.l %d2,%d0
dbf %d1,5b
moveq #3,%d1
lea (0xc,%a0),%a1
.balignw 4,0x2048
8:
move.w (0xe,%a1),%d2
addq.l #6,%a1
ext.l %d2
add.l %d2,%d0
dbf %d1,8b
moveq #3,%d1
lea (4,%a0),%a1
.balignw 4,0x2048
11:
add.l (0x2a,%a1),%d0
addq.l #4,%a1
dbf %d1,11b
move.l (-0x10,%fp),%d2
unlk %fp
rts
.globl SOF_$SUMREC
SOF_$SUMREC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
pea (0x54,%a5)
bsr.s SOF_$SUMREC_IN
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SOF_$WITHREC_IN
SOF_$WITHREC_IN:
link.w %fp,#-0x10
movem.l %d2-%d3,-(%sp)
clr.l %d0
moveq #4,%d1
movea.l (8,%fp),%a0
lea (6,%a0),%a1
.balignw 4,0x2048
2:
movea.l %a1,%a0
move.w (0xa,%a0),%d2
muls.w (0xe,%a0),%d2
add.l %d0,%d2
move.w (0xc,%a0),%d0
ext.l %d0
lsl.l #2,%d0
move.l %d0,%d3
lsl.l #2,%d3
add.l %d3,%d0
sub.l %d0,%d2
addq.l #6,%a1
move.l %d2,%d0
addi.l #0x10,%d0
dbf %d1,2b
movem.l (-0x18,%fp),%d2-%d3
unlk %fp
rts
.globl SOF_$WITHREC
SOF_$WITHREC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
pea (0x54,%a5)
bsr.s SOF_$WITHREC_IN
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SOF_$LOGREC_IN
SOF_$LOGREC_IN:
link.w %fp,#-4
movem.l %d2/%a2-%a3,-(%sp)
movea.l (8,%fp),%a2
moveq #4,%d2
lea (%a2),%a3
.balignw 4,0x2048
2:
pea (6,%a3)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
addq.l #2,%a3
dbf %d2,2b
moveq #3,%d2
lea (4,%a2),%a3
.balignw 4,0x2048
5:
move.l (0x2a,%a3),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
addq.l #4,%a3
dbf %d2,5b
movem.l (-0x10,%fp),%d2/%a2-%a3
unlk %fp
rts
.globl SOF_$LOGREC
SOF_$LOGREC:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
pea (0x54,%a5)
bsr.s SOF_$LOGREC_IN
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SOF_$DOWNREC_IN
SOF_$DOWNREC_IN:
link.w %fp,#-0xc
move.l %d2,-(%sp)
clr.l %d0
moveq #3,%d1
movea.l (8,%fp),%a0
lea (8,%a0),%a1
.balignw 4,0x2048
2:
move.l %d0,%d2
add.l %d2,%d2
move.l %d2,%d0
lsl.l #2,%d0
add.l %d0,%d2
move.w (6,%a1),%d0
ext.l %d0
subq.l #2,%a1
add.l %d0,%d2
move.l %d2,%d0
dbf %d1,2b
subi.l #0x457,%d0
move.l (-0x10,%fp),%d2
unlk %fp
rts
.globl SOF_$DOWNREC
SOF_$DOWNREC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
pea (0x54,%a5)
bsr.s SOF_$DOWNREC_IN
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SOF_$PTRSUM_IN
SOF_$PTRSUM_IN:
link.w %fp,#-0xc
movem.l %d2-%d3,-(%sp)
movea.l (8,%fp),%a0
clr.l %d0
moveq #4,%d1
lea (6,%a0),%a1
.balignw 4,0x2048
2:
move.w (0xa,%a1),%d2
ext.l %d2
move.l %d2,%d3
add.l %d3,%d3
addq.l #6,%a1
add.l %d3,%d2
add.l %d2,%d0
dbf %d1,2b
movem.l (-0x14,%fp),%d2-%d3
unlk %fp
rts
.globl SOF_$PTRSUM
SOF_$PTRSUM:
link.w %fp,#-4
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
pea (0x54,%a5)
bsr.s SOF_$PTRSUM_IN
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SOF.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 148
