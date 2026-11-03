.set __dpc_fragment_origin,0xe151fe
.section ".text.PROC1_.WIRED_PROC","ax",@progbits
.space 2
.globl EC_$INIT
EC_$INIT:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.l (%a0)
move.l %a0,(4,%a0)
move.l %a0,(8,%a0)
unlk %fp
rts
