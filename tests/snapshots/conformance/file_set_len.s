.set __dpc_fragment_origin,0xe73f90
.text
.globl FILE_$SET_LEN
FILE_$SET_LEN:
link.w %fp,#0
move.l (0x10,%fp),-(%sp)
pea (.Lc0,%pc)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
.short 0x613e
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0000
