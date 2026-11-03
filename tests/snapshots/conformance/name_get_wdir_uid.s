.set __dpc_fragment_origin,0xe58960
.text
.globl NAME_$GET_WDIR_UID
NAME_$GET_WDIR_UID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
movea.l (8,%fp),%a1
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a0
lea (0x950,%a0),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NAME_$GET_NDIR_UID
NAME_$GET_NDIR_UID:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
movea.l (8,%fp),%a1
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a0
lea (0x3e0,%a0),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.NAME","aw",@nobits
.balign 4
.Ldata:
.space 2848
