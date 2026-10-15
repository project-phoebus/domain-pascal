.text
.globl SC_$GET_ROOT
SC_$GET_ROOT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (8,%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SC_$SET_NODE
SC_$SET_NODE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(%a5)
move.l (%a0)+,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SC_$LOCAL
SC_$LOCAL:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
lea (%a5),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
lea (-8,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl SC_$NIL
SC_$NIL:
link.w %fp,#0
movea.l #UID_$NIL,%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl SC_$GET_NOW
SC_$GET_NOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (0x10,%a5),(%a0)
move.w (0x14,%a5),(4,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SC_$SET_NOW
SC_$SET_NOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(0x10,%a5)
move.w (4,%a0),(0x14,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SC_$INQ
SC_$INQ:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (SC_$GET_DESC).l
move.l %a0,%d0
movea.l %d0,%a0
movea.l (0xc,%fp),%a2
lea (0x4c,%a0),%a1
move.l (%a1)+,(%a2)
move.l (%a1)+,(4,%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SC","aw",@nobits
.balign 4
.Ldata:
.space 24
