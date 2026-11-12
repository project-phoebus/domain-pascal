.text
.globl ONE
ONE:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
moveq #0x6f,%d0
lea (%a5),%a0
sub.w %d2,%d0
lsr.w #3,%d0
btst %d2,(0,%a0,%d0.w)
sne %d0
moveq #0x6f,%d1
moveq #0,%d3
ext.l %d1
move.w %d2,%d3
sub.l %d3,%d1
bcs.s 2f
lsr.w #3,%d1
lea (.Lc0,%pc),%a0
btst %d3,(0,%a0,%d1.w)
2:
sgt %d1
or.b %d1,%d0
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 11f
1:
subq.l #2,%sp
clr.w -(%sp)
11:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
3:
moveq #0x6f,%d0
lea (%a5),%a0
sub.w %d2,%d0
lsr.w #3,%d0
btst %d2,(0,%a0,%d0.w)
sne %d0
moveq #0x6f,%d1
lea (0xe,%a5),%a0
sub.w %d2,%d1
lsr.w #3,%d1
btst %d2,(0,%a0,%d1.w)
sne %d1
and.b %d1,%d0
bpl.s 4f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 12f
4:
subq.l #2,%sp
clr.w -(%sp)
12:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
5:
moveq #0x6f,%d0
lea (%a5),%a0
sub.w %d2,%d0
lsr.w #3,%d0
btst %d2,(0,%a0,%d0.w)
sne %d0
moveq #0x6f,%d1
lea (0xe,%a5),%a0
sub.w %d2,%d1
lsr.w #3,%d1
btst %d2,(0,%a0,%d1.w)
seq %d1
and.b %d1,%d0
bpl.s 6f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 13f
6:
subq.l #2,%sp
clr.w -(%sp)
13:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
7:
moveq #0x6f,%d0
lea (0xe,%a5),%a0
sub.w %d2,%d0
lsr.w #3,%d0
btst %d2,(0,%a0,%d0.w)
sne %d0
moveq #0x4f,%d1
moveq #0,%d3
ext.l %d1
move.w %d2,%d3
sub.l %d3,%d1
bcs.s 8f
lsr.w #3,%d1
lea (.Lc1,%pc),%a0
btst %d3,(0,%a0,%d1.w)
8:
sgt %d1
or.b %d1,%d0
not.b %d0
move.b %d0,(0x1c,%a5)
tst.b (0x1c,%a5)
bpl.s 9f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 14f
9:
subq.l #2,%sp
clr.w -(%sp)
14:
jsr (EXEC_$LOGV_W).l
10:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.Lc0:
.short 0x03f0,0x0000,0x0000,0x0000,0x0100,0x0000,0x0000
.Lc1:
.short 0x0040,0x0000,0x0000,0x0000,0x0000
.globl IN_SET_EXPR_$RUN
IN_SET_EXPR_$RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
move.l #0x400000,(4,%a5)
clr.l (8,%a5)
move.w #8,(0xc,%a5)
move.l #0x100000,(0xe,%a5)
clr.l (0x12,%a5)
clr.l (0x16,%a5)
move.w #8,(0x1a,%a5)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w ONE
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.IN_SET_EXPR","aw",@nobits
.balign 4
.Ldata:
.space 32
