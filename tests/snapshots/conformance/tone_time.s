.text
.globl TONE_$TIME
TONE_$TIME:
link.w %fp,#-0xc
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
move.w (4,%a0),(-4,%fp)
subq.l #2,%sp
move.w #0xe,-(%sp)
jsr (PROC1_$SET_LOCK).l
addq.w #4,%sp
pea (.Lc0,%pc)
jsr (TONE_$ENABLE).l
addq.w #4,%sp
pea (-0xc,%fp)
pea (-8,%fp)
pea (.Lc1,%pc)
jsr (TIME_$WAIT).l
lea (0xc,%sp),%sp
pea (.Lc2,%pc)
jsr (TONE_$ENABLE).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0xe,-(%sp)
jsr (PROC1_$CLR_LOCK).l
unlk %fp
rts
.Lc0:
.short 0xff00
.Lc2:
.short 0x0000
.Lc1:
.short 0x0000
.balignw 4,0x2048
