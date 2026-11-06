.text
.globl NO_DATA_T
NO_DATA_T:
link.w %fp,#0
jsr (PGM_$EXIT).l
unlk %fp
rts
.balignw 4,0x2048
