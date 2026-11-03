.set __dpc_fragment_origin,0xe1af0a
.section ".text.TTY.WIRED_PROC","ax",@progbits
.space 2
TTY_$I_BUF_INSERT:
link.w %fp,#-8
move.l %d2,-(%sp)
move.b (8,%fp),%d0
movea.l (0xa,%fp),%a0
cmpi.w #0x100,(2,%a0)
bne.s 1f
moveq #1,%d1
bra.s 2f
1:
move.w (2,%a0),%d1
addq.w #1,%d1
2:
cmp.w (%a0),%d1
beq.s 3f
move.w (2,%a0),%d2
move.b %d0,(5,%a0,%d2.w)
move.w %d1,(2,%a0)
3:
move.l (-0xc,%fp),%d2
unlk %fp
rts
TTY_$I_BUF_PUT:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
move.b (8,%fp),%d2
movea.l (0xa,%fp),%a2
move.l #TTY_$SPIN_LOCK,-(%sp)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-6,%fp)
cmpi.w #0x100,(2,%a2)
bne.s 1f
moveq #1,%d0
bra.s 2f
1:
move.w (2,%a2),%d0
addq.w #1,%d0
2:
cmp.w (%a2),%d0
beq.s 3f
move.w (2,%a2),%d1
move.b %d2,(5,%a2,%d1.w)
move.w %d0,(2,%a2)
3:
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
move.l #TTY_$SPIN_LOCK,-(%sp)
jsr (ML_$SPIN_UNLOCK).l
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
