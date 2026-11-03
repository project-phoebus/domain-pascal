.set __dpc_fragment_origin,0xe41b56
.text
.space 2
.globl PROC2_$GET_BOOT_FLAGS
PROC2_$GET_BOOT_FLAGS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0x1e4,%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.PROC2","aw",@nobits
.balign 4
.Ldata:
.space 488
