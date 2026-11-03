.text
.globl EB_$WORD
EB_$WORD:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d0
unlk %fp
rts
.globl EB_$REC_B
EB_$REC_B:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$RECS,%a0
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d0
add.w %d0,%d0
add.w %d0,%d1
move.l (4,%a0,%d1.w),%d0
unlk %fp
rts
.globl EB_$REC_C_K
EB_$REC_C_K:
link.w %fp,#-4
move.w (EB_$K).l,%d0
movea.l #EB_$RECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
move.l (8,%a0,%d0.w),%d0
unlk %fp
rts
.globl EB_$BIG_Z
EB_$BIG_Z:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
movea.l #EB_$BIG,%a0
add.w %d0,%d1
move.l (-4,%a0,%d1.w),%d0
unlk %fp
rts
.globl EB_$BIG_A
EB_$BIG_A:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d2
jsr (EXEC_$CLOBBER).l
move.w %d2,%d0
lsl.w #3,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #3,%d1
movea.l #EB_$BIG,%a0
add.w %d1,%d0
move.l (-0x38,%a0,%d0.w),%d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl EB_$BYTE
EB_$BYTE:
link.w %fp,#-4
clr.w %d0
movea.l #EB_$BYTES,%a0
move.b (8,%fp),%d0
move.b (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl EB_$THROUGH
EB_$THROUGH:
link.w %fp,#-4
clr.w %d0
movea.l (EB_$PTR).l,%a0
move.b (8,%fp),%d0
move.b (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl EB_$SET
EB_$SET:
link.w %fp,#0
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w #-7,(-2,%a0,%d1.w)
unlk %fp
rts
.globl EB_$LOG_REC
EB_$LOG_REC:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d2
movea.l #EB_$RECS,%a0
move.w %d2,%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (0,%a0,%d0.w)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w %d2,%d0
movea.l #EB_$RECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (4,%a0,%d0.w)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w %d2,%d0
movea.l #EB_$RECS,%a0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (8,%a0,%d0.w)
jsr (EXEC_$LOG_L).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl EB_$NEGEL
EB_$NEGEL:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$NEG,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (0xa,%a0,%d1.w),%d0
unlk %fp
rts
.globl EB_$LONG
EB_$LONG:
link.w %fp,#-4
move.l (8,%fp),%d0
movea.l #EB_$LONGS,%a0
move.l %d0,%d1
lsl.l #2,%d1
move.l (0,%a0,%d1.l),%d0
unlk %fp
rts
.globl EB_$NEXT
EB_$NEXT:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (0,%a0,%d1.w),%d0
unlk %fp
rts
.globl EB_$COND
EB_$COND:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
cmpi.w #-0x1f4,(-2,%a0,%d1.w)
bge.s 1f
moveq #1,%d0
bra.s 2f
1:
moveq #2,%d0
2:
unlk %fp
rts
.globl EB_$BYTE_W
EB_$BYTE_W:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$BYTES,%a0
move.b (0,%a0,%d0.w),%d0
unlk %fp
rts
.globl EB_$TWO
EB_$TWO:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d2
move.w %d0,%d1
movea.l #EB_$WORDS,%a0
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d3
jsr (EXEC_$CLOBBER).l
move.w %d2,%d1
movea.l #EB_$WORDS,%a0
add.w %d1,%d1
move.w %d3,%d0
add.w (-2,%a0,%d1.w),%d0
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl EB_$PUT
EB_$PUT:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (0xa,%fp),%d0
add.w %d0,%d0
move.w (8,%fp),%d1
addq.w #1,%d0
move.w %d1,%d2
lsl.w #2,%d0
movea.l #EB_$WORDS,%a0
add.w %d2,%d2
move.w %d0,(-2,%a0,%d2.w)
move.l (-4,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
