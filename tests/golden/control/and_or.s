.text
.globl AO_$BOTH
AO_$BOTH:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
cmpi.w #1,(2,%a5)
seq %d0
cmpi.w #2,(6,%a5)
sgt %d1
and.b %d1,%d0
bpl.s 1f
clr.w (2,%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AO_$CHAIN
AO_$CHAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
cmpi.l #0xe0002b,(0xc,%a5)
seq %d0
cmpi.l #0xe00020,(0xc,%a5)
seq %d1
or.b %d1,%d0
cmpi.l #0xe00033,(0xc,%a5)
seq %d1
or.b %d1,%d0
bpl.s 1f
clr.w (2,%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AO_$VARS
AO_$VARS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (%a5),%d0
and.b (4,%a5),%d0
bpl.s 1f
clr.w (2,%a5)
1:
move.b (%a5),%d0
not.b %d0
tst.w (6,%a5)
smi %d1
or.b %d1,%d0
bpl.s 2f
clr.w (6,%a5)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AO_$NOT
AO_$NOT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (%a5),%d0
cmpi.w #3,(6,%a5)
sne %d1
or.b %d1,%d0
not.b %d0
bpl.s 1f
clr.w (2,%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AO_$STORE
AO_$STORE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
tst.w (2,%a5)
sne %d0
move.b (4,%a5),%d1
not.b %d1
and.b %d1,%d0
move.b %d0,(8,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AO_$ODD
AO_$ODD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.w (0xc,%fp),%d1
btst #0,%d0
beq.s 1f
clr.w (2,%a5)
1:
btst #0,%d1
sne %d0
or.b (%a5),%d0
bpl.s 2f
clr.w (6,%a5)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.AND_OR","aw",@nobits
.balign 4
.Ldata:
.space 16
