.text
.globl DRAINED
DRAINED:
link.w %fp,#0
movea.l (8,%fp),%a0
bclr #0,(9,%a0)
unlk %fp
rts
.globl SET_FLAG
SET_FLAG:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
movea.l (0x10,%fp),%a2
tst.b (%a2)
bpl.s 1f
clr.l %d0
move.w (%a1),%d1
bset %d1,%d0
or.l %d0,(0x10,%a0)
bra.s 2f
1:
clr.l %d0
move.w (%a1),%d1
bset %d1,%d0
not.l %d0
and.l %d0,(0x10,%a0)
2:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl SEVERAL
SEVERAL:
link.w %fp,#0
movea.l (8,%fp),%a0
andi.w #-0xe1,(8,%a0)
bset #1,(0x12,%a0)
bset #7,(9,%a0)
unlk %fp
rts
.globl MASKS
MASKS:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (MASK).l,%d0
and.l (0x10,%a0),%d0
move.l %d0,(0x14,%a0)
move.l (MASK).l,%d0
not.l %d0
and.l %d0,(0x10,%a0)
move.l (0x14,%a0),%d0
or.l %d0,(0x10,%a0)
clr.l (0x14,%a0)
unlk %fp
rts
.globl SAVED
SAVED:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
clr.l (0x14,%a0)
move.w %d0,%d1
andi.w #0x80,%d1
or.w %d1,(8,%a0)
unlk %fp
rts
.globl ANY_OF
ANY_OF:
link.w %fp,#-4
clr.b %d0
movea.l (8,%fp),%a0
moveq #7,%d1
and.w (8,%a0),%d1
beq.s 1f
st %d0
1:
unlk %fp
rts
.globl PENDING
PENDING:
link.w %fp,#-4
clr.b %d0
movea.l (8,%fp),%a0
tst.w (0xa,%a0)
beq.s 1f
st %d0
1:
unlk %fp
rts
.globl IN_CONST
IN_CONST:
link.w %fp,#-4
move.l %d2,-(%sp)
clr.b %d0
movea.l (8,%fp),%a0
move.w (%a0),%d1
moveq #0x1f,%d2
btst %d1,%d2
bne.s 1f
st %d0
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl SEP
SEP:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
clr.b %d0
clr.w %d2
move.w #0xff,%d3
move.b (0x1f,%a5,%d1.w),%d2
lea (%a5),%a0
sub.w %d2,%d3
lsr.w #3,%d3
btst %d2,(0,%a0,%d3.w)
beq.s 1f
st %d0
1:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET_OPS","aw",@nobits
.balign 4
.Ldata:
.space 44
