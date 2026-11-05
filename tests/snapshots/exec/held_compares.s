.text
.globl HELD_CMP_$W
HELD_CMP_$W:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
move.w %d2,(%a5)
clr.w (4,%a5)
cmp.w %d2,%d0
bne.s 1f
addq.w #1,(4,%a5)
1:
cmp.w %d2,%d0
beq.s 2f
addq.w #2,(4,%a5)
2:
cmp.w %d0,%d2
bge.s 3f
addq.w #4,(4,%a5)
3:
cmp.w %d0,%d2
blt.s 4f
addq.w #8,(4,%a5)
4:
cmp.w %d1,%d0
ble.s 5f
addi.w #0x10,(4,%a5)
5:
cmp.w %d0,%d1
bgt.s 6f
addi.w #0x20,(4,%a5)
6:
cmp.w %d1,%d0
bne.s 7f
addi.w #0x40,(4,%a5)
7:
cmp.w %d2,%d0
bge.s 8f
addi.w #0x80,(4,%a5)
8:
cmp.w %d0,%d1
beq.s 9f
addi.w #0x100,(4,%a5)
9:
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl HELD_CMP_$U
HELD_CMP_$U:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
move.w %d2,(2,%a5)
clr.w (4,%a5)
cmp.w %d2,%d0
bne.s 1f
addq.w #1,(4,%a5)
1:
cmp.w %d2,%d0
beq.s 2f
addq.w #2,(4,%a5)
2:
cmp.w %d0,%d2
bcc.s 3f
addq.w #4,(4,%a5)
3:
cmp.w %d0,%d2
bcs.s 4f
addq.w #8,(4,%a5)
4:
cmp.w %d1,%d0
bls.s 5f
addi.w #0x10,(4,%a5)
5:
cmp.w %d0,%d1
bhi.s 6f
addi.w #0x20,(4,%a5)
6:
cmp.w %d1,%d0
bne.s 7f
addi.w #0x40,(4,%a5)
7:
cmp.w %d2,%d0
bcc.s 8f
addi.w #0x80,(4,%a5)
8:
cmp.w %d0,%d1
beq.s 9f
addi.w #0x100,(4,%a5)
9:
pea (4,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.HELD_CMP","aw",@nobits
.balign 4
.Ldata:
.space 8
