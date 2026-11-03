.text
.globl PCK_$SETUP
PCK_$SETUP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
lea (8,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(%a5)
lea (0x10,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(8,%a5)
movea.l #-1,%a0
move.l %a0,(0x10,%a5)
lea (%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0x18,%a5)
lea (0x5c,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
move.l %a0,(0xa4,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PCK_$FILL
PCK_$FILL:
link.w %fp,#-0x48
pea (%a5)
lea (.Ldata).l,%a5
moveq #0xf,%d0
lea (%fp),%a0
.balignw 4,0x2048
2:
movea.l #-1,%a1
move.l %a1,(-0x42,%a0)
addq.l #4,%a0
dbf %d0,2b
clr.l (-0x32,%fp)
moveq #0xf,%d0
lea (%fp),%a0
lea (%a5),%a1
.balignw 4,0x2048
5:
move.l (-0x42,%a0),(0x1c,%a1)
addq.l #4,%a1
addq.l #4,%a0
dbf %d0,5b
movea.l #5,%a0
movea.l (0xa4,%a5),%a1
move.l %a0,(0x22,%a1)
movea.l (-0x4c,%fp),%a5
unlk %fp
rts
.globl PCK_$SHOW
PCK_$SHOW:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xa4,%a5),%a0
move.l #0x7000,(2,%a0)
clr.w (-4,%fp)
moveq #0xf,%d0
lea (%a5),%a0
.balignw 4,0x2048
2:
cmpi.l #-1,(0x1c,%a0)
bne.s 4f
addq.w #1,(-4,%fp)
4:
addq.l #4,%a0
dbf %d0,2b
pea (-4,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.l (0x2c,%a5),(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
movea.l (0xa4,%a5),%a0
move.l (0x22,%a0),(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
move.l (0x5e,%a5),(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
move.l (0x10,%a5),(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$LOG_X).l
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PCK_$WALK
PCK_$WALK:
link.w %fp,#-0xc
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
clr.l %d0
movea.l (0x18,%a5),%a1
bra.s 2f
.balignw 4,0x2048
1:
add.l (4,%a1),%d0
movea.l (%a1),%a1
cmp.l (%a0),%d0
ble.s 4f
movea.l #-1,%a1
4:
2:
cmpa.l #-1,%a1
bne.s 1b
movea.l (-0x10,%fp),%a5
unlk %fp
rts
.globl PCK_$WALK_NIL
PCK_$WALK_NIL:
link.w %fp,#-0xc
pea (%a5)
lea (.Ldata).l,%a5
clr.l %d0
movea.l (0x18,%a5),%a0
bra.s 2f
.balignw 4,0x2048
1:
add.l (4,%a0),%d0
cmpi.l #-1,(%a0)
bne.s 4f
suba.l %a0,%a0
bra.s 5f
4:
movea.l (%a0),%a0
5:
2:
cmpa.w #0,%a0
bne.s 1b
movea.l (-0x10,%fp),%a5
unlk %fp
rts
.globl PCK_$IS_NONE
PCK_$IS_NONE:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
pea (-8,%fp)
jsr (EXEC_$ID_P).l
cmpa.l #-1,%a0
seq %d0
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PCK","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0a,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x14
.byte 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x1e
.space 144
