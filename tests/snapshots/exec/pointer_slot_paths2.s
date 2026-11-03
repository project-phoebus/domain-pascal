.text
.globl YS_SLOT2_$INIT
YS_SLOT2_$INIT:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0x1e,%d0
move.l %d0,(8,%a5)
moveq #0x14,%d1
move.l %d1,(4,%a5)
moveq #0xa,%d2
move.l %d2,(%a5)
moveq #1,%d3
move.l %d3,(0xc,%a5)
lea (8,%a5),%a0
move.l %a0,(0x10,%a5)
move.l #2,(0x14,%a5)
clr.l (0x18,%a5)
clr.l (0x1c,%a5)
lea (0x14,%a5),%a0
move.l %a0,(0x20,%a5)
lea (0xc,%a5),%a0
move.l %a0,(0x24,%a5)
lea (4,%a5),%a0
move.l %a0,(0x3c,%a5)
lea (%a5),%a0
move.l %a0,(0x40,%a5)
lea (8,%a5),%a0
move.l %a0,(0x44,%a5)
clr.l (0x48,%a5)
lea (4,%a5),%a0
move.l %a0,(0x5c,%a5)
moveq #0x4d,%d0
move.l %d0,(0x70,%a5)
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SHOW
SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
tst.l (0x64,%a5)
bne.s 1f
pea (-1).w
bra.s 3f
1:
movea.l (0x64,%a5),%a0
move.l (%a0),-(%sp)
3:
jsr (EXEC_$LOGV_L).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
INNER:
link.w %fp,#-8
movea.l (%fp),%a0
clr.l (0x60,%a5)
tst.w (-6,%a0)
bne.s 1f
clr.l (-8,%fp)
bra.s 2f
1:
move.l (-4,%a0),(-8,%fp)
2:
movea.l (-8,%fp),%a0
unlk %fp
rts
.globl YS_SLOT2_$T_NEST
YS_SLOT2_$T_NEST:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0x40,%a5),(-4,%fp)
move.w %d0,(-6,%fp)
bsr.s INNER
move.l %a0,(0x64,%a5)
bsr.s SHOW
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl GETR
GETR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (0x60,%a5)
move.w %d0,%d1
lsl.w #2,%d1
move.l (0x1c,%a5,%d1.w),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl A0L
A0L:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s GETR
addq.w #4,%sp
move.l (%a0),(0x60,%a5)
cmpi.l #1,(0x60,%a5)
bne.s 1f
move.l (0x3c,%a5),(-8,%fp)
bra.s 2f
1:
cmpi.l #2,(0x60,%a5)
bne.s 3f
move.l (4,%a0),(-8,%fp)
bra.s 4f
3:
clr.l (-8,%fp)
4:
2:
movea.l (-8,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl YS_SLOT2_$T_A0L
YS_SLOT2_$T_A0L:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s A0L
addq.w #4,%sp
move.l %a0,(0x64,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FGOTO
FGOTO:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d0
clr.l (0x60,%a5)
cmpi.w #2,%d0
bgt.s 1f
move.w %d0,%d1
lsl.w #2,%d1
move.l (0x40,%a5,%d1.w),(-4,%fp)
1:
movea.l (-4,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl YS_SLOT2_$T_GOTO
YS_SLOT2_$T_GOTO:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0x5c,%a5),(0x64,%a5)
pea (8,%fp)
bsr.s FGOTO
addq.w #4,%sp
move.l %a0,(0x64,%a5)
cmpi.w #2,%d2
ble.s 1f
pea (-1).w
jsr (EXEC_$LOGV_L).l
bra.s 2f
1:
bsr.w SHOW
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl FLOOP
FLOOP:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w %d1
bra.s 2f
.balignw 4,0x2048
1:
cmp.w %d0,%d1
beq.s 3f
addq.w #1,%d1
2:
cmpi.w #8,%d1
blt.s 1b
3:
cmpi.w #8,%d1
blt.s 4f
clr.l (-4,%fp)
bra.s 5f
4:
move.w %d1,%d0
lsl.w #2,%d0
move.l (0x30,%a5,%d0.w),(-4,%fp)
5:
movea.l (-4,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl YS_SLOT2_$T_LOOP
YS_SLOT2_$T_LOOP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s FLOOP
addq.w #4,%sp
move.l %a0,(0x64,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FTEMP
FTEMP:
link.w %fp,#-0x28
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
lea (0x68,%a5),%a0
lea (-0x20,%fp),%a1
moveq #7,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
move.l (-0x18,%fp),(-0x24,%fp)
move.l (-0x24,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
cmpi.w #1,%d2
bne.s 2f
move.l (0x3c,%a5),(-0x28,%fp)
bra.s 3f
2:
move.l (0x44,%a5),(-0x28,%fp)
3:
movea.l (-0x28,%fp),%a0
movem.l (-0x30,%fp),%d2/%a5
unlk %fp
rts
.globl YS_SLOT2_$T_TEMP
YS_SLOT2_$T_TEMP:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
cmpi.w #1,%d2
beq.w 1f
1:
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s FTEMP
addq.w #4,%sp
move.l %a0,(0x64,%a5)
bsr.w SHOW
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl FRAMED
FRAMED:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.l (0x1c,%a5,%d1.w),(-4,%fp)
tst.l (-4,%fp)
bne.s 1f
moveq #1,%d0
move.l %d0,(0x60,%a5)
clr.l (-8,%fp)
bra.s 2f
1:
movea.l (-4,%fp),%a0
tst.l (4,%a0)
bne.s 3f
moveq #2,%d0
move.l %d0,(0x60,%a5)
clr.l (-8,%fp)
bra.s 4f
3:
movea.l (-4,%fp),%a0
move.l (4,%a0),(-8,%fp)
4:
2:
movea.l (-8,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl YS_SLOT2_$T_FRAMED
YS_SLOT2_$T_FRAMED:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s FRAMED
addq.w #4,%sp
move.l %a0,(0x64,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.YS_SLOT2","aw",@nobits
.balign 4
.Ldata:
.space 136
