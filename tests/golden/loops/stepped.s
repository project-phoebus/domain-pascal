.text
.globl SP_$COPY
SP_$COPY:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0xf,%d0
movea.l (4,%a5),%a0
movea.l (%a5),%a1
lea (2,%a0),%a2
lea (2,%a1),%a0
.balignw 4,0x2048
2:
move.w (-2,%a2),(-2,%a0)
addq.l #2,%a0
addq.l #2,%a2
dbf %d0,2b
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl SP_$NUMBER
SP_$NUMBER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #7,%d0
moveq #8,%d1
movea.l (8,%a5),%a0
lea (0x40,%a0),%a1
.balignw 4,0x2048
2:
move.w %d1,(-4,%a1)
clr.b (-2,%a1)
subq.w #1,%d1
subq.l #8,%a1
dbf %d0,2b
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SP_$CLEAR
SP_$CLEAR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #3,%d0
movea.l (0xc,%a5),%a0
lea (0xc,%a0),%a1
.balignw 4,0x2048
2:
clr.l (-8,%a1)
lea (0xc,%a1),%a1
dbf %d0,2b
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SP_$DUMP
SP_$DUMP:
link.w %fp,#-4
movem.l %d2-%d3/%a2,-(%sp)
movea.l (8,%fp),%a0
st (6,%a0)
moveq #3,%d2
moveq #1,%d3
lea (8,%a0),%a2
.balignw 4,0x2048
2:
move.w %d3,(-4,%a2)
move.l (-8,%a2),-(%sp)
jsr (EXT_$PUT).l
addq.w #4,%sp
addq.w #1,%d3
addq.l #8,%a2
dbf %d2,2b
movem.l (-0x10,%fp),%d2-%d3/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SP","aw",@nobits
.balign 4
.Ldata:
.space 16
