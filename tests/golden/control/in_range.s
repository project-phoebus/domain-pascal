.text
.globl SET_SLOT
SET_SLOT:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0x10,%fp),%a1
cmpi.w #0x1f,(%a0)
bhi.s 2f
cmpi.w #0x11,(%a0)
bls.s 1f
2:
move.l #0x350002,(%a1)
bra.s 3f
1:
clr.l (%a1)
3:
unlk %fp
rts
.globl CHECK
CHECK:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
cmpi.w #0x1f,(%a0)
sls %d0
move.b %d0,(0x12,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.IN_RANGE","aw",@nobits
.balign 4
.Ldata:
.space 20
