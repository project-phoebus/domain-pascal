.set __dpc_fragment_origin,0xe589bc
.text
.globl NAME_$GET_ROOT_UID
NAME_$GET_ROOT_UID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (0x38,%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NAME_$GET_NODE_UID
NAME_$GET_NODE_UID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (0x30,%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NAME_$GET_NODE_DATA_UID
NAME_$GET_NODE_DATA_UID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.NAME","aw",@nobits
.balign 4
.Ldata:
.space 64
