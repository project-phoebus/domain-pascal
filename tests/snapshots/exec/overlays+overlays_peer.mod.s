.text
.globl OVP_$SHOW
OVP_$SHOW:
link.w %fp,#0
move.l #OVL_$INIT,-(%sp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (OVL_$INIT+4).l
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (OVL_$INIT+8).l
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
move.l #OVL_$ZERO,-(%sp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (OVL_$ZERO+8).l
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (OVL_$ZERO+0xc).l
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl OVP_$SET
OVP_$SET:
link.w %fp,#0
move.w #-7,(OVL_$INIT).l
moveq #0x63,%d0
move.l %d0,(OVL_$ZERO+0xc).l
unlk %fp
rts
.balignw 4,0x2048
.section ".data.overlay.OVL_$INIT","aw",@progbits
.balign 4
.globl OVL_$INIT
.type OVL_$INIT,@object
.size OVL_$INIT,12
OVL_$INIT:
.byte 0x00,0x05,0x00,0x00,0x12,0x34,0x56,0x78,0x51
.space 3
.comm OVL_$ZERO,16,4
