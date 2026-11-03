.text
.globl FILL_$RUN
FILL_$RUN:
link.w %fp,#-4
jsr (FILL_$RESET).l
movea.l #FILL_$WA,%a0
moveq #4,%d0
.balignw 4,0x2048
1:
clr.w (%a0)+
dbf %d0,1b
pea (.Lc0,%pc)
move.l #FILL_$WA,-(%sp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
movea.l #FILL_$LA,%a0
moveq #2,%d0
.balignw 4,0x2048
2:
clr.l (%a0)+
dbf %d0,2b
pea (.Lc0,%pc)
move.l #FILL_$LA,-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc0:
.short 0x0010
.balignw 4,0x2048
