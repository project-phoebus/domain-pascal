.text
REC:
link.w %fp,#-4
pea (%a2)
movea.l %a1,%a2
addq.l #1,(%a5)
subq.w #1,(-2,%a2)
tst.w (-2,%a2)
ble.s 1f
movea.l %a2,%a1
bsr.s REC
1:
move.l (%a5),-(%sp)
jsr (EXT_$P).l
clr.w (-2,%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
TWICE:
link.w %fp,#-8
movea.l (%fp),%a0
move.w (8,%fp),%d0
add.w (-2,%a0),%d0
unlk %fp
rts
.globl NB_$WALK
NB_$WALK:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
movea.l %fp,%a1
bsr.s REC
subq.l #2,%sp
move.w #3,-(%sp)
bsr.s TWICE
move.w %d0,(4,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.NB","aw",@nobits
.balign 4
.Ldata:
.space 8
