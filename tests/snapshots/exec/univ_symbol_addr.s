.text
.globl USA_$CB
USA_$CB:
rts
.globl TAKE_BIG
TAKE_BIG:
link.w %fp,#0
pea (%a2)
subq.l #2,%sp
movea.l (0xa,%fp),%a2
move.w (8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
movea.l (%a2),%a0
move.w (%a0),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
movea.l (%a2),%a0
move.w (2,%a0),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl TAKE_PTRS
TAKE_PTRS:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a1
move.l (%a1),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
movea.l (0xc,%fp),%a0
movea.l (%a0),%a1
move.w (%a1),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
movea.l (0x10,%fp),%a0
movea.l (%a0),%a1
move.w (%a1),-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.globl TAKE_INT
TAKE_INT:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #USA_$CTL,%d1
move.l (%a0),%d0
cmp.l %d1,%d0
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
unlk %fp
rts
.globl TAKE_ROUTINE
TAKE_ROUTINE:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #USA_$CB,%d1
move.l (%a0),%d0
cmp.l %d1,%d0
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
unlk %fp
rts
.globl USA_$RUN
USA_$RUN:
link.w %fp,#-8
move.w #0x141,(USA_OV+0xc).l
move.w #7,-(%sp)
pea (.Lc0,%pc)
move.w #6,-(%sp)
bsr.w TAKE_BIG
addq.w #8,%sp
pea (.Lc1,%pc)
move.l (%sp),-(%sp)
pea (.Lc2,%pc)
bsr.w TAKE_PTRS
lea (0xc,%sp),%sp
pea (.Lc0,%pc)
bsr.w TAKE_INT
addq.w #4,%sp
pea (.Lc3,%pc)
bsr.s TAKE_ROUTINE
addq.w #4,%sp
lea (USA_OV+0xc).l,%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
lea (USA_OV+0xc).l,%a1
move.l %a1,(-8,%fp)
pea (-8,%fp)
pea (.Lc2,%pc)
bsr.w TAKE_PTRS
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.long USA_$CTL
.Lc3:
.long USA_$CB
.Lc2:
.long USA_$LONG
.Lc1:
.long USA_$CTL+2
.balignw 4,0x2048
.comm USA_OV,16,4
