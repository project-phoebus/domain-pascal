.set __dpc_fragment_origin,0xe76470
.text
DISCARD_PACKET:
link.w %fp,#-8
pea (%a2)
movea.l (%fp),%a2
pea (8,%fp)
jsr (NETBUF_$RTN_HDR).l
addq.w #4,%sp
subq.l #2,%sp
movea.l (0x16,%a2),%a0
move.w (%a0),-(%sp)
move.l (0xc,%fp),-(%sp)
jsr (PKT_$DUMP_DATA).l
movea.l (-0xc,%fp),%a2
unlk %fp
rts
RECEIVE_PACKET:
link.w %fp,#-4
subq.l #2,%sp
movea.l (0x16,%fp),%a0
move.w (%a0),-(%sp)
move.l (0xe,%fp),-(%sp)
movea.l (0xa,%fp),%a1
move.l (%a1),-(%sp)
bsr.s DISCARD_PACKET
unlk %fp
rts
