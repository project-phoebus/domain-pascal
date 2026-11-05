.text
.globl ADDRIN_$SETUP
ADDRIN_$SETUP:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0x4d,%d0
move.l %d0,(4,%a5)
moveq #1,%d1
move.l %d1,(%a5)
moveq #5,%d2
move.l %d2,(8,%a5)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl PUT
PUT:
link.w %fp,#-4
pea (%a2)
jsr (EXEC_$CLOBBER).l
movea.l (8,%fp),%a0
lea (%a0),%a2
pea (4,%a2)
jsr (EXEC_$LOG_L).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl ADDRIN_$PUT
ADDRIN_$PUT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
bsr.s PUT
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl INIT
INIT:
link.w %fp,#0
movea.l (8,%fp),%a0
moveq #7,%d0
move.l %d0,(%a0)
move.l %a0,(4,%a0)
move.l %a0,(8,%a0)
unlk %fp
rts
.globl ADDRIN_$INIT
ADDRIN_$INIT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
pea (8,%a5)
bsr.s INIT
addq.w #4,%sp
move.l (0xc,%a5),(-4,%fp)
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
moveq #5,%d0
move.l %d0,(8,%a5)
move.l (0x10,%a5),(-4,%fp)
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOG_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SHOW
SHOW:
link.w %fp,#-4
pea (%a2)
move.l (8,%fp),%d0
movea.l %d0,%a2
pea (4,%a2)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (%a2)
jsr (EXEC_$LOG_L).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PASS
PASS:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
pea (%a2)
bsr.s SHOW
addq.w #4,%sp
lea (4,%a2),%a0
move.l %a0,(0x14,%a5)
movea.l %a0,%a2
pea (%a2)
jsr (EXEC_$LOG_L).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl ADDRIN_$PASS
ADDRIN_$PASS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (%a5)
bsr.s PASS
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl VAL
VAL:
link.w %fp,#-8
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xc,%fp),%d2
lea (8,%fp),%a0
move.l %a0,(0x14,%a5)
movea.l %a0,%a2
pea (%a2)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.l %d2,%d0
addq.l #1,%d0
move.l %d0,(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
movem.l (-0x14,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl ADDRIN_$VAL
ADDRIN_$VAL:
link.w %fp,#0
pea (0x29).w
pea (0x29).w
bsr.s VAL
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ADDRIN","aw",@nobits
.balign 4
.Ldata:
.space 24
