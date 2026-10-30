.text
.globl C90_INT
C90_INT:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (%a0),%a0
move.b (0x10,%a0),(-6,%fp)
tst.w (-6,%fp)
smi %d0
unlk %fp
rts
.balignw 4,0x2048
