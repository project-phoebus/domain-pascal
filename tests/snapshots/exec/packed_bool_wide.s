.text
.globl ONE
ONE:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,%d1
lsr.w #3,%d1
movea.l (0x180,%a5),%a0
lea (%a0),%a1
lea (0,%a1,%d1.w),%a0
moveq #7,%d1
moveq #7,%d3
and.b %d0,%d1
sub.b %d1,%d3
btst %d3,(%a0)
beq.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
clr.w -(%sp)
5:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
cmpi.w #4,%d2
sgt %d0
move.w %d2,%d1
subq.w #1,%d1
move.w %d1,%d2
lsr.w #3,%d2
movea.l (0x180,%a5),%a0
lea (%a0),%a1
lea (0,%a1,%d2.w),%a0
moveq #7,%d2
moveq #7,%d3
and.b %d1,%d2
sub.b %d2,%d3
btst %d3,(%a0)
sne %d1
and.b %d1,%d0
move.b %d0,(0x184,%a5)
tst.b (0x184,%a5)
bpl.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 6f
3:
subq.l #2,%sp
clr.w -(%sp)
6:
jsr (EXEC_$LOGV_W).l
4:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl PACKED_BOOL_WIDE_$RUN
PACKED_BOOL_WIDE_$RUN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0xe,%fp),%d2
lea (%a5),%a0
move.l %a0,(0x180,%a5)
bset #7,(%a5)
bset #0,(0xf9,%a5)
bset #0,(0xff,%a5)
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
move.w %d2,-(%sp)
bsr.w ONE
addq.w #4,%sp
bset #7,(0x100,%a5)
bclr #6,(0x101,%a5)
tst.b (0x100,%a5)
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 15f
1:
subq.l #2,%sp
clr.w -(%sp)
15:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
btst #6,(0x101,%a5)
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 16f
3:
subq.l #2,%sp
clr.w -(%sp)
16:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
4:
btst #0,(0xf9,%a5)
sne %d0
move.b %d0,(0x184,%a5)
tst.b (0x184,%a5)
bpl.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 17f
5:
subq.l #2,%sp
clr.w -(%sp)
17:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
6:
bset #6,(0x101,%a5)
btst #6,(0x101,%a5)
sne %d0
btst #5,(0x101,%a5)
seq %d1
and.b %d1,%d0
bpl.s 7f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 18f
7:
subq.l #2,%sp
clr.w -(%sp)
18:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
8:
btst #1,(0xff,%a5)
beq.s 9f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 19f
9:
subq.l #2,%sp
clr.w -(%sp)
19:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
10:
move.w #0x800,%d2
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,%d1
lsr.w #3,%d1
movea.l (0x180,%a5),%a0
lea (%a0),%a1
lea (0,%a1,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(%a0)
beq.s 11f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 20f
11:
subq.l #2,%sp
clr.w -(%sp)
20:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
12:
move.w #0x7fe,%d2
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,%d1
lsr.w #3,%d1
movea.l (0x180,%a5),%a0
lea (%a0),%a1
lea (0,%a1,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(%a0)
beq.s 13f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 21f
13:
subq.l #2,%sp
clr.w -(%sp)
21:
jsr (EXEC_$LOGV_W).l
14:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PACKED_BOOL_WIDE","aw",@nobits
.balign 4
.Ldata:
.space 396
