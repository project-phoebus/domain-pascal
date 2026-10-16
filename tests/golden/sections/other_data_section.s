.section ".text.KBD.OS_INIT_PROC","ax",@progbits
.globl KBD_$INIT
KBD_$INIT:
link.w %fp,#0
pea (%a2)
movea.l #.Ldata.WIRED_DATA,%a0
movea.l (8,%fp),%a2
clr.w (4,%a0)
clr.w (%a2)
moveq #1,%d0
move.l %d0,(8,%a2)
pea (8,%a0)
jsr (EC_$INIT).l
addq.w #4,%sp
movea.l #.Ldata.WIRED_DATA,%a0
lea (4,%a0),%a1
move.l %a1,(4,%a2)
pea (8,%a2)
jsr (EC_$INIT).l
addq.w #4,%sp
move.l #.Ldata.WIRED_DATA,-(%sp)
jsr (EC_$INIT).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.KBD.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 12
