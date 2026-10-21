.text
MASK:
link.w %fp,#-8
movea.l (0xa,%fp),%a0
move.l (%a0),%d0
unlk %fp
rts
.globl MEMBER
MEMBER:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0x10,%fp),%a1
movea.l (0xc,%fp),%a2
move.l (%a2),(-4,%fp)
movea.l (-4,%fp),%a2
btst #3,(7,%a2)
sne %d0
move.b %d0,(0x68,%a5)
clr.w (%a1)
btst #4,(0x18,%a0)
bne.s 1f
move.w #1,(%a1)
1:
tst.b (0x19,%a0)
bpl.s 2f
move.w #2,(%a1)
2:
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl CLEAR
CLEAR:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.b (0x1b,%a0)
move.b #0x10,(0x18,%a0)
move.b #0xf,(0x1a,%a0)
movea.l (0xc,%fp),%a1
move.b (0x19,%a1),(0x1c,%a0)
unlk %fp
rts
.globl UPDATE
UPDATE:
link.w %fp,#0
movea.l (8,%fp),%a0
bset #2,(0x1c,%a0)
bclr #6,(0x18,%a0)
bclr #0,(0x1b,%a0)
unlk %fp
rts
.globl MINR
MINR:
link.w %fp,#-4
movea.l (8,%fp),%a0
clr.w %d0
move.b (0x1b,%a0),%d0
btst #4,(0x18,%a0)
bne.s 1f
clr.w %d1
move.b (0x18,%a0),%d1
not.w %d1
and.w %d1,%d0
1:
btst #4,(0x19,%a0)
bne.s 2f
clr.w %d1
move.b (0x19,%a0),%d1
not.w %d1
and.w %d1,%d0
2:
btst #4,(0x1c,%a0)
bne.s 3f
clr.w %d1
move.b (0x1c,%a0),%d1
or.w %d1,%d0
3:
movea.l (0xc,%fp),%a1
move.w %d0,(%a1)
unlk %fp
rts
.globl EMPTY
EMPTY:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
clr.w (%a1)
btst #4,(0x1b,%a0)
bne.s 1f
clr.w %d0
move.b (0x1b,%a0),%d0
tst.w %d0
beq.s 1f
move.w #1,(%a1)
1:
unlk %fp
rts
ARG:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
subq.l #2,%sp
pea (%a3)
clr.w %d0
move.b (0x18,%a2),%d0
move.w %d0,-(%sp)
bsr.w MASK
addq.w #8,%sp
move.l %d0,(0x64,%a5)
subq.l #2,%sp
pea (%a3)
clr.w %d0
move.b (0x19,%a2),%d0
move.w %d0,-(%sp)
bsr.w MASK
move.l %d0,(0x64,%a5)
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl NARROW
NARROW:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
move.w (%a0),%d2
move.l (8,%fp),-(%sp)
pea (.Lc0,%pc)
jsr (CONVERT).l
move.b %d0,(0x53,%a5)
move.w %d2,%d0
add.w %d0,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #3,%d1
add.w %d1,%d0
move.b (0xd,%a5,%d0.w),(0x54,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0200,0x0000
.balignw 4,0x2048
.section ".bss.moddata.BYTE_SETS","aw",@nobits
.balign 4
.Ldata:
.space 108
