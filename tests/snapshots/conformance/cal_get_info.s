.set __dpc_fragment_origin,0xe68568
.text
.globl CAL_$GET_INFO
CAL_$GET_INFO:
link.w %fp,#0
movea.l #CAL_$TIMEZONE,%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
unlk %fp
rts
