.text
.globl TAKE
TAKE:
link.w %fp,#0
pea (8,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
pea (0xa,%fp)
jsr (EXEC_$LOG_BOOL).l
unlk %fp
rts
.globl TAKE_REF
TAKE_REF:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_BOOL).l
unlk %fp
rts
.globl PROBE
PROBE:
link.w %fp,#-4
move.l %d2,-(%sp)
subq.l #2,%sp
move.b (0xa,%fp),%d2
move.w (8,%fp),-(%sp)
jsr (EXEC_$MARK).l
move.b %d2,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl IS_POS
IS_POS:
link.w %fp,#-4
tst.w (8,%fp)
sgt %d0
unlk %fp
rts
.globl BOOL_VALUES_$NOTS
BOOL_VALUES_$NOTS:
link.w %fp,#-0xc
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.b (0xa,%fp),%d3
cmpi.w #1,%d2
seq %d0
move.b %d0,(%a5)
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b %d3,%d0
not.b %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
pea (-2,%fp)
bsr.w TAKE_REF
addq.w #4,%sp
move.b %d3,(8,%a5)
lea (6,%a5),%a0
move.l %a0,(0xc,%a5)
move.b (2,%a0),%d0
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
movea.l (0xc,%a5),%a0
move.b (2,%a0),%d0
not.b %d0
move.b %d0,(2,%a0)
pea (8,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w IS_POS
addq.w #4,%sp
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b %d3,%d0
lsr.b #7,%d0
andi.b #0xfe,(0x10,%a5)
or.b %d0,(0x10,%a5)
move.b %d3,%d0
not.b %d0
lsr.b #7,%d0
andi.b #0xfd,(0x10,%a5)
add.b %d0,%d0
or.b %d0,(0x10,%a5)
btst #0,(0x10,%a5)
seq %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
btst #1,(0x10,%a5)
seq %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
cmpi.w #1,%d2
sne %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b (%a5),%d0
or.b %d3,%d0
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b (%a5),%d0
and.b %d3,%d0
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b (%a5),%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w %d2
sle %d0
move.b %d0,-(%sp)
move.b %d3,%d0
not.b %d0
move.b %d0,-(%sp)
bsr.w TAKE
addq.w #4,%sp
move.b (%a5),%d0
not.b %d0
move.b %d0,(-0xa,%fp)
pea (-0xa,%fp)
bsr.w TAKE_REF
movem.l (-0x18,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOOL_VALUES_$SHORT
BOOL_VALUES_$SHORT:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d3,(4,%a5)
tst.w %d2
bne.s 1f
cmpi.w #4,%d3
bne.s 1f
st %d0
bra.s 2f
1:
clr.b %d0
2:
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w %d2
bne.s 5f
cmpi.w #4,(4,%a5)
bne.s 3f
5:
st %d0
bra.s 4f
3:
clr.b %d0
4:
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w %d2
bne.s 6f
cmpi.w #4,%d3
seq %d0
move.b %d0,-(%sp)
move.w #1,-(%sp)
bsr.w PROBE
addq.w #4,%sp
tst.b %d0
bpl.s 6f
st %d0
bra.s 7f
6:
clr.b %d0
7:
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w %d2
bne.s 10f
cmpi.w #3,%d3
seq %d0
move.b %d0,-(%sp)
move.w #1,-(%sp)
bsr.w PROBE
addq.w #4,%sp
tst.b %d0
bpl.s 8f
10:
st %d0
bra.s 9f
8:
clr.b %d0
9:
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.b (%a5)
bpl.s 11f
tst.b (2,%a5)
bpl.s 11f
st %d0
bra.s 12f
11:
clr.b %d0
12:
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.b (%a5)
bpl.s 15f
tst.b (2,%a5)
bmi.s 13f
15:
st %d0
bra.s 14f
13:
clr.b %d0
14:
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w %d2
bne.s 19f
cmpi.w #3,%d3
bgt.s 18f
19:
cmpi.w #2,%d2
bne.s 16f
cmpi.w #4,%d3
bne.s 16f
18:
st %d0
bra.s 17f
16:
clr.b %d0
17:
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
cmpi.w #4,%d3
beq.s 22f
tst.w %d2
bne.s 20f
22:
st %d0
bra.s 21f
20:
clr.b %d0
21:
move.b %d0,-(%sp)
tst.w %d2
bne.s 23f
cmpi.w #4,(4,%a5)
bne.s 23f
st %d0
bra.s 24f
23:
clr.b %d0
24:
move.b %d0,-(%sp)
bsr.w TAKE
addq.w #4,%sp
tst.w %d2
bne.s 25f
cmpi.w #4,%d3
bne.s 25f
st %d0
bra.s 26f
25:
clr.b %d0
26:
move.b %d0,(-4,%fp)
pea (-4,%fp)
bsr.w TAKE_REF
addq.w #4,%sp
tst.w %d2
bne.s 27f
cmpi.w #4,%d3
bne.s 27f
st %d0
bra.s 28f
27:
clr.b %d0
28:
not.b %d0
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOOL_VALUES_$HELD
BOOL_VALUES_$HELD:
link.w %fp,#-8
movem.l %d2-%d5,-(%sp)
move.w (8,%fp),%d2
clr.w %d3
clr.w %d4
clr.b %d5
.balignw 4,0x2048
1:
move.b %d5,%d0
not.b %d0
move.b %d0,(-6,%fp)
cmpi.w #1,%d3
beq.s 4f
cmpi.w #2,%d3
bne.s 2f
4:
st %d0
bra.s 3f
2:
clr.b %d0
3:
move.b %d0,%d5
tst.b (-6,%fp)
bpl.s 5f
addq.w #1,%d4
5:
tst.b %d5
bpl.s 6f
addi.w #0xa,%d4
6:
pea (-6,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
addq.w #1,%d3
cmp.w %d2,%d3
blt.s 1b
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x18,%fp),%d2-%d5
unlk %fp
rts
.globl BOOL_VALUES_$FUNC
BOOL_VALUES_$FUNC:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
subq.l #2,%sp
move.w (0xa,%fp),%d3
move.w #0x15,-(%sp)
jsr (EXEC_$MARK).l
tst.w %d2
bne.s 3f
cmpi.w #4,%d3
bne.s 1f
cmpi.w #1,%d2
bne.s 1f
3:
st %d0
bra.s 2f
1:
clr.b %d0
2:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BOOL_VALUES_$NOTF
BOOL_VALUES_$NOTF:
link.w %fp,#-4
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w IS_POS
not.b %d0
unlk %fp
rts
.globl BOOL_VALUES_$MIXED
BOOL_VALUES_$MIXED:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
cmpi.w #1,%d2
seq %d0
move.b %d0,(%a5)
cmpi.w #1,%d3
sle %d0
and.b (%a5),%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
cmpi.w #1,%d2
bne.s 1f
tst.w %d3
bne.s 1f
st %d0
bra.s 2f
1:
clr.b %d0
2:
cmpi.w #7,%d3
seq %d1
or.b %d1,%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b (%a5),%d0
tst.w %d3
beq.s 5f
cmpi.w #5,%d2
bne.s 3f
5:
st %d1
bra.s 4f
3:
clr.b %d1
4:
and.b %d1,%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b (%a5),%d0
not.b %d0
cmpi.w #7,%d3
sne %d1
or.b %d1,%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.b (%a5),%d0
not.b %d0
tst.w %d3
sne %d1
and.b %d1,%d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.b (%a5)
bpl.s 6f
tst.w %d3
bne.s 6f
st %d0
bra.s 7f
6:
clr.b %d0
7:
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOOL_VALUES_$LANES
BOOL_VALUES_$LANES:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d2
moveq #0xf,%d0
and.w %d2,%d0
cmpi.w #0xf,%d0
seq %d0
move.b %d0,(-2,%fp)
bpl.s 1f
move.w #1,(4,%a5)
bra.s 2f
1:
clr.w (4,%a5)
2:
subq.l #2,%sp
move.w (4,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
btst #4,%d2
sne %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BOOL_VALUES_$LANES2
BOOL_VALUES_$LANES2:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d1
moveq #0xf,%d0
and.w %d1,%d0
cmpi.w #0xf,%d0
seq %d0
move.b %d0,%d2
btst #4,%d1
sne %d3
move.b %d3,-(%sp)
move.b %d2,-(%sp)
bsr.w TAKE
addq.w #4,%sp
move.b %d2,%d0
not.b %d0
and.b %d3,%d0
move.b %d0,%d2
move.b %d2,%d0
not.b %d0
move.b %d0,-(%sp)
move.b %d2,-(%sp)
bsr.w TAKE
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.globl BOOL_VALUES_$PK7
BOOL_VALUES_$PK7:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.b (0xa,%fp),%d1
andi.b #0x80,(0x12,%a5)
move.b %d0,%d2
andi.b #0x7f,%d2
or.b %d2,(0x12,%a5)
andi.b #0x7f,(0x12,%a5)
move.b %d1,%d2
andi.b #0x80,%d2
or.b %d2,(0x12,%a5)
andi.b #1,(0x14,%a5)
move.b %d0,%d2
add.b %d2,%d2
andi.b #0xfe,%d2
or.b %d2,(0x14,%a5)
move.b %d1,%d0
lsr.b #7,%d0
andi.b #0xfe,(0x14,%a5)
or.b %d0,(0x14,%a5)
tst.w (0x12,%a5)
spl %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
btst #0,(0x14,%a5)
seq %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w (0x12,%a5)
smi %d0
move.b %d0,-(%sp)
tst.w (0x12,%a5)
spl %d0
move.b %d0,-(%sp)
bsr.w TAKE
addq.w #4,%sp
tst.w (0x12,%a5)
bpl.s 1f
btst #0,(0x14,%a5)
beq.s 1f
st %d0
bra.s 2f
1:
clr.b %d0
2:
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w (0x12,%a5)
bpl.s 3f
btst #0,(0x14,%a5)
beq.s 3f
st %d0
bra.s 4f
3:
clr.b %d0
4:
not.b %d0
move.b %d0,(2,%a5)
pea (2,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BOOL_VALUES","aw",@nobits
.balign 4
.Ldata:
.space 24
