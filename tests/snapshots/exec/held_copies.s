.text
.globl HELD_$REFCNT
HELD_$REFCNT:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a1),%d0
cmpi.l #-1,%d0
bne.s 1f
clr.w (%a0)
bra.s 2f
1:
cmpi.l #0xfff5,%d0
bge.s 3f
move.w %d0,(%a0)
bra.s 4f
3:
move.w #-0xb,(%a0)
4:
2:
unlk %fp
rts
.globl HELD_$ACROSS
HELD_$ACROSS:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a0),%d2
move.b (%a1),%d3
jsr (EXEC_$CLOBBER).l
tst.b %d3
bpl.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
jsr (EXEC_$CLOBBER).l
subq.l #2,%sp
move.w %d2,%d0
addq.w #1,%d0
move.w %d0,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl HELD_$STORE
HELD_$STORE:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (%a0),%d0
move.b (%a1),%d2
move.w %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w %d0
move.b %d2,%d0
addq.w #1,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.HELD_COPIES","aw",@nobits
.balign 4
.Ldata:
.space 4
