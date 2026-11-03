.text
.globl IXF_$XRESET
IXF_$XRESET:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
moveq #-1,%d2
lsl.w #2,%d1
move.l %d2,(-4,%a5,%d1.w)
addq.w #1,%d0
2:
cmpi.w #6,%d0
ble.s 1b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.INDEXED_FOR_DATA","aw",@nobits
.balign 4
.Ldata:
.globl IXF_$XL
IXF_$XL:
.space 24
