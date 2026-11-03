.set __dpc_fragment_origin,0xe16c5c
.text
.globl TIME_$Q_INIT
TIME_$Q_INIT:
rts
.globl TIME_$Q_INIT_QUEUE
TIME_$Q_INIT_QUEUE:
link.w %fp,#0
move.b (8,%fp),%d0
move.w (0xa,%fp),%d1
movea.l (0xc,%fp),%a0
clr.l (%a0)
clr.l (4,%a0)
move.b %d0,(8,%a0)
move.w %d1,(0xa,%a0)
unlk %fp
rts
.globl TIME_$Q_FLUSH_QUEUE
TIME_$Q_FLUSH_QUEUE:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.l (%a0)
unlk %fp
rts
