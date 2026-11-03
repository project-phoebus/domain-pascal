.text
.globl PARK_INT_IF_$GET
PARK_INT_IF_$GET:
link.w %fp,#-0xc
movem.l %d2-%d3/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
clr.l %d3
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a2
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (4,%a2),%d3
add.l %d2,%d3
1:
move.l %d3,%d0
movem.l (-0x18,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl PARK_INT_IF_$COND
PARK_INT_IF_$COND:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
cmpi.l #0x64,%d2
ble.s 2f
moveq #2,%d0
move.l %d0,(%a2)
bra.s 1f
2:
movea.l %d0,%a2
cmpi.l #0xc,(4,%a2)
bge.s 3f
addq.l #1,(4,%a2)
3:
move.l (4,%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl PARK_INT_IF_$ELSE
PARK_INT_IF_$ELSE:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
beq.s 1f
moveq #-1,%d0
move.l %d0,(%a5)
bra.s 2f
1:
movea.l %d0,%a0
move.l (4,%a0),%d1
sub.l (%a0),%d1
add.l %d2,%d1
move.l %d1,(%a5)
2:
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl PARK_INT_IF_$SHOW
PARK_INT_IF_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PARK_INT_IF","aw",@progbits
.balign 4
.Ldata:
.space 4
