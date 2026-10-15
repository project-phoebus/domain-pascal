.text
.globl AOC_$SET_UNIT
AOC_$SET_UNIT:
link.w %fp,#0
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 1f
jsr (N_DEVICES).l
cmp.w (%a2),%d0
sge %d2
tst.b %d2
bpl.s 1f
move.w (%a2),(%a5)
1:
movem.l (-0xc,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl AOC_$CHECK
AOC_$CHECK:
link.w %fp,#0
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
tst.w (%a2)
ble.s 2f
jsr (N_DEVICES).l
cmp.w (%a2),%d0
slt %d2
tst.b %d2
bpl.s 1f
2:
move.l #0x130001,(%a3)
bra.s 3f
1:
move.w (%a2),(%a5)
clr.l (%a3)
3:
movem.l (-0x10,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl AOC_$CALL_LEFT
AOC_$CALL_LEFT:
link.w %fp,#0
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.w (%a2)
bmi.s 2f
jsr (N_DEVICES).l
cmp.w (%a2),%d0
seq %d2
tst.b %d2
bpl.s 1f
2:
clr.w (%a5)
1:
movem.l (-0xc,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl AOC_$ZERO
AOC_$ZERO:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
cmpi.w #1,(%a0)
beq.s 1f
jsr (N_DEVICES).l
tst.w %d0
sne %d2
tst.b %d2
bpl.s 1f
move.w #1,(%a5)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl AOC_$CONST
AOC_$CONST:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
cmpi.w #3,(%a0)
beq.s 2f
jsr (N_DEVICES).l
cmpi.w #2,%d0
seq %d2
tst.b %d2
bpl.s 1f
2:
move.w #2,(%a5)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl AOC_$HELD
AOC_$HELD:
link.w %fp,#-4
movem.l %d2-%d3/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.l (%a2),%d2
addq.l #1,%d2
tst.l (%a2)
bgt.s 2f
pea (%a2)
jsr (COUNT).l
addq.w #4,%sp
cmp.l %d2,%d0
sne %d3
tst.b %d3
bpl.s 1f
2:
move.l (%a2),(4,%a5)
1:
movem.l (-0x14,%fp),%d2-%d3/%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.AND_OR_CALL","aw",@nobits
.balign 4
.Ldata:
.space 8
