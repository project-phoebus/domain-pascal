.text
.globl STORES
STORES:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
cmpi.w #3,(2,%a0)
seq %d0
move.b %d0,(%a5)
clr.w %d0
.balignw 4,0x2048
1:
tst.w (%a0)
seq %d1
tst.b %d1
bpl.s 2f
addq.w #1,%d0
2:
tst.b %d1
bmi.s 3f
addq.w #2,%d0
3:
cmp.w (2,%a5),%d0
ble.s 1b
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PUSHES
PUSHES:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (4,%a0),%d0
btst #1,%d0
sne %d0
move.b %d0,-(%sp)
movea.l (8,%fp),%a1
cmpi.w #3,(2,%a1)
sgt %d0
move.b %d0,-(%sp)
jsr (TAKE).l
unlk %fp
rts
.globl FRAME_STORE
FRAME_STORE:
link.w %fp,#-4
movea.l (8,%fp),%a0
cmpi.w #5,(%a0)
slt %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (Q).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BOOL_STORES","aw",@nobits
.balign 4
.Ldata:
.space 4
