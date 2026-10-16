.text
FLR_$FIND:
link.w %fp,#-8
move.w (8,%a5),%d0
tst.w %d0
bmi.s 1f
move.w %d0,%d0
clr.w %d1
.balignw 4,0x2048
2:
cmpi.b #0x78,(0,%a5,%d1.w)
bne.s 4f
move.w %d1,(W).l
4:
addq.w #1,%d1
dbf %d0,2b
1:
unlk %fp
rts
FLR_$ALL:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
move.w (8,%a5),%d0
tst.w %d0
bmi.s 1f
move.w %d0,%d2
clr.w %d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXT_$PUT).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
FLR_$INIT:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
move.w (LEN).l,%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
clr.w %d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXT_$PUT).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
FLR_$TAIL:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
move.w (LEN).l,%d0
subq.w #2,%d0
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
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FLR","aw",@nobits
.balign 4
.Ldata:
.space 12
