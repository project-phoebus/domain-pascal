.text
.globl SEND
SEND:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
movea.l (0xc,%fp),%a0
clr.l %d2
move.w %d0,%d2
move.w %d1,%d0
ext.l %d0
add.l %d0,%d2
clr.l %d0
move.w (2,%a0),%d0
addi.l #0x100,%d0
cmp.l %d0,%d2
ble.s 1f
move.w #2,(%a5)
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl SUM3
SUM3:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (4,%a5),%d0
clr.l %d1
add.l (8,%a5),%d0
move.w (0x10,%a5),%d1
add.l (0xc,%a5),%d0
cmp.l %d1,%d0
bge.s 1f
move.w #3,(%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ADDU
ADDU:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l #0xfffff,%d0
move.l %d0,%d1
and.l (4,%a0),%d0
movea.l (0xc,%fp),%a0
and.l (4,%a0),%d1
cmp.l %d1,%d0
beq.s 1f
move.w #1,(%a5)
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FRM
FRM:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
move.l #0xfffff,%d0
move.l %d0,%d1
and.l (0x18,%a5),%d0
and.l (-4,%fp),%d1
cmp.l %d1,%d0
beq.s 1f
move.w #5,(%a5)
1:
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl BYTES
BYTES:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
clr.w %d0
clr.w %d1
move.b (-1,%fp),%d0
move.b (0x1d,%a5),%d1
cmp.w %d1,%d0
bne.s 1f
move.w #6,(%a5)
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl DUMP
DUMP:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
moveq #3,%d1
moveq #1,%d2
.balignw 4,0x2048
2:
clr.l %d3
move.w %d2,%d3
subq.l #1,%d3
lsl.l #8,%d3
lsl.l #2,%d3
cmp.l %d3,%d0
ble.s 1f
move.w %d2,(%a5)
addq.w #1,%d2
dbf %d1,2b
1:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CMPCOMP","aw",@nobits
.balign 4
.Ldata:
.space 32
