.text
.globl SETS_$SMALL
SETS_$SMALL:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
clr.w (%a5)
clr.w %d2
bset %d0,%d2
or.w %d2,(%a5)
clr.w %d2
bset %d1,%d2
or.w %d2,(%a5)
bset #1,(1,%a5)
clr.w %d1
bset %d0,%d1
not.w %d1
and.w (%a5),%d1
move.w %d1,(8,%a5)
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
bset #1,(9,%a5)
pea (.Lc0,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
moveq #6,%d0
and.w (%a5),%d0
move.w %d0,(8,%a5)
pea (.Lc0,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
bclr #1,(9,%a5)
pea (.Lc0,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
btst #3,(1,%a5)
beq.s 1f
st (0x1a,%a5)
bra.s 2f
1:
clr.b (0x1a,%a5)
2:
pea (0x1a,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
btst #4,(1,%a5)
beq.s 3f
st (0x1a,%a5)
bra.s 4f
3:
clr.b (0x1a,%a5)
4:
pea (0x1a,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w (%a5)
beq.s 5f
st (0x1a,%a5)
bra.s 6f
5:
clr.b (0x1a,%a5)
6:
pea (0x1a,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.Lc0:
.short 0x0002
.globl SETS_$LARGE
SETS_$LARGE:
link.w %fp,#0
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w (0xc,%fp),%d4
pea (.Lc1,%pc)
pea (0xa,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
moveq #0x7f,%d0
lea (0xa,%a5),%a0
sub.w %d2,%d0
lsr.w #3,%d0
btst %d2,(0,%a0,%d0.w)
beq.s 1f
st (0x1a,%a5)
bra.s 2f
1:
clr.b (0x1a,%a5)
2:
pea (0x1a,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
moveq #0x7f,%d0
lea (0xa,%a5),%a0
sub.w %d3,%d0
lsr.w #3,%d0
btst %d3,(0,%a0,%d0.w)
beq.s 3f
st (0x1a,%a5)
bra.s 4f
3:
clr.b (0x1a,%a5)
4:
pea (0x1a,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
moveq #0x7f,%d0
lea (0xa,%a5),%a0
sub.w %d4,%d0
lsr.w #3,%d0
btst %d4,(0,%a0,%d0.w)
beq.s 5f
st (0x1a,%a5)
bra.s 6f
5:
clr.b (0x1a,%a5)
6:
pea (0x1a,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x10,%fp),%d2-%d4/%a5
unlk %fp
rts
.Lc1:
.short 0x0010
.balignw 4,0x2048
.section ".moddata.SETS","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x80,0x00,0x00,0x00,0x00,0x00
.byte 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x21
.space 2
