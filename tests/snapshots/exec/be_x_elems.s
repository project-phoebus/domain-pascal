.text
.globl BX_ELEMS_$SUM
BX_ELEMS_$SUM:
link.w %fp,#-8
move.l %d2,-(%sp)
clr.w %d0
moveq #1,%d1
bra.s 2f
.balignw 4,0x2048
1:
move.w %d1,%d2
movea.l #EB_$WORDS,%a0
add.w %d2,%d2
addq.w #1,%d1
add.w (-2,%a0,%d2.w),%d0
2:
cmpi.w #8,%d1
ble.s 1b
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BX_ELEMS_$SET
BX_ELEMS_$SET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l #EB_$WORDS,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_ELEMS_$THROUGH
BX_ELEMS_$THROUGH:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l (%a5),%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BX_ELEMS_$SELF
BX_ELEMS_$SELF:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d0
ext.l %d0
addi.l #0x3e8,%d0
bpl.s 1f
addi.l #0x3f,%d0
1:
asr.l #6,%d0
movea.l #EB_$WORDS,%a0
add.w %d0,%d0
move.w (-2,%a0,%d0.w),%d0
unlk %fp
rts
.globl BX_ELEMS_$KEPT
BX_ELEMS_$KEPT:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #3,%d0
movea.l #EB_$BIG,%a0
add.w %d0,%d1
move.l (-0x38,%a0,%d1.w),%d0
add.l (-4,%a0,%d1.w),%d0
unlk %fp
rts
.globl BX_ELEMS_$TWO
BX_ELEMS_$TWO:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (0xa,%fp),%d0
movea.l #EB_$NEG,%a0
move.w %d0,%d1
move.w (8,%fp),%d0
add.w %d1,%d1
move.w %d0,%d2
movea.l #EB_$WORDS,%a1
add.w %d2,%d2
move.w (0xa,%a0,%d1.w),(-2,%a1,%d2.w)
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BX_ELEMS_$WORD
BX_ELEMS_$WORD:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #EB_$WORDS,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w (-2,%a0,%d1.w),%d0
unlk %fp
rts
.globl BX_ELEMS_$UPD
BX_ELEMS_$UPD:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d3
add.w %d3,%d3
move.w #0xa,(2,%a5,%d3.w)
move.w #2,(4,%a5,%d3.w)
move.w #7,(6,%a5,%d3.w)
move.w #4,(8,%a5,%d3.w)
addq.w #5,(2,%a5,%d3.w)
subq.w #7,(4,%a5,%d3.w)
andi.w #6,(6,%a5,%d3.w)
ori.w #0x10,(8,%a5,%d3.w)
pea (2,%a5,%d3.w)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
add.w %d0,%d0
pea (4,%a5,%d0.w)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
add.w %d0,%d0
pea (6,%a5,%d0.w)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
add.w %d0,%d0
pea (8,%a5,%d0.w)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BX_ELEMS_$UPDF
BX_ELEMS_$UPDF:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d2
move.w %d2,%d0
add.w %d0,%d0
move.w #0xa,(-0xa,%fp,%d0.w)
jsr (EXEC_$CLOBBER).l
move.w %d2,%d0
add.w %d0,%d0
addq.w #2,(-0xa,%fp,%d0.w)
move.w %d2,%d0
add.w %d0,%d0
pea (-0xa,%fp,%d0.w)
jsr (EXEC_$LOG_W).l
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BX_ELEMS_$UPDP
BX_ELEMS_$UPDP:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
lea (4,%a5),%a0
move.l %a0,(0x14,%a5)
move.w %d2,%d3
movea.l (0x14,%a5),%a0
add.w %d3,%d3
move.w #7,(-2,%a0,%d3.w)
movea.l (0x14,%a5),%a0
move.w #6,(0,%a0,%d3.w)
movea.l (0x14,%a5),%a0
move.w #7,(2,%a0,%d3.w)
movea.l (0x14,%a5),%a0
addi.w #0x3e8,(-2,%a0,%d3.w)
movea.l (0x14,%a5),%a0
subi.w #0x64,(0,%a0,%d3.w)
movea.l (0x14,%a5),%a0
andi.w #3,(2,%a0,%d3.w)
movea.l (0x14,%a5),%a0
pea (-2,%a0,%d3.w)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
movea.l (0x14,%a5),%a0
add.w %d0,%d0
pea (0,%a0,%d0.w)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
movea.l (0x14,%a5),%a0
add.w %d0,%d0
pea (2,%a0,%d0.w)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_ELEMS","aw",@nobits
.balign 4
.Ldata:
.space 24
