.text
.globl BH_X_$NARROW
BH_X_$NARROW:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l #0x11223344,(%a5)
clr.w (-2,%fp)
cmpi.l #1,%d0
beq.s 1f
move.w #1,(-2,%fp)
1:
move.b %d0,(2,%a5)
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BH_X_$FF
BH_X_$FF:
link.w %fp,#0
subq.l #2,%sp
st -(%sp)
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
subq.l #2,%sp
move.w #-0x200,-(%sp)
jsr (EXEC_$LOGV_C).l
unlk %fp
rts
.globl BH_X_$PAST
BH_X_$PAST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (4,%a5),%a0
lea (0xc,%a5),%a1
move.l %a1,%d0
sub.l %a0,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
lea (0xc,%a5),%a0
lea (0x1c,%a5),%a1
move.l %a1,%d0
sub.l %a0,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
lea (0xc,%a5),%a0
lea (8,%a5),%a1
move.l %a1,%d0
sub.l %a0,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_H4BA_MISC","aw",@nobits
.balign 4
.Ldata:
.space 28
