.text
.globl BE_L_ARGS_$VARS
BE_L_ARGS_$VARS:
link.w %fp,#-0xc
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
move.w %d0,(-2,%fp)
move.l %d1,(-8,%fp)
ext.l %d0
move.l %d0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w (-6,%fp),(-0xa,%fp)
pea (-0xa,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BE_L_ARGS_$EXPRS
BE_L_ARGS_$EXPRS:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.l (0xa,%fp),%d3
move.w %d2,%d0
ext.l %d0
addq.l #1,%d0
move.l %d0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.l %d3,%d0
lsr.l #1,%d0
add.l %d0,%d0
subi.l #0xd6,%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BE_L_ARGS_$FIELD
BE_L_ARGS_$FIELD:
link.w %fp,#-8
move.l (8,%fp),%d0
move.l %d0,(-4,%fp)
move.w (-4,%fp),(-6,%fp)
pea (-6,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w (-2,%fp),(-6,%fp)
pea (-6,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
