.set __dpc_fragment_origin,0xe18fd6
.text
.space 2
.globl XNS_IDP_$GET_STATS
XNS_IDP_$GET_STATS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a5),(%a0)
move.l (4,%a5),(4,%a0)
move.l (8,%a5),(8,%a0)
movea.l (0xc,%fp),%a1
clr.l (%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.XNS_IDP","aw",@nobits
.balign 4
.Ldata:
.space 12
