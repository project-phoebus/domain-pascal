.text
.globl P
P:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a0),(%a1)
move.w #3,(%a0)
move.w (%a0),%d0
movea.l (0x10,%fp),%a2
move.b (1,%a0,%d0.w),(%a2)
move.b #0x61,(2,%a0)
tst.w (%a0)
ble.s 1f
addq.w #1,(%a1)
1:
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
