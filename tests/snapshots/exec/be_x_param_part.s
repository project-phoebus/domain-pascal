.text
.globl BE_X_PARAM_PART_$P
BE_X_PARAM_PART_$P:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x63,(8,%fp)
move.l (8,%fp),(%a5)
move.w (%a5),(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w (2,%a5),(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BE_X_PARAM_PART_$Q
BE_X_PARAM_PART_$Q:
link.w %fp,#-4
move.w #0x63,(8,%fp)
move.w (8,%fp),(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w (0xa,%fp),(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_PARAM_PART","aw",@nobits
.balign 4
.Ldata:
.space 4
