.text
.globl SMOKE_$MAIN
SMOKE_$MAIN:
link.w %fp,#-4
pea (.Lc0,%pc)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (.Lc1,%pc)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (.Lc2,%pc)
jsr (EXEC_$LOG_UW).l
addq.w #4,%sp
pea (.Lc3,%pc)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
st (-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
pea (.Lc4,%pc)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
subq.l #2,%sp
move.w #9,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
move.l #0x186a0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w #-7,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.b #0xc8,(-4,%fp)
subq.l #2,%sp
move.b (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_C).l
unlk %fp
rts
.Lc1:
.short 0xfffe
.Lc3:
.short 0x4100
.Lc2:
.short 0xffff
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0001
.Lc4:
.short 0x1234,0x5678
.globl SMOKE_$ADD
SMOKE_$ADD:
link.w %fp,#-4
move.w (8,%fp),%d0
ext.l %d0
add.l (0xa,%fp),%d0
unlk %fp
rts
.globl SMOKE_$SET
SMOKE_$SET:
link.w %fp,#-4
move.w (0xc,%fp),%d0
movea.l (8,%fp),%a0
move.w %d0,%d1
move.w %d1,(%a0)
unlk %fp
rts
.balignw 4,0x2048
