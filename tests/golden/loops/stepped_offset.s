.text
.globl SO_$CLEANUP
SO_$CLEANUP:
link.w %fp,#-4
movem.l %d2-%d3/%a2,-(%sp)
move.w (0xc,%fp),%d2
moveq #9,%d3
movea.l (8,%fp),%a0
lea (8,%a0),%a2
.balignw 4,0x2048
2:
tst.b (0x5a,%a2)
bpl.s 4f
cmp.w (0x5c,%a2),%d2
bne.s 4f
clr.b (0x5a,%a2)
subq.l #2,%sp
move.w (0x5e,%a2),-(%sp)
jsr (EXT_$CLOSE).l
addq.w #4,%sp
4:
addq.l #8,%a2
dbf %d3,2b
movem.l (-0x10,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl SO_$COUNT
SO_$COUNT:
link.w %fp,#-8
clr.w %d0
moveq #3,%d1
movea.l (8,%fp),%a0
lea (4,%a0),%a1
.balignw 4,0x2048
2:
tst.l (0x3c,%a1)
bne.s 4f
addq.w #1,%d0
4:
addq.l #4,%a1
dbf %d1,2b
unlk %fp
rts
.globl SO_$RECEIVE
SO_$RECEIVE:
link.w %fp,#-0xc
pea (%a2)
movea.l (8,%fp),%a2
pea (-8,%fp)
jsr (EXT_$GET).l
addq.w #4,%sp
move.w (-8,%fp),(%a2)
moveq #2,%d0
lea (%fp),%a0
lea (%a2),%a1
.balignw 4,0x2048
2:
move.w (-6,%a0),(2,%a1)
addq.l #2,%a1
addq.l #2,%a0
dbf %d0,2b
movea.l (-0x10,%fp),%a2
unlk %fp
rts
.globl SO_$RESET
SO_$RESET:
link.w %fp,#-8
moveq #9,%d0
movea.l (8,%fp),%a0
lea (8,%a0),%a1
.balignw 4,0x2048
2:
movea.l %a1,%a0
clr.b (0x5a,%a0)
clr.w (0x60,%a0)
addq.l #8,%a1
dbf %d0,2b
unlk %fp
rts
.balignw 4,0x2048
