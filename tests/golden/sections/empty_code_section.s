.text
.globl PCHIST_$HITS
PCHIST_$HITS:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
movea.l (8,%fp),%a0
move.l (4,%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PCHIST.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 8
