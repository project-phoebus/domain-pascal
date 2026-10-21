.text
CHECK_UNIT:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
clr.l (%a0)
cmpi.w #0x1f,%d0
bls.s 1f
move.l #0x100008,(%a0)
bra.s 4f
1:
move.w %d0,%d1
lsl.w #5,%d1
lea (0,%a5,%d1.w),%a1
tst.w (0x12c,%a1)
bne.s 3f
move.l #0x100009,(%a0)
3:
2:
tst.l (%a0)
4:
seq %d0
unlk %fp
rts
.globl RELEASE
RELEASE:
link.w %fp,#-4
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.s CHECK_UNIT
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a3
move.w (0x12e,%a3),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x74,%a2)
beq.s 2f
move.l (0x118,%a3),-(%sp)
movea.l (0x74,%a2),%a0
jsr (%a0)
addq.w #4,%sp
2:
move.l (0x118,%a3),-(%sp)
move.w (0x12e,%a3),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x7c,%a0),%a1
jsr (%a1)
clr.w (0x12c,%a3)
1:
movem.l (-0x14,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl DISABLE_DEVICE
DISABLE_DEVICE:
link.w %fp,#-4
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.w CHECK_UNIT
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a3
move.w (0x12e,%a3),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x74,%a2)
beq.s 2f
move.l (0x118,%a3),-(%sp)
movea.l (0x74,%a2),%a0
jsr (%a0)
2:
1:
movem.l (-0x14,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl ENABLE_DEVICE
ENABLE_DEVICE:
link.w %fp,#-4
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.w CHECK_UNIT
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a3
move.w (0x12e,%a3),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x70,%a2)
beq.s 2f
move.l (0x118,%a3),-(%sp)
movea.l (0x70,%a2),%a0
jsr (%a0)
2:
1:
movem.l (-0x14,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.EHOLD","aw",@nobits
.balign 4
.Ldata:
.space 1304
