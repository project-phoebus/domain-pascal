.arch 68020
.text
.globl P
P:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l #0x1869f,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.l (%a5),%d2
add.l %d1,%d2
move.l %d2,(%a5)
addq.l #1,%d1
subq.l #1,%d0
bcc.s 2b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.LONG_FOR","aw",@nobits
.balign 4
.Ldata:
.space 4
