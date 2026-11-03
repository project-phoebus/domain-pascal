.text
.globl IM_$FILL
IM_$FILL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
moveq #0x2c,%d2
addi.w #0x3e8,%d1
mulu.w %d0,%d2
move.w %d1,(-2,%a5,%d2.w)
moveq #0x54,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
addi.w #0xbb8,%d1
move.w %d1,(0x42e,%a0)
addq.w #1,%d0
2:
cmpi.w #8,%d0
ble.s 1b
moveq #-4,%d0
bra.s 5f
.balignw 4,0x2048
4:
moveq #0x2c,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
subi.w #0x3e8,%d1
move.w %d1,(0x23a,%a0)
addq.w #1,%d0
5:
cmpi.w #3,%d0
ble.s 4b
clr.w %d0
bra.s 8f
.balignw 4,0x2048
7:
moveq #0x2e,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
addi.w #0x7d0,%d1
move.w %d1,(0x2ec,%a0)
move.w %d0,%d1
mulu.w #0x10c,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
addi.w #0x1388,%d1
move.w %d1,(0xbc2,%a0)
addq.w #1,%d0
8:
cmpi.w #7,%d0
ble.s 7b
clr.w %d0
bra.s 11f
.balignw 4,0x2048
10:
moveq #0x64,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
addi.w #0xfa0,%d1
move.w %d1,(0x732,%a0)
addq.w #1,%d0
11:
cmpi.w #9,%d0
ble.s 10b
move.w #-0x12c,%d0
bra.s 14f
.balignw 4,0x2048
13:
moveq #0x2c,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,(0x46d2,%a0)
addq.w #1,%d0
14:
cmpi.w #0x12c,%d0
ble.s 13b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl IM_$GET44
IM_$GET44:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x2c,%d1
mulu.w %d0,%d1
move.w (-2,%a5,%d1.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$IN44
IM_$IN44:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x2c,%d0
movea.l (8,%fp),%a0
mulu.w (%a0),%d0
move.w (-2,%a5,%d0.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$SET44
IM_$SET44:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x2c,%d0
movea.l (8,%fp),%a0
mulu.w (%a0),%d0
movea.l (0xc,%fp),%a0
move.w (%a0),(-2,%a5,%d0.w)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl IM_$UNIT44
IM_$UNIT44:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(0x7a64,%a5)
moveq #0x2c,%d0
mulu.w (0x7a64,%a5),%d0
move.w (-2,%a5,%d0.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$NEG44
IM_$NEG44:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(0x7a64,%a5)
moveq #0x2c,%d0
mulu.w (0x7a64,%a5),%d0
lea (0,%a5,%d0.w),%a0
move.w (0x23a,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$GET46
IM_$GET46:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x2e,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w (0x2ec,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$GET84
IM_$GET84:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x54,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w (0x42e,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$GET100
IM_$GET100:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x64,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w (0x732,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$GET268
IM_$GET268:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
mulu.w #0x10c,%d1
lea (0,%a5,%d1.w),%a0
move.w (0xbc2,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$OTHER92
IM_$OTHER92:
link.w %fp,#-4
moveq #0x5c,%d0
movea.l (8,%fp),%a0
movea.l #IM_$O92,%a1
mulu.w (%a0),%d0
move.w (-2,%a1,%d0.w),%d0
unlk %fp
rts
.globl IM_$PROD
IM_$PROD:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x2c,%d1
muls.w %d0,%d1
addi.w #0x34,%d1
move.w %d1,%d0
unlk %fp
rts
.globl IM_$LPROD
IM_$LPROD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(0x7a64,%a5)
moveq #0x2c,%d0
muls.w (0x7a64,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$LPRODV
IM_$LPRODV:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x2c,%d1
muls.w %d0,%d1
move.l %d1,%d0
unlk %fp
rts
.globl IM_$BIG44
IM_$BIG44:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
moveq #0x2c,%d1
mulu.w %d0,%d1
lea (0,%a5,%d1.w),%a0
move.w (0x46d2,%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl IM_$BYTE44
IM_$BYTE44:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w %d0
move.b (8,%fp),%d0
mulu.w #0x2c,%d0
move.w (-2,%a5,%d0.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.INDEX_MUL","aw",@nobits
.balign 4
.Ldata:
.space 31336
