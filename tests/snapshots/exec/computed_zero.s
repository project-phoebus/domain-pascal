.text
.globl COMPUTED_ZERO_$ADD_LT
COMPUTED_ZERO_$ADD_LT:
link.w %fp,#-4
move.l (8,%fp),%d0
add.l (0xc,%fp),%d0
smi %d0
unlk %fp
rts
.globl COMPUTED_ZERO_$SUB_GE
COMPUTED_ZERO_$SUB_GE:
link.w %fp,#-4
move.l (8,%fp),%d0
sub.l (0xc,%fp),%d0
spl %d0
unlk %fp
rts
.globl COMPUTED_ZERO_$SUB_GT
COMPUTED_ZERO_$SUB_GT:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l %d0,%d2
sub.l %d1,%d2
tst.l %d2
ble.s 1f
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
.globl COMPUTED_ZERO_$ADD_LE
COMPUTED_ZERO_$ADD_LE:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l %d0,%d2
add.l %d1,%d2
tst.l %d2
bgt.s 1f
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
.globl COMPUTED_ZERO_$LOGIC
COMPUTED_ZERO_$LOGIC:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.l (0xc,%fp),%d2
move.w %d0,%d3
or.w %d1,%d3
bne.s 1f
subq.l #2,%sp
move.w #0xb,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
move.l %d2,%d0
andi.l #0x180,%d0
beq.s 2f
subq.l #2,%sp
move.w #0x15,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.l %d2,(%a5)
subq.l #2,%sp
move.w #0x13,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl COMPUTED_ZERO_$BIT
COMPUTED_ZERO_$BIT:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
btst #16,%d2
beq.s 1f
subq.l #2,%sp
move.w #0x1f,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
btst #30,%d2
bne.s 2f
subq.l #2,%sp
move.w #0x21,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.l %d2,(%a5)
subq.l #2,%sp
move.w #0x27,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl COMPUTED_ZERO_$TESTED
COMPUTED_ZERO_$TESTED:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0xa,%fp),%d2
move.b (0xc,%fp),%d0
move.b %d0,(4,%a5)
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
tst.w %d0
bne.s 1f
subq.l #2,%sp
move.w #0x29,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
move.w %d2,%d0
ext.l %d0
divs.w #7,%d0
tst.w %d0
bne.s 2f
subq.l #2,%sp
move.w #0x33,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.w %d2,%d0
ext.l %d0
divs.w #7,%d0
swap %d0
tst.w %d0
beq.s 3f
subq.l #2,%sp
move.w #0x3d,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
clr.w %d0
move.b (4,%a5),%d0
tst.w %d0
bne.s 4f
subq.l #2,%sp
move.w #0x47,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
subq.l #2,%sp
move.w #0x31,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl COMPUTED_ZERO_$ASSIGNED
COMPUTED_ZERO_$ASSIGNED:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (0xa,%fp),%d2
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d3
sub.w %d2,%d3
bne.s 1f
subq.l #2,%sp
move.w #0x51,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.w #1,%d3
bpl.s 2f
subq.l #2,%sp
move.w #0x5b,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
subq.l #2,%sp
move.w #0x59,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.COMPUTED_ZERO","aw",@nobits
.balign 4
.Ldata:
.space 8
