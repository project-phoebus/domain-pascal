.text
.globl BE_L_COPIES_$RUN
BE_L_COPIES_$RUN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #3,%d0
moveq #1,%d1
.balignw 4,0x2048
2:
move.b %d1,(-1,%a5,%d1.w)
addq.w #1,%d1
dbf %d0,2b
moveq #5,%d0
moveq #1,%d1
.balignw 4,0x2048
5:
move.b %d1,(7,%a5,%d1.w)
addq.w #1,%d1
dbf %d0,5b
moveq #9,%d0
moveq #1,%d1
.balignw 4,0x2048
8:
move.b %d1,(0x13,%a5,%d1.w)
addq.w #1,%d1
dbf %d0,8b
move.b #0x61,(0x28,%a5)
move.b #0x62,(0x29,%a5)
move.b #0x63,(0x2a,%a5)
move.b #0x64,(0x2b,%a5)
move.b #0x65,(0x2c,%a5)
move.b #0x66,(0x2d,%a5)
move.b #0x41,(0x34,%a5)
move.b #0x42,(0x35,%a5)
move.b #0x43,(0x36,%a5)
move.b #0x44,(0x37,%a5)
lea (%a5),%a0
lea (4,%a5),%a1
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
lea (8,%a5),%a0
lea (0xe,%a5),%a1
moveq #5,%d0
.balignw 4,0x2048
10:
move.b (%a0)+,(%a1)+
dbf %d0,10b
lea (0x14,%a5),%a0
lea (0x1e,%a5),%a1
moveq #9,%d0
.balignw 4,0x2048
11:
move.b (%a0)+,(%a1)+
dbf %d0,11b
lea (0x28,%a5),%a0
lea (0x2e,%a5),%a1
moveq #5,%d0
.balignw 4,0x2048
12:
move.b (%a0)+,(%a1)+
dbf %d0,12b
lea (0x34,%a5),%a0
lea (0x38,%a5),%a1
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
pea (.Lc0,%pc)
pea (4,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0xe,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc2,%pc)
pea (0x1e,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x2e,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc0,%pc)
pea (0x38,%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0004
.Lc2:
.short 0x000a
.Lc1:
.short 0x0006
.balignw 4,0x2048
.section ".bss.moddata.BE_L_COPIES","aw",@nobits
.balign 4
.Ldata:
.space 60
