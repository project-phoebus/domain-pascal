.text
.globl CC_$W
CC_$W:
link.w %fp,#-4
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
clr.w (%a5)
move.w %d0,%d3
move.w %d0,%d4
ext.l %d3
add.w %d1,%d4
move.w %d2,%d5
subq.w #1,%d5
cmp.w %d5,%d4
ble.s 1f
addq.w #1,(%a5)
1:
move.w %d0,%d4
move.w %d2,%d5
sub.w %d1,%d4
addq.w #1,%d5
cmp.w %d5,%d4
bne.s 2f
addq.w #2,(%a5)
2:
moveq #0xf,%d4
moveq #0xf,%d5
and.w %d0,%d4
and.w %d1,%d5
cmp.w %d5,%d4
bne.s 3f
addq.w #4,(%a5)
3:
move.w %d1,%d4
move.w %d2,%d5
ext.l %d4
ext.l %d5
sub.l %d5,%d4
cmp.l %d4,%d3
bge.s 4f
addq.w #8,(%a5)
4:
move.w %d2,%d3
move.w %d0,%d2
subq.w #1,%d3
add.w %d1,%d2
cmp.w %d2,%d3
blt.s 5f
addi.w #0x10,(%a5)
5:
move.w %d0,%d2
move.w %d1,%d0
andi.w #0xff0,%d2
andi.w #0xff0,%d0
cmp.w %d0,%d2
beq.s 6f
addi.w #0x20,(%a5)
6:
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0x18,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl CC_$L
CC_$L:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
clr.w (%a5)
move.l %d0,%d2
move.l %d1,%d3
andi.l #0xfffff,%d2
andi.l #0xfffff,%d3
cmp.l %d3,%d2
bne.s 1f
addq.w #1,(%a5)
1:
move.l %d0,%d2
move.l %d1,%d3
addq.l #1,%d2
subq.l #1,%d3
cmp.l %d3,%d2
ble.s 2f
addq.w #2,(%a5)
2:
move.l %d0,%d2
move.l %d1,%d3
sub.l %d1,%d2
addq.l #5,%d3
cmp.l %d3,%d2
bge.s 3f
addq.w #4,(%a5)
3:
move.l %d0,%d2
move.l %d1,%d0
andi.l #0xfffff,%d2
andi.l #0xfffff,%d0
cmp.l %d0,%d2
beq.s 4f
addq.w #8,(%a5)
4:
pea (%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.COMP_CMP","aw",@nobits
.balign 4
.Ldata:
.space 4
