.text
.globl W_$VAR
W_$VAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (0x12,%a5),(%a5)
move.w #3,(6,%a5)
movea.l (8,%fp),%a0
move.l (0x14,%a5),(2,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl W_$PTR
W_$PTR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%a5),%a0
move.l (0x14,%a5),(2,%a0)
lea (%a5),%a1
move.l %a1,(8,%a5)
move.w #3,(6,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl W_$ELEM
W_$ELEM:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.w (0x10,%a5),%d1
movea.l (0xc,%a5),%a0
lsl.w #3,%d1
lea (0,%a0,%d1.w),%a2
move.b %d0,(-8,%a2)
move.l (0x14,%a5),(-6,%a2)
move.b (0x12,%a5),(-7,%a2)
pea (-2,%a2)
jsr (POKE).l
move.w #5,(-2,%a2)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl W_$PARAM
W_$PARAM:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (0x14,%a5),(2,%a0)
move.w #3,(6,%a0)
pea (%a5)
jsr (TAKE).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.WITHS","aw",@nobits
.balign 4
.Ldata:
.space 24
