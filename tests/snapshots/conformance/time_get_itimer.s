.set __dpc_fragment_origin,0xe58f06
.text
.space 2
.globl TIME_$GET_ITIMER
TIME_$GET_ITIMER:
link.w %fp,#-8
movem.l %a2-%a4,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
movea.l (0x10,%fp),%a4
subq.l #2,%sp
pea (%a4)
pea (%a3)
move.w (%a2),-(%sp)
bsr.w TIME_$GET_ITIMER_INTERNAL
lea (0xc,%sp),%sp
cmpi.w #1,(%a2)
bne.s 1f
pea (%a3)
pea (-8,%fp)
bsr.w TIME_$CLOCK_TO_ITIMER
addq.w #8,%sp
move.l (-8,%fp),(%a3)
move.w (-4,%fp),(4,%a3)
pea (%a4)
pea (-8,%fp)
bsr.w TIME_$CLOCK_TO_ITIMER
move.l (-8,%fp),(%a4)
move.w (-4,%fp),(4,%a4)
1:
movem.l (-0x14,%fp),%a2-%a4
unlk %fp
rts
