.set __dpc_fragment_origin,0xe68580
.text
.globl CAL_$SET_DRIFT
CAL_$SET_DRIFT:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (%a0),(CAL_$TIMEZONE+6).l
move.w (4,%a0),(CAL_$TIMEZONE+0xa).l
unlk %fp
rts
