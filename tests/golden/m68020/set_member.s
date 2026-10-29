.arch 68020
.text
.globl ALLOC
ALLOC:
link.w %fp,#-4
move.l %d2,-(%sp)
movea.l (8,%fp),%a0
moveq #0x39,%d0
clr.w %d1
.balignw 4,0x2048
2:
lea (%a0),%a1
clr.l %d2
move.w %d1,%d2
not.l %d2
bftst (8,%a1){%d2:#1}
bne.s 4f
move.w %d1,%d0
bra.s 5f
4:
addq.w #1,%d1
dbf %d0,2b
moveq #-1,%d0
5:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl DRAINED
DRAINED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
clr.l %d1
move.w (%a0),%d0
lea (%a5),%a0
movea.l (8,%fp),%a1
move.b (0,%a1,%d0.w),%d1
not.l %d1
bftst (0x20,%a0){%d1:#1}
beq.s 1f
clr.w (G).l
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl HPIPC
HPIPC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
clr.l %d1
move.w (KEY).l,%d1
not.l %d1
bftst (0x2c,%a5,%d0.w){%d1:#1}
beq.s 1f
move.w #1,(G).l
1:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl INIT
INIT:
link.w %fp,#-4
move.b (8,%fp),%d0
clr.w %d1
move.b %d0,%d1
clr.l %d0
move.w %d1,%d0
not.l %d0
cmpi.l #-0x60,%d0
bcs.s 1f
bftst (.Lc0+12,%pc){%d0:#1}
beq.s 1f
move.w #2,(G).l
1:
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0xffff,0xffff,0xffff,0xffff,0xc010,0x07ff
.globl COUNT
COUNT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
movea.l (8,%fp),%a0
cmpi.w #1,(%a0)
bne.s 1f
move.w %d0,%d1
clr.l %d0
subq.w #1,%d1
move.w %d1,%d0
bftst (0x98,%a5){%d0:#1}
beq.s 2f
clr.w (G).l
2:
1:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl USED
USED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$CURRENT).l,%d0
clr.l %d1
subq.w #1,%d0
move.w %d0,%d1
bftst (0x98,%a5){%d1:#1}
sne %d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET_MEMBER","aw",@nobits
.balign 4
.Ldata:
.space 160
