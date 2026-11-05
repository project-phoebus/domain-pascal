.text
.globl WAIT
WAIT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
clr.w (%a0)
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
cmpi.w #5,(%a0)
blt.s 1b
move.w (%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PARAM_ATTRS_$WAIT
PARAM_ATTRS_$WAIT:
link.w %fp,#-4
move.l #EXEC_$COUNTER,-(%sp)
bsr.s WAIT
unlk %fp
rts
.globl TWICE
TWICE:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
clr.w (%a2)
move.w (%a2),(2,%a5)
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (%a2),(4,%a5)
subq.l #2,%sp
move.w (4,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (%a2),%d0
add.w (%a2),%d0
move.w %d0,(2,%a5)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl PARAM_ATTRS_$TWICE
PARAM_ATTRS_$TWICE:
link.w %fp,#0
move.l #EXEC_$COUNTER,-(%sp)
bsr.s TWICE
unlk %fp
rts
.globl SET_STATUS
SET_STATUS:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #0xb002a,(%a0)
unlk %fp
rts
.globl PASS_ON
PASS_ON:
link.w %fp,#0
pea (.Lc0,%pc)
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc0:
.short 0x0004
.globl PARAM_ATTRS_$STATUS
PARAM_ATTRS_$STATUS:
link.w %fp,#-8
pea (-4,%fp)
bsr.s SET_STATUS
addq.w #4,%sp
pea (-4,%fp)
bsr.s PASS_ON
addq.w #4,%sp
move.w #0x2a,(-6,%fp)
pea (-6,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PARAM_ATTRS","aw",@nobits
.balign 4
.Ldata:
.space 8
