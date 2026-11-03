.text
.globl USED
USED:
link.w %fp,#-4
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
bra.s 2f
.balignw 4,0x2048
1:
subq.w #1,%d0
2:
tst.w %d0
sgt %d1
cmpi.b #0x20,(-1,%a0,%d0.w)
seq %d2
and.b %d2,%d1
bmi.s 1b
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TAKE_NAME
TAKE_NAME:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
pea (0xc,%fp)
pea (%a2)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
subq.l #2,%sp
subq.l #2,%sp
move.w #0x10,-(%sp)
pea (%a2)
bsr.s USED
addq.w #8,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl TAKE_LONG
TAKE_LONG:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
pea (.Lc0,%pc)
pea (%a2)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
subq.l #2,%sp
subq.l #2,%sp
move.w #0xc8,-(%sp)
pea (%a2)
bsr.w USED
addq.w #8,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.Lc0:
.short 0x0010
.globl TAKE_ODD
TAKE_ODD:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
pea (.Lc1,%pc)
pea (%a2)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
subq.l #2,%sp
subq.l #2,%sp
move.w #5,-(%sp)
pea (%a2)
bsr.w USED
addq.w #8,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.Lc1:
.short 0x0005
.globl TAKE_TAIL
TAKE_TAIL:
link.w %fp,#-0x104
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
moveq #0xf,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
lea (0,%a2,%d1.w),%a0
lea (0,%fp,%d1.w),%a1
move.b (0xb7,%a0),(-0x101,%a1)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc0,%pc)
pea (-0x100,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
subq.l #2,%sp
move.w #0xc8,%d2
subq.l #2,%sp
move.w #0xc8,-(%sp)
pea (%a2)
bsr.w USED
addq.w #8,%sp
sub.w %d0,%d2
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x10c,%fp),%d2/%a2
unlk %fp
rts
.globl STR_CONST_ARG_$RUN
STR_CONST_ARG_$RUN:
link.w %fp,#0
subq.l #2,%sp
move.w #0x10,-(%sp)
pea (.Lc2,%pc)
bsr.w TAKE_NAME
addq.w #8,%sp
subq.l #2,%sp
move.w #0x10,-(%sp)
pea (.Lc2,%pc)
bsr.w TAKE_NAME
addq.w #8,%sp
pea (.Lc3,%pc)
bsr.w TAKE_LONG
addq.w #4,%sp
pea (.Lc4,%pc)
bsr.w TAKE_ODD
addq.w #4,%sp
pea (.Lc3,%pc)
bsr.w TAKE_TAIL
addq.w #4,%sp
pea (.Lc5,%pc)
bsr.w TAKE_ODD
addq.w #4,%sp
pea (.Lc6,%pc)
bsr.w TAKE_ODD
unlk %fp
rts
.Lc4:
.short 0x6162,0x6320,0x2000
.Lc6:
.short 0x6162,0x6364,0x6500
.Lc5:
.short 0x7879,0x2020,0x2000
.balignw 4,0x2048
.Lc2:
.short 0x6273,0x636f,0x6d2f,0x7262,0x616b,0x5f73,0x6865,0x6c6c
.Lc3:
.short 0x7368,0x6f72,0x7420,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.balignw 4,0x2048
