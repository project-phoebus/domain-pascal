.text
.globl TP_$GET
TP_$GET:
link.w %fp,#0
movea.l #1,%a0
movea.l (8,%fp),%a1
move.l %a0,(%a1)
unlk %fp
rts
.globl TP_$PASS
TP_$PASS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l #1,%a0
move.l %a0,(%a5)
pea (.Lc0,%pc)
jsr (TP_$TAKE).l
addq.w #4,%sp
pea (1).w
jsr (TP_$TAKE_V).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0001
.balignw 4,0x2048
.section ".bss.moddata.TP","aw",@nobits
.balign 4
.Ldata:
.space 4
