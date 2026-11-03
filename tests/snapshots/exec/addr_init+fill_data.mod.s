.text
.globl FILL_$RESET
FILL_$RESET:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
add.w %d1,%d1
move.w #-1,(-2,%a5,%d1.w)
addq.w #1,%d0
2:
cmpi.w #8,%d0
ble.s 1b
moveq #1,%d0
bra.s 5f
.balignw 4,0x2048
4:
move.w %d0,%d1
moveq #-1,%d2
lsl.w #2,%d1
move.l %d2,(0xc,%a5,%d1.w)
addq.w #1,%d0
5:
cmpi.w #4,%d0
ble.s 4b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FILL_DATA","aw",@nobits
.balign 4
.Ldata:
.globl FILL_$WA
FILL_$WA:
.space 16
.globl FILL_$LA
FILL_$LA:
.space 16
