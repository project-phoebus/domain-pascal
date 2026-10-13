.text
.globl CRIT
CRIT:
link.w %fp,#0
ori.w #0x700,%sr
movea.l (8,%fp),%a0
move.l (DELTA).l,(%a0)
andi.w #0xf8ff,%sr
unlk %fp
rts
.globl SAVE_RESTORE
SAVE_RESTORE:
link.w %fp,#0
move.w %sr,%d0
move.w #0x2700,%sr
move.w %d0,(SAVED).l
move.w #1,(W).l
move.w %sr,%d0
move.w (SAVED).l,%sr
unlk %fp
rts
.globl HELD_SR
HELD_SR:
link.w %fp,#-4
move.w %sr,%d0
move.w #0x2700,%sr
move.w #1,(W).l
move.w %sr,%d1
move.w %d0,%sr
unlk %fp
rts
.globl DISCARD_CALL
DISCARD_CALL:
link.w %fp,#0
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXT_$F).l
clr.w (W).l
unlk %fp
rts
.globl UNSIGNED_SR
UNSIGNED_SR:
link.w %fp,#0
move.w %sr,%d0
move.w #-0x701,%sr
move.w %d0,(U).l
unlk %fp
rts
.balignw 4,0x2048
