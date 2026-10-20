.text
.globl NOTS
NOTS:
link.w %fp,#0
movem.l %d2-%d3/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.b (0xc,%fp),%d2
move.w (0xe,%fp),%d3
move.b %d2,%d0
not.b %d0
move.b %d0,(%a5)
move.b (2,%a2),%d0
not.b %d0
move.b %d0,(2,%a5)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (BUSY).l
not.b %d0
move.b %d0,(%a5)
move.b %d2,%d0
or.b (2,%a2),%d0
not.b %d0
move.b %d0,(2,%a5)
btst #0,(4,%a5)
seq %d0
move.b %d0,(%a5)
cmpi.w #3,%d3
sne %d0
move.b %d0,(2,%a5)
movem.l (-0x10,%fp),%d2-%d3/%a2/%a5
unlk %fp
rts
.globl SHORTS
SHORTS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
bne.s 1f
cmpi.w #4,(%a0)
bne.s 1f
st %d1
bra.s 2f
1:
clr.b %d1
2:
move.b %d1,(%a5)
cmpi.w #2,%d0
bgt.s 5f
tst.b (2,%a0)
bpl.s 3f
5:
st %d0
bra.s 4f
3:
clr.b %d0
4:
move.b %d0,(2,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NOT_VALUES","aw",@nobits
.balign 4
.Ldata:
.space 8
