.set __dpc_fragment_origin,0xe3ccce
.text
.space 2
SET_CHAIN_STATUS:
link.w %fp,#-8
move.l (8,%fp),%d0
movea.l (0xc,%fp),%a0
movea.l %d0,%a1
bra.s 2f
1:
move.l (%a0),(0xc,%a1)
movea.l (%a1),%a1
2:
cmpa.w #0,%a1
bne.s 1b
unlk %fp
rts
