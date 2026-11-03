.text
.globl EB_$FILL
EB_$FILL:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d1
bra.s 2f
.balignw 4,0x2048
1:
move.w %d1,%d0
move.w %d1,%d2
lsl.w #6,%d0
add.w %d2,%d2
subi.w #0x3e8,%d0
move.w %d0,(-2,%a5,%d2.w)
addq.w #1,%d1
2:
cmpi.w #8,%d1
ble.s 1b
clr.w %d1
bra.s 5f
.balignw 4,0x2048
4:
move.w %d1,%d0
ext.l %d0
add.l %d0,%d0
move.l %d0,%d2
lsl.l #2,%d2
add.l %d2,%d0
move.w %d1,%d2
addq.l #1,%d0
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.l %d0,(0x10,%a5,%d2.w)
move.w %d1,%d0
ext.l %d0
add.l %d0,%d0
move.l %d0,%d2
lsl.l #2,%d2
add.l %d2,%d0
move.w %d1,%d2
addq.l #2,%d0
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.l %d0,(0x14,%a5,%d2.w)
move.w %d1,%d0
ext.l %d0
add.l %d0,%d0
move.l %d0,%d2
lsl.l #2,%d2
add.l %d2,%d0
move.w %d1,%d2
addq.l #3,%d0
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.l %d0,(0x18,%a5,%d2.w)
addq.w #1,%d1
5:
cmpi.w #5,%d1
ble.s 4b
moveq #1,%d1
bra.s 8f
.balignw 4,0x2048
7:
move.w %d1,%d0
lsl.w #3,%d0
move.w %d0,%d2
neg.w %d0
lsl.w #3,%d2
add.w %d2,%d0
move.w %d1,%d2
muls.w #0x3e8,%d2
move.l %d2,(0x20,%a5,%d0.w)
move.w %d1,%d0
lsl.w #3,%d0
move.w %d0,%d2
neg.w %d0
lsl.w #3,%d2
add.w %d2,%d0
move.w %d1,%d2
ext.l %d2
neg.l %d2
move.l %d2,(0x54,%a5,%d0.w)
addq.w #1,%d1
8:
cmpi.w #4,%d1
ble.s 7b
clr.w %d1
bra.s 11f
.balignw 4,0x2048
10:
move.w %d1,%d0
lea (0,%a5,%d1.w),%a0
lsl.w #4,%d0
addi.w #0xf,%d0
move.b %d0,(0x138,%a0)
addq.w #1,%d1
11:
cmpi.w #0xf,%d1
ble.s 10b
moveq #-5,%d1
bra.s 14f
.balignw 4,0x2048
13:
move.w %d1,%d0
move.w %d0,%d2
add.w %d2,%d2
add.w %d2,%d0
move.w %d1,%d2
subq.w #1,%d0
add.w %d2,%d2
lea (0,%a5,%d2.w),%a0
move.w %d0,(0x158,%a0)
addq.w #1,%d1
14:
cmpi.w #5,%d1
ble.s 13b
clr.w %d1
bra.s 17f
.balignw 4,0x2048
16:
move.w %d1,%d0
move.l #0x186a0,-(%sp)
ext.l %d0
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d2
move.w %d1,%d0
lsl.w #2,%d0
lea (0,%a5,%d0.w),%a0
move.l %d2,(0x164,%a0)
addq.w #1,%d1
17:
cmpi.w #9,%d1
ble.s 16b
moveq #-3,%d1
bra.s 20f
.balignw 4,0x2048
19:
move.w %d1,%d0
ext.l %d0
add.l %d0,%d0
move.l %d0,%d2
lsl.l #2,%d2
add.l %d2,%d0
move.w %d1,%d2
addq.l #1,%d0
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
lea (0,%a5,%d2.w),%a0
move.l %d0,(0x1b0,%a0)
move.w %d1,%d0
ext.l %d0
add.l %d0,%d0
move.l %d0,%d2
lsl.l #2,%d2
add.l %d2,%d0
move.w %d1,%d2
addq.l #2,%d0
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
lea (0,%a5,%d2.w),%a0
move.l %d0,(0x1b4,%a0)
move.w %d1,%d0
ext.l %d0
add.l %d0,%d0
move.l %d0,%d2
lsl.l #2,%d2
add.l %d2,%d0
move.w %d1,%d2
addq.l #3,%d0
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
lea (0,%a5,%d2.w),%a0
move.l %d0,(0x1b8,%a0)
addq.w #1,%d1
20:
cmpi.w #3,%d1
ble.s 19b
lea (0x138,%a5),%a0
move.l %a0,(0x148,%a5)
move.w #3,(0x14c,%a5)
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELEM_BASES_DATA","aw",@nobits
.balign 4
.Ldata:
.globl EB_$WORDS
EB_$WORDS:
.space 16
.globl EB_$RECS
EB_$RECS:
.space 72
.globl EB_$BIG
EB_$BIG:
.space 224
.globl EB_$BYTES
EB_$BYTES:
.space 16
.globl EB_$PTR
EB_$PTR:
.space 4
.globl EB_$K
EB_$K:
.space 2
.globl EB_$NEG
EB_$NEG:
.space 22
.globl EB_$LONGS
EB_$LONGS:
.space 40
.globl EB_$NEGR
EB_$NEGR:
.space 84
