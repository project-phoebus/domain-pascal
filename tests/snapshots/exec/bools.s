.text
.globl PROBE
PROBE:
link.w %fp,#-4
move.l %d2,-(%sp)
subq.l #2,%sp
move.w (8,%fp),%d2
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
tst.w %d2
sgt %d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BOOLS_$SET
BOOLS_$SET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(2,%a5)
clr.b (%a5)
move.w %d1,(6,%a5)
tst.w %d0
ble.s 1f
st (%a5)
1:
tst.w %d1
ble.s 2f
st (4,%a5)
bra.s 3f
2:
clr.b (4,%a5)
3:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BOOLS_$TEST
BOOLS_$TEST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0xa,%a5)
move.b (%a5),%d0
and.b (4,%a5),%d0
bpl.s 1f
move.w #1,(0xa,%a5)
1:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
clr.w (0xa,%a5)
move.b (%a5),%d0
or.b (4,%a5),%d0
bpl.s 2f
move.w #1,(0xa,%a5)
2:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0xa,(0xa,%a5)
cmpi.w #1,(2,%a5)
seq %d0
cmpi.w #2,(6,%a5)
seq %d1
and.b %d1,%d0
bpl.s 3f
move.w #0xb,(0xa,%a5)
3:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0x14,(0xa,%a5)
move.w (2,%a5),%d0
cmp.w (6,%a5),%d0
sgt %d0
or.b (4,%a5),%d0
bpl.s 4f
move.w #0x15,(0xa,%a5)
4:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w #0x1e,(0xa,%a5)
move.b (%a5),%d0
tst.w (6,%a5)
smi %d1
or.b %d1,%d0
not.b %d0
bpl.s 5f
move.w #0x1f,(0xa,%a5)
5:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.b (%a5),%d0
move.b (4,%a5),%d1
not.b %d1
and.b %d1,%d0
move.b %d0,(8,%a5)
pea (8,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w (2,%a5)
sne %d0
tst.w (6,%a5)
sne %d1
or.b %d1,%d0
or.b (%a5),%d0
move.b %d0,(8,%a5)
pea (8,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w #0x32,(0xa,%a5)
subq.l #2,%sp
move.w (2,%a5),-(%sp)
bsr.w PROBE
addq.w #4,%sp
or.b (4,%a5),%d0
bpl.s 6f
move.w #0x33,(0xa,%a5)
6:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BOOLS_$ODD
BOOLS_$ODD:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d3
clr.w (0xa,%a5)
btst #0,%d2
beq.s 1f
move.w #1,(0xa,%a5)
1:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
clr.w (0xa,%a5)
btst #0,%d3
beq.s 2f
move.w #1,(0xa,%a5)
2:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w %d2,%d0
addq.w #1,%d0
move.w #2,(0xa,%a5)
btst #0,%d0
sne %d0
or.b (%a5),%d0
bpl.s 3f
move.w #3,(0xa,%a5)
3:
subq.l #2,%sp
move.w (0xa,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOOLS_$LOOP
BOOLS_$LOOP:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w %d2
.balignw 4,0x2048
1:
addq.w #1,%d2
move.b (%a5),%d0
cmpi.w #3,%d2
sgt %d1
or.b %d1,%d0
bpl.s 1b
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.w %d2,(0xa,%a5)
clr.w %d2
.balignw 4,0x2048
2:
addq.w #1,%d2
cmpi.w #2,%d2
sgt %d0
move.b (%a5),%d1
not.b %d1
and.b %d1,%d0
bpl.s 2b
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
move.w %d2,(0xa,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BOOLS","aw",@nobits
.balign 4
.Ldata:
.space 12
