.text
.globl MARK0
MARK0:
link.w %fp,#0
subq.l #2,%sp
clr.w -(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.globl ELEM_HELD_TEST_$SETUP
ELEM_HELD_TEST_$SETUP:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d0
lea (%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.l #EXEC_$LOGV_L,(0x1c,%a1)
move.l #EXEC_$LOGV_L,(0x20,%a1)
lea (0x10,%a0),%a0
dbf %d0,2b
clr.l (0x3c,%a5)
clr.l (0x4c,%a5)
clr.l (0x30,%a5)
moveq #3,%d0
clr.w %d1
lea (%a5),%a0
.balignw 4,0x2048
5:
movea.l %a0,%a1
move.w %d1,%d2
ext.l %d2
addi.l #0x64,%d2
move.l %d2,(0x58,%a1)
move.w #1,(0x5c,%a1)
move.w %d1,(0x5e,%a1)
addq.w #1,%d1
addq.l #8,%a0
dbf %d0,5b
clr.w (0x6e,%a5)
move.l #0x12c,(0x68,%a5)
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$ENABLE
ELEM_HELD_TEST_$ENABLE:
link.w %fp,#-4
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
lea (0,%a5,%d1.w),%a3
move.w (0x5e,%a3),%d0
lsl.w #4,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x1c,%a2)
beq.s 1f
move.l (0x58,%a3),-(%sp)
movea.l (0x1c,%a2),%a0
jsr (%a0)
addq.w #4,%sp
1:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x10,%fp),%a2-%a3/%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$RELEASE
ELEM_HELD_TEST_$RELEASE:
link.w %fp,#-4
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
lea (0,%a5,%d1.w),%a3
move.w (0x5e,%a3),%d0
lsl.w #4,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x1c,%a2)
beq.s 1f
move.l (0x58,%a3),-(%sp)
movea.l (0x1c,%a2),%a0
jsr (%a0)
addq.w #4,%sp
1:
move.l (0x58,%a3),%d0
addq.l #1,%d0
move.l %d0,-(%sp)
move.w (0x5e,%a3),%d0
lsl.w #4,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x20,%a0),%a1
jsr (%a1)
clr.w (0x5c,%a3)
movem.l (-0x10,%fp),%a2-%a3/%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$OTHER_FIELD
ELEM_HELD_TEST_$OTHER_FIELD:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d3
lsl.w #4,%d3
tst.l (0x20,%a5,%d3.w)
beq.s 1f
move.w %d2,%d0
ext.l %d0
move.l %d0,-(%sp)
lea (0,%a5,%d3.w),%a0
movea.l (0x1c,%a0),%a1
jsr (%a1)
addq.w #4,%sp
1:
move.w %d2,%d0
lsl.w #4,%d0
tst.l (0x20,%a5,%d0.w)
bne.s 2f
pea (7).w
jsr (EXEC_$LOGV_L).l
2:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$WITH_ELSE
ELEM_HELD_TEST_$WITH_ELSE:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d3
lsl.w #4,%d3
tst.l (0x20,%a5,%d3.w)
beq.s 1f
move.w %d2,%d0
ext.l %d0
addq.l #2,%d0
move.l %d0,-(%sp)
lea (0,%a5,%d3.w),%a0
movea.l (0x20,%a0),%a1
jsr (%a1)
bra.s 3f
1:
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
3:
addq.w #4,%sp
2:
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BUMP
BUMP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
addq.w #1,(0x78,%a5)
move.w (0x78,%a5),%d0
ext.l %d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$CHANGING
ELEM_HELD_TEST_$CHANGING:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0x78,%a5)
move.l #MARK0,(0x2c,%a5)
move.w (0x78,%a5),%d0
lsl.w #4,%d0
tst.l (0x1c,%a5,%d0.w)
beq.s 1f
bsr.s BUMP
move.l %d0,-(%sp)
move.w (0x78,%a5),%d0
lsl.w #4,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x1c,%a0),%a1
jsr (%a1)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BUMP_NIL
BUMP_NIL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
addq.w #1,(0x78,%a5)
movea.l #0,%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$CHANGING_TEST
ELEM_HELD_TEST_$CHANGING_TEST:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w (0x78,%a5)
move.l #MARK0,(0x2c,%a5)
move.w (0x78,%a5),%d0
lsl.w #4,%d0
move.l (0x1c,%a5,%d0.w),%d2
bsr.s BUMP_NIL
move.l %a0,%d0
cmp.l %d0,%d2
beq.s 1f
pea (5).w
move.w (0x78,%a5),%d0
lsl.w #4,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x1c,%a0),%a1
jsr (%a1)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl ELEM_HELD_TEST_$TAGS
ELEM_HELD_TEST_$TAGS:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d2
lea (%a5),%a2
.balignw 4,0x2048
2:
move.w (0x5c,%a2),%d0
ext.l %d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
addq.l #8,%a2
dbf %d2,2b
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELEM_HELD_TEST","aw",@nobits
.balign 4
.Ldata:
.space 124
