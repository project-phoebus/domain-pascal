.text
.globl AI_$PUT
AI_$PUT:
link.w %fp,#-8
movea.l (0xc,%fp),%a0
clr.l (%a0)
jsr (AI_$LOCK).l
move.l (8,%fp),(-8,%fp)
pea (8).w
move.l (-4,%fp),-(%sp)
move.l (-8,%fp),-(%sp)
jsr (AI_$COPY).l
unlk %fp
rts
.globl AI_$INIT
AI_$INIT:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.l (%a0)
move.l %a0,(4,%a0)
move.l %a0,(8,%a0)
unlk %fp
rts
.globl AI_$PASS
AI_$PASS:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
pea (%a2)
jsr (AI_$TAKE).l
addq.w #4,%sp
movea.l (0xc,%fp),%a0
pea (8,%a0)
jsr (AI_$TAKE).l
lea (4,%a2),%a0
move.l %a0,(%a5)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl AI_$VAL
AI_$VAL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (0xc,%fp)
jsr (AI_$TAKE).l
lea (8,%fp),%a0
move.l %a0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.AI","aw",@nobits
.balign 4
.Ldata:
.space 4
