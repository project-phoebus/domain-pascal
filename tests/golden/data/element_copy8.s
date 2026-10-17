.text
.globl GET_WDIR
GET_WDIR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
movea.l (8,%fp),%a1
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a0
lea (0x504,%a0),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl GET_NDIR
GET_NDIR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
movea.l (8,%fp),%a1
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a0
lea (0x334,%a0),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl UNLOCK
UNLOCK:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a0
lea (0x164,%a0),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
pea (-8,%fp)
jsr (UNLOCK_IT).l
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl GET_INFO
GET_INFO:
link.w %fp,#0
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
movea.l (0xe,%fp),%a2
subq.l #2,%sp
pea (%a2)
move.w %d2,-(%sp)
jsr (BAT).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (-0xc,%a5,%d0.w),%a0
movea.l (0xa,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
1:
movem.l (-0xc,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl TOUCH
TOUCH:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
move.w (0xc,%fp),%d2
pea (%a2)
jsr (SHOW).l
addq.w #4,%sp
clr.l %d0
move.w %d2,%d0
lsl.l #3,%d0
lea (-8,%a2,%d0.l),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
pea (-8,%fp)
jsr (SHOW).l
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELEMCOPY","aw",@nobits
.balign 4
.Ldata:
.space 1748
