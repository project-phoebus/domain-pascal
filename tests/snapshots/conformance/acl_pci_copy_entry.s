.set __dpc_fragment_origin,0xe45122
.text
.space 2
PCI_COPY_ENTRY:
link.w %fp,#-8
pea (%a2)
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
lea (%a0),%a2
moveq #0x17,%d0
.balignw 4,0x2048
1:
move.b (%a1)+,(%a2)+
dbf %d0,1b
clr.l (0x20,%a0)
movea.l #UID_$NIL,%a1
move.l (%a1)+,(0x18,%a0)
move.l (%a1)+,(0x1c,%a0)
movea.l (-0xc,%fp),%a2
unlk %fp
rts
PRIM_CREATE_INTERNAL:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
bsr.s PCI_COPY_ENTRY
unlk %fp
rts
