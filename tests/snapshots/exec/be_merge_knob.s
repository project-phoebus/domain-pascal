.text
FOUR:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.l #0x10002,(%a0)
move.w #3,(4,%a0)
move.w #4,(6,%a0)
unlk %fp
rts
.globl BE_MERGE_KNOB_$FOUR
BE_MERGE_KNOB_$FOUR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (%a5)
clr.w (2,%a5)
jsr (EXEC_$CLOBBER).l
clr.w (4,%a5)
jsr (EXEC_$CLOBBER).l
clr.w (6,%a5)
pea (%a5)
bsr.s FOUR
addq.w #4,%sp
pea (%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (2,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (6,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_MERGE_KNOB_$THREE
BE_MERGE_KNOB_$THREE:
link.w %fp,#-8
move.w #3,(-6,%fp)
move.w #2,(-4,%fp)
move.w #1,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-6,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BE_MERGE_KNOB_$DOWN
BE_MERGE_KNOB_$DOWN:
link.w %fp,#-4
move.w #0xa,(-2,%fp)
move.w #0x14,(-4,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BE_MERGE_KNOB_$EXT
BE_MERGE_KNOB_$EXT:
link.w %fp,#0
move.w #7,(BM_$EXT).l
move.w #8,(BM_$EXT+2).l
move.l #BM_$EXT,-(%sp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (BM_$EXT+2).l
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_MERGE_KNOB","aw",@nobits
.balign 4
.Ldata:
.space 8
