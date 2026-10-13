.text
FL_$FIND:
link.w %fp,#-4
moveq #7,%d0
clr.w %d1
.balignw 4,0x2048
2:
cmpi.b #0x78,(0,%a5,%d1.w)
bne.s 4f
move.w %d1,(W).l
bra.s 1f
4:
addq.w #1,%d1
dbf %d0,2b
1:
unlk %fp
rts
FL_$DOWN:
link.w %fp,#-4
moveq #0x61,%d0
moveq #0x64,%d1
.balignw 4,0x2048
2:
cmpi.w #0x32,%d1
beq.s 3f
move.w %d1,(W).l
3:
subq.w #1,%d1
dbf %d0,2b
unlk %fp
rts
FL_$CALLS:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (LEN).l,%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXT_$PUT).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
FL_$COUNT:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w #0x12d,%d2
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXT_$PUT).l
addq.w #4,%sp
dbf %d2,2b
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FL","aw",@nobits
.balign 4
.Ldata:
.space 8
