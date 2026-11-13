.text
.globl LCE_$RUN
LCE_$RUN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(%a5)
cmpi.w #0x1f,%d0
sls %d0
cmpi.w #0x1f,(%a5)
bhi.s 1f
clr.l %d1
move.l #0x5f84006,%d2
move.w (%a5),%d1
btst %d1,%d2
beq.s 1f
st %d1
bra.s 2f
1:
clr.b %d1
2:
and.b %d1,%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
cmpi.w #0x1f,(%a5)
bhi.s 3f
clr.l %d0
move.l #0x5f84006,%d1
move.w (%a5),%d0
btst %d0,%d1
beq.s 3f
st %d0
bra.s 4f
3:
clr.b %d0
4:
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.LCE","aw",@nobits
.balign 4
.Ldata:
.space 4
