.text
.globl TAKEB
TAKEB:
link.w %fp,#-4
move.b (8,%fp),%d0
clr.w %d1
move.b %d0,%d1
move.w %d1,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl TAKEW
TAKEW:
link.w %fp,#0
pea (8,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BE_L_NARROW_$MAIN
BE_L_NARROW_$MAIN:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l %d2,%d0
lsr.l #3,%d0
move.b %d0,(%a5)
clr.w %d0
move.b (%a5),%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.l %d2,%d0
lsr.l #3,%d0
move.b %d0,-(%sp)
bsr.s TAKEB
addq.w #4,%sp
move.l %d2,%d0
move.l %d0,%d1
add.l %d1,%d1
add.l %d1,%d0
move.l %d0,(0x10,%a5)
subq.l #2,%sp
move.l (0x10,%a5),%d0
move.w %d0,-(%sp)
bsr.s TAKEW
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BE_L_NARROW_$RES
BE_L_NARROW_$RES:
link.w %fp,#-4
move.l (8,%fp),%d0
lsr.l #3,%d0
unlk %fp
rts
.globl BE_L_NARROW_$CALLRES
BE_L_NARROW_$CALLRES:
link.w %fp,#-4
pea (8,%fp)
jsr (EXEC_$ID_L).l
unlk %fp
rts
.globl BE_L_NARROW_$TRANSFER
BE_L_NARROW_$TRANSFER:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
jsr (M$MIS$LLL).l
unlk %fp
rts
.globl BE_L_NARROW_$PARTS
BE_L_NARROW_$PARTS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #9,%d0
move.l %d0,(8,%a5)
clr.w %d1
move.b %d0,%d1
move.w %d1,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
movea.l (8,%fp),%a0
clr.w %d1
move.l (%a0),%d0
move.b %d0,%d1
move.w %d1,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
moveq #0x4d,%d0
move.l %d0,(0x10,%a5)
move.l (0x10,%a5),%d0
clr.w %d1
move.b %d0,%d1
move.w %d1,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_L_NARROW","aw",@nobits
.balign 4
.Ldata:
.space 20
