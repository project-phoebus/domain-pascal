.set __dpc_fragment_origin,0xe1a9c2
.text
.space 2
.globl IO_$ALLOC
IO_$ALLOC:
link.w %fp,#-4
movea.l (0x14,%fp),%a0
clr.l (%a0)
clr.l %d0
unlk %fp
rts
.balignw 4,0x2048
