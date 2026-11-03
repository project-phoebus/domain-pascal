.text
.globl UID
UID:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),%d0
unlk %fp
rts
.globl AOC_$OR
AOC_$OR:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 2f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmp.w (%a2),%d0
slt %d2
tst.b %d2
bpl.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
clr.w -(%sp)
4:
jsr (EXEC_$MARK).l
3:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl AOC_$AND
AOC_$AND:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 1f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmp.w (%a2),%d0
sge %d2
tst.b %d2
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$MARK).l
2:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl AOC_$LEFT
AOC_$LEFT:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
tst.w (%a2)
bmi.s 2f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmp.w (%a2),%d0
seq %d2
tst.b %d2
bpl.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
clr.w -(%sp)
4:
jsr (EXEC_$MARK).l
3:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl AOC_$ZERO
AOC_$ZERO:
link.w %fp,#0
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
cmpi.w #1,(%a0)
beq.s 1f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
tst.w %d0
sne %d2
tst.b %d2
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$MARK).l
2:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl AOC_$CONST
AOC_$CONST:
link.w %fp,#0
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
cmpi.w #3,(%a0)
beq.s 2f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmpi.w #0x80,%d0
sge %d2
tst.b %d2
bpl.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
clr.w -(%sp)
4:
jsr (EXEC_$MARK).l
3:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl AOC_$LONG
AOC_$LONG:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
movea.l (8,%fp),%a0
move.l (%a0),%d2
addq.l #1,%d2
tst.l (%a0)
bgt.s 2f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
cmp.l %d2,%d0
sne %d3
tst.b %d3
bpl.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
clr.w -(%sp)
4:
jsr (EXEC_$MARK).l
3:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl AOC_$LONG_LT
AOC_$LONG_LT:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
tst.l (%a2)
beq.s 2f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
cmp.l (%a2),%d0
slt %d2
tst.b %d2
bpl.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
clr.w -(%sp)
4:
jsr (EXEC_$MARK).l
3:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl AOC_$UNS
AOC_$UNS:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
tst.w (%a2)
beq.s 2f
move.l (0xc,%fp),-(%sp)
bsr.w UID
addq.w #4,%sp
cmp.w (%a2),%d0
scs %d2
tst.b %d2
bpl.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
clr.w -(%sp)
4:
jsr (EXEC_$MARK).l
3:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
