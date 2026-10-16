.text
.globl RCV
RCV:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0x58,%a5),%d0
lea (0x1c,%a0),%a1
ext.l %d0
lsl.l #4,%d0
lea (0x5c,%a5,%d0.l),%a2
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
move.l (%a1)+,(%a2)+
clr.w (%a0)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.KBDLIKE","aw",@nobits
.balign 4
.Ldata:
.space 65628
