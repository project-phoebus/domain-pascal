.text
.globl BB_$FILL
BB_$FILL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l #0x3e8,(%a5)
move.l #0x3e9,(4,%a5)
move.l #0x3ea,(8,%a5)
move.l #0x3eb,(0xc,%a5)
move.l #0x3ec,(0x10,%a5)
move.l #0x3ed,(0x14,%a5)
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
moveq #0x64,%d1
move.w %d0,%d2
muls.w %d0,%d1
lsl.w #3,%d2
addq.l #1,%d1
move.l %d1,(0x10,%a5,%d2.w)
moveq #0x64,%d1
move.w %d0,%d2
muls.w %d0,%d1
lsl.w #3,%d2
addq.l #2,%d1
move.l %d1,(0x14,%a5,%d2.w)
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d2
add.w %d2,%d2
add.w %d2,%d1
moveq #0x64,%d2
muls.w %d0,%d2
addi.l #0xb,%d2
move.l %d2,(0x2c,%a5,%d1.w)
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d2
add.w %d2,%d2
add.w %d2,%d1
moveq #0x64,%d2
muls.w %d0,%d2
addi.l #0xc,%d2
move.l %d2,(0x30,%a5,%d1.w)
move.w %d0,%d1
lsl.w #2,%d1
move.w %d1,%d2
add.w %d2,%d2
add.w %d2,%d1
moveq #0x64,%d2
muls.w %d0,%d2
addi.l #0xd,%d2
move.l %d2,(0x34,%a5,%d1.w)
addq.w #1,%d0
2:
cmpi.w #4,%d0
ble.s 1b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_BLOCKS_DATA","aw",@nobits
.balign 4
.Ldata:
.globl BB_$OREC
BB_$OREC:
.space 24
.globl BB_$OARR
BB_$OARR:
.space 32
.globl BB_$OARR12
BB_$OARR12:
.space 48
