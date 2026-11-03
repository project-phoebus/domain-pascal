.text
SET_SCALES:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a1),(%a0)
movea.l (0x10,%fp),%a2
move.w (%a2),(2,%a0)
movea.l (0x14,%fp),%a3
move.w (%a3),(4,%a0)
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
SET_MODE:
link.w %fp,#-4
movea.l (8,%fp),%a0
clr.w (%a0)
movea.l (0xc,%fp),%a1
move.w (%a1),(2,%a0)
unlk %fp
rts
.globl ADJACENT_STORES_$SCALES
ADJACENT_STORES_$SCALES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x10,%fp),-(%sp)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
pea (%a5)
bsr.s SET_SCALES
lea (0x10,%sp),%sp
pea (%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (2,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ADJACENT_STORES_$MODE
ADJACENT_STORES_$MODE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),-(%sp)
pea (%a5)
bsr.s SET_MODE
addq.w #8,%sp
pea (%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (2,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ADJACENT_STORES_$ODD_BYTES
ADJACENT_STORES_$ODD_BYTES:
link.w %fp,#-4
clr.b (-4,%fp)
jsr (EXEC_$CLOBBER).l
move.b #0x61,(-3,%fp)
move.b #0x62,(-2,%fp)
jsr (EXEC_$CLOBBER).l
clr.b (-1,%fp)
pea (.Lc0,%pc)
pea (-4,%fp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc0:
.short 0x0004
.balignw 4,0x2048
.section ".bss.moddata.ADJACENT_STORES","aw",@nobits
.balign 4
.Ldata:
.space 8
