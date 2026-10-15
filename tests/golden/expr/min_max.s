.text
.globl ADD_PAGES
ADD_PAGES:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
move.w (%a5),%d0
ext.l %d2
ext.l %d0
move.l #0xb0,%d3
sub.l %d0,%d3
cmp.l %d3,%d2
ble.s 1f
move.l %d3,%d2
1:
move.w %d1,%d0
move.l (4,%a5),%d1
ext.l %d0
sub.l (8,%a5),%d1
cmp.l %d1,%d0
ble.s 2f
move.l %d1,%d0
2:
move.w %d2,%d1
add.w %d0,%d1
move.w %d1,(0xc,%a5)
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl GET_IMPURE
GET_IMPURE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x64,%d0
movea.l (8,%fp),%a0
cmp.l (4,%a0),%d0
bls.s 1f
move.l (4,%a0),%d0
1:
move.l %d0,(0x10,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DAT_COPY
DAT_COPY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
cmpi.w #0x400,%d0
ble.s 1f
move.w #0x400,%d0
1:
move.w %d0,(0x14,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl COPY_PACKET
COPY_PACKET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x400,%d0
cmp.w (8,%fp),%d0
bls.s 1f
move.w (8,%fp),%d0
1:
move.w %d0,(0x16,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DEL_PAGES
DEL_PAGES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (%a5),%d1
ext.l %d0
ext.l %d1
subi.l #0xb0,%d1
sub.l %d1,%d0
bpl.s 1f
clr.l %d0
1:
move.w %d0,(0x18,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.MIN_MAX","aw",@nobits
.balign 4
.Ldata:
.space 28
