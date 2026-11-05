.text
Q:
link.w %fp,#-0x10
movea.l (0xc,%fp),%a0
move.l (4,%a0),(-0xc,%fp)
move.l (%a0),(-0x10,%fp)
lea (-0x10,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl T
T:
link.w %fp,#-8
pea (%a2)
movea.l (8,%fp),%a2
st -(%sp)
pea (%a2)
pea (-8,%fp)
bsr.s Q
lea (-8,%fp),%a0
move.l (%a0)+,(%a2)
move.l (%a0)+,(4,%a2)
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
