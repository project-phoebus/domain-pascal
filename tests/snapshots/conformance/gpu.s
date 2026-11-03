.text
.globl GPU_$INIT
GPU_$INIT:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l #0x2d0001,(%a0)
unlk %fp
rts
.globl GPU_$TERM
GPU_$TERM:
rts
.balignw 4,0x2048
