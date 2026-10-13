.text
WORKER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl M_$USES
M_$USES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl M_$NOT
M_$NOT:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.w (%a0)
unlk %fp
rts
.globl M_$BACK
M_$BACK:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w #1,(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl M_$CALLS
M_$CALLS:
link.w %fp,#0
bsr.s WORKER
unlk %fp
rts
.globl M_$ABSOLUTE
M_$ABSOLUTE:
link.w %fp,#0
lea (.Ldata).l,%a0
move.l (CLOCK).l,(4,%a0)
st (8,%a0)
unlk %fp
rts
.globl M_$EMPTY
M_$EMPTY:
rts
.globl M_$FROM
M_$FROM:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
clr.l (%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl M_$EMPTY2
M_$EMPTY2:
rts
.globl M_$LATER
M_$LATER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l #0x3a0001,(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.M","aw",@nobits
.balign 4
.Ldata:
.space 12
