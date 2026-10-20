.text
.globl TRY
TRY:
link.w %fp,#0
movea.l (8,%fp),%a0
moveq #-5,%d0
and.w (0x54,%a0),%d0
movea.l (8,%fp),%a0
ori.w #2,%d0
move.w %d0,(0x54,%a0)
unlk %fp
rts
.globl SIG
SIG:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (4,%a0),%d0
movea.l (8,%fp),%a0
not.l %d0
and.l (%a0),%d0
movea.l (8,%fp),%a0
or.l (8,%a0),%d0
movea.l (8,%fp),%a0
move.l %d0,(%a0)
unlk %fp
rts
.globl OVF
OVF:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
bclr #7,(-4,%a5,%d1.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SUSP
SUSP:
link.w %fp,#-4
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
move.w (%a0),%d2
jsr (CLOBBER).l
btst #1,%d2
sne %d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl UNION
UNION:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a0),%d0
or.w (%a1),%d0
move.w %d0,(0x20,%a5)
move.w (%a0),%d0
ori.w #8,%d0
move.w %d0,(0x20,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CDIFF
CDIFF:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d0
not.w %d0
andi.w #0x21,%d0
move.w %d0,(0x20,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SUB
SUB:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a1),%d0
not.w %d0
and.w (%a0),%d0
seq %d0
move.b %d0,(0x22,%a5)
move.w (%a0),%d0
not.w %d0
and.w (%a1),%d0
seq %d0
move.b %d0,(0x22,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CLEFT
CLEFT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
cmpi.w #4,(%a0)
seq %d0
move.b %d0,(0x22,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET_VALUES","aw",@nobits
.balign 4
.Ldata:
.space 36
