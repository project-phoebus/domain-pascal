.text
.globl BE_MERGE_$COPY
BE_MERGE_$COPY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #5,(%a5)
jsr (EXEC_$CLOBBER).l
move.w #6,(2,%a5)
jsr (EXEC_$CLOBBER).l
move.w (%a5),(4,%a5)
move.w (2,%a5),(6,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (6,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_MERGE_$BYTES
BE_MERGE_$BYTES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b #0x61,(8,%a5)
move.b #0x62,(9,%a5)
pea (8,%a5)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
pea (9,%a5)
jsr (EXEC_$LOG_C).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_MERGE_$EXT
BE_MERGE_$EXT:
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
.section ".bss.moddata.BE_MERGE","aw",@nobits
.balign 4
.Ldata:
.space 12
