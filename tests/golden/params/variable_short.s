.text
.globl P
P:
link.w %fp,#0
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
jsr (Q).l
unlk %fp
rts
.balignw 4,0x2048
