.text
.globl HELD_ACROSS
HELD_ACROSS:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
move.w (0xc,%fp),%d2
pea (0xc,%a2)
jsr (EC_$INIT).l
addq.w #4,%sp
pea (%a2)
jsr (EC_$INIT).l
clr.b (0x1a,%a2)
move.w %d2,(0x18,%a2)
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl THIRD_WORD
THIRD_WORD:
link.w %fp,#0
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
move.w (0xe,%fp),%d1
move.w (0x10,%fp),%d2
move.w %d0,(%a0)
move.w %d1,(4,%a0)
move.w %d2,(8,%a0)
move.l (-4,%fp),%d2
unlk %fp
rts
.globl REUSE
REUSE:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.w (%a0)
clr.w (4,%a0)
movea.l (OTHER).l,%a0
clr.w (8,%a0)
unlk %fp
rts
.globl DIES_BEFORE_CALL
DIES_BEFORE_CALL:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
move.w %d0,(%a0)
subq.l #2,%sp
move.w #0x10,-(%sp)
jsr (TICK).l
unlk %fp
rts
.globl IN_LOOP
IN_LOOP:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l (0xa,%fp),%a2
clr.w (2,%a2)
.balignw 4,0x2048
1:
move.w %d2,(%a2)
subq.l #2,%sp
move.w #0x10,-(%sp)
jsr (TICK).l
addq.w #4,%sp
tst.b (DONE).l
bpl.s 1b
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl PUSHED_LAST
PUSHED_LAST:
link.w %fp,#0
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d2
move.w %d2,(%a0)
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (TICK).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl STATUS_LAST
STATUS_LAST:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
clr.l (%a2)
pea (%a2)
jsr (SET_STATUS).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
