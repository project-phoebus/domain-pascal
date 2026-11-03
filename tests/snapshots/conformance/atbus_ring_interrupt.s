.set __dpc_fragment_origin,0xe0ab0c
.text
.globl RING_INTERRUPT
RING_INTERRUPT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (4,%a5),-(%sp)
movea.l (%a5),%a0
jsr (%a0)
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.ATBUS_","aw",@nobits
.balign 4
.Ldata:
.space 8
