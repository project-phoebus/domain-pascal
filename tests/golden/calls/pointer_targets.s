.text
ADVANCE:
link.w %fp,#-4
move.l (8,%fp),-(%sp)
jsr (EC_$ADVANCE_WITHOUT_DISPATCH).l
unlk %fp
rts
.globl PTARGETS_$DRAINED
PTARGETS_$DRAINED:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
bclr #0,(9,%a2)
move.l (0x7c,%a2),-(%sp)
bsr.s ADVANCE
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PTARGETS_$BUFFER_DRAINED
PTARGETS_$BUFFER_DRAINED:
link.w %fp,#0
movea.l (8,%fp),%a0
pea (0x4c,%a0)
jsr (EC_$ADVANCE_WITHOUT_DISPATCH).l
unlk %fp
rts
PUT_TWO:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
movea.l (8,%a2),%a0
pea (0x4c,%a0)
pea (.Lc0,%pc)
jsr (EC_$PUT).l
addq.w #8,%sp
movea.l (8,%a2),%a0
pea (0x4c,%a0)
pea (.Lc0,%pc)
jsr (EC_$PUT).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.Lc0:
.short 0x0000
.globl PTARGETS_$OUTER
PTARGETS_$OUTER:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
move.w (%a0),(-2,%fp)
tst.w (-2,%fp)
ble.s 1f
bsr.s PUT_TWO
1:
unlk %fp
rts
.globl PTARGETS_$LOCAL
PTARGETS_$LOCAL:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
movea.l (-4,%fp),%a0
pea (0x4c,%a0)
jsr (EC_$ADVANCE_WITHOUT_DISPATCH).l
addq.w #4,%sp
movea.l (-4,%fp),%a0
move.l (0x7c,%a0),(-8,%fp)
move.l (-8,%fp),-(%sp)
jsr (EC_$ADVANCE_WITHOUT_DISPATCH).l
unlk %fp
rts
.globl PTARGETS_$STATIC
PTARGETS_$STATIC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (%a5),%a0
pea (0x4c,%a0)
jsr (EC_$ADVANCE_WITHOUT_DISPATCH).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTARGETS","aw",@nobits
.balign 4
.Ldata:
.space 4
