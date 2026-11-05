.text
.globl BE_PTR_VIEWS_$VIEW
BE_PTR_VIEWS_$VIEW:
link.w %fp,#-0x14
move.l %d2,-(%sp)
move.l (8,%fp),%d0
move.l %d0,(-0x10,%fp)
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,(-0xc,%fp)
move.w #7,(-8,%fp)
moveq #4,%d2
move.w #0x63,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (-0x10,%fp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w %d2,%d0
add.w %d0,%d0
move.w #5,(-0x10,%fp,%d0.w)
pea (-8,%fp)
jsr (EXEC_$LOG_W).l
move.l (-0x18,%fp),%d2
unlk %fp
rts
.globl BE_PTR_VIEWS_$VOL
BE_PTR_VIEWS_$VOL:
link.w %fp,#-4
move.w #5,(EXEC_$COUNTER).l
move.w (EXEC_$COUNTER).l,(-2,%fp)
move.w (EXEC_$COUNTER).l,(-4,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BE_PTR_VIEWS_$COMP
BE_PTR_VIEWS_$COMP:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w #0xa,(%a5)
move.w #0xb,(2,%a5)
move.w #0xc,(4,%a5)
move.w #0xd,(6,%a5)
move.w #7,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
lea (%a5),%a0
move.l %a0,(0x10,%a5)
move.w %d2,%d0
ext.l %d0
add.l %a0,%d0
movea.l %d0,%a0
move.w #0x4d,(%a0)
pea (4,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w #0xb,(2,%a5)
move.l (0x10,%a5),%d0
addq.l #2,%d0
movea.l %d0,%a0
pea (%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
lea (6,%a5),%a0
move.l %a0,(0x10,%a5)
move.l %a0,%d0
neg.l %d0
move.l %d0,%d1
neg.l %d1
movea.l %d1,%a0
move.w #0x21,(%a0)
pea (6,%a5)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BE_PTR_VIEWS_$ELEM
BE_PTR_VIEWS_$ELEM:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
jsr (EXEC_$CLOBBER).l
move.w %d2,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BE_PTR_VIEWS_$ELEM2
BE_PTR_VIEWS_$ELEM2:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
lea (0,%a5,%d1.w),%a2
jsr (EXEC_$CLOBBER).l
movea.l %a2,%a0
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl BE_PTR_VIEWS_$NEXT
BE_PTR_VIEWS_$NEXT:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),%d2
jsr (EXEC_$CLOBBER).l
move.l %d2,%d0
addq.l #2,%d0
movea.l %d0,%a0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BE_PTR_VIEWS_$WHOLE
BE_PTR_VIEWS_$WHOLE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
lea (0xc,%a5),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BE_PTR_VIEWS_$PLUS
BE_PTR_VIEWS_$PLUS:
link.w %fp,#-4
move.l (8,%fp),%d0
subq.l #2,%d0
move.l %d0,(-4,%fp)
movea.l %d0,%a0
unlk %fp
rts
.globl BE_PTR_VIEWS_$RES
BE_PTR_VIEWS_$RES:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w #0xd,(6,%a5)
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w BE_PTR_VIEWS_$ELEM
addq.w #4,%sp
move.l %a0,(-4,%fp)
pea (%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w #0x1f,(8,%a5)
move.w #0x20,(0xa,%a5)
subq.l #2,%sp
move.w %d2,%d0
addq.w #1,%d0
move.w %d0,-(%sp)
bsr.w BE_PTR_VIEWS_$ELEM2
addq.w #4,%sp
move.l %a0,(-4,%fp)
pea (%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.l (-4,%fp),-(%sp)
bsr.w BE_PTR_VIEWS_$NEXT
addq.w #4,%sp
move.l %a0,(-4,%fp)
pea (%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w #0x42,(0xc,%a5)
bsr.w BE_PTR_VIEWS_$WHOLE
move.l %a0,(-4,%fp)
pea (%a0)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.l (-4,%fp),-(%sp)
bsr.w BE_PTR_VIEWS_$PLUS
addq.w #4,%sp
move.l %a0,(-4,%fp)
pea (%a0)
jsr (EXEC_$LOG_W).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BE_PTR_VIEWS_$PLOCAL
BE_PTR_VIEWS_$PLOCAL:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a2
move.w #5,(0xa,%a5)
tst.w %d0
bpl.s 1f
clr.l (0x10,%a5)
bra.s 2f
1:
move.w %d0,%d1
add.w %d1,%d1
lea (0,%a5,%d1.w),%a0
move.l %a0,(0x10,%a5)
2:
pea (0x10,%a5)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a1
clr.w (%a2)
cmpa.w #0,%a1
bne.s 3f
move.w #-1,(%a2)
bra.s 4f
3:
addi.w #0x11,(%a1)
move.w (%a1),(%a2)
4:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl BE_PTR_VIEWS_$PLOOP
BE_PTR_VIEWS_$PLOOP:
link.w %fp,#-0xc
movem.l %d2-%d4/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d2,%d4
add.w %d4,%d4
clr.w (0,%a5,%d4.w)
clr.w (2,%a5,%d4.w)
lea (0,%a5,%d4.w),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a2
clr.w %d4
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,(%a2)
cmpi.w #1,%d4
bne.s 4f
move.w %d2,%d0
add.w %d3,%d0
add.w %d0,%d0
lea (0,%a5,%d0.w),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a2
4:
tst.w %d3
beq.s 5f
jsr (EXEC_$CLOBBER).l
5:
addq.w #1,%d4
2:
cmpi.w #3,%d4
blt.s 1b
move.w %d2,%d0
add.w %d0,%d0
pea (0,%a5,%d0.w)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.w %d2,%d0
add.w %d0,%d0
pea (2,%a5,%d0.w)
jsr (EXEC_$LOG_W).l
movem.l (-0x20,%fp),%d2-%d4/%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_PTR_VIEWS","aw",@nobits
.balign 4
.Ldata:
.space 20
