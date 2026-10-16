.text
.globl INIT_BUFS
INIT_BUFS:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w #1,(0x2ca,%a0)
move.w #1,(0x2cc,%a0)
move.w #1,(0x2ce,%a0)
move.w #0x100,(0x2d0,%a0)
move.w #1,(0x3d2,%a0)
move.w #1,(0x3d4,%a0)
move.w #0x100,(0x3d6,%a0)
unlk %fp
rts
.globl CLEAR_DOWN
CLEAR_DOWN:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.w (0x5e2,%a0)
clr.w (0x5e0,%a0)
unlk %fp
rts
.globl FLAGS
FLAGS:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.b (2,%a0)
st (3,%a0)
unlk %fp
rts
.globl CNTL
CNTL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0x70,%a5)
clr.w (0x72,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.APART","aw",@nobits
.balign 4
.Ldata:
.space 116
