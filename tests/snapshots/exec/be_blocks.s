.text
.globl BB_$VIA
BB_$VIA:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
movea.l (4,%a0),%a0
lea (%a0),%a1
movea.l (0xc,%fp),%a0
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
unlk %fp
rts
.globl BB_$VIA12
BB_$VIA12:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
movea.l (8,%a0),%a0
lea (%a0),%a1
movea.l (0xc,%fp),%a0
move.l (%a1)+,(%a0)+
move.l (%a1)+,(%a0)+
move.l (%a1)+,(%a0)+
unlk %fp
rts
.globl BB_$VIA_T
BB_$VIA_T:
link.w %fp,#-0x44
move.l #0x457,(-8,%fp)
move.l #0x8ae,(-4,%fp)
move.l #0xc3b,(-0x18,%fp)
move.l #0xca0,(-0x14,%fp)
move.l #0xd05,(-0x10,%fp)
lea (-8,%fp),%a0
move.l %a0,(-0x24,%fp)
lea (-0x18,%fp),%a0
move.l %a0,(-0x20,%fp)
pea (-0x30,%fp)
lea (-0x28,%fp),%a0
move.l %a0,(-0x44,%fp)
pea (-0x44,%fp)
bsr.w BB_$VIA
addq.w #8,%sp
pea (-0x30,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x2c,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x40,%fp)
lea (-0x28,%fp),%a0
move.l %a0,(-0x44,%fp)
pea (-0x44,%fp)
bsr.w BB_$VIA12
addq.w #8,%sp
pea (-0x40,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x3c,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x38,%fp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl BB_$INTOIDX
BB_$INTOIDX:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d0
lsl.w #3,%d0
movea.l (0xc,%fp),%a0
move.l (%a0)+,(-8,%a5,%d0.w)
move.l (%a0)+,(-4,%a5,%d0.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BB_$INTOIDX_T
BB_$INTOIDX_T:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l #0x1e61,(-8,%fp)
move.l #0x22b8,(-4,%fp)
pea (-8,%fp)
pea (8,%fp)
bsr.s BB_$INTOIDX
addq.w #8,%sp
move.w %d2,%d0
lsl.w #3,%d0
pea (-8,%a5,%d0.w)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w %d2,%d0
lsl.w #3,%d0
pea (-4,%a5,%d0.w)
jsr (EXEC_$LOG_L).l
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl BB_$FIELD
BB_$FIELD:
link.w %fp,#0
lea (BB_$OREC+4).l,%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl BB_$FIELD12
BB_$FIELD12:
link.w %fp,#0
lea (BB_$OREC+0xc).l,%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
unlk %fp
rts
.globl BB_$FIELD_T
BB_$FIELD_T:
link.w %fp,#-0x18
pea (-8,%fp)
bsr.s BB_$FIELD
addq.w #4,%sp
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x18,%fp)
bsr.s BB_$FIELD12
addq.w #4,%sp
pea (-0x18,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x14,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x10,%fp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl BB_$ELEM8
BB_$ELEM8:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (%a0),%d0
movea.l #BB_$OARR,%a0
lsl.w #3,%d0
lea (-8,%a0,%d0.w),%a0
movea.l (0xc,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl BB_$ELEM12
BB_$ELEM12:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (%a0),%d0
movea.l #BB_$OARR12,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
movea.l (0xc,%fp),%a1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a0
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
unlk %fp
rts
.globl BB_$ELEM_T
BB_$ELEM_T:
link.w %fp,#-0x18
pea (-8,%fp)
pea (8,%fp)
bsr.s BB_$ELEM8
addq.w #8,%sp
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x18,%fp)
pea (8,%fp)
bsr.s BB_$ELEM12
addq.w #8,%sp
pea (-0x18,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x14,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x10,%fp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl BB_$BOTH
BB_$BOTH:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
move.w (%a0),%d0
movea.l #BB_$OARR12,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
movea.l (8,%fp),%a1
add.w %d1,%d0
lea (-0xc,%a0,%d0.w),%a0
move.w (%a1),%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
lea (0x14,%a5,%d0.w),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BB_$BOTH_T
BB_$BOTH_T:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
pea (0xa,%fp)
pea (8,%fp)
bsr.s BB_$BOTH
addq.w #8,%sp
move.w %d2,%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (0x14,%a5,%d0.w)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w %d2,%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (0x18,%a5,%d0.w)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w %d2,%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (0x1c,%a5,%d0.w)
jsr (EXEC_$LOG_L).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BB_$COMPUTED
BB_$COMPUTED:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (%a0),%d0
movea.l #BB_$OARR12,%a0
add.w %d0,%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
movea.l (0xc,%fp),%a1
add.w %d1,%d0
lea (-0x18,%a0,%d0.w),%a0
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
unlk %fp
rts
.globl BB_$UNROLLED
BB_$UNROLLED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
lea (0x14,%a5,%d1.w),%a0
movea.l (0xa,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BB_$COMPUTED_T
BB_$COMPUTED_T:
link.w %fp,#-0x20
pea (-0x10,%fp)
pea (8,%fp)
bsr.s BB_$COMPUTED
addq.w #8,%sp
pea (-0x10,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0xc,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
subq.l #2,%sp
pea (-0x20,%fp)
move.w #1,-(%sp)
bsr.s BB_$UNROLLED
addq.w #8,%sp
pea (-0x20,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x1c,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x18,%fp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl BB_$SLOT
BB_$SLOT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
movea.l (%a0),%a0
movea.l (8,%fp),%a1
move.w (%a1),%d0
lsl.w #3,%d0
lea (-8,%a5,%d0.w),%a1
move.l (%a1)+,(4,%a0)
move.l (%a1)+,(8,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BB_$SLOT_T
BB_$SLOT_T:
link.w %fp,#-0x1c
move.l #-0x3e8,(-0x18,%fp)
move.l #0x1388,(-0x14,%fp)
move.l #0x1770,(-0x10,%fp)
move.l #-0x7d0,(-0xc,%fp)
lea (-0x18,%fp),%a0
move.l %a0,(-0x1c,%fp)
pea (-0x1c,%fp)
pea (8,%fp)
bsr.s BB_$SLOT
addq.w #8,%sp
pea (-0x18,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x14,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x10,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0xc,%fp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_BLOCKS","aw",@nobits
.balign 4
.Ldata:
.space 80
