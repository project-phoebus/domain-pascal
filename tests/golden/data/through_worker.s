.text
READ_INTERNAL:
link.w %fp,#0
movea.l (0x10,%fp),%a0
move.w (0x14,%a5),(%a0)
unlk %fp
rts
.globl LOG_$READ
LOG_$READ:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x10,%fp),-(%sp)
movea.l (0xc,%fp),%a0
move.w (%a0),-(%sp)
clr.w -(%sp)
move.l (8,%fp),-(%sp)
bsr.s READ_INTERNAL
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.LOG_","aw",@nobits
.balign 4
.Ldata:
.space 24
