.text
.globl TF_$CREATE
TF_$CREATE:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0xe,%fp),%d2
subq.l #2,%sp
move.l (0x10,%fp),-(%sp)
move.w (0xc,%fp),-(%sp)
jsr (TF_$ALLOC).l
addq.w #8,%sp
move.l %d0,(-4,%fp)
move.w %d2,%d1
subq.w #3,%d1
cmpi.w #6,%d1
bcc.s 4f
add.w %d1,%d1
move.w (5f,%pc,%d1.w),%d1
jmp (5f,%pc,%d1.w)
5:
.short 2f-5b
.short 2f-5b
.short 2f-5b
.short 4f-5b
.short 4f-5b
.short 3f-5b
2:
move.w #5,(%a5)
bra.s 1f
3:
move.w #6,(%a5)
bra.s 1f
4:
clr.w (%a5)
1:
subq.l #2,%sp
move.l %d0,-(%sp)
move.w %d2,-(%sp)
jsr (TF_$USE).l
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl TF_$WORD
TF_$WORD:
link.w %fp,#-0x10
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
movea.l (0x10,%fp),%a0
move.w (%a0),(-0xa,%fp)
move.w (-4,%fp),(%a5)
movea.l (0xc,%fp),%a1
move.b (1,%a1),(2,%a5)
move.l #0x10002,(-0x10,%fp)
subq.l #2,%sp
move.w (-0xe,%fp),-(%sp)
jsr (TF_$TAKE).l
move.w (-0xa,%fp),%d0
lsl.w #3,%d0
move.w (0,%a5,%d0.w),(%a5)
movea.l (-0x14,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.TF","aw",@nobits
.balign 4
.Ldata:
.space 68
