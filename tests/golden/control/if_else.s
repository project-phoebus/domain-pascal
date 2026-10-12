.text
IF_HEAD:
link.w %fp,#0
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
movea.l (0xc,%fp),%a0
tst.l (0x14,%a5)
bne.s 1f
clr.w (%a0)
bra.s 2f
1:
move.w #0x400,(%a0)
cmpi.w #0x400,%d1
bcc.s 3f
move.w #2,(%a0)
3:
cmpi.w #0x400,%d0
bls.s 4f
clr.w (%a0)
bra.s 5f
4:
move.w #5,(%a0)
5:
cmpi.w #0x400,%d0
bls.s 6f
cmpi.w #0x400,%d1
bls.s 7f
clr.w (%a0)
bra.s 8f
7:
move.w #1,(%a0)
8:
6:
2:
unlk %fp
rts
IF_FAR:
link.w %fp,#0
tst.l (0x14,%a5)
bne.w 1f
move.l #0x12345,(B1).l
move.l #0x12345,(B2).l
move.l #0x12345,(B3).l
move.l #0x12345,(B4).l
move.l #0x12345,(B5).l
move.l #0x12345,(B6).l
move.l #0x12345,(B7).l
move.l #0x12345,(B8).l
move.l #0x12345,(B9).l
move.l #0x12345,(B10).l
move.l #0x12345,(B11).l
move.l #0x12345,(B12).l
move.l #0x12345,(B13).l
1:
clr.l (B1).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CTL","aw",@nobits
.balign 4
.Ldata:
.space 24
