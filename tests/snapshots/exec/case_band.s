.text
.globl SEL
SEL:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
subi.w #0x30,%d1
cmpi.w #0x23,%d1
bcc.s 5f
add.w %d1,%d1
move.w (6f,%pc,%d1.w),%d1
jmp (6f,%pc,%d1.w)
6:
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 2f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 3f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 5f-6b
.short 4f-6b
2:
moveq #1,%d0
bra.s 1f
3:
moveq #2,%d0
bra.s 1f
4:
moveq #3,%d0
bra.s 1f
5:
moveq #4,%d0
1:
unlk %fp
rts
.globl CASE_BAND_$RUN
CASE_BAND_$RUN:
link.w %fp,#-4
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w SEL
addq.w #4,%sp
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
bsr.w SEL
addq.w #4,%sp
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bsr.w SEL
addq.w #4,%sp
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w SEL
addq.w #4,%sp
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.balignw 4,0x2048
