.text
.globl HELLO
HELLO:
link.w %fp,#0
subq.l #2,%sp
move.w #0x63,-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.globl NILS_$SETUP
NILS_$SETUP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
lea (0xc,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(2,%a5)
lea (%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(8,%a5)
lea (6,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0xe,%a5)
lea (0xc,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0x14,%a5)
lea (6,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
move.l %a0,(0x18,%a5)
move.l #HELLO,(0x1c,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl NILS_$CLEAR
NILS_$CLEAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (2,%a5)
clr.l (0x18,%a5)
clr.l (0x1c,%a5)
tst.l (0x18,%a5)
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
tst.l (0x1c,%a5)
bne.s 2f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NILS_$COUNT
NILS_$COUNT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0x20,%a5)
movea.l (0x14,%a5),%a0
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,(0x20,%a5)
movea.l (2,%a0),%a0
2:
cmpa.w #0,%a0
bne.s 1b
subq.l #2,%sp
move.w (0x20,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl NILS_$PICK
NILS_$PICK:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l %d0
tst.l (0x14,%a5)
beq.s 1f
move.l (0x14,%a5),%d0
1:
move.l %d0,(0x18,%a5)
tst.l (0x18,%a5)
bne.s 2f
subq.l #2,%sp
move.w #6,-(%sp)
bra.s 4f
2:
subq.l #2,%sp
movea.l (0x18,%a5),%a0
move.w (%a0),-(%sp)
4:
jsr (EXEC_$MARK).l
3:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl NILS_$EMPTY
NILS_$EMPTY:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (0x14,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NILS_$FIND
NILS_$FIND:
link.w %fp,#-8
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
suba.l %a2,%a2
movea.l (0x14,%a5),%a0
bra.s 2f
.balignw 4,0x2048
1:
cmpi.w #0x14,(%a0)
bne.s 4f
movea.l %a0,%a2
4:
movea.l (2,%a0),%a0
2:
cmpa.w #0,%a0
bne.s 1b
cmpa.w #0,%a2
bne.s 5f
subq.l #2,%sp
clr.w -(%sp)
bra.s 7f
5:
subq.l #2,%sp
move.w (%a2),-(%sp)
7:
jsr (EXEC_$MARK).l
6:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.globl SET_PTR
SET_PTR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (0x14,%a5),(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl NILS_$FRAME
NILS_$FRAME:
link.w %fp,#-4
pea (-4,%fp)
bsr.s SET_PTR
addq.w #4,%sp
clr.l (-4,%fp)
tst.l (-4,%fp)
bne.s 1f
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
1:
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.NILS","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x0a,0x00,0x00,0x00,0x00,0x00,0x14,0x00,0x00,0x00,0x00,0x00,0x1e
.space 22
