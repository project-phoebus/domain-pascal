.text
.globl BX_CTL_$RET
BX_CTL_$RET:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 1f
pea (%a2)
jsr (EXEC_$LOG_W).l
bra.s 2f
move.w #5,(%a5)
1:
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
2:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl BX_CTL_$EXIT
BX_CTL_$EXIT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
clr.w (-2,%fp)
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,(-2,%fp)
move.w (-2,%fp),%d0
cmp.w (%a0),%d0
bne.s 4f
bra.s 3f
move.w #7,(%a5)
4:
2:
cmpi.w #0xa,(-2,%fp)
blt.s 1b
3:
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BX_CTL_$SAMEPOP
BX_CTL_$SAMEPOP:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
tst.w (%a2)
ble.s 1f
pea (%a2)
jsr (EXEC_$LOG_W).l
bra.s 3f
1:
subq.l #2,%sp
move.w (%a2),-(%sp)
jsr (EXEC_$MARK).l
3:
addq.w #4,%sp
2:
subq.l #2,%sp
move.w #7,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl BX_CTL_$CLAMP
BX_CTL_$CLAMP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a1),(4,%a5)
move.l (4,%a5),%d0
move.l (%a0),%d1
addq.l #1,%d1
cmp.l %d1,%d0
ble.s 1f
move.l (%a0),%d0
addq.l #1,%d0
move.l %d0,(4,%a5)
1:
pea (4,%a5)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_CTL_$EMPTY1
BX_CTL_$EMPTY1:
rts
.globl BX_CTL_$EMPTY2
BX_CTL_$EMPTY2:
rts
.globl BX_CTL_$SPIN
BX_CTL_$SPIN:
link.w %fp,#0
.balignw 4,0x2048
1:
pea (.Lc0,%pc)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
tst.b %d0
bpl.s 1b
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.Lc0:
.short 0xff00
.globl BX_CTL_$NEXTEND
BX_CTL_$NEXTEND:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w (%a5)
movea.l (8,%fp),%a0
move.w (%a0),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w (%a5),%d2
add.w %d1,%d2
move.w %d2,(%a5)
cmpi.w #0x64,%d2
bgt.w 3f
3:
addq.w #1,%d1
dbf %d0,2b
1:
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BX_CTL_$EXITEND
BX_CTL_$EXITEND:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (%a5)
bra.s 2f
.balignw 4,0x2048
1:
addq.w #7,(%a5)
cmpi.w #0x1e,(%a5)
bgt.s 3f
2:
cmpi.w #0x32,(%a5)
blt.s 1b
3:
pea (%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_CTL_$SUM
BX_CTL_$SUM:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w (%a5)
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.w (%a5),%d2
add.w %d1,%d2
move.w %d2,(%a5)
addq.w #1,%d1
dbf %d0,2b
1:
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BX_CTL_$FILL
BX_CTL_$FILL:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d0
move.w %d0,(%a5)
move.w %d2,%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d0
moveq #1,%d1
lea (2,%a5),%a0
.balignw 4,0x2048
2:
move.w (%a5),%d3
add.w %d1,%d3
move.w %d3,(6,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
1:
move.w %d2,%d0
add.w %d0,%d0
pea (6,%a5,%d0.w)
jsr (EXEC_$LOG_W).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BX_CTL_$RUN
BX_CTL_$RUN:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #3,%d1
move.w %d0,%d2
subq.w #1,%d2
bmi.s 1f
move.w %d2,%d0
moveq #1,%d2
.balignw 4,0x2048
2:
add.w %d2,%d1
move.w %d1,(%a5)
addq.w #1,%d2
dbf %d0,2b
1:
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BX_CTL_$TWICE
BX_CTL_$TWICE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w (%a5)
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d0
.balignw 4,0x2048
2:
addq.w #2,(%a5)
dbf %d0,2b
1:
pea (%a5)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_CTL","aw",@nobits
.balign 4
.Ldata:
.space 108
