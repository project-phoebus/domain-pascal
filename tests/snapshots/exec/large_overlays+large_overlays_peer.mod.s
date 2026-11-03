.text
.globl LOV_$SHOW
LOV_$SHOW:
link.w %fp,#0
pea (LOV_$DATA+4).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (LOV_$DATA+0x7c00).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (LOV_$DATA+0x7c7e).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (LOV_$DATA+0x8480).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (LOV_$DATA+0x84fe).l
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl LOV_$SET
LOV_$SET:
link.w %fp,#0
move.w #-1,(LOV_$DATA+0x847e).l
move.w #-2,(LOV_$DATA+0x84fe).l
pea (LOV_$DATA+0x847e).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (LOV_$DATA+0x84fe).l
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
.comm LOV_$DATA,34048,4
