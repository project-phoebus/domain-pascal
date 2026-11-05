.text
.globl BX_CMPFOR_$PTRS
BX_CMPFOR_$PTRS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,(-2,%fp)
move.w %d1,(-4,%fp)
lea (-2,%fp),%a0
move.l %a0,(4,%a5)
cmp.w %d1,%d0
bne.s 1f
lea (-2,%fp),%a0
bra.s 5f
1:
lea (-4,%fp),%a0
5:
move.l %a0,(8,%a5)
2:
move.l (4,%a5),%d0
cmp.l (8,%a5),%d0
bne.s 3f
move.w #1,(2,%a5)
bra.s 4f
3:
clr.w (2,%a5)
4:
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BX_CMPFOR_$ENTRYORD
BX_CMPFOR_$ENTRYORD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
ble.s 1f
move.w #1,(2,%a5)
bra.s 2f
1:
clr.w (2,%a5)
2:
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_CMPFOR_$ANDCALL
BX_CMPFOR_$ANDCALL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
cmpi.w #2,%d0
sgt %d1
move.b %d1,%d2
cmpi.w #0xa,%d0
slt %d0
and.b %d0,%d2
bmi.s 2f
pea (8,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmpi.w #7,%d0
seq %d2
tst.b %d2
bpl.s 1f
2:
move.w #1,(2,%a5)
bra.s 3f
1:
clr.w (2,%a5)
3:
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BX_CMPFOR_$EQCOUNT
BX_CMPFOR_$EQCOUNT:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
moveq #4,%d2
moveq #4,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BX_CMPFOR_$NOCALL
BX_CMPFOR_$NOCALL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.w %d2
move.w %d0,(%a5)
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
add.w %d1,%d2
addq.w #1,%d1
dbf %d0,2b
1:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_CMPFOR","aw",@nobits
.balign 4
.Ldata:
.space 12
