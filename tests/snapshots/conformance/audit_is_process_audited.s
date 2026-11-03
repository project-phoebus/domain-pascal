.set __dpc_fragment_origin,0xe70d94
.text
.globl AUDIT_$IS_PROCESS_AUDITED
AUDIT_$IS_PROCESS_AUDITED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
add.w %d0,%d0
tst.w (-2,%a5,%d0.w)
seq %d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.AUDIT","aw",@nobits
.balign 4
.Ldata:
.space 128
