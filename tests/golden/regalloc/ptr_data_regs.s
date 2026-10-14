.text
.globl INIT
INIT:
link.w %fp,#0
move.l (8,%fp),%d0
cmpi.l #0x3e8,%d0
bls.s 1f
movea.l %d0,%a0
clr.l (%a0)
clr.w (4,%a0)
1:
unlk %fp
rts
.globl FREE_STACK
FREE_STACK:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l %d0,%d1
subq.l #4,%d1
move.l %d1,(%a5)
movea.l %d0,%a0
move.l (4,%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl WAKE
WAKE:
link.w %fp,#0
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
move.l %d2,%d0
subq.l #4,%d0
move.l %d0,(%a5)
movea.l %d2,%a0
clr.l (%a0)
pea (%a2)
move.l %d2,-(%sp)
jsr (WAKEUP).l
movea.l %d2,%a0
clr.w (4,%a0)
move.l #2,(%a0)
clr.l (%a2)
movem.l (-0xc,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl JOINS
JOINS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
movea.l (0xc,%fp),%a0
move.l %d0,%d1
subq.l #4,%d1
move.l %d1,(%a5)
movea.l %d0,%a1
tst.l (%a1)
bne.s 1f
move.w #1,(4,%a1)
1:
move.l #2,(%a1)
clr.l (%a0)
cmpi.l #1,(%a1)
bne.s 2f
move.l #1,(%a0)
bra.s 3f
2:
clr.l (%a5)
3:
move.l #3,(%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl STORED
STORED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l %a0,(%a5)
move.l (4,%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTR_DATA_REGS","aw",@nobits
.balign 4
.Ldata:
.space 8
