.text
.globl ETHERNET_$INIT
ETHERNET_$INIT:
link.w %fp,#-4
move.l #0x100002,%d0
unlk %fp
rts
.balignw 4,0x2048
