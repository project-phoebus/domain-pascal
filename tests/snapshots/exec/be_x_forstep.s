.text
.globl BX_FORSTEP_$FIRST
BX_FORSTEP_$FIRST:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d0
clr.w %d1
lea (%a5),%a0
.balignw 4,0x2048
2:
move.w %d1,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.w %d2,(0,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
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
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BX_FORSTEP_$TWO
BX_FORSTEP_$TWO:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #3,%d0
lea (%a5),%a0
lea (%a5),%a1
.balignw 4,0x2048
2:
move.w (0,%a0),%d1
addq.w #3,%d1
move.w %d1,(8,%a1)
addq.l #2,%a1
addq.l #2,%a0
dbf %d0,2b
pea (0xa,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BX_FORSTEP_$PTR
BX_FORSTEP_$PTR:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (0x10,%a5),%a0
move.l %a0,(0x1c,%a5)
move.w #7,(0x16,%a5)
move.w #8,(0x18,%a5)
move.w #5,(0x1a,%a5)
moveq #2,%d0
movea.l (0x1c,%a5),%a0
movea.l (0x1c,%a5),%a1
lea (2,%a0),%a2
lea (2,%a1),%a0
.balignw 4,0x2048
2:
move.w (4,%a2),(-2,%a0)
addq.l #2,%a0
addq.l #2,%a2
dbf %d0,2b
pea (0x10,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x12,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x14,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl BX_FORSTEP_$MIX
BX_FORSTEP_$MIX:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (0x20,%a5),%a0
move.l %a0,(0x30,%a5)
movea.l #EB_$WORDS,%a0
moveq #2,%d0
addq.l #4,%a0
movea.l (0x30,%a5),%a1
lea (4,%a1),%a2
.balignw 4,0x2048
2:
move.w (-2,%a0),(-2,%a2)
addq.l #2,%a2
addq.l #2,%a0
dbf %d0,2b
pea (0x22,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x24,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x26,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl OFF
OFF:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
moveq #2,%d0
moveq #1,%d1
movea.l (8,%fp),%a0
lea (2,%a0),%a1
.balignw 4,0x2048
2:
move.w %d1,%d2
add.w %d2,%d2
move.w %d2,%d3
lsl.w #2,%d3
add.w %d3,%d2
move.w %d2,(4,%a1)
addq.w #1,%d1
addq.l #2,%a1
dbf %d0,2b
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BX_FORSTEP_$OFF
BX_FORSTEP_$OFF:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (0x10,%a5)
bsr.s OFF
addq.w #4,%sp
pea (0x16,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x18,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0x1a,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_FORSTEP","aw",@nobits
.balign 4
.Ldata:
.space 52
