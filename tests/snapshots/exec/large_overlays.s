.text
.globl LARGE_OVERLAYS_$RUN
LARGE_OVERLAYS_$RUN:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #LOV_$DATA,%a0
move.w %d0,%d1
add.w %d1,%d1
move.w %d0,(-2,%a0,%d1.w)
moveq #5,%d0
movea.l #LOV_$DATA+60000,%a0
moveq #0x3f,%d1
addq.l #2,%a0
.balignw 4,0x2048
2:
move.w %d0,(-0x6e62,%a0)
addq.l #2,%a0
dbf %d1,2b
moveq #6,%d0
movea.l #LOV_$DATA+60000,%a0
moveq #0x3f,%d1
addq.l #2,%a0
.balignw 4,0x2048
5:
move.w %d0,(-0x65e2,%a0)
addq.l #2,%a0
dbf %d1,5b
jsr (LOV_$SHOW).l
jsr (LOV_$SET).l
pea (LOV_$DATA+0x847e).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (LOV_$DATA+0x84fe).l
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
.comm LOV_$DATA,34048,4
