.set __dpc_fragment_origin,0xe42c60
.text
.globl EC2_$INIT
EC2_$INIT:
link.w %fp,#-4
move.l (8,%fp),%d0
cmpi.l #0x3e8,%d0
bls.s 1f
movea.l %d0,%a0
clr.l (%a0)
clr.w (4,%a0)
1:
unlk %fp
rts
