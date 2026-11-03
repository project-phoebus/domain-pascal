.set __dpc_fragment_origin,0xe1aafc
.text
.globl TERM_$SEND_KBD_STRING
TERM_$SEND_KBD_STRING:
link.w %fp,#-4
pea (-4,%fp)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
pea (.Lc0,%pc)
move.l (%sp),-(%sp)
jsr (KBD_$PUT).l
lea (0x14,%sp),%sp
pea (-4,%fp)
.short 0x6186
unlk %fp
rts
.Lc0:
.short 0x0000
.balignw 4,0x2048
