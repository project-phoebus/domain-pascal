.text
TWO_VARS:
link.w %fp,#-4
move.w (%a5),%d0
cmp.w (2,%a5),%d0
beq.s 1f
move.w #1,(0xc,%a5)
1:
move.w (%a5),%d0
cmp.w (2,%a5),%d0
bge.s 2f
move.w #2,(0xc,%a5)
2:
move.l (4,%a5),%d0
cmp.l (4,%a5),%d0
blt.s 3f
move.w #3,(0xc,%a5)
3:
move.b (0xa,%a5),%d0
cmp.b (0xb,%a5),%d0
bls.s 4f
move.w #4,(0xc,%a5)
4:
unlk %fp
rts
MIXED:
link.w %fp,#-4
move.w (%a5),%d0
clr.l %d1
ext.l %d0
move.w (8,%a5),%d1
cmp.l %d1,%d0
bge.s 1f
move.w #1,(0xc,%a5)
1:
unlk %fp
rts
MIXED2:
link.w %fp,#-4
clr.l %d0
move.w (%a5),%d1
move.w (8,%a5),%d0
ext.l %d1
cmp.l %d1,%d0
ble.s 1f
move.w #2,(0xc,%a5)
1:
unlk %fp
rts
WORD_LONG:
link.w %fp,#-4
move.w (%a5),%d0
ext.l %d0
cmp.l (4,%a5),%d0
bgt.s 1f
move.w #3,(0xc,%a5)
1:
unlk %fp
rts
RECORDS:
link.w %fp,#-4
lea (0x10,%a5),%a0
lea (0x18,%a5),%a1
moveq #1,%d0
cmpm.l (%a1)+,(%a0)+
bne.s 1f
cmpm.l (%a1)+,(%a0)+
bne.s 1f
move.w #1,(0xc,%a5)
1:
lea (0x10,%a5),%a0
movea.l #UID_$NIL,%a1
moveq #1,%d0
cmpm.l (%a1)+,(%a0)+
bne.s 4f
cmpm.l (%a1)+,(%a0)+
beq.s 3f
4:
move.w #2,(0xc,%a5)
3:
lea (0x20,%a5),%a0
lea (0x28,%a5),%a1
moveq #2,%d0
.balignw 4,0x2048
7:
cmpm.w (%a1)+,(%a0)+
dbne %d0,7b
bne.s 5f
move.w #3,(0xc,%a5)
5:
lea (0x30,%a5),%a0
lea (0x50,%a5),%a1
moveq #7,%d0
.balignw 4,0x2048
10:
cmpm.l (%a1)+,(%a0)+
dbne %d0,10b
beq.s 8f
move.w #4,(0xc,%a5)
8:
unlk %fp
rts
UID_SET:
link.w %fp,#-8
lea (0x10,%a5),%a0
movea.l #UID_$NIL,%a1
moveq #1,%d0
cmpm.l (%a1)+,(%a0)+
bne.s 1f
cmpm.l (%a1)+,(%a0)+
1:
sne %d0
unlk %fp
rts
SAME_CLOCK:
link.w %fp,#-8
lea (0x20,%a5),%a0
lea (0x28,%a5),%a1
moveq #2,%d0
.balignw 4,0x2048
2:
cmpm.w (%a1)+,(%a0)+
dbne %d0,2b
seq %d0
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CMPM","aw",@nobits
.balign 4
.Ldata:
.space 112
