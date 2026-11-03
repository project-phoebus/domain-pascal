.arch 68020
.text
.globl ASSOC
ASSOC:
link.w %fp,#0
movea.l (DCTE).l,%a0
bfextu (0x10,%a0){#5:#3},%d0
jsr (READY).l
move.w #3,(W).l
unlk %fp
rts
.balignw 4,0x2048
