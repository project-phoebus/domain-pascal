.text
.globl GET_ROOT
GET_ROOT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SET_NODE
SET_NODE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(8,%a5)
move.l (%a0)+,(0xc,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PUT_CLOCK
PUT_CLOCK:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0),(%a1)
move.w (4,%a0),(4,%a1)
unlk %fp
rts
.globl SC_$MAIN
SC_$MAIN:
link.w %fp,#-0x28
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d0
ext.l %d0
move.l %d0,(%a5)
move.l %d0,%d1
neg.l %d1
move.l %d1,(4,%a5)
pea (-8,%fp)
bsr.s GET_ROOT
addq.w #4,%sp
pea (.Lc0,%pc)
pea (-8,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (-8,%fp)
bsr.s SET_NODE
addq.w #4,%sp
pea (.Lc0,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
lea (0x10,%a5),%a0
move.l %a0,(-0x28,%fp)
pea (-0x28,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a2
lea (8,%a5),%a0
move.l (%a0)+,(4,%a2)
move.l (%a0)+,(8,%a2)
pea (.Lc1,%pc)
pea (0x10,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
lea (4,%a2),%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
pea (.Lc0,%pc)
pea (-0x10,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.w %d2,%d0
ext.l %d0
move.l %d0,(-0x1c,%fp)
move.w #7,(-0x18,%fp)
pea (-0x1c,%fp)
pea (-0x24,%fp)
bsr.w PUT_CLOCK
addq.w #8,%sp
pea (.Lc2,%pc)
pea (-0x24,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.l (-0x24,%fp),(0x22,%a5)
move.w (-0x20,%fp),(0x26,%a5)
pea (.Lc3,%pc)
pea (0x20,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.l (-0x10,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x34,%fp),%d2/%a2/%a5
unlk %fp
rts
.Lc0:
.short 0x0008
.Lc3:
.short 0x000a
.Lc2:
.short 0x0006
.Lc1:
.short 0x0010
.balignw 4,0x2048
.section ".moddata.SC","aw",@progbits
.balign 4
.Ldata:
.space 16
.byte 0x11,0x11,0x11,0x11,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x22,0x22,0x22,0x22
.byte 0x33,0x33,0x00,0x00,0x00,0x00,0x00,0x00,0x44,0x44
.space 2
