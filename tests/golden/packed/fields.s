.text
.globl FORK_FAIL
FORK_FAIL:
link.w %fp,#0
movea.l (8,%fp),%a0
tst.w (2,%a0)
beq.s 1f
bset #7,(%a0)
1:
unlk %fp
rts
.globl NORMALISE
NORMALISE:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
pea (%a2)
jsr (EXT_$OP).l
tst.l (%a2)
sne %d0
andi.b #0x7f,(%a2)
andi.b #0x80,%d0
or.b %d0,(%a2)
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl INHERIT
INHERIT:
link.w %fp,#-4
movea.l (8,%fp),%a0
btst #3,(0xd,%a0)
sne %d0
unlk %fp
rts
.globl ROUTE_STATE
ROUTE_STATE:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w #0xc0,%d0
and.b (0x10,%a0),%d0
lsr.w #6,%d0
unlk %fp
rts
.globl SET_STATE
SET_STATE:
link.w %fp,#0
movea.l (8,%fp),%a0
andi.b #0x3f,(0x10,%a0)
ori.b #0x80,(0x10,%a0)
unlk %fp
rts
.globl CLEAR_STATE
CLEAR_STATE:
link.w %fp,#0
movea.l (8,%fp),%a0
andi.b #0x3f,(0x10,%a0)
unlk %fp
rts
.globl SET_DEPTH
SET_DEPTH:
link.w %fp,#0
move.b (QUEUE_DEPTH+1).l,%d0
movea.l (8,%fp),%a0
andi.l #0xff,%d0
andi.l #-0x7f8001,(0x26,%a0)
lsl.l #8,%d0
lsl.l #7,%d0
or.l %d0,(0x26,%a0)
unlk %fp
rts
.balignw 4,0x2048
