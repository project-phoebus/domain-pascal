.text
.globl DIVMOD_$W
DIVMOD_$W:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d2,%d0
ext.l %d0
divs.w %d3,%d0
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d0
ext.l %d0
divs.w %d3,%d0
swap %d0
move.w %d0,(2,%a5)
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl DIVMOD_$L
DIVMOD_$L:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l (0xc,%fp),%d3
move.l %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$DIS$LLL).l
addq.w #8,%sp
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$LLL).l
addq.w #8,%sp
move.l %d0,(8,%a5)
move.l (8,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl DIVMOD_$POW2
DIVMOD_$POW2:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d3
move.w %d2,%d0
bpl.s 1f
addq.w #3,%d0
1:
asr.w #2,%d0
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d0
ext.l %d0
divs.w #4,%d0
swap %d0
move.w %d0,(2,%a5)
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l %d3,%d0
bpl.s 2f
addq.l #3,%d0
2:
asr.l #2,%d0
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl DIVMOD_$WIDEN
DIVMOD_$WIDEN:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
subq.l #2,%sp
ext.l %d2
move.w %d1,-(%sp)
move.l %d2,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
move.l %d0,(0xc,%a5)
move.l (0xc,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.DIVMOD","aw",@nobits
.balign 4
.Ldata:
.space 16
