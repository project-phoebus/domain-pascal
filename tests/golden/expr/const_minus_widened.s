.text
.globl FROM_WORD
FROM_WORD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (%a5),%d0
move.l #0x400,%d1
ext.l %d0
sub.l %d0,%d1
move.l %d1,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FROM_UWORD
FROM_UWORD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l %d0
moveq #0x20,%d1
move.w (8,%fp),%d0
sub.l %d0,%d1
move.l %d1,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CONST_MINUS_WIDENED","aw",@nobits
.balign 4
.Ldata:
.space 8
