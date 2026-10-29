.text
F:
link.w %fp,#-8
move.w (0xc,%fp),%d0
movea.l (0xe,%fp),%a0
move.w %d0,%d1
ext.l %d1
move.l %d1,(%a0)
st %d0
unlk %fp
rts
INNER:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (%fp),%a2
move.l (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
pea (-0x40,%a2)
bsr.s F
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl OUTER
OUTER:
link.w %fp,#-0x40
subq.l #2,%sp
move.l (0xc,%fp),-(%sp)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
bsr.s INNER
unlk %fp
rts
.balignw 4,0x2048
