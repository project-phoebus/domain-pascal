.text
.globl ADDR_CONST
ADDR_CONST:
link.w %fp,#-4
pea (.Lc0,%pc)
lea (.Lc1,%pc),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (PUT).l
unlk %fp
rts
.Lc1:
.short 0x0a00
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0001
.balignw 4,0x2048
