.text
SUM:
link.w %fp,#-8
movea.l (%fp),%a0
move.w (8,%fp),%d0
add.w (8,%a0),%d0
add.w (-2,%a0),%d0
unlk %fp
rts
PUT:
link.w %fp,#-4
movea.l (%fp),%a0
move.w #0x64,(-2,%a0)
unlk %fp
rts
.globl NESTED_$UP
NESTED_$UP:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xa,%fp),%a2
move.w (%a2),(-2,%fp)
subq.l #2,%sp
move.w #1,-(%sp)
bsr.s SUM
addq.w #4,%sp
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
bsr.s PUT
subq.l #2,%sp
clr.w -(%sp)
bsr.s SUM
addq.w #4,%sp
move.w %d0,(-2,%fp)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w #3,-(%sp)
bsr.s SUM
move.w %d0,(%a2)
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
MID:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
movea.l %a1,%a2
moveq #1,%d2
move.w (-2,%a2),%d0
add.w %d2,%d0
move.w %d0,(-2,%a2)
subq.l #2,%sp
move.w (-2,%a2),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,%d0
add.w (-2,%a2),%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
SIB:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
move.w (-2,%a2),%d0
add.w %d0,%d0
move.w %d0,(-2,%a2)
movea.l %a2,%a1
bsr.s MID
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl NESTED_$TWO
NESTED_$TWO:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
movea.l %fp,%a1
bsr.s MID
bsr.s SIB
unlk %fp
rts
WALK:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l %a1,%a2
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
tst.w %d2
ble.s 1f
move.w (-2,%a2),%d0
add.w %d2,%d0
move.w %d0,(-2,%a2)
subq.l #2,%sp
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,-(%sp)
movea.l %a2,%a1
bsr.s WALK
addq.w #1,(-4,%a2)
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl NESTED_$REC
NESTED_$REC:
link.w %fp,#-8
clr.w (-2,%fp)
jsr (EXEC_$CLOBBER).l
clr.w (-4,%fp)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
movea.l %fp,%a1
bsr.s WALK
addq.w #4,%sp
subq.l #2,%sp
move.w (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
move.w (-2,%fp),%d0
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NESTED","aw",@nobits
.balign 4
.Ldata:
.space 4
