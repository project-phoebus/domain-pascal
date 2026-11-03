.text
.globl BE_PACKED_$FILL
BE_PACKED_$FILL:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.b #0x11,(%a5)
move.b #5,(1,%a5)
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.w %d2,(8,%a5)
move.b (9,%a5),(1,%a5)
clr.w (8,%a5)
clr.w %d0
move.b (1,%a5),%d0
move.w %d0,(8,%a5)
pea (8,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.b #5,(%a5)
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.b #0xff,(1,%a5)
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.Lc0:
.short 0x0002
.globl BE_PACKED_$ONEBIT
BE_PACKED_$ONEBIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
andi.b #0x80,(2,%a5)
ori.b #0x6a,(2,%a5)
ori.b #0x80,(2,%a5)
pea (.Lc1,%pc)
pea (2,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
andi.b #0x7f,(2,%a5)
pea (.Lc1,%pc)
pea (2,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc1:
.short 0x0001
.globl BE_PACKED_$COPY
BE_PACKED_$COPY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
andi.b #0xf,(4,%a5)
ori.b #0xa0,(4,%a5)
andi.b #0xf0,(4,%a5)
ori.b #3,(4,%a5)
andi.b #0xf,(6,%a5)
ori.b #0xc0,(6,%a5)
andi.b #0xf0,(6,%a5)
ori.b #0xb,(6,%a5)
moveq #0xf,%d0
and.b (4,%a5),%d0
andi.b #0xf,(6,%a5)
lsl.b #4,%d0
or.b %d0,(6,%a5)
move.w #0xf0,%d0
and.b (4,%a5),%d0
lsr.w #4,%d0
andi.b #0xf0,(6,%a5)
or.b %d0,(6,%a5)
pea (.Lc1,%pc)
pea (6,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_PACKED","aw",@nobits
.balign 4
.Ldata:
.space 12
