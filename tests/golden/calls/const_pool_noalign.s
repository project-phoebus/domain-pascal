.text
.globl CPOOLNA_$CHECK
CPOOLNA_$CHECK:
link.w %fp,#0
pea (.Lc0,%pc)
pea (.Lc1,%pc)
jsr (EXT_$RIGHTS).l
unlk %fp
rts
.Lc1:
.short 0x0000
.Lc0:
.short 0x0000,0x000f
.balignw 4,0x0000
