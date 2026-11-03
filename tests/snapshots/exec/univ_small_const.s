.text
.globl CRASH
CRASH:
link.w %fp,#-4
pea (%a2)
movea.l (8,%fp),%a2
pea (%a2)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.l (%a2),(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (.Lc0,%pc)
pea (%a2)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.Lc0:
.short 0x0004
.globl TAKE_L
TAKE_L:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl UNIV_SMALL_CONST_$RUN
UNIV_SMALL_CONST_$RUN:
link.w %fp,#0
pea (.Lc1,%pc)
bsr.s CRASH
addq.w #4,%sp
pea (.Lc2,%pc)
bsr.s CRASH
addq.w #4,%sp
pea (.Lc3,%pc)
bsr.s CRASH
addq.w #4,%sp
pea (.Lc4,%pc)
bsr.s CRASH
addq.w #4,%sp
pea (.Lc5,%pc)
bsr.w CRASH
addq.w #4,%sp
pea (.Lc6,%pc)
bsr.w CRASH
addq.w #4,%sp
pea (.Lc1,%pc)
bsr.s TAKE_L
addq.w #4,%sp
pea (.Lc1,%pc)
bsr.s TAKE_L
unlk %fp
rts
.balignw 4,0x2048
.Lc1:
.short 0x0000,0x002a
.Lc6:
.short 0x0001,0x2345
.Lc5:
.short 0xffff,0x8000
.Lc4:
.short 0x0000,0xffff
.Lc3:
.short 0xffff,0xfffe
.Lc2:
.short 0xffff,0xffff
.balignw 4,0x2048
