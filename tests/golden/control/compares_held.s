.text
.globl CLAMP
CLAMP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w %d1
cmp.w %d0,%d1
bge.s 1f
move.w %d0,%d1
1:
move.w %d1,(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BOTH_HELD
BOTH_HELD:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
move.w (0xe,%fp),%d3
cmp.w %d1,%d0
bne.s 1f
move.w #1,(2,%a5)
1:
cmp.w %d0,%d1
beq.s 2f
move.w #2,(2,%a5)
2:
cmp.w %d1,%d0
ble.s 3f
move.w #3,(2,%a5)
3:
cmp.w %d3,%d2
bhi.s 4f
move.w #4,(2,%a5)
4:
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FIND
FIND:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
clr.w %d0
moveq #5,%d2
moveq #1,%d3
lea (4,%a5),%a0
.balignw 4,0x2048
2:
cmp.w (2,%a0),%d1
bne.s 4f
move.w %d3,%d0
bra.s 5f
4:
addq.w #1,%d3
addq.l #4,%a0
dbf %d2,2b
5:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
PARSE:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
cmp.w (%a0),%d0
bcs.s 1f
move.w #1,(2,%a5)
1:
move.w (%a0),%d1
cmp.w %d0,%d1
bhi.s 2f
move.w #2,(2,%a5)
2:
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CMPHELD","aw",@nobits
.balign 4
.Ldata:
.space 28
