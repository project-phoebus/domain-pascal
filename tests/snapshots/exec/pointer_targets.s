.text
.globl BUMP
BUMP:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
addq.l #1,(2,%a2)
pea (2,%a2)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PTARGETS_$SETUP
PTARGETS_$SETUP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w #1,(%a5)
moveq #0xa,%d0
move.l %d0,(8,%a5)
lea (0x10,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0xc,%a5)
lea (%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
move.l %a0,(0x18,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
bclr #0,(1,%a2)
move.l (0xc,%a2),-(%sp)
bsr.s BUMP
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PTARGETS_$HELD
PTARGETS_$HELD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
bsr.s HELD
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl SLOT
SLOT:
link.w %fp,#0
movea.l (8,%fp),%a0
pea (6,%a0)
bsr.w BUMP
unlk %fp
rts
.globl PTARGETS_$SLOT
PTARGETS_$SLOT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
bsr.s SLOT
movea.l (-4,%fp),%a5
unlk %fp
rts
TWO:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
movea.l (8,%a2),%a0
pea (6,%a0)
bsr.w BUMP
addq.w #4,%sp
movea.l (8,%a2),%a0
pea (6,%a0)
bsr.w BUMP
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl OUTER
OUTER:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
move.w (%a0),(-2,%fp)
tst.w (-2,%fp)
ble.s 1f
bsr.s TWO
1:
unlk %fp
rts
.globl PTARGETS_$OUTER
PTARGETS_$OUTER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (8,%fp)
pea (%a5)
bsr.s OUTER
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PTARGETS_$LOCAL
PTARGETS_$LOCAL:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%a5),(-4,%fp)
move.l (-4,%fp),-(%sp)
bsr.w BUMP
addq.w #4,%sp
move.l (0x18,%a5),(-8,%fp)
movea.l (-8,%fp),%a0
pea (6,%a0)
bsr.w BUMP
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PTARGETS_$STATIC
PTARGETS_$STATIC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x18,%a5),%a0
pea (6,%a0)
bsr.w BUMP
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PTARGETS","aw",@progbits
.balign 4
.Ldata:
.space 21
.byte 0x64
.space 6
