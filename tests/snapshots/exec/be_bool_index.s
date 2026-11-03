.text
.globl BI_$PICK
BI_$PICK:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w #0xb,(%a5)
move.w #0x16,(2,%a5)
move.l #0x186a0,(4,%a5)
move.l #0x30d40,(8,%a5)
cmpi.w #5,%d2
sgt %d3
subq.l #2,%sp
move.b %d3,%d0
lsr.b #7,%d0
andi.w #0xff,%d0
add.w %d0,%d0
move.w (0,%a5,%d0.w),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
tst.w %d2
smi %d0
lsr.b #7,%d0
andi.w #0xff,%d0
lsl.w #2,%d0
move.l (4,%a5,%d0.w),-(%sp)
jsr (EXEC_$LOGV_L).l
cmpi.w #3,%d2
seq %d0
lsr.b #7,%d0
andi.w #0xff,%d0
add.w %d0,%d0
addq.w #1,(0,%a5,%d0.w)
move.w (2,%a5),%d0
add.w %d0,%d0
add.w (%a5),%d0
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_BOOL_INDEX","aw",@nobits
.balign 4
.Ldata:
.space 12
