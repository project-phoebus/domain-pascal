.text
.globl DECODE
DECODE:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
clr.w %d0
move.b (0xa,%a2),%d0
cmpi.w #0xdf,%d0
bne.s 1f
moveq #0x70,%d0
and.b (0xb,%a2),%d0
lsr.w #4,%d0
move.w %d0,(%a5)
1:
move.b (0xc,%a2),%d0
ext.w %d0
muls.w (2,%a5),%d0
move.w %d0,(0x208,%a5)
clr.w %d0
clr.w %d1
move.b (0xd,%a2),%d0
move.b (0xc,%a2),%d1
lsl.w #6,%d0
add.w %d1,%d0
move.w %d0,(4,%a5)
move.w #0xf0,%d0
and.b (0xe,%a2),%d0
move.w %d0,(%a5)
move.b (0xf,%a2),(6,%a5)
clr.b (0xc,%a2)
move.b #0xc8,(0xe,%a2)
move.b (0xf,%a2),-(%sp)
move.b (0xe,%a2),-(%sp)
jsr (TAKE_BYTE).l
addq.w #4,%sp
clr.w %d0
move.b (0xc,%a2),%d0
add.w %d0,%d0
move.w (8,%a5,%d0.w),(0x208,%a5)
clr.w %d0
move.b (0xb,%a2),%d0
cmpi.w #1,%d0
beq.s 3f
cmpi.w #2,%d0
beq.s 4f
bra.s 5f
3:
move.w #2,(2,%a5)
bra.s 2f
4:
move.w #4,(2,%a5)
bra.s 2f
5:
move.w #1,(2,%a5)
2:
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BYTE_INTS","aw",@nobits
.balign 4
.Ldata:
.space 524
