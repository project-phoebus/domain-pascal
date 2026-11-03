.set __dpc_fragment_origin,0xe1c084
.section ".text.TTY.WIRED_PROC","ax",@progbits
.globl TTY_$K_FLUSH_INPUT
TTY_$K_FLUSH_INPUT:
link.w %fp,#-0xc
movem.l %a2/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
movea.l (0xc,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a2
move.l #TTY_$SPIN_LOCK,-(%sp)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-6,%fp)
pea (%a2)
bsr.w TTY_$I_FLUSH_INPUT
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
move.l #TTY_$SPIN_LOCK,-(%sp)
jsr (ML_$SPIN_UNLOCK).l
1:
movem.l (-0x14,%fp),%a2/%a5
unlk %fp
rts
.globl TTY_$K_FLUSH_OUTPUT
TTY_$K_FLUSH_OUTPUT:
link.w %fp,#-0xc
movem.l %a2/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
movea.l (0xc,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a2
move.l #TTY_$SPIN_LOCK,-(%sp)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-6,%fp)
pea (%a2)
bsr.w TTY_$I_FLUSH_OUTPUT
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
move.l #TTY_$SPIN_LOCK,-(%sp)
jsr (ML_$SPIN_UNLOCK).l
1:
movem.l (-0x14,%fp),%a2/%a5
unlk %fp
rts
.section ".bss.moddata.TTY.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 4
