.text
.globl COMPARES
COMPARES:
link.w %fp,#0
cmpi.l #0x3e8,(U).l
bls.s 1f
move.w #1,(C).l
1:
move.l (U).l,%d0
cmp.l (V).l,%d0
bhi.s 2f
move.w #2,(C).l
2:
move.l (L).l,%d0
cmp.l (U).l,%d0
bge.s 3f
move.w #3,(C).l
3:
move.w (W).l,%d0
ext.l %d0
cmp.l (U).l,%d0
bge.s 4f
move.w #4,(C).l
4:
unlk %fp
rts
.globl ARITH
ARITH:
link.w %fp,#0
subq.l #2,%sp
move.w #0x3c,-(%sp)
move.l (V).l,-(%sp)
jsr (M$DIU$LLW).l
addq.w #8,%sp
move.l %d0,(U).l
move.l (U).l,-(%sp)
move.l (V).l,-(%sp)
jsr (M$DIU$LLL).l
addq.w #8,%sp
move.l %d0,(U).l
move.l (U).l,-(%sp)
move.l (V).l,-(%sp)
jsr (M$OIU$LLL).l
addq.w #8,%sp
move.l %d0,(U).l
move.l (V).l,%d0
lsr.l #8,%d0
lsr.l #2,%d0
move.l %d0,(U).l
move.l (U).l,-(%sp)
move.l (V).l,-(%sp)
jsr (M$MIU$LLL).l
addq.w #8,%sp
move.l %d0,(U).l
move.l (V).l,-(%sp)
move.l (L).l,-(%sp)
jsr (M$DIU$LLL).l
addq.w #8,%sp
move.l %d0,(U).l
move.l (V).l,-(%sp)
move.l (L).l,-(%sp)
jsr (M$DIS$LLL).l
move.l %d0,(L).l
subq.l #1,(U).l
unlk %fp
rts
.balignw 4,0x2048
