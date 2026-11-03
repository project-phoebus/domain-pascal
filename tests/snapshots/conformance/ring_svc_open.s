.set __dpc_fragment_origin,0xe76df2
.text
.space 2
.globl RING_$SVC_OPEN
RING_$SVC_OPEN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (-4,%fp)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
clr.w -(%sp)
bsr.w RING_$OPEN_INTERNAL
movea.l (0x16,%fp),%a0
move.l (-4,%fp),(%a0)
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.RING","aw",@nobits
.balign 4
.Ldata:
.space 4
