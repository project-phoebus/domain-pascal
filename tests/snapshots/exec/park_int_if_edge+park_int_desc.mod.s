.text
.globl PARK_INT_$GETD
PARK_INT_$GETD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
tst.l (%a1)
bpl.s 1f
moveq #1,%d0
move.l %d0,(%a0)
bra.s 2f
1:
clr.l (%a0)
2:
move.l (8,%a5),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PARK_INT_DESC","aw",@progbits
.balign 4
.Ldata:
.globl PARK_INT_$DESC
PARK_INT_$DESC:
.byte 0x00,0x00,0x00,0x07,0x00,0x00,0x00,0x0a
.long .Ldata
