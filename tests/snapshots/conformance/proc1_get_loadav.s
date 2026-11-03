.set __dpc_fragment_origin,0xe14bba
.section ".text.PROC1_.WIRED_PROC","ax",@progbits
.space 2
.globl PROC1_$GET_LOADAV
PROC1_$GET_LOADAV:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
lea (%a5),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.PROC1_.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 12
