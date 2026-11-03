.set __dpc_fragment_origin,0xe64598
.text
.globl ASKNODE_$INFO
ASKNODE_$INFO:
link.w %fp,#0
move.l (0x18,%fp),-(%sp)
move.l (0x14,%fp),-(%sp)
pea (.Lc0,%pc)
move.l (0x10,%fp),-(%sp)
pea (.Lc1,%pc)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
.short 0x6130
unlk %fp
rts
.Lc0:
.short 0x0098
.balignw 4,0x2048
.Lc1:
.short 0xffff,0xffff
