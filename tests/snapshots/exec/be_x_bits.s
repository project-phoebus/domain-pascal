.text
.globl BX_BITS_$SETS
BX_BITS_$SETS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #2,(0xc,%a5)
clr.w %d1
bset %d0,%d1
move.w (0xc,%a5),%d0
add.w %d0,%d0
move.w %d1,(-2,%a5,%d0.w)
move.w (0xc,%a5),%d0
add.w %d0,%d0
move.w (-2,%a5,%d0.w),%d0
btst #15,%d0
beq.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
clr.w -(%sp)
5:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.w (0xc,%a5),%d0
add.w %d0,%d0
move.w (-2,%a5,%d0.w),%d0
btst #7,%d0
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 6f
3:
subq.l #2,%sp
clr.w -(%sp)
6:
jsr (EXEC_$MARK).l
4:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_BITS_$LSETS
BX_BITS_$LSETS:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #1,(0xc,%a5)
clr.l %d1
bset %d0,%d1
move.w (0xc,%a5),%d0
lsl.w #2,%d0
move.l %d1,(0,%a5,%d0.w)
move.w (0xc,%a5),%d0
lsl.w #2,%d0
move.l (0,%a5,%d0.w),%d0
btst #31,%d0
beq.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
clr.w -(%sp)
5:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.w (0xc,%a5),%d0
lsl.w #2,%d0
move.l (0,%a5,%d0.w),%d0
btst #15,%d0
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 6f
3:
subq.l #2,%sp
clr.w -(%sp)
6:
jsr (EXEC_$MARK).l
4:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_BITS","aw",@nobits
.balign 4
.Ldata:
.space 16
