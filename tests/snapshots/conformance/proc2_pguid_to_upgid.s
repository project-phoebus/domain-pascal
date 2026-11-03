.set __dpc_fragment_origin,0xe41072
.text
.space 2
.globl PROC2_$PGUID_TO_UPGID
PROC2_$PGUID_TO_UPGID:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
subq.l #2,%sp
move.w #4,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
pea (-8,%fp)
bsr.w PROC2_$UID_TO_UPGID_INTERNAL
addq.w #4,%sp
move.w %d0,%d2
subq.l #2,%sp
move.w #4,-(%sp)
jsr (ML_$UNLOCK).l
movea.l (0xc,%fp),%a0
move.w %d2,(%a0)
movea.l (0x10,%fp),%a1
clr.l (%a1)
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.section ".bss.moddata.PROC2","aw",@nobits
.balign 4
.Ldata:
.space 4
