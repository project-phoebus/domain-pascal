.text
.globl PARAM_ASSIGN_$ROUND
PARAM_ASSIGN_$ROUND:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d2
subq.l #2,%sp
clr.w -(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addi.l #0x7fff,%d2
andi.l #-0x8000,%d2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d2,(0xc,%fp)
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl PARAM_ASSIGN_$COUNT
PARAM_ASSIGN_$COUNT:
link.w %fp,#0
move.l %d2,-(%sp)
subq.l #2,%sp
move.w (8,%fp),%d2
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
bra.s 2f
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.w #3,%d2
2:
tst.w %d2
bgt.s 1b
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl PARAM_ASSIGN_$CLAMP
PARAM_ASSIGN_$CLAMP:
link.w %fp,#0
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
subq.l #2,%sp
move.w (0xa,%fp),%d3
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmpi.w #0x11,%d2
ble.s 1f
moveq #5,%d2
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
cmp.w %d3,%d2
ble.s 2f
move.w %d3,%d2
2:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2-%d3
unlk %fp
rts
.globl PARAM_ASSIGN_$STRIP
PARAM_ASSIGN_$STRIP:
link.w %fp,#0
movem.l %d2-%d3,-(%sp)
move.b (8,%fp),%d2
subq.l #2,%sp
move.b (0xa,%fp),%d3
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
tst.b %d3
bpl.s 1f
andi.b #0x7f,%d2
1:
subq.l #2,%sp
clr.w %d0
move.b %d2,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2-%d3
unlk %fp
rts
.globl PARAM_ASSIGN_$SHIFT
PARAM_ASSIGN_$SHIFT:
link.w %fp,#0
move.l %d2,-(%sp)
subq.l #2,%sp
move.l (8,%fp),%d2
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
lsr.l #3,%d2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
addq.l #1,%d2
lsl.l #2,%d2
subq.l #4,%d2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-4,%fp),%d2
unlk %fp
rts
SHOW:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
move.l (8,%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PARAM_ASSIGN_$UP
PARAM_ASSIGN_$UP:
link.w %fp,#0
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.l (8,%fp),%d0
addi.l #0x7fff,%d0
andi.l #-0x8000,%d0
move.l %d0,(8,%fp)
bsr.s SHOW
addi.l #0x64,(8,%fp)
bsr.s SHOW
unlk %fp
rts
.globl PARAM_ASSIGN_$PUSHED
PARAM_ASSIGN_$PUSHED:
link.w %fp,#0
move.l %d2,-(%sp)
subq.l #2,%sp
move.b (0xa,%fp),%d2
move.w #6,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
tst.b %d2
bpl.s 1f
clr.w (8,%fp)
1:
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl PARAM_ASSIGN_$COPY
PARAM_ASSIGN_$COPY:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
addq.w #7,(-2,%fp)
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.globl PARAM_ASSIGN_$CALLERS
PARAM_ASSIGN_$CALLERS:
link.w %fp,#-8
move.l %d2,-(%sp)
subq.l #2,%sp
move.w #7,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
moveq #5,%d2
move.l %d2,(-4,%fp)
clr.l -(%sp)
move.l %d2,-(%sp)
bsr.w PARAM_ASSIGN_$ROUND
addq.w #8,%sp
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w #5,(-6,%fp)
pea (-6,%fp)
bsr.s PARAM_ASSIGN_$COPY
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
bsr.w PARAM_ASSIGN_$COUNT
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-0xc,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
