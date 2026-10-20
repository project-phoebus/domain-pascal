.text
.globl PID_ARG
PID_ARG:
link.w %fp,#0
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (%a0),%d0
bpl.s 1f
neg.w %d0
1:
move.w %d0,-(%sp)
jsr (FIND).l
unlk %fp
rts
.globl ADJUST
ADJUST:
link.w %fp,#0
movea.l (0xc,%fp),%a0
clr.l (%a0)
movea.l (8,%fp),%a1
move.l (%a1),%d0
bpl.s 2f
neg.l %d0
2:
cmpi.l #0x1f40,%d0
bls.s 1f
move.l #0xd000c,(%a0)
1:
unlk %fp
rts
.globl LOCATE
LOCATE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),%d0
sub.l (CLOCKH).l,%d0
bpl.s 2f
neg.l %d0
2:
cmp.l (4,%a5),%d0
bls.s 1f
clr.b (8,%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl HYST
HYST:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0xa,%a5),%d0
move.w %d0,%d1
bpl.s 2f
neg.w %d1
2:
cmp.w (2,%a0),%d1
bge.s 1f
clr.w %d0
1:
move.w %d0,(0xa,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl QUOT
QUOT:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
ext.l %d2
divs.w %d1,%d2
tst.w %d2
bpl.s 2f
neg.w %d2
2:
cmpi.w #5,%d2
bls.s 1f
clr.w (0xa,%a5)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl VELOCITY
VELOCITY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
muls.w (4,%a0),%d0
bpl.s 1f
neg.w %d0
1:
movea.l (8,%fp),%a0
move.w (0xe,%fp),%d1
muls.w (6,%a0),%d1
bpl.s 2f
neg.w %d1
2:
add.w %d1,%d0
move.w %d0,(0xa,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PUNCH
PUNCH:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
move.w (0xe,%fp),%d1
move.w %d0,%d2
bpl.s 2f
neg.w %d2
2:
clr.l %d0
move.w %d2,%d0
move.w %d1,%d2
bpl.s 3f
neg.w %d2
3:
clr.l %d1
move.w %d2,%d1
add.l %d1,%d0
move.w (8,%a0),%d1
ext.l %d1
cmp.l %d1,%d0
blt.s 1f
clr.w (0xa,%a5)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ABS","aw",@nobits
.balign 4
.Ldata:
.space 12
