.set __dpc_fragment_origin,0xe32b76
.text
.space 2
INIT_DTTE:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
move.w (0xc,%fp),%d2
pea (0xc,%a2)
jsr (EC_$INIT).l
addq.w #4,%sp
pea (0x18,%a2)
jsr (EC_$INIT).l
addq.w #4,%sp
pea (%a2)
jsr (EC_$INIT).l
clr.b (0x36,%a2)
move.w %d2,(0x34,%a2)
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
