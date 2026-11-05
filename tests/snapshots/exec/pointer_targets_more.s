.text
.globl BUMP
BUMP:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
addq.l #1,(2,%a2)
pea (2,%a2)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PAIR
PAIR:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
addq.l #1,(2,%a2)
addi.l #0xa,(2,%a3)
pea (2,%a2)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (2,%a3)
jsr (EXEC_$LOG_L).l
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.globl MIX
MIX:
link.w %fp,#0
pea (%a2)
movea.l (0xc,%fp),%a2
movea.l (8,%fp),%a0
move.l (2,%a2),%d0
add.l (%a0),%d0
move.l %d0,(2,%a2)
pea (2,%a2)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl MIXV
MIXV:
link.w %fp,#0
pea (%a2)
movea.l (0xc,%fp),%a2
pea (8,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
addq.l #1,(2,%a2)
pea (2,%a2)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PADV_$SETUP
PADV_$SETUP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #0xa,%d0
move.l %d0,(8,%a5)
moveq #0x64,%d1
move.l %d1,(0x18,%a5)
lea (0x20,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0xc,%a5)
lea (%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0x28,%a5)
lea (0x10,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
move.l %a0,(0x2c,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PADV_$TWO_LOCALS
PADV_$TWO_LOCALS:
link.w %fp,#-0xc
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0x28,%a5),%a2
lea (0x10,%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a3
pea (6,%a3)
pea (6,%a2)
bsr.w PAIR
movem.l (-0x18,%fp),%a2-%a3/%a5
unlk %fp
rts
.globl SLOTS
SLOTS:
link.w %fp,#0
movea.l (0xc,%fp),%a0
pea (6,%a0)
movea.l (8,%fp),%a1
pea (6,%a1)
bsr.w PAIR
unlk %fp
rts
.globl PADV_$TWO_SLOTS
PADV_$TWO_SLOTS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (0x10,%a5)
pea (%a5)
bsr.s SLOTS
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl LATER
LATER:
link.w %fp,#-4
pea (%a2)
movea.l (8,%fp),%a2
pea (6,%a2)
pea (.Lc0,%pc)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,(-4,%fp)
pea (-4,%fp)
bsr.w MIX
addq.w #8,%sp
pea (6,%a2)
pea (7).w
bsr.w MIXV
movea.l (-8,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0005
.globl PADV_$CALL_LATER
PADV_$CALL_LATER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
bsr.s LATER
movea.l (-4,%fp),%a5
unlk %fp
rts
INNER1:
link.w %fp,#-4
pea (%a2)
movea.l %a1,%a2
movea.l (8,%a2),%a0
pea (6,%a0)
bsr.w BUMP
addq.w #4,%sp
movea.l (8,%a2),%a0
pea (6,%a0)
pea (0x16,%a5)
bsr.w PAIR
movea.l (-8,%fp),%a2
unlk %fp
rts
SIB:
link.w %fp,#-4
pea (%a2)
movea.l (%fp),%a2
movea.l %a2,%a1
bsr.s INNER1
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl OUTER
OUTER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
move.w (%a0),(-2,%fp)
tst.w (-2,%fp)
ble.s 1f
bsr.s SIB
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PADV_$NESTED
PADV_$NESTED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (8,%fp)
pea (%a5)
bsr.s OUTER
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PADV_$PLOCAL
PADV_$PLOCAL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%a5),(-4,%fp)
move.l (-4,%fp),-(%sp)
bsr.w BUMP
addq.w #4,%sp
lea (0x16,%a5),%a0
move.l %a0,(-4,%fp)
pea (%a0)
bsr.w BUMP
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HELD
HELD:
link.w %fp,#0
pea (%a2)
movea.l (8,%fp),%a2
clr.w (%a2)
pea (6,%a2)
move.l (0xc,%a2),-(%sp)
bsr.w PAIR
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl PADV_$HELD
PADV_$HELD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
bsr.s HELD
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PADV_$VOL
PADV_$VOL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x28,%a5),%a0
pea (6,%a0)
bsr.w BUMP
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PADV","aw",@progbits
.balign 4
.Ldata:
.space 37
.byte 0x64
.space 10
