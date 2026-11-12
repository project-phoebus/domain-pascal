.text
.globl INT_IN
INT_IN:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w #0xcf,%d0
move.w (8,%fp),%d1
moveq #0,%d2
ext.l %d0
move.w %d1,%d2
sub.l %d2,%d0
bcs.s 1f
lsr.w #3,%d0
lea (.Lc0,%pc),%a0
btst %d2,(0,%a0,%d0.w)
1:
sgt %d0
move.l (-8,%fp),%d2
unlk %fp
rts
.Lc0:
.short 0x0100,0x0000,0x0000,0x0000,0x0000,0x0000,0x0020,0x00ff,0xe000,0x0000,0x0000,0x0000,0x0000
.globl UNS_IN
UNS_IN:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
clr.b %d0
move.w #0xcf,%d1
movea.l (8,%fp),%a0
moveq #0,%d3
move.w (%a0),%d2
ext.l %d1
move.w %d2,%d3
sub.l %d3,%d1
bcs.s 2f
lsr.w #3,%d1
lea (.Lc1,%pc),%a0
btst %d3,(0,%a0,%d1.w)
2:
ble.s 1f
st %d0
1:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.Lc1:
.short 0xff00,0x0000,0x0000,0x0000,0x0000,0x0000,0x0020,0x00ff,0xe000,0x0000,0x0000,0x0000,0x0000
.globl BOTH
BOTH:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d2
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w INT_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d2,(-2,%fp)
pea (-2,%fp)
bsr.s UNS_IN
addq.w #4,%sp
move.b %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_BOOL).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl IN_WIDE_CONST_SET_$RUN
IN_WIDE_CONST_SET_$RUN:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s BOTH
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
bsr.s BOTH
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bsr.s BOTH
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.s BOTH
unlk %fp
rts
.globl IN_WIDE_CONST_SET_$UNS
IN_WIDE_CONST_SET_$UNS:
link.w %fp,#-4
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d2
move.w (0xc,%fp),%d3
move.w (0xe,%fp),%d4
move.w %d0,(-2,%fp)
pea (-2,%fp)
bsr.w UNS_IN
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
move.w %d3,(-2,%fp)
pea (-2,%fp)
bsr.w UNS_IN
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
movem.l (-0x10,%fp),%d2-%d4
unlk %fp
rts
.balignw 4,0x2048
