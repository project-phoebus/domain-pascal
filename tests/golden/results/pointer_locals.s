.text
.globl HELD
HELD:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
movea.l %a0,%a2
cmpa.w #0,%a2
bne.s 1f
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (FORCE).l
addq.w #8,%sp
movea.l %a0,%a2
1:
move.w (%a2),(%a5)
pea (%a2)
jsr (USE_REC).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl FRAMED
FRAMED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
bne.s 1f
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (FORCE).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
beq.s 2f
bra.s 3f
1:
move.w #1,(2,%a0)
3:
move.w (%a0),(%a5)
move.l (-4,%fp),-(%sp)
jsr (USE_PTR).l
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
INNER:
link.w %fp,#-4
movea.l (%fp),%a0
movea.l (-4,%a0),%a0
move.w (%a0),(%a5)
unlk %fp
rts
.globl UPLEVEL
UPLEVEL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
cmpa.w #0,%a0
bne.s 1f
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (FORCE).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
cmpa.w #0,%a0
beq.s 2f
bra.s 3f
1:
move.w #1,(2,%a0)
3:
movea.l (-4,%fp),%a0
move.w (2,%a0),(%a5)
bsr.s INNER
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ESCAPED
ESCAPED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (4,%a5)
move.w (8,%fp),-(%sp)
jsr (LOOKUP).l
addq.w #8,%sp
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (SET_PTR).l
movea.l (-4,%fp),%a0
move.w (%a0),(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PLOCAL","aw",@nobits
.balign 4
.Ldata:
.space 8
