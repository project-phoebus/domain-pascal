.text
.globl BU_$FILL
BU_$FILL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d2
neg.w %d1
lsl.w #5,%d2
add.w %d2,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
ext.l %d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
move.l %d1,(-0xf8,%a0)
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d2
neg.w %d1
lsl.w #5,%d2
add.w %d2,%d1
moveq #0x64,%d2
muls.w %d0,%d2
addq.l #7,%d2
move.l %d2,(-4,%a5,%d1.w)
addq.w #1,%d0
2:
cmpi.w #4,%d0
ble.s 1b
lea (%a5),%a0
move.l %a0,(0x3e0,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_U_DATA","aw",@nobits
.balign 4
.Ldata:
.globl BU_$BLKS
BU_$BLKS:
.space 992
.globl BU_$BP
BU_$BP:
.space 4
