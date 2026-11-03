.set __dpc_fragment_origin,0xe1a1f6
.text
.space 2
.globl PCHIST_$INTERRUPT
PCHIST_$INTERRUPT:
link.w %fp,#-4
pea (.Lc0,%pc)
move.l (8,%fp),-(%sp)
bsr.w PCHIST_$COUNT
unlk %fp
rts
.Lc0:
.short 0x0001
.balignw 4,0x2048
