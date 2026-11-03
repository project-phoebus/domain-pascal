.text
.globl BE_L_STMTS_$SELECT
BE_L_STMTS_$SELECT:
link.w %fp,#-0xc
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
move.l %d1,(8,%a5)
lea (8,%a5),%a0
move.l %a0,(-8,%fp)
move.w %d0,%d1
movea.l (-8,%fp),%a0
move.w %d1,%d0
ext.l %d0
cmp.l (%a0),%d0
bge.s 1f
movea.l (-8,%fp),%a0
move.w (2,%a0),%d1
1:
move.w %d1,%d0
movea.l (-0x10,%fp),%a5
unlk %fp
rts
.globl BE_L_STMTS_$WHILE
BE_L_STMTS_$WHILE:
link.w %fp,#-4
clr.w (EXEC_$COUNTER).l
bra.w 2f
.balignw 4,0x2048
1:
2:
cmpi.w #5,(EXEC_$COUNTER).l
blt.s 1b
move.w (EXEC_$COUNTER).l,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BE_L_STMTS_$WITH
BE_L_STMTS_$WITH:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
lea (%a5),%a0
move.l %a0,(4,%a5)
movea.l (4,%a5),%a0
move.w %d0,(%a0)
move.w %d0,%d1
addq.w #1,%d1
move.w %d1,(2,%a0)
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
.globl BE_L_STMTS_$FOR
BE_L_STMTS_$FOR:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d0
.balignw 4,0x2048
2:
dbf %d0,2b
1:
pea (.Lc0,%pc)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.Lc0:
.short 0x0009
.balignw 4,0x2048
.section ".bss.moddata.BE_L_STMTS","aw",@nobits
.balign 4
.Ldata:
.space 12
