.text
.globl P
P:
link.w %fp,#-0xc
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-0xc,%fp)
tst.b (%a5)
bmi.s 1f
moveq #1,%d0
move.l %d0,(0x14,%a5)
bra.s 2f
1:
moveq #2,%d2
move.l (4,%a5),%d0
bpl.s 3f
addi.l #0x3ff,%d0
3:
asr.l #8,%d0
asr.l #2,%d0
addi.l #0x1c0,%d0
move.l %d0,(4,%a5)
moveq #7,%d3
clr.w %d4
.balignw 4,0x2048
5:
cmp.w (-0xc,%fp),%d4
beq.s 2f
move.w %d4,(-8,%fp)
pea (-8,%fp)
jsr (WORK).l
addq.w #4,%sp
move.w #3,(0xa,%a5)
move.w #2,(0xc,%a5)
moveq #2,%d0
subq.w #1,%d0
cmpi.w #4,%d0
bcc.s 7f
add.w %d0,%d0
move.w (9f,%pc,%d0.w),%d0
jmp (9f,%pc,%d0.w)
9:
.short 8f-9b
.short 8f-9b
.short 8f-9b
.short 8f-9b
8:
move.w %d4,(0xe,%a5)
7:
andi.b #0x1f,(0x10,%a5)
move.b %d2,%d0
lsl.b #5,%d0
or.b %d0,(0x10,%a5)
addq.w #1,%d4
dbf %d3,5b
2:
clr.l (0x14,%a5)
movem.l (-0x1c,%fp),%d2-%d4/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CONST_SUBST","aw",@nobits
.balign 4
.Ldata:
.space 24
