.text
A:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l %a1,%a2
addq.w #1,(-2,%a2)
move.w (-4,%a2),%d0
add.w %d2,%d0
move.w %d0,(-4,%a2)
tst.w %d2
ble.s 1f
subq.l #2,%sp
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,-(%sp)
movea.l %a2,%a1
bsr.s B
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
B:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l %a1,%a2
move.w (-4,%a2),%d0
add.w %d2,%d0
move.w %d0,(-4,%a2)
tst.w %d2
ble.s 1f
subq.l #2,%sp
move.w %d2,%d0
subq.w #1,%d0
move.w %d0,-(%sp)
movea.l %a2,%a1
bsr.s A
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl NESTED_FORWARD_$RUN
NESTED_FORWARD_$RUN:
link.w %fp,#-4
clr.w (-2,%fp)
clr.w (-4,%fp)
subq.l #2,%sp
move.w (8,%fp),-(%sp)
movea.l %fp,%a1
bsr.s A
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.balignw 4,0x2048
