.text
.globl WI_$FILL
WI_$FILL:
link.w %fp,#-0x1c
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (%a5),%a0
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.l %d0,%d1
addi.l #0x186a0,%d1
move.l %d1,(%a0)
move.l %a0,%d1
addq.l #1,%d0
addi.l #0xe4,%d1
movea.l %d1,%a0
2:
cmpi.l #0x12c,%d0
ble.s 1b
movea.l %a0,%a1
moveq #1,%d0
bra.s 5f
.balignw 4,0x2048
4:
move.l %d0,%d1
addi.l #0x30d40,%d1
move.l %d1,(%a1)
moveq #0x30,%d1
addq.l #1,%d0
add.l %a1,%d1
movea.l %d1,%a1
5:
cmpi.l #0x540,%d0
ble.s 4b
movea.l %a1,%a0
move.l #-0x3e8,%d0
bra.s 8f
.balignw 4,0x2048
7:
move.l %d0,%d1
addi.l #0x493e0,%d1
move.l %d1,(%a0)
moveq #0x18,%d1
addq.l #1,%d0
add.l %a0,%d1
movea.l %d1,%a0
8:
cmpi.l #0x578,%d0
ble.s 7b
movea.l %a0,%a1
moveq #1,%d0
bra.s 11f
.balignw 4,0x2048
10:
move.l %d0,%d1
addi.l #0x61a80,%d1
move.l %d1,(%a1)
moveq #0x2c,%d1
addq.l #1,%d0
add.l %a1,%d1
movea.l %d1,%a1
11:
cmpi.l #0x3e8,%d0
ble.s 10b
movea.l %a1,%a0
clr.l %d0
clr.w %d1
bra.s 14f
.balignw 4,0x2048
13:
move.b %d1,(%a0)
move.l %a0,%d2
addq.l #1,%d0
addq.l #1,%d2
addq.w #1,%d1
movea.l %d2,%a0
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
