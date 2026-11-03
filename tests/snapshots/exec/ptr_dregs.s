.text
.globl BUMP
BUMP:
link.w %fp,#0
movea.l (8,%fp),%a0
addq.l #1,(%a0)
movea.l (0xc,%fp),%a1
clr.l (%a1)
unlk %fp
rts
.globl WORK
WORK:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
movea.l (0xc,%fp),%a0
move.l %d0,%d1
subq.l #4,%d1
move.l %d1,(8,%a5)
movea.l %d0,%a1
tst.l (%a1)
bne.s 1f
move.w #1,(4,%a1)
1:
moveq #2,%d1
move.l %d1,(%a1)
clr.l (%a0)
cmpi.l #2,(%a1)
bne.s 2f
moveq #7,%d1
move.l %d1,(%a0)
bra.s 3f
2:
clr.l (8,%a5)
3:
addq.l #3,(%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AGAIN
AGAIN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l %d2,%d0
subq.l #4,%d0
move.l %d0,(8,%a5)
move.l (0xc,%fp),-(%sp)
move.l %d2,-(%sp)
bsr.s BUMP
movea.l %d2,%a0
move.w #6,(4,%a0)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl PTR_DREGS_$MAIN
PTR_DREGS_$MAIN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
clr.w (4,%a5)
pea (-4,%fp)
pea (%a5)
bsr.w WORK
addq.w #8,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
moveq #-1,%d0
move.l %d0,(%a5)
pea (-4,%fp)
pea (%a5)
bsr.s AGAIN
addq.w #8,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTR_DREGS","aw",@nobits
.balign 4
.Ldata:
.space 12
