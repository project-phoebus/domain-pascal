.section ".text.BYTES.WIRED_PROC","ax",@progbits
.globl SET_BREAK
SET_BREAK:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
movea.l (8,%fp),%a0
move.b (0xc,%fp),%d0
bpl.s 1f
move.b (0x4a,%a5),(5,%a0)
bra.s 2f
1:
move.b (0x48,%a5),(5,%a0)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BUSY
BUSY:
link.w %fp,#-4
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
move.b (0x62,%a5),%d0
or.b (0x64,%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.BYTES.WIRED_DATA","aw",@progbits
.balign 4
.Ldata.WIRED_DATA:
.space 72
.byte 0x70,0x00,0x60,0x00,0x00,0x0e,0x00,0x0e,0x00,0x01,0x00,0x02,0x10,0x00,0x45,0x00
.byte 0x2a,0x00,0x3a,0x00,0x0a,0x00,0x07,0x00,0x0b
.space 1
.globl ACTIVITY_FLAG
ACTIVITY_FLAG:
.space 2
.globl LOOPBACK_FLAG
LOOPBACK_FLAG:
.space 2
.globl DO_CHKSUM
DO_CHKSUM:
.byte 0xff
.space 1
