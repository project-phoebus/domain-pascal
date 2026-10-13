.text
.globl STAT_$GET_FLAGS
STAT_$GET_FLAGS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (4,%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl STAT_$RESET
STAT_$RESET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
STAT_$BUMP:
link.w %fp,#-4
move.w #3,(6,%a5)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.STAT","aw",@nobits
.balign 4
.Ldata:
.space 6
.globl LEVEL
LEVEL:
.space 2
