.text
.globl PAS_$NEW
PAS_$NEW:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xc,%fp),%d2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w (0x100,%a5),%d0
movea.l (8,%fp),%a1
lsl.w #2,%d0
lea (-4,%a5,%d0.w),%a0
move.l %a0,(%a1)
move.l %d2,%d0
addq.l #3,%d0
bpl.s 1f
addq.l #3,%d0
1:
asr.l #2,%d0
add.w (0x100,%a5),%d0
move.w %d0,(0x100,%a5)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl PAS_$DISPOSE
PAS_$DISPOSE:
link.w %fp,#0
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (8,%fp),%a0
clr.l (%a0)
unlk %fp
rts
.globl NEW_DISPOSE_$RUN
NEW_DISPOSE_$RUN:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w #1,(0x100,%a5)
pea (6).w
pea (-4,%fp)
jsr (PAS_$NEW).l
addq.w #8,%sp
pea (0xc).w
pea (-8,%fp)
jsr (PAS_$NEW).l
addq.w #8,%sp
movea.l (-4,%fp),%a0
move.w #0x64,(%a0)
movea.l (-4,%fp),%a0
move.w #0xc8,(2,%a0)
moveq #1,%d0
movea.l (-8,%fp),%a0
move.l %d0,(%a0)
moveq #3,%d1
movea.l (-8,%fp),%a0
move.l %d1,(8,%a0)
movea.l (-8,%fp),%a0
move.l (%a0),%d0
add.l %d1,%d0
addq.l #8,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
movea.l (-4,%fp),%a0
movea.l (-4,%fp),%a1
move.w (%a0),%d0
move.w (2,%a1),%d1
ext.l %d0
ext.l %d1
add.l %d1,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
tst.l (-4,%fp)
beq.s 1f
pea (1).w
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
1:
pea (6).w
pea (-4,%fp)
jsr (PAS_$DISPOSE).l
addq.w #8,%sp
tst.l (-4,%fp)
bne.s 2f
pea (1).w
jsr (EXEC_$LOGV_L).l
2:
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NEW_DISPOSE","aw",@nobits
.balign 4
.Ldata:
.space 260
