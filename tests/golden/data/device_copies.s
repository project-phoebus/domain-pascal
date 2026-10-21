.text
.globl SEEK
SEEK:
link.w %fp,#-0x10
pea (%a5)
lea (.Ldata).l,%a5
lea (0x80,%a5),%a0
lea (-0x10,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (0xc,%fp),%a0
move.w (%a0),(0x8a,%a5)
movea.l (0x10,%fp),%a1
move.w (%a1),(0x80,%a5)
move.b #1,(4,%a5)
move.l (8,%fp),-(%sp)
jsr (START).l
lea (-0x10,%fp),%a0
lea (0x80,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (-0x14,%fp),%a5
unlk %fp
rts
.globl SWAP
SWAP:
link.w %fp,#-0x10
pea (%a5)
lea (.Ldata).l,%a5
lea (0x80,%a5),%a0
lea (-0x10,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
lea (0x8c,%a5),%a0
lea (0x80,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
lea (-0x10,%fp),%a0
lea (0x8c,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.w (0x28,%a5),(0x2d8,%a5)
movea.l (-0x14,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.DEVICE_COPIES","aw",@nobits
.balign 4
.Ldata:
.space 732
