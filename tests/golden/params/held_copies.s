.text
.globl HC_$REFCNT
HC_$REFCNT:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),%d0
cmpi.l #-1,%d0
bne.s 1f
clr.w (-6,%fp)
bra.s 2f
1:
cmpi.l #0xfff5,%d0
bge.s 3f
move.w %d0,(-6,%fp)
bra.s 4f
3:
move.w #-0xb,(-6,%fp)
4:
2:
move.l (0xc,%fp),-(%sp)
pea (-6,%fp)
jsr (HC_$SET).l
unlk %fp
rts
.globl HC_$SERVER
HC_$SERVER:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
subq.l #2,%sp
move.b (%a0),%d2
move.w #4,-(%sp)
jsr (HC_$LOCK).l
addq.w #4,%sp
tst.b %d2
bpl.s 1f
movea.l (8,%fp),%a0
move.w (%a0),(2,%a5)
1:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl HC_$MODE
HC_$MODE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d0
move.w %d0,(%a5)
movea.l (0xc,%fp),%a1
clr.l (%a1)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HC_$PUSHED
HC_$PUSHED:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (HC_$TAKE).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (HC_$TAKE).l
unlk %fp
rts
.globl HC_$LOOP
HC_$LOOP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-4,%fp)
clr.w %d0
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,%d0
2:
cmp.w (-4,%fp),%d0
blt.s 1b
move.w %d0,(2,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HC_$PTR
HC_$PTR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
tst.l (-4,%fp)
beq.s 1f
move.l (-4,%fp),(4,%a5)
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HC_$ASSIGNED
HC_$ASSIGNED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
addq.w #1,(-2,%fp)
move.w (-2,%fp),(2,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HC_$ZERO
HC_$ZERO:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
tst.w (-2,%fp)
ble.s 1f
move.w (-2,%fp),(2,%a5)
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.HC","aw",@nobits
.balign 4
.Ldata:
.space 8
