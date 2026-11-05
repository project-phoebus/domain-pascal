.text
.globl BX_STORES_$UNREAD
BX_STORES_$UNREAD:
link.w %fp,#0
move.w #3,(8,%fp)
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.globl BX_STORES_$ONE
BX_STORES_$ONE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_STORES_$SHOWG
BX_STORES_$SHOWG:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
BUMP:
link.w %fp,#-4
movea.l (%fp),%a0
addi.w #0xa,(8,%a0)
unlk %fp
rts
.globl BX_STORES_$OUTER
BX_STORES_$OUTER:
link.w %fp,#0
bsr.s BUMP
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
INNER:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a0
movea.l (8,%a0),%a1
lea (%a1),%a2
move.l %a2,(-4,%a0)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl OUTER2
OUTER2:
link.w %fp,#-4
bsr.s INNER
movea.l (-4,%fp),%a0
move.w #5,(%a0)
unlk %fp
rts
.globl BX_STORES_$ADDROUT
BX_STORES_$ADDROUT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (4,%a5)
pea (4,%a5)
bsr.s OUTER2
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_STORES_$SINGLE
BX_STORES_$SINGLE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w %d1
bset %d0,%d1
move.w %d1,(0x10,%a5)
btst #3,(0x11,%a5)
beq.s 1f
subq.l #2,%sp
move.w #3,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$MARK).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_STORES_$MAXSTORE
BX_STORES_$MAXSTORE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
lea (4,%a5),%a0
move.l %a0,(0xc,%a5)
move.w %d0,%d2
cmp.w %d1,%d2
bge.s 1f
move.w %d1,%d2
1:
move.w %d2,(2,%a0)
movea.l (0xc,%a5),%a0
pea (2,%a0)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BX_STORES_$CALLSTORE
BX_STORES_$CALLSTORE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (4,%a5),%a0
move.l %a0,(0xc,%a5)
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
movea.l (0xc,%a5),%a0
addq.w #1,%d1
move.w %d1,(2,%a0)
movea.l (0xc,%a5),%a0
pea (2,%a0)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_STORES_$SUB4
BX_STORES_$SUB4:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0xa,%d1
move.w %d0,%d2
addq.w #1,%d2
sub.w %d2,%d1
move.w %d1,%d0
move.w %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl CALL_IND
CALL_IND:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a0
pea (4,%a0)
movea.l (8,%fp),%a1
movea.l (%a1),%a2
jsr (%a2)
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl BX_STORES_$IND
BX_STORES_$IND:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l #EXEC_$LOG_W,(0x14,%a5)
move.w #0x4d,(0x18,%a5)
pea (0x14,%a5)
bsr.s CALL_IND
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DPART
DPART:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
beq.s 1f
movea.l %d2,%a0
pea (2,%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
1:
cmp.l (0xc,%a5),%d2
bne.s 2f
move.l %d2,-(%sp)
jsr (EXEC_$LOG_W).l
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BX_STORES_$DPART
BX_STORES_$DPART:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #0xa,(4,%a5)
move.w #0x14,(6,%a5)
lea (4,%a5),%a0
move.l %a0,(0xc,%a5)
cmpi.w #1,%d0
bne.s 1f
pea (%a0)
bra.s 3f
1:
clr.l -(%sp)
3:
bsr.s DPART
addq.w #4,%sp
2:
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_STORES_$SAME
BX_STORES_$SAME:
link.w %fp,#0
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
move.w (EB_$WORDS).l,%d0
add.w %d1,%d1
addq.w #1,%d0
move.w %d0,(-2,%a0,%d1.w)
unlk %fp
rts
.globl BX_STORES_$WORD
BX_STORES_$WORD:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d0
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_STORES","aw",@nobits
.balign 4
.Ldata:
.space 28
