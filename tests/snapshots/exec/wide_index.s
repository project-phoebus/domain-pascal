.text
.globl WI_$U228
WI_$U228:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #WI_$R228,%a0
move.w %d0,%d1
mulu.w #0xe4,%d1
lea (0,%a0,%d1.l),%a1
move.l (-0xe4,%a1),%d0
unlk %fp
rts
.globl WI_$S228
WI_$S228:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #WI_$R228,%a0
move.w %d0,%d1
muls.w #0xe4,%d1
lea (0,%a0,%d1.l),%a1
move.l (-0xe4,%a1),%d0
unlk %fp
rts
.globl WI_$U48
WI_$U48:
link.w %fp,#-4
move.w (8,%fp),%d0
bne.s 1f
clr.l %d0
bra.s 2f
1:
clr.l %d1
movea.l #WI_$R48,%a0
move.w %d0,%d1
lsl.l #4,%d1
move.l %d1,%d0
add.l %d0,%d0
add.l %d0,%d1
move.l (-0x30,%a0,%d1.l),%d0
2:
unlk %fp
rts
.globl WI_$SET48
WI_$SET48:
link.w %fp,#0
move.w (8,%fp),%d0
beq.s 1f
clr.l %d1
movea.l #WI_$R48,%a0
move.w %d0,%d1
lsl.l #4,%d1
move.l %d1,%d0
add.l %d0,%d0
add.l %d0,%d1
move.l #-0x2bc,(-0x30,%a0,%d1.l)
1:
unlk %fp
rts
.globl WI_$S24
WI_$S24:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l #WI_$R24,%a0
ext.l %d0
lsl.l #3,%d0
move.l %d0,%d1
add.l %d1,%d1
add.l %d1,%d0
lea (0,%a0,%d0.l),%a1
move.l (0x5dc0,%a1),%d0
unlk %fp
rts
.globl WI_$S44
WI_$S44:
link.w %fp,#-4
moveq #0x2c,%d0
movea.l (8,%fp),%a0
movea.l #WI_$R44,%a1
muls.w (%a0),%d0
move.l (-0x2c,%a1,%d0.l),%d0
unlk %fp
rts
.globl WI_$CHAR
WI_$CHAR:
link.w %fp,#-4
move.w (8,%fp),%d0
cmpi.w #-1,%d0
bne.s 1f
moveq #-1,%d0
bra.s 2f
1:
clr.l %d1
movea.l #WI_$CHARS,%a0
move.w %d0,%d1
clr.w %d0
move.b (0,%a0,%d1.l),%d0
2:
unlk %fp
rts
.balignw 4,0x2048
