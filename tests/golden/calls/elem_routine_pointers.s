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
.globl CLEAR_PENDING
CLEAR_PENDING:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
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
lea (0,%a5,%d0.w),%a2
move.l (0x118,%a2),-(%sp)
move.w (0x12e,%a2),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x7c,%a0),%a1
jsr (%a1)
1:
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl DEVICE_INTERRUPTING
DEVICE_INTERRUPTING:
link.w %fp,#-8
movem.l %d2/%a2/%a5,-(%sp)
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
lea (0,%a5,%d0.w),%a2
move.l (0x118,%a2),-(%sp)
move.w (0x12e,%a2),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x78,%a0),%a1
jsr (%a1)
1:
movem.l (-0x14,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl XLATE_IOVA
XLATE_IOVA:
link.w %fp,#-8
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xe,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.w CHECK_UNIT
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a2
move.l (0xa,%fp),-(%sp)
move.w (0x12e,%a2),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x64,%a0),%a1
jsr (%a1)
1:
movem.l (-0x14,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl FIRST_FREE
FIRST_FREE:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.l #0x10000b,(%a1)
moveq #0x1f,%d0
clr.w %d1
movea.l %a5,%a2
.balignw 4,0x2048
2:
tst.w (0x12c,%a2)
bne.s 4f
move.w %d1,(%a0)
clr.l (%a1)
bra.s 6f
4:
addq.w #1,%d1
lea (0x20,%a2),%a2
dbf %d0,2b
1:
tst.l (%a1)
bne.s 5f
6:
move.w (%a0),%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a0
move.w #1,(0x12c,%a0)
5:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.EPTR","aw",@nobits
.balign 4
.Ldata:
.space 1304
