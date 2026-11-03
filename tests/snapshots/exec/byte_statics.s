.text
.globl COUNT
COUNT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w %d0
move.b (0x10,%a5),%d0
addq.w #1,%d0
move.b %d0,(0x10,%a5)
clr.w %d0
move.b (0x10,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BYTE_STATICS_$MAIN
BYTE_STATICS_$MAIN:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d2
move.b (0xa,%fp),%d3
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.b %d2,(8,%a5)
move.b %d3,(0xa,%a5)
pea (.Lc0,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
bsr.s COUNT
move.w %d0,(6,%a5)
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.b (8,%a5),-(%sp)
jsr (EXEC_$LOGV_C).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc0:
.short 0x0008
.balignw 4,0x2048
.section ".moddata.BYTE_STATICS","aw",@progbits
.balign 4
.Ldata:
.byte 0x70,0x00,0x60,0x00,0x00,0x0e,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0e
.space 6
