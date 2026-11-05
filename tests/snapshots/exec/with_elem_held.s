.text
.globl WITH_ELEM_HELD_$SETUP
WITH_ELEM_HELD_$SETUP:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d0
moveq #1,%d1
lea (0xe,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
clr.w (-0xe,%a1)
moveq #0x64,%d2
muls.w %d1,%d2
move.l %d2,(-0xc,%a1)
move.w %d1,%d2
ext.l %d2
add.l %d2,%d2
move.l %d2,%d3
lsl.l #2,%d3
add.l %d3,%d2
move.l %d2,(-8,%a1)
clr.l (-4,%a1)
addq.w #1,%d1
lea (0xe,%a0),%a0
dbf %d0,2b
move.w #1,(0x1c,%a5)
moveq #0x28,%d0
move.l %d0,(0x14,%a5)
moveq #0xa,%d1
move.l %d1,(0x10,%a5)
move.l #EXEC_$LOG_L,(0x26,%a5)
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl WITH_ELEM_HELD_$SHOW
WITH_ELEM_HELD_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (0x38,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl WITH_ELEM_HELD_$PUSH
WITH_ELEM_HELD_$PUSH:
link.w %fp,#-8
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.w (-0xe,%a2)
bne.s 1f
pea (-0xe,%a2)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
bra.s 2f
1:
move.w #1,(0x38,%a5)
2:
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.w (-0xe,%a2)
beq.s 3f
pea (-8,%a2)
jsr (EXEC_$LOG_L).l
3:
movem.l (-0x14,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl WITH_ELEM_HELD_$FEED
WITH_ELEM_HELD_$FEED:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a5,%d1.w),%a2
move.l (-0xc,%a2),%d0
cmp.l (-8,%a2),%d0
ble.s 1f
sub.l (-8,%a2),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
bra.s 2f
1:
move.w #2,(0x38,%a5)
2:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl WITH_ELEM_HELD_$TARGET
WITH_ELEM_HELD_$TARGET:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a5,%d1.w),%a2
tst.l (-4,%a2)
beq.s 1f
pea (-0xc,%a2)
movea.l (-4,%a2),%a0
jsr (%a0)
addq.w #4,%sp
bra.s 2f
1:
move.w #3,(0x38,%a5)
2:
pea (0x38,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.WITH_ELEM_HELD","aw",@nobits
.balign 4
.Ldata:
.space 60
