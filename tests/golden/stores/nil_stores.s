.text
.globl FRAME_LOCAL
FRAME_LOCAL:
link.w %fp,#-4
pea (-4,%fp)
jsr (SET_PTR).l
addq.w #4,%sp
clr.l (-4,%fp)
pea (-4,%fp)
jsr (SET_PTR).l
unlk %fp
rts
.globl UNIT_VAR
UNIT_VAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl OTHER_UNIT_VAR
OTHER_UNIT_VAR:
link.w %fp,#0
clr.l (OTHER_HEAD).l
unlk %fp
rts
.globl THROUGH_POINTER
THROUGH_POINTER:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.l (6,%a0)
clr.l (2,%a0)
unlk %fp
rts
.globl IN_REGISTER
IN_REGISTER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l %d0
tst.l (%a5)
beq.s 1f
move.l (%a5),%d0
1:
move.l %d0,(OTHER_HEAD).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NILS","aw",@nobits
.balign 4
.Ldata:
.space 4
