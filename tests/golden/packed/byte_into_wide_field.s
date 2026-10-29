.text
.globl T
T:
link.w %fp,#-0x24
movea.l (8,%fp),%a0
clr.l (%a0)
tst.b (-0x21,%fp)
bmi.s 1f
clr.l %d0
move.b (-4,%fp),%d0
andi.l #-0x80000000,(%a0)
or.l %d0,(%a0)
1:
unlk %fp
rts
.balignw 4,0x2048
