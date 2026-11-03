.text
.globl STORE_ORDER_$GETD
STORE_ORDER_$GETD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
tst.w (%a1)
bpl.s 1f
moveq #1,%d0
move.l %d0,(%a0)
bra.s 2f
1:
clr.l (%a0)
2:
move.l (0x10,%a5),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.STORE_ORDER_DESC","aw",@progbits
.balign 4
.Ldata:
.globl STORE_ORDER_$DESC
STORE_ORDER_$DESC:
.byte 0x00,0x00,0x00,0x00,0x00,0x0a,0x00,0x0b,0x00,0x0c,0x00,0x0d,0x00,0x0e
.space 2
.long .Ldata
