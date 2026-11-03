.text
.globl BCR_$YES
BCR_$YES:
link.w %fp,#-4
st %d0
unlk %fp
rts
.globl BCR_$NO
BCR_$NO:
link.w %fp,#-4
clr.b %d0
unlk %fp
rts
.globl BCR_$MAIN
BCR_$MAIN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
bsr.s BCR_$YES
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
bsr.s BCR_$NO
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
bsr.s BCR_$YES
move.l %d0,%d2
bsr.s BCR_$NO
move.l %d0,%d1
not.b %d1
and.b %d1,%d2
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BOOL_CONST_RESULT","aw",@nobits
.balign 4
.Ldata:
.space 4
