.set __dpc_fragment_origin,0xe148f8
.text
.globl PROC1_$SET_ASID
PROC1_$SET_ASID:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d2
movea.l (PROC1_$CURRENT_PCB).l,%a0
move.w %d2,(0x46,%a0)
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (MMU_$INSTALL_ASID).l
move.l (-4,%fp),%d2
unlk %fp
rts
