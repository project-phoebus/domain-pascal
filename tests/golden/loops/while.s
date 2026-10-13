.text
WHL_$COUNT:
link.w %fp,#-4
clr.w (W).l
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,(W).l
2:
cmpi.w #0xa,(W).l
blt.s 1b
unlk %fp
rts
WHL_$SCAN:
link.w %fp,#-4
move.w #1,(N).l
bra.s 2f
.balignw 4,0x2048
1:
addq.w #1,(N).l
2:
cmpi.w #0x40,(N).l
bgt.s 4f
move.w (N).l,%d0
cmpi.b #0x2f,(-1,%a5,%d0.w)
beq.s 1b
4:
unlk %fp
rts
WHL_$WALK:
link.w %fp,#-8
move.l (8,%fp),%d0
movea.l (0xc,%fp),%a0
movea.l %d0,%a1
bra.s 2f
.balignw 4,0x2048
1:
move.l (4,%a1),(%a0)
movea.l (%a1),%a1
2:
cmpa.w #0,%a1
bne.s 1b
unlk %fp
rts
WHL_$EXITS:
link.w %fp,#-4
bra.s 2f
.balignw 4,0x2048
1:
tst.b (B).l
bmi.s 3f
cmpi.w #3,(N).l
beq.s 2f
subq.w #1,(W).l
2:
tst.w (W).l
bne.s 1b
3:
clr.w (N).l
unlk %fp
rts
WHL_$REPEAT:
link.w %fp,#-4
.balignw 4,0x2048
1:
tst.b (B).l
bmi.s 2f
cmpi.w #3,(N).l
beq.s 3f
subq.w #1,(W).l
3:
tst.w (W).l
bne.s 1b
2:
clr.w (N).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.WHL","aw",@nobits
.balign 4
.Ldata:
.space 64
