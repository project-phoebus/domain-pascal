.text
.globl WRAP
WRAP:
link.w %fp,#-4
subq.l #2,%sp
move.l (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
move.l %a0,(-4,%fp)
unlk %fp
rts
.globl LOCKED
LOCKED:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
move.w #0xf,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
subq.l #2,%sp
move.l (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
movea.l %a0,%a2
subq.l #2,%sp
move.w #0xf,-(%sp)
jsr (ML_$UNLOCK).l
movea.l %a2,%a0
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl STORES
STORES:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xa,%fp),%a2
subq.l #2,%sp
pea (%a2)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
move.l %a0,(%a5)
subq.l #2,%sp
pea (%a2)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
move.l %a0,(EXT_CELL).l
subq.l #2,%sp
pea (%a2)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
movea.l %a0,%a2
pea (%a2)
pea (%a2)
pea (%a2)
jsr (TAKE).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl TESTS
TESTS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (8,%a5)
move.w (4,%a5),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
cmpa.w #0,%a0
bne.s 1f
move.w #1,(4,%a5)
1:
subq.l #2,%sp
pea (8,%a5)
move.w (4,%a5),-(%sp)
jsr (LOOKUP).l
move.w (%a0),(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (8,%a5)
move.w (4,%a5),-(%sp)
jsr (LOOKUP).l
move.l %a0,%d0
clr.w (4,%a5)
move.l %d0,(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ARGS
ARGS:
link.w %fp,#0
pea (%a2)
subq.l #2,%sp
movea.l (0x10,%fp),%a0
move.w (%a0),-(%sp)
movea.l (0xc,%fp),%a1
move.w (%a1),-(%sp)
movea.l (8,%fp),%a2
move.w (%a2),-(%sp)
jsr (TAKE_VALUES).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PCALL","aw",@nobits
.balign 4
.Ldata:
.space 12
