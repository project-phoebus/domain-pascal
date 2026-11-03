.set __dpc_fragment_origin,0xe16e48
.text
.globl TIME_$Q_REMOVE_ELEM
TIME_$Q_REMOVE_ELEM:
link.w %fp,#-8
pea (%a2)
movea.l (8,%fp),%a2
pea (4,%a2)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-6,%fp)
pea (-4,%fp)
move.l (0xc,%fp),-(%sp)
pea (%a2)
bsr.w TIME_$Q_REMOVE_INTERNAL
lea (0xc,%sp),%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
pea (4,%a2)
jsr (ML_$SPIN_UNLOCK).l
movea.l (0x10,%fp),%a0
move.l (-4,%fp),(%a0)
movea.l (-0xc,%fp),%a2
unlk %fp
rts
