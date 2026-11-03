.text
.globl MASKED
MASKED:
link.w %fp,#-4
clr.b %d0
movea.l (8,%fp),%a0
moveq #0xb,%d1
and.w (2,%a0),%d1
cmpi.w #8,%d1
bne.s 1f
st %d0
1:
unlk %fp
rts
.globl MASKED_VALUE
MASKED_VALUE:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w #-0x7ffc,%d0
and.w (%a0),%d0
cmpi.w #4,%d0
seq %d0
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d2
jsr (EXEC_$CLOBBER).l
cmpi.w #0x14,%d2
bne.s 1f
st (%a5)
bra.s 2f
1:
clr.b (%a5)
2:
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl TWO
TWO:
link.w %fp,#0
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
moveq #1,%d0
moveq #1,%d1
and.l (%a2),%d0
and.l (%a3),%d1
cmp.l %d1,%d0
beq.s 1f
st (%a5)
bra.s 2f
1:
clr.b (%a5)
2:
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.l (%a2),%d0
cmp.l (%a3),%d0
bne.s 3f
st (%a5)
bra.s 4f
3:
clr.b (%a5)
4:
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0xc,%fp),%a2-%a3/%a5
unlk %fp
rts
.globl SET_COMPARES_$RUN
SET_COMPARES_$RUN:
link.w %fp,#-0xc
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.l (0xc,%fp),%d4
move.l (0x10,%fp),%d5
move.w %d2,(-2,%fp)
pea (-4,%fp)
bsr.w MASKED
addq.w #4,%sp
move.b %d0,(-6,%fp)
pea (-6,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d3,(-8,%fp)
pea (-8,%fp)
bsr.w MASKED_VALUE
addq.w #4,%sp
move.b %d0,(-6,%fp)
pea (-6,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d3,(-6,%fp)
pea (-6,%fp)
bsr.w HELD
addq.w #4,%sp
move.l %d5,(-8,%fp)
pea (-8,%fp)
move.l %d4,(-0xc,%fp)
pea (-0xc,%fp)
bsr.w TWO
addq.w #8,%sp
cmp.w %d3,%d2
beq.s 1f
st (%a5)
bra.s 2f
1:
clr.b (%a5)
2:
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x20,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl WORD_CELL
WORD_CELL:
link.w %fp,#0
pea (.Lc0,%pc)
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc0:
.short 0x0002
.globl LONG_CELL
LONG_CELL:
link.w %fp,#0
pea (.Lc1,%pc)
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc1:
.short 0x0004
.globl SET_COMPARES_$CONSTS
SET_COMPARES_$CONSTS:
link.w %fp,#0
pea (.Lc2,%pc)
bsr.s WORD_CELL
addq.w #4,%sp
pea (.Lc3,%pc)
bsr.s WORD_CELL
addq.w #4,%sp
pea (.Lc4,%pc)
bsr.s WORD_CELL
addq.w #4,%sp
pea (.Lc5,%pc)
bsr.s LONG_CELL
addq.w #4,%sp
pea (.Lc6,%pc)
bsr.s LONG_CELL
addq.w #4,%sp
pea (.Lc7,%pc)
bsr.s LONG_CELL
addq.w #4,%sp
pea (.Lc2,%pc)
bsr.s WORD_CELL
unlk %fp
rts
.Lc2:
.short 0x0001
.Lc4:
.short 0x0008
.Lc3:
.short 0x0000
.balignw 4,0x2048
.Lc5:
.short 0x0000,0x0001
.Lc7:
.short 0x0000,0x0010
.Lc6:
.short 0x8000,0x0000
.balignw 4,0x2048
.section ".bss.moddata.SET_COMPARES","aw",@nobits
.balign 4
.Ldata:
.space 4
