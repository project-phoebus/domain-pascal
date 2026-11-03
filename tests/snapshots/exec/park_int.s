.text
.globl PARK_INT_$AFTER_CALL
PARK_INT_$AFTER_CALL:
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
movea.l %d0,%a2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a2),%d0
add.l %d2,%d0
move.l %d0,(4,%a2)
move.l (4,%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl PARK_INT_$NO_CALL
PARK_INT_$NO_CALL:
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
bne.s 1f
movea.l %d0,%a0
move.l (4,%a0),%d1
add.l %d2,%d1
move.l %d1,(4,%a0)
move.l (4,%a0),(%a5)
1:
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl PARK_INT_$IN_A0
PARK_INT_$IN_A0:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
move.l (4,%a0),%d0
add.l (%a0),%d0
subq.l #2,%d0
move.l %d0,(%a5)
1:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl PARK_INT_$SHOW
PARK_INT_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PARK_INT","aw",@progbits
.balign 4
.Ldata:
.space 4
