.set __dpc_fragment_origin,0xe70db6
.text
.space 2
.globl AUDIT_$SUSPEND
AUDIT_$SUSPEND:
link.w %fp,#0
move.w (PROC1_$CURRENT).l,%d0
lea (.Ldata).l,%a0
add.w %d0,%d0
move.w (-2,%a0,%d0.w),%d1
addq.w #1,%d1
move.w %d1,(-2,%a0,%d0.w)
unlk %fp
rts
.globl AUDIT_$RESUME
AUDIT_$RESUME:
link.w %fp,#0
move.w (PROC1_$CURRENT).l,%d0
lea (.Ldata).l,%a0
add.w %d0,%d0
move.w (-2,%a0,%d0.w),%d1
subq.w #1,%d1
move.w %d1,(-2,%a0,%d0.w)
unlk %fp
rts
.section ".bss.moddata.AUDIT","aw",@nobits
.balign 4
.Ldata:
.space 128
