.text
MAKE:
link.w %fp,#-0x10
move.w (0xc,%fp),%d0
move.l #0x12345678,(-0x10,%fp)
move.w %d0,%d1
ext.l %d1
move.l %d1,(-0xc,%fp)
lea (-0x10,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl CHOOSE
CHOOSE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
cmpi.w #1,(%a1)
bne.s 1f
lea (0x14,%a5),%a1
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
bra.s 2f
1:
cmpi.w #3,(%a1)
bne.s 3f
lea (0x1c,%a5),%a1
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
bra.s 4f
3:
tst.w (%a1)
bne.s 5f
lea (0x14,%a5),%a1
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
5:
4:
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NEG
NEG:
link.w %fp,#-8
movea.l (0xc,%fp),%a0
move.w (%a0),%d0
ext.l %d0
move.l %d0,(-8,%fp)
move.w (%a0),%d0
ext.l %d0
neg.l %d0
move.l %d0,(-4,%fp)
lea (-8,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl LOG_PAIR
LOG_PAIR:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
pea (%a2)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a2)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl LOG_TWO
LOG_TWO:
link.w %fp,#0
move.l (8,%fp),-(%sp)
bsr.s LOG_PAIR
addq.w #4,%sp
move.l (0xc,%fp),-(%sp)
bsr.s LOG_PAIR
unlk %fp
rts
.globl HIDDEN_RESULTS_$BUILD
HIDDEN_RESULTS_$BUILD:
link.w %fp,#-0x14
move.l %d2,-(%sp)
move.w (8,%fp),%d2
move.w %d2,(-0xa,%fp)
move.w %d2,-(%sp)
pea (-0x14,%fp)
bsr.w MAKE
addq.w #6,%sp
lea (-0x14,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
pea (-8,%fp)
bsr.s LOG_PAIR
move.l (-0x18,%fp),%d2
unlk %fp
rts
.globl HIDDEN_RESULTS_$PICK
HIDDEN_RESULTS_$PICK:
link.w %fp,#-0xc
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (8,%a5),%a2
pea (8,%fp)
pea (-0xc,%fp)
bsr.w CHOOSE
addq.w #8,%sp
lea (-0xc,%fp),%a0
move.l (%a0)+,(%a5)
move.l (%a0)+,(4,%a5)
pea (%a5)
bsr.w LOG_PAIR
addq.w #4,%sp
pea (.Lc0,%pc)
pea (-0xc,%fp)
bsr.w CHOOSE
addq.w #8,%sp
lea (-0xc,%fp),%a0
move.l (%a0)+,(4,%a2)
move.l (%a0)+,(8,%a2)
pea (0xc,%a5)
bsr.w LOG_PAIR
movem.l (-0x14,%fp),%a2/%a5
unlk %fp
rts
.Lc0:
.short 0x0001
.globl HIDDEN_RESULTS_$PASS
HIDDEN_RESULTS_$PASS:
link.w %fp,#-0x10
pea (0xa,%fp)
pea (-8,%fp)
bsr.w NEG
addq.w #8,%sp
pea (-8,%fp)
pea (8,%fp)
pea (-0x10,%fp)
bsr.w NEG
addq.w #8,%sp
pea (-0x10,%fp)
bsr.w LOG_TWO
unlk %fp
rts
.globl DBL
DBL:
link.w %fp,#-8
movea.l (0xc,%fp),%a0
move.l (%a0),%d0
add.l (%a0),%d0
move.l %d0,(-8,%fp)
move.w (4,%a0),%d0
add.w (4,%a0),%d0
move.w %d0,(-4,%fp)
cmpi.w #-0x8000,(4,%a0)
bcs.s 1f
addq.l #1,(-8,%fp)
1:
movea.l (8,%fp),%a1
move.l (-8,%fp),(%a1)
move.w (-4,%fp),(4,%a1)
unlk %fp
rts
.globl HIDDEN_RESULTS_$CLOCK
HIDDEN_RESULTS_$CLOCK:
link.w %fp,#-0x10
move.l (8,%fp),%d0
move.w (0xc,%fp),%d1
move.l %d0,(-8,%fp)
move.w %d1,(-4,%fp)
pea (-8,%fp)
pea (-0x10,%fp)
bsr.s DBL
addq.w #8,%sp
move.l (-0x10,%fp),(-8,%fp)
move.w (-0xc,%fp),(-4,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w (-4,%fp),(-0xa,%fp)
pea (-0xa,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (.Lc1,%pc)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.Lc1:
.short 0x0005
.balignw 4,0x2048
.section ".moddata.HIDDEN_RESULTS","aw",@progbits
.balign 4
.Ldata:
.space 23
.byte 0x11,0x00,0x00,0x00,0x22,0x00,0x00,0x00,0x33,0x00,0x00,0x00,0x44
