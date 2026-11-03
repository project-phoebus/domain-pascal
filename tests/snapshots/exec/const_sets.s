.text
.globl INT_IN
INT_IN:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x7e,%d1
btst %d0,%d1
sne %d1
move.b %d1,%d0
unlk %fp
rts
.globl UNS_IN
UNS_IN:
link.w %fp,#-4
move.l %d2,-(%sp)
clr.b %d0
movea.l (8,%fp),%a0
clr.l %d1
move.l #0x7fe,%d2
move.w (%a0),%d1
btst %d1,%d2
beq.s 1f
st %d0
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl ENUM_IN
ENUM_IN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x7a,%d1
btst %d0,%d1
beq.s 1f
st (%a5)
bra.s 2f
1:
clr.b (%a5)
2:
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CONST_SETS_$RUN
CONST_SETS_$RUN:
link.w %fp,#-4
movem.l %d2-%d5,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w (0xc,%fp),%d4
subq.l #2,%sp
move.w (0xe,%fp),%d5
move.w %d2,-(%sp)
bsr.w INT_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d2,(-2,%fp)
pea (-2,%fp)
bsr.w UNS_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d3,-(%sp)
bsr.w INT_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d3,(-2,%fp)
pea (-2,%fp)
bsr.w UNS_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d4,-(%sp)
bsr.w INT_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d4,(-2,%fp)
pea (-2,%fp)
bsr.w UNS_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d5,-(%sp)
bsr.w INT_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d5,(-2,%fp)
pea (-2,%fp)
bsr.w UNS_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x14,%fp),%d2-%d5
unlk %fp
rts
.globl CONST_SETS_$ENUM
CONST_SETS_$ENUM:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w ENUM_IN
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
bsr.w ENUM_IN
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bsr.w ENUM_IN
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w ENUM_IN
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CONST_SETS","aw",@nobits
.balign 4
.Ldata:
.space 4
