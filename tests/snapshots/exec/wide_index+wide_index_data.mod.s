.text
.globl WI_$FILL
WI_$FILL:
link.w %fp,#-0x1c
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (%a5),%a0
move.l %a0,(-8,%fp)
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.l %d0,%d1
movea.l (-8,%fp),%a0
addi.l #0x186a0,%d1
move.l %d1,(%a0)
move.l (-8,%fp),%d1
addi.l #0xe4,%d1
move.l %d1,(-8,%fp)
addq.l #1,%d0
2:
cmpi.l #0x12c,%d0
ble.s 1b
move.l (-8,%fp),(-0xc,%fp)
moveq #1,%d0
bra.s 5f
.balignw 4,0x2048
4:
move.l %d0,%d1
movea.l (-0xc,%fp),%a0
addi.l #0x30d40,%d1
move.l %d1,(%a0)
moveq #0x30,%d1
add.l (-0xc,%fp),%d1
move.l %d1,(-0xc,%fp)
addq.l #1,%d0
5:
cmpi.l #0x540,%d0
ble.s 4b
move.l (-0xc,%fp),(-0x10,%fp)
move.l #-0x3e8,%d0
bra.s 8f
.balignw 4,0x2048
7:
move.l %d0,%d1
movea.l (-0x10,%fp),%a0
addi.l #0x493e0,%d1
move.l %d1,(%a0)
moveq #0x18,%d1
add.l (-0x10,%fp),%d1
move.l %d1,(-0x10,%fp)
addq.l #1,%d0
8:
cmpi.l #0x578,%d0
ble.s 7b
move.l (-0x10,%fp),(-0x14,%fp)
moveq #1,%d0
bra.s 11f
.balignw 4,0x2048
10:
move.l %d0,%d1
movea.l (-0x14,%fp),%a0
addi.l #0x61a80,%d1
move.l %d1,(%a0)
moveq #0x2c,%d1
add.l (-0x14,%fp),%d1
move.l %d1,(-0x14,%fp)
addq.l #1,%d0
11:
cmpi.l #0x3e8,%d0
ble.s 10b
move.l (-0x14,%fp),(-0x18,%fp)
clr.l %d0
clr.w %d1
bra.s 14f
.balignw 4,0x2048
13:
movea.l (-0x18,%fp),%a0
move.b %d1,(%a0)
move.l (-0x18,%fp),%d2
addq.l #1,%d2
move.l %d2,(-0x18,%fp)
addq.l #1,%d0
addq.w #1,%d1
cmpi.w #0xfb,%d1
bne.s 16f
clr.w %d1
16:
14:
cmpi.l #0x9c40,%d0
ble.s 13b
movem.l (-0x24,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.WIDE_INDEX_DATA","aw",@nobits
.balign 4
.Ldata:
.globl WI_$R228
WI_$R228:
.space 68400
.globl WI_$R48
WI_$R48:
.space 64512
.globl WI_$R24
WI_$R24:
.space 57624
.globl WI_$R44
WI_$R44:
.space 44000
.globl WI_$CHARS
WI_$CHARS:
.space 40004
