.text
.globl IP_$FILL
IP_$FILL:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (%a5),%a0
move.l %a0,(-8,%fp)
clr.l %d0
bra.s 2f
.balignw 4,0x2048
1:
move.l %d0,%d1
move.l %d1,%d2
neg.l %d1
lsl.l #3,%d2
movea.l (-8,%fp),%a0
add.l %d2,%d1
addq.l #3,%d1
move.l %d1,(%a0)
move.l (-8,%fp),%d1
addq.l #4,%d1
move.l %d1,(-8,%fp)
addq.l #1,%d0
2:
cmpi.l #0x3ff,%d0
ble.s 1b
lea (0x1000,%a5),%a0
move.l %a0,(-8,%fp)
moveq #1,%d0
bra.s 5f
.balignw 4,0x2048
4:
move.l %d0,%d1
movea.l (-8,%fp),%a0
addi.l #0x3e8,%d1
move.l %d1,(%a0)
move.l (-8,%fp),%d1
addq.l #4,%d1
move.l %d1,(-8,%fp)
addq.l #1,%d0
5:
cmpi.l #0x38,%d0
ble.s 4b
lea (0x10e0,%a5),%a0
move.l %a0,(-0xc,%fp)
moveq #1,%d0
bra.s 8f
.balignw 4,0x2048
7:
move.l %d0,%d1
add.l %d1,%d1
move.l %d1,%d2
movea.l (-0xc,%fp),%a0
lsl.l #2,%d2
add.l %d2,%d1
move.w %d1,(%a0)
move.l (-0xc,%fp),%d1
addq.l #2,%d1
move.l %d1,(-0xc,%fp)
addq.l #1,%d0
8:
cmpi.l #8,%d0
ble.s 7b
lea (0x10f0,%a5),%a0
move.l %a0,(-8,%fp)
moveq #1,%d0
bra.s 11f
.balignw 4,0x2048
10:
move.l %d0,%d1
movea.l (-8,%fp),%a0
addi.l #0x1f4,%d1
move.l %d1,(%a0)
move.l (-8,%fp),%d1
addq.l #4,%d1
move.l %d1,(-8,%fp)
addq.l #1,%d0
11:
cmpi.l #8,%d0
ble.s 10b
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.INDEX_PARTS_DATA","aw",@nobits
.balign 4
.Ldata:
.globl IP_$TBL
IP_$TBL:
.space 4096
.globl IP_$T56
IP_$T56:
.space 224
.globl IP_$W8
IP_$W8:
.space 16
.globl IP_$T8L
IP_$T8L:
.space 32
