.text
.globl SKFP_$PTR
SKFP_$PTR:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d0
sne %d0
move.w %d2,%d1
lsl.w #2,%d1
add.w (%a5),%d1
move.b %d0,(1,%a5,%d1.w)
pea (0xa,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(-4,%fp)
seq %d0
move.w %d2,%d1
lsl.w #2,%d1
add.w (%a5),%d1
move.b %d0,(1,%a5,%d1.w)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl SKFP_$SHOW
SKFP_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
tst.b (1,%a5,%d0.w)
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.SKFP","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x02
.space 102
