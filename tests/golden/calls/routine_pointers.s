.text
.globl KICK
KICK:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
clr.l (%a2)
tst.l (0x38,%a2)
beq.s 1f
subq.l #2,%sp
clr.w -(%sp)
move.l (%a2),-(%sp)
movea.l (0x38,%a2),%a0
jsr (%a0)
addq.w #8,%sp
1:
pea (%a2)
jsr (ADVANCE).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl ERR
ERR:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
subq.l #2,%sp
st -(%sp)
move.l (%a2),-(%sp)
movea.l (0x3c,%a2),%a0
jsr (%a0)
addq.w #8,%sp
move.l %d0,(LAST_STATUS).l
pea (%a2)
jsr (ADVANCE).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PLAIN
PLAIN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #5,%d0
move.l %d0,(-4,%fp)
pea (.Lc0,%pc)
pea (-4,%fp)
movea.l (%a5),%a0
jsr (%a0)
addq.w #8,%sp
addq.l #1,(-4,%fp)
pea (.Lc0,%pc)
pea (-4,%fp)
movea.l (%a5),%a0
jsr (%a0)
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0007
.globl HELD
HELD:
link.w %fp,#-4
move.l %d2,-(%sp)
jsr (GET_XON).l
move.l %a0,%d2
subq.l #2,%sp
st -(%sp)
move.l (8,%fp),-(%sp)
movea.l %d2,%a0
jsr (%a0)
move.l (-8,%fp),%d2
unlk %fp
rts
.globl HANDLER
HANDLER:
rts
.globl INSTALL
INSTALL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l #HANDLER,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.RP","aw",@nobits
.balign 4
.Ldata:
.space 4
