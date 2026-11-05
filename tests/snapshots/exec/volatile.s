.text
.globl VOLATILE_$WAIT
VOLATILE_$WAIT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (EXEC_$COUNTER).l
clr.w (%a5)
.balignw 4,0x2048
1:
addq.w #1,(%a5)
cmpi.w #5,(EXEC_$COUNTER).l
blt.s 1b
move.w (%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl VOLATILE_$TWICE
VOLATILE_$TWICE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (EXEC_$COUNTER).l
move.w (EXEC_$COUNTER).l,(2,%a5)
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (EXEC_$COUNTER).l,(4,%a5)
subq.l #2,%sp
move.w (4,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w (EXEC_$COUNTER).l,%d0
add.w (EXEC_$COUNTER).l,%d0
move.w %d0,(2,%a5)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.VOLATILE","aw",@nobits
.balign 4
.Ldata:
.space 8
