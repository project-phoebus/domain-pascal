.set __dpc_fragment_origin,0xe0d8f0
.text
.globl MMAP_$ALLOC_CONTIG
MMAP_$ALLOC_CONTIG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xa,%fp),%a0
clr.l (%a0)
movea.l (0xe,%fp),%a1
move.l #0x6000e,(%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.MMAP_","aw",@nobits
.balign 4
.Ldata:
.space 4
