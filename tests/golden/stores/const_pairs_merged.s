.text
.globl RESET
RESET:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #0x10001,(0x3d2,%a0)
move.w #0x100,(0x3d6,%a0)
clr.l (0x56,%a0)
unlk %fp
rts
.globl LOG_INIT
LOG_INIT:
link.w %fp,#0
moveq #1,%d0
movea.l (8,%fp),%a0
move.l %d0,(%a0)
unlk %fp
rts
.globl OPEN
OPEN:
link.w %fp,#-0x2c
move.l #0x498fc02,(-0x2c,%fp)
pea (-0x2c,%fp)
jsr (USE_OPT).l
unlk %fp
rts
.globl PAIR_KINDS
PAIR_KINDS:
link.w %fp,#0
movea.l (8,%fp),%a0
st (%a0)
clr.w (2,%a0)
move.w #7,(8,%a0)
moveq #5,%d0
move.l %d0,(4,%a0)
unlk %fp
rts
.globl GLOBALS
GLOBALS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (%a5)
clr.w (2,%a5)
st (8,%a5)
st (9,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FLAGS
FLAGS:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.b (2,%a0)
clr.b (3,%a0)
unlk %fp
rts
.globl DOWN
DOWN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x19,(6,%a5)
move.w #0x1f4,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.MERGED","aw",@nobits
.balign 4
.Ldata:
.space 12
