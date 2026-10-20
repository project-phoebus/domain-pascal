.text
.globl GET_DESC
GET_DESC:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
movea.l (0xa,%fp),%a2
subq.l #2,%sp
pea (%a2)
move.w (8,%fp),-(%sp)
jsr (REAL_LINE).l
addq.w #8,%sp
move.w %d0,%d1
tst.l (%a2)
bne.s 1f
move.w %d1,%d0
lsl.w #3,%d0
move.w %d0,%d2
neg.w %d0
lsl.w #3,%d2
movea.l #DTTE,%a0
add.w %d2,%d0
tst.l (0x28,%a0,%d0.w)
bne.s 2f
move.l #0xb000d,(%a2)
bra.s 1f
2:
move.w %d1,%d0
lsl.w #3,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #3,%d1
add.w %d1,%d0
move.l (0x28,%a0,%d0.w),(-8,%fp)
1:
movea.l (-8,%fp),%a0
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
.globl FIND
FIND:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
move.w %d0,%d1
lsl.w #2,%d1
tst.l (0,%a5,%d1.w)
bne.s 1f
move.l #0x2b0003,(%a0)
clr.l (-4,%fp)
bra.s 2f
1:
move.w %d0,%d1
lsl.w #2,%d1
movea.l (0,%a5,%d1.w),%a1
tst.l (0x2c,%a1)
bne.s 3f
move.l #0x11001d,(%a0)
clr.l (-4,%fp)
bra.s 4f
3:
clr.l (%a0)
move.w %d0,%d1
lsl.w #2,%d1
movea.l (0,%a5,%d1.w),%a0
movea.l (0x2c,%a0),%a0
move.l (%a0),(-4,%fp)
4:
2:
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FRAMED
FRAMED:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
move.w %d0,%d1
lsl.w #2,%d1
move.l (0,%a5,%d1.w),(-4,%fp)
tst.l (-4,%fp)
bne.s 1f
moveq #1,%d0
move.l %d0,(%a0)
clr.l (-8,%fp)
bra.s 2f
1:
movea.l (-4,%fp),%a1
tst.l (0x2c,%a1)
bne.s 3f
moveq #2,%d0
move.l %d0,(%a0)
clr.l (-8,%fp)
bra.s 4f
3:
move.l (-4,%fp),(-8,%fp)
4:
2:
movea.l (-8,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl OUT_LOCAL
OUT_LOCAL:
link.w %fp,#-8
pea (%a2)
movea.l (8,%fp),%a2
pea (%a2)
pea (-4,%fp)
jsr (FILL).l
addq.w #8,%sp
pea (%a2)
jsr (SCAN).l
move.l (-4,%fp),(-8,%fp)
movea.l (-8,%fp),%a0
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#-8
pea (%a2)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (MAKE).l
addq.w #4,%sp
movea.l %a0,%a2
clr.l (0x34,%a2)
move.l (0xa,%fp),-(%sp)
jsr (SCAN).l
movea.l %a2,%a0
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTRSLOT","aw",@nobits
.balign 4
.Ldata:
.space 32
