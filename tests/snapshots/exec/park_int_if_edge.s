.text
.globl PIE_$RES
PIE_$RES:
link.w %fp,#-8
movem.l %d2-%d3/%a2,-(%sp)
move.l (8,%fp),%d3
movea.l (0xc,%fp),%a2
clr.l %d2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
move.l (4,%a0),%d2
add.l %d3,%d2
1:
move.l %d2,%d0
movem.l (-0x14,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PIE_$PRE
PIE_$PRE:
link.w %fp,#-0x10
movem.l %d2-%d3/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
moveq #-1,%d3
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
move.l %d2,%d1
addq.l #4,%d1
move.l %d1,%d3
movea.l %d0,%a0
add.l %d1,%d3
add.l %d2,%d3
move.l %d3,%d1
add.l (4,%a0),%d1
add.l (%a0),%d1
sub.l %d2,%d1
subq.l #4,%d1
move.l %d1,%d3
1:
move.l %d3,%d0
movem.l (-0x1c,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PIE_$CMPX
PIE_$CMPX:
link.w %fp,#-0xc
movem.l %d2-%d3/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
moveq #5,%d3
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
seq %d1
tst.l %d2
sne %d2
and.b %d2,%d1
bpl.s 1f
movea.l %d0,%a0
move.l (4,%a0),%d3
add.l (4,%a0),%d3
1:
move.l %d3,%d0
movem.l (-0x18,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PIE_$NEST
PIE_$NEST:
link.w %fp,#-0xc
movem.l %d2-%d3/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
moveq #2,%d3
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
moveq #1,%d3
cmpi.l #0xa,%d2
bge.s 2f
movea.l %d0,%a0
move.l (4,%a0),%d3
add.l %d2,%d3
2:
1:
move.l %d3,%d0
movem.l (-0x18,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PIE_$LOOP
PIE_$LOOP:
link.w %fp,#-0xc
movem.l %d2-%d3/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
moveq #-1,%d3
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
move.l %d2,%d3
bra.s 3f
.balignw 4,0x2048
2:
addq.l #1,%d3
3:
cmpi.l #5,%d3
blt.s 2b
movea.l %d0,%a0
add.l (4,%a0),%d3
1:
move.l %d3,%d0
movem.l (-0x18,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PIE_$STAT
PIE_$STAT:
link.w %fp,#-4
pea (%a2)
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
addi.l #0,(%a0)
1:
addi.l #0x64,(%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PIE_$ARITH
PIE_$ARITH:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
beq.s 1f
moveq #3,%d0
bra.s 2f
1:
movea.l %d0,%a0
move.l (4,%a0),%d1
add.l %d2,%d1
cmpi.l #0xc,%d1
ble.s 3f
moveq #1,%d0
bra.s 4f
3:
moveq #2,%d0
4:
2:
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
