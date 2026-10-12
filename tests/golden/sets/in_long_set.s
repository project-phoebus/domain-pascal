.text
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
.balignw 4,0x2048
