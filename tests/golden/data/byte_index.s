.text
FROM_MEMORY:
link.w %fp,#-4
clr.w %d0
move.b (0x368,%a5),%d0
add.w %d0,%d0
move.w #0x11,(0,%a5,%d0.w)
clr.w %d0
move.b (0x368,%a5),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w (0,%a5,%d1.w),(0x36a,%a5)
lea (0,%a5,%d0.w),%a0
move.b #0x78,(0x200,%a0)
clr.w %d0
move.b (0x368,%a5),%d0
lsl.w #2,%d0
lea (0,%a5,%d0.w),%a0
clr.l (0x17c,%a0)
unlk %fp
rts
.globl IN_RECORD
IN_RECORD:
link.w %fp,#0
movea.l (8,%fp),%a0
move.b (0xc,%fp),%d0
clr.w %d1
move.b %d0,%d1
add.w %d1,%d1
lea (0,%a0,%d1.w),%a1
move.w #0x12,(0xa4,%a1)
clr.w %d0
move.b (0x2a4,%a0),%d0
add.w %d0,%d0
lea (0,%a0,%d0.w),%a0
move.w #0x11,(0xa4,%a0)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BIDX","aw",@nobits
.balign 4
.Ldata:
.space 876
