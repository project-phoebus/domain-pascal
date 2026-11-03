.text
.globl NOSAVE_$REPRO
NOSAVE_$REPRO:
link.w %fp,#-4
movem.l %d2-%d7/%a2-%a4,-(%sp)
move.w (8,%fp),%d0
move.l %d0,(-4,%fp)
jsr (EXEC_$NOSAVE).l
jsr (EXEC_$NOSAVE).l
move.l (-4,%fp),%d0
cmpi.w #3,%d0
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 4f
1:
subq.l #2,%sp
move.w #2,-(%sp)
4:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
jsr (EXEC_$NOSAVE).l
move.l (-4,%fp),%d0
cmpi.w #3,%d0
ble.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
3:
movem.l (-0x28,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl NOSAVE_$BYADDR
NOSAVE_$BYADDR:
link.w %fp,#0
movem.l %d2-%d7/%a2-%a4,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a0
jsr (EXEC_$NOSAVE).l
movea.l (8,%fp),%a0
move.w (%a0),%d0
movea.l (0xc,%fp),%a0
ext.l %d0
add.l (%a0),%d0
movea.l (0xc,%fp),%a0
move.l %d0,(%a0)
jsr (EXEC_$NOSAVE).l
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x24,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl NOSAVE_$COPY
NOSAVE_$COPY:
link.w %fp,#-0x10
movem.l %d2-%d7/%a2-%a4,-(%sp)
movea.l (8,%fp),%a0
move.w (%a0),%d0
move.l %d0,(-0x10,%fp)
movea.l (0xc,%fp),%a0
move.l (%a0),%d0
move.l %d0,(-0xc,%fp)
move.l (-0x10,%fp),%d0
move.w %d0,%d1
addq.w #1,%d1
move.w %d1,(-6,%fp)
jsr (EXEC_$NOSAVE).l
subq.l #2,%sp
move.l (-0x10,%fp),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w -(%sp)
jsr (EXEC_$NOSAVE_W).l
addq.w #4,%sp
move.l (-0xc,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
jsr (EXEC_$NOSAVE).l
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l (-0xc,%fp),%d0
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x34,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl NOSAVE_$LOOP
NOSAVE_$LOOP:
link.w %fp,#-0x14
movem.l %d2-%d7/%a2-%a4,-(%sp)
move.w (8,%fp),%d0
clr.w %d1
move.l %d1,(-0xc,%fp)
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d0
move.l %d0,(-0x14,%fp)
moveq #1,%d0
move.l %d0,(-0x10,%fp)
.balignw 4,0x2048
2:
move.l (-0xc,%fp),%d0
move.l (-0x10,%fp),%d1
add.w %d1,%d0
move.l %d0,(-0xc,%fp)
jsr (EXEC_$NOSAVE).l
subq.l #2,%sp
move.l (-0xc,%fp),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l (-0x10,%fp),%d0
addq.w #1,%d0
move.l %d0,(-0x10,%fp)
subq.w #1,(-0x12,%fp)
bcc.s 2b
1:
move.l (-0xc,%fp),%d0
move.w %d0,%d1
move.w %d1,%d0
movem.l (-0x38,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl NOSAVE_$RESULT
NOSAVE_$RESULT:
link.w %fp,#-0xc
movem.l %d2-%d7/%a2-%a4,-(%sp)
move.l (0xc,%fp),%d0
move.l %d0,(-0xc,%fp)
pea (8,%fp)
jsr (EXEC_$NOSAVE_ID_L).l
addq.w #4,%sp
move.l %d0,%d2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l %d2,%d0
move.l (-0xc,%fp),%d1
add.l %d1,%d0
movem.l (-0x30,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl NOSAVE_$MIXED
NOSAVE_$MIXED:
link.w %fp,#-0xc
movem.l %d2-%d7/%a2-%a4,-(%sp)
move.w (8,%fp),%d0
move.l %d0,(-8,%fp)
move.l (0xa,%fp),%d0
move.l %d0,(-0xc,%fp)
subq.l #2,%sp
move.l (-8,%fp),%d0
move.w %d0,-(%sp)
move.l (-0xc,%fp),%d0
move.l %d0,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
move.l %d0,(-4,%fp)
jsr (EXEC_$CLOBBER).l
subq.l #2,%sp
move.l (-8,%fp),%d0
move.w %d0,-(%sp)
jsr (EXEC_$NOSAVE_W).l
addq.w #4,%sp
move.l (-0xc,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
jsr (EXEC_$CLOBBER).l
subq.l #2,%sp
move.l (-8,%fp),%d0
move.w %d0,-(%sp)
jsr (EXEC_$NOSAVE_W).l
addq.w #4,%sp
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x30,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl NOSAVE_$NOTHING_LIVE
NOSAVE_$NOTHING_LIVE:
link.w %fp,#0
movem.l %d2-%d7/%a2-%a4,-(%sp)
jsr (EXEC_$NOSAVE).l
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x24,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.balignw 4,0x2048
