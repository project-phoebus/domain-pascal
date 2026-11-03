.text
.globl PKT_$INIT
PKT_$INIT:
link.w %fp,#-4
pea (-4,%fp)
move.l #0x800000f,-(%sp)
move.l #PKT_$PING_SERVER,-(%sp)
jsr (PROC1_$CREATE_P).l
lea (0xc,%sp),%sp
tst.l (-4,%fp)
beq.s 1f
pea (-4,%fp)
jsr (CRASH_SYSTEM).l
1:
unlk %fp
rts
.balignw 4,0x2048
