.text
.globl IM_$FILL_O92
IM_$FILL_O92:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
moveq #0x5c,%d2
addi.w #0x1770,%d1
mulu.w %d0,%d2
move.w %d1,(-2,%a5,%d2.w)
addq.w #1,%d0
2:
cmpi.w #8,%d0
ble.s 1b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.INDEX_MUL_DATA","aw",@nobits
.balign 4
.Ldata:
.globl IM_$O92
IM_$O92:
.space 736
