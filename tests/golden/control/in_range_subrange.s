.text
.globl SET_PARAM
SET_PARAM:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
clr.l (%a1)
cmpi.w #3,(0x14,%a0)
bls.s 1f
4:
move.l #0x360002,(%a1)
bra.s 2f
1:
clr.l %d0
move.w (0x12,%a0),%d0
subq.l #1,%d0
cmpi.l #2,%d0
bhi.s 4b
3:
move.w (0x14,%a0),(%a5)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.IN_RANGE_SUBRANGE","aw",@nobits
.balign 4
.Ldata:
.space 4
