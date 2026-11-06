.text
.globl ADDRC_$RUN
ADDRC_$RUN:
link.w %fp,#-8
pea (%a2)
lea (.Lc0,%pc),%a2
pea (%a2)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
pea (4,%a2)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
lea (.Lc1,%pc),%a2
pea (%a2)
jsr (EXEC_$LOG_C).l
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.Lc0:
.short 0x6865,0x6c6c,0x6f00
.Lc1:
.short 0x0a00
.balignw 4,0x2048
