.text
.globl BYTES_$MAIN
BYTES_$MAIN:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d2
move.b (0xa,%fp),%d3
move.b %d2,(1,%a5)
pea (1,%a5)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
move.b %d3,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
clr.w %d0
move.b %d2,%d0
add.w %d0,%d0
move.w #0x11,(4,%a5,%d0.w)
clr.w %d0
move.b (1,%a5),%d0
add.w %d0,%d0
move.w (4,%a5,%d0.w),(0x204,%a5)
subq.l #2,%sp
move.w (0x204,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
clr.w %d0
move.b %d2,%d0
move.w %d0,(0x204,%a5)
subq.l #2,%sp
move.w (0x204,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
cmpi.w #0x7f,(0x204,%a5)
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
1:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc0:
.short 0x0004
.balignw 4,0x2048
.section ".bss.moddata.BYTES","aw",@nobits
.balign 4
.Ldata:
.space 520
