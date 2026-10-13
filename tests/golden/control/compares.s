.text
ZEROS:
link.w %fp,#-4
tst.w (W).l
bne.s 1f
move.w #1,(R).l
1:
tst.l (L).l
beq.s 2f
move.w #2,(R).l
2:
tst.w (W).l
bpl.s 3f
move.w #3,(R).l
3:
tst.l (L).l
bmi.s 4f
move.w #4,(R).l
4:
tst.w (W).l
bgt.s 5f
move.w #5,(R).l
5:
tst.w (W).l
ble.s 6f
move.w #6,(R).l
6:
tst.l (P).l
bne.s 7f
move.w #7,(R).l
7:
tst.w (U).l
bne.s 8f
move.w #8,(R).l
8:
unlk %fp
rts
CONSTANTS:
link.w %fp,#-4
cmpi.w #5,(W).l
bne.s 1f
move.w #1,(R).l
1:
cmpi.w #-2,(W).l
bge.s 2f
move.w #2,(R).l
2:
cmpi.l #0x12345,(L).l
ble.s 3f
move.w #3,(R).l
3:
cmpi.w #-0x8000,(U).l
bcc.s 4f
move.w #4,(R).l
4:
cmpi.w #-1,(U).l
bcs.s 5f
move.w #5,(R).l
5:
cmpi.b #0x7a,(C).l
bhi.s 6f
move.w #6,(R).l
6:
cmpi.b #0xc8,(C).l
bls.s 7f
move.w #7,(R).l
7:
unlk %fp
rts
HELD:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
cmp.w (W).l,%d0
bge.s 1f
move.w #1,(R).l
1:
cmp.w (U).l,%d1
bls.s 2f
move.w #2,(R).l
2:
cmpi.w #7,%d0
bne.s 3f
move.w #3,(R).l
3:
unlk %fp
rts
BOOLEANS:
link.w %fp,#-4
tst.b (B).l
bpl.s 1f
move.w #1,(R).l
1:
tst.b (B).l
bmi.s 2f
move.w #2,(R).l
2:
tst.b (B).l
bpl.s 3f
tst.b (B2).l
bpl.s 3f
move.w #3,(R).l
3:
tst.b (B).l
bmi.s 5f
tst.b (B2).l
bpl.s 4f
5:
move.w #4,(R).l
4:
tst.b (B).l
bpl.s 7f
cmpi.w #1,(W).l
beq.s 6f
7:
move.w #5,(R).l
6:
cmpi.w #1,(W).l
beq.s 9f
tst.b (B2).l
bmi.s 8f
9:
move.w #6,(R).l
8:
unlk %fp
rts
.balignw 4,0x2048
