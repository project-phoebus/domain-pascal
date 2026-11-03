.text
.globl OVD_$SHOW
OVD_$SHOW:
link.w %fp,#0
move.l #OVD_$BSS,-(%sp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (OVD_$BSS+4).l
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (OVD_$BSS+0xa).l
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (OVD_$DATA+4).l
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl OVD_$SET
OVD_$SET:
link.w %fp,#0
move.l %d2,-(%sp)
move.w #0x4d,(OVD_$BSS+0xc).l
moveq #-2,%d0
move.l %d0,(OVD_$DATA+4).l
moveq #9,%d1
move.l %d1,(OVD_$BSS+4).l
moveq #4,%d2
move.l %d2,(OVD_$DATA).l
move.l (-4,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.overlay.OVD_$BSS","aw",@nobits
.balign 4
.globl OVD_$BSS
.type OVD_$BSS,@object
.size OVD_$BSS,16
OVD_$BSS:
.space 4
.globl OVD_$TAIL
OVD_$TAIL:
.space 4
.globl OVD_$ARR
OVD_$ARR:
.space 8
.section ".data.overlay.OVD_$DATA","aw",@progbits
.balign 4
.globl OVD_$DATA
.type OVD_$DATA,@object
.size OVD_$DATA,8
OVD_$DATA:
.byte 0x00,0x00,0x00,0x03
.globl OVD_$K
OVD_$K:
.byte 0x12,0x34,0x56,0x78
