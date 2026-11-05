.text
.globl PUSH_IN_ELSE
PUSH_IN_ELSE:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w %d1,%d0
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a5,%d1.w),%a2
tst.w (-0xe,%a2)
bne.s 1f
clr.l (TOTAL).l
bra.s 2f
1:
pea (-0x12,%a2)
jsr (WAKE).l
2:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl FEED
FEED:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w %d1,%d0
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a5,%d1.w),%a2
move.l (-0xc,%a2),%d0
cmp.l (-8,%a2),%d0
ble.s 1f
sub.l (-8,%a2),%d0
move.l %d0,-(%sp)
jsr (TRIM).l
1:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl TARGET
TARGET:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w %d1,%d0
lsl.w #3,%d0
add.w %d0,%d1
lea (0,%a5,%d1.w),%a2
tst.l (-4,%a2)
beq.s 1f
pea (-0xc,%a2)
movea.l (-4,%a2),%a0
jsr (%a0)
1:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.REGALLOC","aw",@nobits
.balign 4
.Ldata:
.space 144
