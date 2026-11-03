.set __dpc_fragment_origin,0xe148ca
.text
.space 2
.globl PROC1_$TST_LOCK
PROC1_$TST_LOCK:
link.w %fp,#-4
movea.l (PROC1_$CURRENT_PCB).l,%a0
move.w (8,%fp),%d0
move.l (0x40,%a0),%d1
btst %d0,%d1
sne %d1
move.b %d1,%d0
unlk %fp
rts
.globl PROC1_$GET_LOCKS
PROC1_$GET_LOCKS:
link.w %fp,#-4
movea.l (PROC1_$CURRENT_PCB).l,%a0
move.l (0x40,%a0),%d0
unlk %fp
rts
