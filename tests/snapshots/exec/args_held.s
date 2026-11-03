.text
.globl ARGS_HELD_$THEN
ARGS_HELD_$THEN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.b (0xa,%fp),%d0
move.w %d2,(%a5)
tst.b %d0
bpl.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl ARGS_HELD_$SHOW
ARGS_HELD_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ARGS_HELD_$PARTS
ARGS_HELD_$PARTS:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
clr.w (%a5)
movea.l (0xc,%fp),%a0
tst.b (%a0)
bpl.s 1f
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (EXEC_$LOGV_W).l
bra.s 2f
1:
pea (%a2)
jsr (EXEC_$LOG_W).l
2:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl ARGS_HELD_$OTHER
ARGS_HELD_$OTHER:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a0
tst.b (%a0)
bpl.s 1f
move.w (%a2),%d0
subq.w #4,%d0
move.w %d0,(%a5)
bra.s 2f
1:
clr.w (%a5)
pea (%a2)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
2:
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl ARGS_HELD_$JOIN
ARGS_HELD_$JOIN:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 1f
tst.w (%a5)
ble.s 2f
move.w (%a5),%d0
add.w (%a2),%d0
move.w %d0,(%a5)
2:
pea (%a2)
jsr (EXEC_$LOG_W).l
bra.s 3f
1:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
3:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl ARGS_HELD_$PREFIX
ARGS_HELD_$PREFIX:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.w (%a2),%d0
addq.w #1,%d0
move.w %d0,(%a5)
pea (%a2)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
tst.w (%a5)
ble.s 1f
subq.w #1,(%a5)
1:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl ARGS_HELD_$CASE
ARGS_HELD_$CASE:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d2
move.w %d2,%d0
cmpi.w #1,%d0
beq.s 2f
cmpi.w #2,%d0
beq.s 3f
bra.s 4f
2:
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
5:
jsr (EXEC_$LOGV_W).l
bra.s 1f
3:
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bra.s 5b
4:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
1:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl ARGS_HELD_$EARLY
ARGS_HELD_$EARLY:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.b (0xa,%fp),%d0
move.w %d2,(%a5)
tst.b %d0
bpl.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
bra.s 2f
1:
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl ARGS_HELD_$FOUR
ARGS_HELD_$FOUR:
link.w %fp,#0
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w (0xc,%fp),%d4
move.w (0xe,%fp),%d5
move.w (0x10,%fp),%d0
move.w %d2,%d1
add.w %d3,%d1
add.w %d4,%d1
add.w %d5,%d1
move.w %d1,(%a5)
tst.w %d0
beq.s 1f
pea (%a5)
jsr (EXEC_$LOG_W).l
bra.s 2f
1:
subq.l #2,%sp
move.w %d5,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$LOGV_W).l
2:
movem.l (-0x14,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl ARGS_HELD_$VARS
ARGS_HELD_$VARS:
link.w %fp,#0
movem.l %a2-%a4,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
movea.l (0x10,%fp),%a4
move.w (0x14,%fp),%d0
beq.s 1f
addi.w #0,(%a2)
addi.w #0,(%a3)
addi.w #0,(%a4)
pea (%a2)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (%a3)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (%a4)
bra.s 3f
1:
addi.w #0,(%a4)
addi.w #0,(%a2)
addi.w #0,(%a3)
pea (%a4)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (%a2)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (%a3)
3:
jsr (EXEC_$LOG_W).l
2:
movem.l (-0xc,%fp),%a2-%a4
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ARGS_HELD","aw",@nobits
.balign 4
.Ldata:
.space 4
