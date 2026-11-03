.text
.globl LOOPCALL
LOOPCALL:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l %d2,%d0
subq.l #4,%d0
move.l %d0,(0xc,%a5)
clr.w %d3
bra.s 2f
.balignw 4,0x2048
1:
movea.l %d2,%a0
addq.l #1,(%a0)
jsr (EXEC_$CLOBBER).l
addq.w #1,%d3
2:
cmpi.w #3,%d3
blt.s 1b
movea.l %d2,%a0
move.w #9,(4,%a0)
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOTHARMS
BOTHARMS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.w (0xc,%fp),%d1
move.l %d0,%d2
subq.l #4,%d2
move.l %d2,(0xc,%a5)
cmpi.w #1,%d1
bne.s 1f
moveq #0xb,%d1
bra.s 3f
1:
moveq #0xd,%d1
3:
movea.l %d0,%a0
move.l %d1,(%a0)
2:
move.w #0xc,(4,%a0)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl WITHP
WITHP:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l %d2,%d0
subq.l #4,%d0
move.l %d0,(0xc,%a5)
moveq #0x15,%d0
movea.l %d2,%a0
move.l %d0,(%a0)
jsr (EXEC_$CLOBBER).l
movea.l %d2,%a0
move.w #0x16,(4,%a0)
moveq #0x17,%d0
move.l %d0,(8,%a0)
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl CMPCALL
CMPCALL:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d2
cmpi.l #0x3e8,%d2
bls.s 1f
movea.l %d2,%a0
addq.l #1,(%a0)
jsr (EXEC_$CLOBBER).l
movea.l %d2,%a0
addq.l #1,(%a0)
1:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl STORED
STORED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l %a0,(0x10,%a5)
move.w #1,(4,%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PTR_DREGS_FLOW_$MAIN
PTR_DREGS_FLOW_$MAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
clr.l (8,%a5)
clr.w (4,%a5)
pea (%a5)
bsr.w LOOPCALL
addq.w #4,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
clr.w -(%sp)
pea (%a5)
bsr.w BOTHARMS
addq.w #8,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w #1,-(%sp)
pea (%a5)
bsr.w BOTHARMS
addq.w #8,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (%a5)
bsr.w WITHP
addq.w #4,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (8,%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
moveq #0xb,%d0
move.l %d0,(%a5)
pea (%a5)
bsr.w CMPCALL
addq.w #4,%sp
pea (%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (%a5)
bsr.w STORED
addq.w #4,%sp
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
moveq #5,%d0
movea.l (0x10,%a5),%a0
move.l %d0,(8,%a0)
pea (8,%a5)
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PTR_DREGS_FLOW","aw",@nobits
.balign 4
.Ldata:
.space 20
