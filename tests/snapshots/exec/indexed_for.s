.text
.globl IXF_$INIT
IXF_$INIT:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d0
moveq #0x65,%d1
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d2
add.w %d2,%d2
move.w %d0,(-2,%a5,%d2.w)
move.w %d0,%d2
add.w %d2,%d2
move.w %d1,(0x12,%a5,%d2.w)
addq.w #1,%d0
addq.w #1,%d1
2:
cmpi.w #0xa,%d0
ble.s 1b
moveq #1,%d0
moveq #0x61,%d1
bra.s 5f
.balignw 4,0x2048
4:
move.w %d0,%d2
lsl.w #2,%d2
clr.l (0x24,%a5,%d2.w)
move.w %d0,%d2
lsl.w #2,%d2
clr.l (0x5c,%a5,%d2.w)
lea (0,%a5,%d0.w),%a0
move.b %d1,(0x83,%a0)
addq.w #1,%d0
addq.w #1,%d1
5:
cmpi.w #6,%d0
ble.s 4b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl IXF_$PLUS1
IXF_$PLUS1:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.s IXF_$INIT
moveq #8,%d0
lea (2,%a5),%a0
lea (2,%a5),%a1
.balignw 4,0x2048
2:
move.w (0x12,%a0),%d1
addq.w #1,%d1
move.w %d1,(0,%a1)
addq.l #2,%a1
addq.l #2,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0014
.globl IXF_$SHIFT
IXF_$SHIFT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
move.w #0x32,(%a5)
moveq #8,%d0
lea (2,%a5),%a0
.balignw 4,0x2048
2:
move.w (-2,%a0),%d1
addq.w #1,%d1
move.w %d1,(0,%a0)
addq.l #2,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IXF_$MINUS1
IXF_$MINUS1:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #8,%d0
lea (4,%a5),%a0
lea (4,%a5),%a1
.balignw 4,0x2048
2:
move.w (0x12,%a0),(-4,%a1)
addq.l #2,%a1
addq.l #2,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IXF_$TIMES2
IXF_$TIMES2:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #2,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d1,%d2
move.w %d1,%d3
add.w %d2,%d2
muls.w #0x3e8,%d3
lsl.w #2,%d2
move.l %d3,(0x24,%a5,%d2.w)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc1,%pc)
pea (0x28,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc1:
.short 0x0018
.globl IXF_$ROWS
IXF_$ROWS:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #3,%d0
moveq #2,%d1
moveq #1,%d2
lea (8,%a5),%a0
.balignw 4,0x2048
2:
move.w %d2,%d3
add.w %d0,%d3
move.w %d3,(0x3a,%a0)
add.w %d3,%d3
move.w %d3,(0x40,%a0)
addq.w #1,%d2
addq.l #8,%a0
dbf %d1,2b
pea (.Lc2,%pc)
pea (0x40,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc2:
.short 0x0020
.globl IXF_$PTRCHG_IN
IXF_$PTRCHG_IN:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (%a5),%a0
moveq #2,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d1,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.w %d1,%d3
add.w %d3,%d3
move.w %d2,(-2,%a0,%d3.w)
lea (0x14,%a5),%a0
addq.w #1,%d1
dbf %d0,2b
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl IXF_$PTRCHG
IXF_$PTRCHG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
bsr.s IXF_$PTRCHG_IN
pea (.Lc3,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc3,%pc)
pea (0x14,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc3:
.short 0x0006
.globl IXF_$WIDE_IN
IXF_$WIDE_IN:
link.w %fp,#-8
movem.l %d2-%d3/%a2,-(%sp)
movea.l (8,%fp),%a2
move.l #0x3e8,%d0
move.l %d0,(%a2)
moveq #3,%d1
moveq #2,%d2
.balignw 4,0x2048
2:
move.w %d2,%d3
ext.l %d3
add.l %d3,%d0
move.w %d2,%d3
ext.l %d3
lsl.l #2,%d3
move.l %d0,(-4,%a2,%d3.l)
addq.w #1,%d2
dbf %d1,2b
moveq #3,%d2
moveq #5,%d3
.balignw 4,0x2048
5:
move.w %d3,%d0
ext.l %d0
lsl.l #2,%d0
move.l (-4,%a2,%d0.l),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.w #1,%d3
dbf %d2,5b
movem.l (-0x14,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl IXF_$WIDE
IXF_$WIDE:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
lea (0x60,%a5),%a2
move.l %a2,(-4,%fp)
pea (%a2)
bsr.s IXF_$WIDE_IN
addq.w #4,%sp
pea (.Lc1,%pc)
pea (0x60,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl IXF_$CALLIDX
IXF_$CALLIDX:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #2,%d2
lea (2,%a5),%a2
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w (0,%a2),%d0
add.w %d0,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
addq.l #2,%a2
dbf %d2,2b
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl IXF_$EXITAFTER
IXF_$EXITAFTER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
clr.w (0xa,%a5)
moveq #8,%d1
moveq #1,%d0
lea (2,%a5),%a0
.balignw 4,0x2048
2:
tst.w (0,%a0)
beq.s 1f
addq.w #1,%d0
addq.l #2,%a0
dbf %d1,2b
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IXF_$TWO
IXF_$TWO:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #1,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w %d1,%d2
add.w %d2,%d2
move.w %d1,%d3
sub.w %d1,%d2
add.w %d3,%d3
add.w %d2,%d2
move.w (0x12,%a5,%d3.w),(0,%a5,%d2.w)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc4,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc4:
.short 0x0008
.globl IXF_$FAR
IXF_$FAR:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #4,%d0
movea.l %a5,%a0
move.w #0xc8,%d1
lea (0x190,%a0),%a0
.balignw 4,0x2048
2:
move.w %d1,%d2
subi.w #0xbe,%d2
move.w %d2,(-0x116,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
pea (.Lc5,%pc)
pea (0x78,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.Lc5:
.short 0x000c
.globl IXF_$FILLW
IXF_$FILLW:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #5,%d0
lea (4,%a5),%a0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d0,1b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IXF_$FILLFRAME
IXF_$FILLFRAME:
link.w %fp,#-0x18
move.l %d2,-(%sp)
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
moveq #-1,%d2
lsl.w #2,%d1
move.l %d2,(-0x1c,%fp,%d1.w)
addq.w #1,%d0
2:
cmpi.w #5,%d0
ble.s 1b
moveq #2,%d0
lea (-0x14,%fp),%a0
.balignw 4,0x2048
4:
clr.l (%a0)+
dbf %d0,4b
pea (.Lc0,%pc)
pea (-0x18,%fp)
jsr (EXEC_$LOG_MEM).l
move.l (-0x1c,%fp),%d2
unlk %fp
rts
.globl IXF_$FILLPTR
IXF_$FILLPTR:
link.w %fp,#-4
moveq #6,%d0
movea.l (8,%fp),%a0
lea (6,%a0),%a1
.balignw 4,0x2048
1:
clr.w (%a1)+
dbf %d0,1b
unlk %fp
rts
.globl IXF_$FILLPTR_RUN
IXF_$FILLPTR_RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
pea (0x14,%a5)
bsr.s IXF_$FILLPTR
addq.w #4,%sp
pea (.Lc0,%pc)
pea (0x14,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl IXF_$FILLVAR
IXF_$FILLVAR:
link.w %fp,#-4
moveq #4,%d0
movea.l (8,%fp),%a0
lea (%a0),%a1
.balignw 4,0x2048
1:
clr.w (%a1)+
dbf %d0,1b
unlk %fp
rts
.globl IXF_$FILLVAR_RUN
IXF_$FILLVAR_RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
pea (0x14,%a5)
bsr.s IXF_$FILLVAR
addq.w #4,%sp
pea (.Lc0,%pc)
pea (0x14,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl IXF_$FILLEXT
IXF_$FILLEXT:
link.w %fp,#-4
jsr (IXF_$XRESET).l
moveq #2,%d0
movea.l #IXF_$XL+8,%a0
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d0,1b
pea (.Lc1,%pc)
move.l #IXF_$XL,-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.globl IXF_$FILLC
IXF_$FILLC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #3,%d0
lea (0x85,%a5),%a0
.balignw 4,0x2048
1:
clr.b (%a0)+
dbf %d0,1b
pea (.Lc3,%pc)
pea (0x84,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IXF_$ADDRS
IXF_$ADDRS:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.w IXF_$INIT
moveq #2,%d0
moveq #1,%d1
lea (4,%a5),%a0
.balignw 4,0x2048
2:
lea (0x24,%a0),%a1
move.l %a1,(0x90,%a5)
move.w %d1,%d2
ext.l %d2
move.l %d2,%d3
neg.l %d2
lsl.l #3,%d3
add.l %d3,%d2
move.l %d2,(%a1)
addq.w #1,%d1
addq.l #4,%a0
dbf %d0,2b
pea (.Lc6,%pc)
pea (0x28,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc6:
.short 0x0010
.balignw 4,0x2048
.section ".bss.moddata.IXF","aw",@nobits
.balign 4
.Ldata:
.space 148
