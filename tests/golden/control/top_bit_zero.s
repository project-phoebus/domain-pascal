.text
.globl CHECK
CHECK:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
tst.w (%a0)
ble.s 2f
cmpi.w #0x4000,(%a0)
bls.s 1f
2:
move.l #0x220003,(%a1)
bra.s 3f
1:
clr.l (%a1)
3:
unlk %fp
rts
.globl POSITIVE
POSITIVE:
link.w %fp,#0
tst.w (N).l
ble.s 1f
move.w #1,(R).l
1:
unlk %fp
rts
.globl UNIT_CHECK
UNIT_CHECK:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
clr.l (%a1)
tst.w (%a0)
bmi.s 2f
cmpi.w #0xf,(%a0)
bls.s 1f
2:
move.l #0x1e000c,(%a1)
1:
unlk %fp
rts
.balignw 4,0x2048
