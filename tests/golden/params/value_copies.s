.text
.globl VC_$SCALARS
VC_$SCALARS:
link.w %fp,#-8
pea (%a2)
movea.l (8,%fp),%a0
move.b (%a0),(-2,%fp)
movea.l (0xc,%fp),%a1
move.w (%a1),(-4,%fp)
movea.l (0x10,%fp),%a2
move.l (%a2),(-8,%fp)
move.l (-8,%fp),-(%sp)
move.w (-4,%fp),-(%sp)
move.b (-2,%fp),-(%sp)
jsr (VC_$TAKE).l
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl VC_$ASSIGN
VC_$ASSIGN:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
addq.w #1,(-2,%fp)
movea.l (0xc,%fp),%a1
move.w (-2,%fp),(%a1)
pea (-2,%fp)
jsr (VC_$VAR).l
addq.w #4,%sp
lea (-2,%fp),%a0
move.l %a0,(-8,%fp)
pea (-8,%fp)
jsr (VC_$PTR).l
unlk %fp
rts
.globl VC_$UID
VC_$UID:
link.w %fp,#-8
pea (%a2)
movea.l (0x10,%fp),%a2
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
movea.l (0xc,%fp),%a0
tst.w (%a0)
bne.s 1f
moveq #1,%d0
move.l %d0,(%a2)
bra.s 2f
1:
pea (%a2)
pea (-8,%fp)
jsr (VC_$UID_OP).l
addq.w #8,%sp
tst.l (%a2)
beq.s 3f
pea (%a2)
pea (-8,%fp)
jsr (VC_$UID_OP).l
3:
2:
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl VC_$CLOCK
VC_$CLOCK:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
move.w (4,%a0),(-4,%fp)
pea (-8,%fp)
jsr (VC_$CLOCK_OP).l
unlk %fp
rts
.globl VC_$BLOCKS
VC_$BLOCKS:
link.w %fp,#-0x28
movea.l (8,%fp),%a0
lea (-0x10,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.w (%a0)+,(%a1)+
movea.l (0xc,%fp),%a0
lea (-0x28,%fp),%a1
moveq #5,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
pea (-0x28,%fp)
pea (-0x10,%fp)
jsr (VC_$OPTS_OP).l
unlk %fp
rts
.balignw 4,0x2048
