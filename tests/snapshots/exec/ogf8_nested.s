.text
INNER:
link.w %fp,#-8
pea (%a2)
movea.l %a1,%a0
movea.l (-4,%a0),%a1
movea.l (-4,%a0),%a2
move.w (-6,%a2),%d0
move.w (-6,%a0),%d1
ext.l %d0
ext.l %d1
add.l (-4,%a1),%d0
add.l %d1,%d0
move.w (8,%fp),%d1
ext.l %d1
add.l %d1,%d0
movea.l (-0xc,%fp),%a2
unlk %fp
rts
COUNT:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l %a1,%a2
tst.w %d2
ble.s 1f
movea.l (-4,%a2),%a0
move.w %d2,%d0
move.w %d2,%d1
add.w (8,%a0),%d0
add.w %d1,%d1
movea.l (-4,%a2),%a0
move.w %d0,(-0x12,%a0,%d1.w)
subq.l #2,%sp
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,-(%sp)
movea.l %a2,%a1
bsr.s COUNT
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
BOTH:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (%fp),%a2
move.w #3,-(%sp)
movea.l %a2,%a1
bsr.s COUNT
addq.w #4,%sp
subq.l #2,%sp
clr.w -(%sp)
movea.l %a2,%a1
bsr.w INNER
movea.l (-4,%a2),%a0
move.l %d0,%d1
movea.l (0xa,%a0),%a0
add.l (%a0),%d1
movea.l (-4,%a2),%a0
movea.l (0xa,%a0),%a0
move.l %d1,(%a0)
movea.l (-8,%fp),%a2
unlk %fp
rts
MID:
link.w %fp,#-8
pea (%a2)
move.w (8,%fp),%d0
movea.l (%fp),%a2
lea (0,%a2),%a0
move.l %a0,(-4,%fp)
move.w %d0,(-6,%fp)
subq.l #2,%sp
move.w #0xa,-(%sp)
movea.l %fp,%a1
bsr.w INNER
addq.w #4,%sp
move.l %d0,(-4,%a2)
bsr.s BOTH
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl OGF8_NESTED_$THREE
OGF8_NESTED_$THREE:
link.w %fp,#-0x14
movem.l %d2/%a2,-(%sp)
moveq #0xa,%d0
move.l %d0,(-4,%fp)
move.w (8,%fp),(-6,%fp)
moveq #3,%d0
lea (-0x10,%fp),%a0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d0,1b
subq.l #2,%sp
move.w #3,-(%sp)
bsr.s MID
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
moveq #3,%d2
lea (2,%fp),%a2
.balignw 4,0x2048
3:
subq.l #2,%sp
move.w (-0x12,%a2),%d0
addq.w #3,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
addq.l #2,%a2
dbf %d2,3b
subq.l #2,%sp
move.w (-6,%fp),%d0
addq.w #8,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x1c,%fp),%d2/%a2
unlk %fp
rts
L3:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
movea.l (%fp),%a0
moveq #0x64,%d0
movea.l (-4,%a0),%a1
movea.l (-4,%a0),%a2
movea.l (-4,%a1),%a1
movea.l (-4,%a2),%a2
muls.w (8,%a2),%d0
add.l (-4,%a1),%d0
movea.l (-4,%a0),%a1
move.w (-6,%a1),%d1
ext.l %d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
add.l %d1,%d0
move.w (-6,%a0),%d1
movea.l (-4,%a0),%a0
ext.l %d1
movea.l (-4,%a0),%a0
add.l %d1,%d0
move.l %d0,(-4,%a0)
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
L2:
link.w %fp,#-8
movea.l (%fp),%a0
lea (0,%a0),%a1
move.l %a1,(-4,%fp)
movea.l (-4,%a0),%a0
move.w (8,%a0),%d0
subq.w #2,%d0
move.w %d0,(-6,%fp)
bsr.s L3
unlk %fp
rts
L1:
link.w %fp,#-8
movea.l (%fp),%a0
lea (0,%a0),%a1
move.l %a1,(-4,%fp)
move.w (8,%a0),%d0
subq.w #2,%d0
move.w %d0,(-6,%fp)
bsr.s L2
unlk %fp
rts
.globl OGF8_NESTED_$FOUR
OGF8_NESTED_$FOUR:
link.w %fp,#-4
move.l #-0xc8,(-4,%fp)
bsr.s L1
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.l #0x4d2,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.balignw 4,0x2048
