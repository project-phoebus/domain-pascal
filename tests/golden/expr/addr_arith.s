.text
.globl AA_$OFFSET
AA_$OFFSET:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
lea (NIL_DRIVER+8).l,%a0
move.l %a0,%d0
.short 0x90bc
.long NIL_DRIVER
move.w %d0,-(%sp)
movea.l (8,%fp),%a1
move.w (%a1),-(%sp)
jsr (FIND).l
move.l %a0,%d0
move.l %d0,(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl AA_$SUMS
AA_$SUMS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l %d0,%d1
andi.l #0x3ff,%d1
.short 0xd2bc
.long BUFPOOL
move.l %d1,(%a5)
move.l %d1,%d0
.short 0x90bc
.long BUFPOOL
move.l %d0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl AA_$FRAME
AA_$FRAME:
link.w %fp,#-0x20
pea (%a5)
lea (.Ldata).l,%a5
lea (-0x20,%fp),%a0
lea (-0x18,%fp),%a1
move.l %a1,%d0
sub.l %a0,%d0
move.l %d0,(%a5)
move.l #BUFPOOL,(%a5)
lea (8,%a5),%a0
move.l %a0,(%a5)
pea (4,%a5)
jsr (VTOP).l
movea.l (-0x24,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.AA","aw",@nobits
.balign 4
.Ldata:
.space 20
