.text
.globl YS_SLOT_$INIT
YS_SLOT_$INIT:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0x28,%d0
move.l %d0,(0xc,%a5)
moveq #0x1e,%d1
move.l %d1,(8,%a5)
moveq #0x14,%d2
move.l %d2,(4,%a5)
moveq #0xa,%d3
move.l %d3,(%a5)
moveq #1,%d4
move.l %d4,(0x10,%a5)
lea (8,%a5),%a0
move.l %a0,(0x14,%a5)
lea (%a5),%a0
move.l %a0,(0x18,%a5)
move.l #2,(0x1c,%a5)
clr.l (0x20,%a5)
lea (4,%a5),%a0
move.l %a0,(0x24,%a5)
clr.w %d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
lsl.w #2,%d1
clr.l (0x28,%a5,%d1.w)
move.w %d0,%d1
lsl.w #2,%d1
clr.l (0x48,%a5,%d1.w)
addq.w #1,%d0
2:
cmpi.w #7,%d0
ble.s 1b
lea (0x1c,%a5),%a0
move.l %a0,(0x2c,%a5)
lea (0x10,%a5),%a0
move.l %a0,(0x30,%a5)
lea (%a5),%a0
move.l %a0,(0x4c,%a5)
lea (4,%a5),%a0
move.l %a0,(0x50,%a5)
lea (8,%a5),%a0
move.l %a0,(0x64,%a5)
lea (4,%a5),%a0
move.l %a0,(0x68,%a5)
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl SHOW
SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
tst.l (0x70,%a5)
bne.s 1f
pea (-1).w
bra.s 3f
1:
movea.l (0x70,%a5),%a0
move.l (%a0),-(%sp)
3:
jsr (EXEC_$LOGV_L).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FIND
FIND:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
move.w %d0,%d1
lsl.w #2,%d1
tst.l (0x28,%a5,%d1.w)
bne.s 1f
moveq #1,%d0
move.l %d0,(%a0)
clr.l (-4,%fp)
bra.s 2f
1:
move.w %d0,%d1
lsl.w #2,%d1
movea.l (0x28,%a5,%d1.w),%a1
tst.l (4,%a1)
bne.s 3f
moveq #2,%d0
move.l %d0,(%a0)
clr.l (-4,%fp)
bra.s 4f
3:
clr.l (%a0)
move.w %d0,%d1
lsl.w #2,%d1
movea.l (0x28,%a5,%d1.w),%a0
move.l (4,%a0),(-4,%fp)
4:
2:
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl YS_SLOT_$T_FIND
YS_SLOT_$T_FIND:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
pea (0x6c,%a5)
move.w (8,%fp),-(%sp)
bsr.s FIND
addq.w #8,%sp
move.l %a0,(0x70,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FCASE
FCASE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
ext.l %d1
move.l %d1,(0x6c,%a5)
move.w %d0,%d1
cmpi.w #4,%d1
bcc.s 6f
add.w %d1,%d1
move.w (7f,%pc,%d1.w),%d1
jmp (7f,%pc,%d1.w)
7:
.short 2f-7b
.short 3f-7b
.short 4f-7b
.short 5f-7b
2:
move.l (0x68,%a5),(-4,%fp)
bra.s 1f
3:
movea.l (0x30,%a5),%a0
move.l (4,%a0),(-4,%fp)
bra.s 1f
4:
move.l (0x4c,%a5),(-4,%fp)
bra.s 1f
5:
move.l (0x18,%a5),(-4,%fp)
bra.s 1f
6:
move.l (0x24,%a5),(-4,%fp)
1:
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl YS_SLOT_$T_CASE
YS_SLOT_$T_CASE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s FCASE
addq.w #4,%sp
move.l %a0,(0x70,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FWITH
FWITH:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (0x6c,%a5)
tst.w %d0
spl %d1
cmpi.w #7,%d0
sle %d2
and.b %d2,%d1
move.w %d0,%d2
lsl.w #2,%d2
tst.l (0x28,%a5,%d2.w)
sne %d0
and.b %d0,%d1
bpl.s 1f
movea.l (0x28,%a5,%d2.w),%a0
move.l (4,%a0),(-4,%fp)
bra.s 2f
1:
clr.l (-4,%fp)
2:
movea.l (-4,%fp),%a0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl YS_SLOT_$T_WITH
YS_SLOT_$T_WITH:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s FWITH
addq.w #4,%sp
move.l %a0,(0x70,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FCSE
FCSE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (0x6c,%a5)
move.w %d0,%d1
lsl.w #2,%d1
tst.l (0x48,%a5,%d1.w)
beq.s 1f
move.l (0x48,%a5,%d1.w),(-4,%fp)
bra.s 2f
1:
clr.l (-4,%fp)
2:
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl YS_SLOT_$T_CSE
YS_SLOT_$T_CSE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
subq.l #2,%sp
move.w %d2,%d0
addq.w #5,%d0
move.w %d0,-(%sp)
bsr.s FCSE
addq.w #4,%sp
move.l %a0,(0x70,%a5)
bsr.w SHOW
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl SETP
SETP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
bne.s 1f
lea (4,%a5),%a1
bra.s 3f
1:
lea (%a5),%a1
3:
move.l %a1,(%a0)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FESC
FESC:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
pea (-4,%fp)
bsr.s SETP
moveq #1,%d0
move.l %d0,(0x6c,%a5)
move.l (-4,%fp),(-8,%fp)
movea.l (-8,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl YS_SLOT_$T_ESC
YS_SLOT_$T_ESC:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s FESC
addq.w #4,%sp
move.l %a0,(0x70,%a5)
bsr.w SHOW
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl FIDX
FIDX:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l (0x6c,%a5)
cmpi.w #5,%d0
bgt.s 1f
move.w %d0,%d1
lsl.w #2,%d1
move.l (0x60,%a5,%d1.w),(-4,%fp)
1:
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl YS_SLOT_$T_IDX
YS_SLOT_$T_IDX:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0x68,%a5),%d0
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s FIDX
addq.w #4,%sp
move.l %a0,%d0
move.l %d0,(0x70,%a5)
cmpi.w #5,%d2
ble.s 1f
pea (-1).w
jsr (EXEC_$LOGV_L).l
bra.s 2f
1:
bsr.w SHOW
2:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.YS_SLOT","aw",@nobits
.balign 4
.Ldata:
.space 116
