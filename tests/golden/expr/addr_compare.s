.text
.globl BUILD_DF
BUILD_DF:
link.w %fp,#-8
jsr (WORK).l
move.l (-4,%fp),%d0
.short 0xb0bc
.long GLOBAL_B
bls.s 1f
move.w #1,(N).l
1:
movea.l (BUS_ERR_VEC).l,%a0
cmpa.l #COLD_BUS_ERR,%a0
bne.s 2f
move.w #2,(N).l
2:
unlk %fp
rts
.balignw 4,0x2048
