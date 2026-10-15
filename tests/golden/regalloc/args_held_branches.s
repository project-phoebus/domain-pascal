.text
.globl IN_THEN
IN_THEN:
link.w %fp,#0
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d2
move.w %d2,(%a0)
tst.b (FLAG).l
bpl.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (TICK).l
1:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BOTH_PARTS
BOTH_PARTS:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a0
clr.l (%a0)
tst.b (FLAG).l
bpl.s 1f
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (TICK).l
bra.s 2f
1:
pea (%a2)
jsr (TAKE).l
2:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl OTHER_PATH
OTHER_PATH:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
tst.b (FLAG).l
bpl.s 1f
move.w #1,(2,%a2)
bra.s 2f
1:
clr.w (2,%a2)
pea (%a2)
jsr (TAKE).l
2:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl AFTER_INNER_JOIN
AFTER_INNER_JOIN:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 1f
tst.b (FLAG).l
bpl.s 2f
move.w #1,(2,%a2)
2:
pea (%a2)
jsr (TAKE).l
1:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl BEFORE_BRANCH
BEFORE_BRANCH:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
clr.w (%a2)
pea (%a2)
jsr (TAKE).l
addq.w #4,%sp
tst.b (FLAG).l
bpl.s 1f
clr.w (COUNT).l
1:
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
