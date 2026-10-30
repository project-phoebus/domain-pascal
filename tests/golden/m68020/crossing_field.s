.arch 68020
.text
.globl P
P:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (3,%a0),%d0
movea.l (0xc,%fp),%a0
cmp.l (3,%a0),%d0
bne.s 1f
moveq #8,%d0
move.l %d0,(%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl Q
Q:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (CB).l,%a0
move.l (%a5),%d1
move.l (3,%a0),%d0
sub.l %d0,%d1
move.l %d1,(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CROSSING_FIELD","aw",@nobits
.balign 4
.Ldata:
.space 4
