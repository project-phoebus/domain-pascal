.set __dpc_fragment_origin,0xe6aa38
.text
GET_NEXT_ARG:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
movea.l (-8,%fp),%a0
unlk %fp
rts
