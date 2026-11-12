.text
.globl F
F:
link.w %fp,#-8
move.w (8,%fp),%d0
move.b #0x61,(-4,%fp)
move.b #0x62,(-3,%fp)
move.b #0x63,(-2,%fp)
move.b %d0,(-1,%fp)
move.l (-4,%fp),(-8,%fp)
move.l (-8,%fp),%d0
unlk %fp
rts
.globl ARRAY_RESULT_$RUN
ARRAY_RESULT_$RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s F
addq.w #4,%sp
move.l %d0,(%a5)
subq.l #2,%sp
clr.w %d0
move.b (%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w %d0
move.b (1,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w %d0
move.b (2,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w %d0
move.b (3,%a5),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ARRAY_RESULT","aw",@nobits
.balign 4
.Ldata:
.space 4
