.text
.globl LSP_$GETD
LSP_$GETD:
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
move.l (0xc,%a5),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.LSP_DESC","aw",@progbits
.balign 4
.Ldata:
.globl LSP_$DESC
LSP_$DESC:
.byte 0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x05,0x00,0x00,0x00,0x06
.long .Ldata
