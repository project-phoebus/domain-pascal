.text
.globl TAPE_$BOOT
TAPE_$BOOT:
link.w %fp,#-4
movea.l (8,%fp),%a0
clr.l (%a0)
clr.b %d0
unlk %fp
rts
.balignw 4,0x2048
