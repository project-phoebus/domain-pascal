.text
TT_$OFFSET:
link.w %fp,#-4
clr.l %d0
move.w (8,%fp),%d0
add.l (%a5),%d0
move.l %d0,(4,%a5)
unlk %fp
rts
.globl TT_$CONSTS
TT_$CONSTS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x1e,%d0
add.l (%a5),%d0
move.l %d0,(4,%a5)
move.l (%a5),%d0
addq.l #1,%d0
move.l %d0,(4,%a5)
move.l (%a5),%d0
subq.l #4,%d0
move.l %d0,(4,%a5)
move.l (%a5),(8,%a5)
move.l (8,%a5),(%a5)
clr.l %d0
move.w (0xe,%a5),%d0
move.l %d0,(8,%a5)
move.w (0xe,%a5),(0xc,%a5)
move.w (0xc,%a5),(0x10,%a5)
move.w (0x10,%a5),(0xc,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl TT_$WALK
TT_$WALK:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (0xc,%a5),%d0
ext.l %d0
add.l (%a5),%d0
movea.l %d0,%a0
bra.s 2f
.balignw 4,0x2048
1:
movea.l (8,%a0),%a0
2:
cmpa.w #0,%a0
bne.s 1b
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.TT","aw",@nobits
.balign 4
.Ldata:
.space 20
