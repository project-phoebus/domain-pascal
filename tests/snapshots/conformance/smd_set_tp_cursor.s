.set __dpc_fragment_origin,0xe6e964
.text
.globl SMD_$SET_TP_CURSOR
SMD_$SET_TP_CURSOR:
link.w %fp,#-8
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
move.l (%a0),(-4,%fp)
movea.l (0x10,%fp),%a1
move.w (%a1),(-6,%fp)
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
move.l (-4,%fp),-(%sp)
movea.l (8,%fp),%a2
move.w (%a2),-(%sp)
clr.w -(%sp)
.short 0x610a
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.section ".bss.moddata.SMD","aw",@nobits
.balign 4
.Ldata:
.space 4
