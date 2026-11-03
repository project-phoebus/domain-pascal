.text
.globl CFL_$SMEAR
CFL_$SMEAR:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
moveq #1,%d1
bra.s 2f
.balignw 4,0x2048
1:
move.b #0xa5,(-1,%a0,%d1.w)
addq.w #1,%d1
2:
cmp.w %d0,%d1
ble.s 1b
unlk %fp
rts
.globl CFL_$MD
CFL_$MD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0x4c,-(%sp)
pea (%a5)
bsr.s CFL_$SMEAR
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CFL_$CHARS
CFL_$CHARS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.s CFL_$MD
moveq #6,%d0
moveq #3,%d1
.balignw 4,0x2048
2:
move.b #0x7a,(-1,%a5,%d1.w)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x004c
.globl CFL_$SBYTES
CFL_$SBYTES:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.s CFL_$MD
moveq #3,%d0
moveq #2,%d1
.balignw 4,0x2048
2:
move.b #0xfe,(0xb,%a5,%d1.w)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$BOOLS
CFL_$BOOLS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
moveq #5,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
st (0x11,%a5,%d1.w)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$WORDS
CFL_$WORDS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
moveq #5,%d0
lea (4,%a5),%a0
.balignw 4,0x2048
2:
move.w #-3,(0x16,%a0)
addq.l #2,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$WORD1
CFL_$WORD1:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
clr.w %d0
lea (0x10,%a5),%a0
.balignw 4,0x2048
2:
move.w #0x1234,(0x16,%a0)
addq.l #2,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$LONGS
CFL_$LONGS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
moveq #3,%d0
lea (4,%a5),%a0
.balignw 4,0x2048
2:
moveq #5,%d1
move.l %d1,(0x28,%a0)
addq.l #4,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$LONGNEG
CFL_$LONGNEG:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
moveq #5,%d0
lea (%a5),%a0
.balignw 4,0x2048
2:
move.l #-0x186a0,(0x28,%a0)
addq.l #4,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$LONG1
CFL_$LONG1:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
clr.w %d0
lea (8,%a5),%a0
.balignw 4,0x2048
2:
move.l #0x12345678,(0x28,%a0)
addq.l #4,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$ENUMS
CFL_$ENUMS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
moveq #2,%d0
lea (4,%a5),%a0
.balignw 4,0x2048
2:
move.w #2,(0x3e,%a0)
addq.l #2,%a0
dbf %d0,2b
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl CFL_$BIGW
CFL_$BIGW:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0x25c,-(%sp)
pea (0x4c,%a5)
bsr.w CFL_$SMEAR
addq.w #8,%sp
move.w #0x12b,%d0
lea (2,%a5),%a0
.balignw 4,0x2048
2:
move.w #0x7fff,(0x4c,%a0)
addq.l #2,%a0
dbf %d0,2b
pea (.Lc1,%pc)
pea (0x4c,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x2a2,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
clr.l (-8,%fp)
moveq #1,%d0
bra.s 5f
.balignw 4,0x2048
4:
move.w %d0,%d1
add.w %d1,%d1
move.w (0x4c,%a5,%d1.w),%d1
ext.l %d1
add.l (-8,%fp),%d1
move.l %d1,(-8,%fp)
addq.w #1,%d0
5:
cmpi.w #0x12c,%d0
ble.s 4b
pea (-8,%fp)
jsr (EXEC_$LOG_L).l
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.Lc1:
.short 0x0006
.globl CFL_$BIGL
CFL_$BIGL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0x328,-(%sp)
pea (0x2a8,%a5)
bsr.w CFL_$SMEAR
addq.w #8,%sp
movea.l %a5,%a0
move.w #0xc5,%d0
addq.l #8,%a0
.balignw 4,0x2048
2:
moveq #-1,%d1
move.l %d1,(0x2a8,%a0)
addq.l #4,%a0
dbf %d0,2b
pea (.Lc2,%pc)
pea (0x2a8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc2,%pc)
pea (0x5c4,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (0x438,%a5)
jsr (EXEC_$LOG_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc2:
.short 0x000c
.globl CFL_$BIGC
CFL_$BIGC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0x3ec,-(%sp)
pea (0x5d0,%a5)
bsr.w CFL_$SMEAR
addq.w #8,%sp
move.w #0x3e7,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
lea (0,%a5,%d1.w),%a0
move.b #0x71,(0x5d1,%a0)
addq.w #1,%d1
dbf %d0,2b
pea (.Lc3,%pc)
pea (0x5d0,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc3,%pc)
pea (0x9b8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (0x7c5,%a5)
jsr (EXEC_$LOG_C).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc3:
.short 0x0003
.globl CFL_$FRAME
CFL_$FRAME:
link.w %fp,#-0x28
subq.l #2,%sp
move.w #0x24,-(%sp)
pea (-0x28,%fp)
bsr.w CFL_$SMEAR
addq.w #8,%sp
moveq #2,%d0
lea (4,%fp),%a0
.balignw 4,0x2048
2:
move.w #-7,(-0x28,%a0)
addq.l #2,%a0
dbf %d0,2b
moveq #2,%d0
lea (4,%fp),%a0
.balignw 4,0x2048
5:
move.l #0x10000,(-0x20,%a0)
addq.l #4,%a0
dbf %d0,5b
moveq #4,%d0
moveq #1,%d1
.balignw 4,0x2048
8:
move.b #0x78,(-0x11,%fp,%d1.w)
addq.w #1,%d1
dbf %d0,8b
moveq #1,%d0
moveq #2,%d1
.balignw 4,0x2048
11:
st (-0xb,%fp,%d1.w)
addq.w #1,%d1
dbf %d0,11b
pea (.Lc4,%pc)
pea (-0x28,%fp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc4:
.short 0x0024
.globl CFL_$FRAMEL
CFL_$FRAMEL:
link.w %fp,#-0x20
subq.l #2,%sp
move.w #0x18,-(%sp)
pea (-0x20,%fp)
bsr.w CFL_$SMEAR
addq.w #8,%sp
moveq #3,%d0
lea (8,%fp),%a0
.balignw 4,0x2048
2:
moveq #-9,%d1
move.l %d1,(-0x24,%a0)
addq.l #4,%a0
dbf %d0,2b
pea (.Lc5,%pc)
pea (-0x20,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
clr.w %d0
lea (0x18,%fp),%a0
.balignw 4,0x2048
5:
moveq #0x7f,%d1
move.l %d1,(-0x24,%a0)
addq.l #4,%a0
dbf %d0,5b
pea (.Lc5,%pc)
pea (-0x20,%fp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc5:
.short 0x0018
.globl CFL_$PTR
CFL_$PTR:
link.w %fp,#-4
movea.l (8,%fp),%a0
moveq #3,%d0
lea (4,%a0),%a1
.balignw 4,0x2048
2:
moveq #-0x80,%d1
move.l %d1,(0,%a1)
addq.l #4,%a1
dbf %d0,2b
unlk %fp
rts
.globl CFL_$PTR_RUN
CFL_$PTR_RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
pea (0x28,%a5)
bsr.s CFL_$PTR
addq.w #4,%sp
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CFL_$VAR
CFL_$VAR:
link.w %fp,#-4
moveq #2,%d0
movea.l (8,%fp),%a0
lea (2,%a0),%a1
.balignw 4,0x2048
2:
move.w #0x64,(-2,%a1)
addq.l #2,%a1
dbf %d0,2b
unlk %fp
rts
.globl CFL_$VAR_RUN
CFL_$VAR_RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
pea (0x18,%a5)
bsr.s CFL_$VAR
addq.w #4,%sp
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CFL_$OUT
CFL_$OUT:
link.w %fp,#-4
moveq #5,%d0
movea.l (8,%fp),%a0
lea (%a0),%a1
.balignw 4,0x2048
2:
move.l #0xfff,(0,%a1)
addq.l #4,%a1
dbf %d0,2b
unlk %fp
rts
.globl CFL_$OUT_RUN
CFL_$OUT_RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
bsr.w CFL_$MD
pea (0x28,%a5)
bsr.s CFL_$OUT
addq.w #4,%sp
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CFL_$HOST
CFL_$HOST:
link.w %fp,#-4
moveq #2,%d0
movea.l (8,%fp),%a0
lea (%a0),%a1
.balignw 4,0x2048
2:
move.w #-1,(2,%a1)
addq.l #2,%a1
dbf %d0,2b
unlk %fp
rts
.globl CFL_$HOST_RUN
CFL_$HOST_RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #0xa,-(%sp)
pea (0x9bc,%a5)
bsr.w CFL_$SMEAR
addq.w #8,%sp
pea (0x9bc,%a5)
bsr.s CFL_$HOST
addq.w #4,%sp
pea (.Lc6,%pc)
pea (0x9bc,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc6:
.short 0x000a
.globl CFL_$EXT
CFL_$EXT:
link.w %fp,#-4
subq.l #2,%sp
move.w #0x1c,-(%sp)
move.l #CFL_$X,-(%sp)
bsr.w CFL_$SMEAR
addq.w #8,%sp
movea.l #CFL_$X,%a0
moveq #3,%d0
addq.l #8,%a0
.balignw 4,0x2048
2:
moveq #0x4d,%d1
move.l %d1,(-2,%a0)
addq.l #4,%a0
dbf %d0,2b
pea (.Lc7,%pc)
move.l #CFL_$X,-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc7:
.short 0x001c
.globl CFL_$EXTW
CFL_$EXTW:
link.w %fp,#-4
move.w #0xb,(CFL_$XG0).l
move.w #0x16,(CFL_$XG1).l
subq.l #2,%sp
move.w #0xa,-(%sp)
move.l #CFL_$XW,-(%sp)
bsr.w CFL_$SMEAR
addq.w #8,%sp
moveq #4,%d0
movea.l #CFL_$XW,%a0
.balignw 4,0x2048
2:
move.w #-0x8000,(0,%a0)
addq.l #2,%a0
dbf %d0,2b
pea (.Lc6,%pc)
move.l #CFL_$XW,-(%sp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.l #CFL_$XG0,-(%sp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
move.l #CFL_$XG1,-(%sp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CFL","aw",@nobits
.balign 4
.Ldata:
.space 2504
