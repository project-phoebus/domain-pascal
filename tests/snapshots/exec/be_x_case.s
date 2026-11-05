.text
.globl BX_CASE_$CALL
BX_CASE_$CALL:
link.w %fp,#0
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
cmpi.w #1,%d1
beq.s 2f
cmpi.w #2,%d1
beq.s 3f
bra.s 4f
2:
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
3:
subq.l #2,%sp
move.w #2,-(%sp)
bra.s 5f
4:
subq.l #2,%sp
move.w #9,-(%sp)
5:
jsr (EXEC_$MARK).l
1:
unlk %fp
rts
.globl BX_CASE_$EMPTY
BX_CASE_$EMPTY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
cmpi.w #1,%d0
beq.s 1f
cmpi.w #5,%d0
beq.s 1f
bra.w 2f
2:
subq.l #2,%sp
move.w #9,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w #8,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_CASE_$LASTFIRST
BX_CASE_$LASTFIRST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
cmpi.w #1,%d0
beq.s 2f
cmpi.w #9,%d0
beq.s 2f
bra.s 1f
2:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w #8,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_CASE_$ZERO
BX_CASE_$ZERO:
link.w %fp,#0
move.w (8,%fp),%d0
move.w %d0,%d1
addq.w #1,%d1
tst.w %d1
beq.s 2f
cmpi.w #5,%d1
beq.s 1f
bra.s 1f
2:
subq.l #2,%sp
clr.w -(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
subq.l #2,%sp
move.w #8,-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_CASE","aw",@nobits
.balign 4
.Ldata:
.space 4
