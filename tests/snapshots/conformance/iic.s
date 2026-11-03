.text
.globl IIC_$INIT
IIC_$INIT:
rts
.globl IIC_$ACQUIRE
IIC_$ACQUIRE:
link.w %fp,#0
movea.l (0xc,%fp),%a0
move.l #0x2c000a,(%a0)
unlk %fp
rts
.globl IIC_$RELEASE
IIC_$RELEASE:
link.w %fp,#0
movea.l (0xc,%fp),%a0
move.l #0x2c000a,(%a0)
unlk %fp
rts
.globl IIC_$STATISTICS
IIC_$STATISTICS:
link.w %fp,#0
movea.l (0x10,%fp),%a0
move.l #0x2c000a,(%a0)
unlk %fp
rts
.globl IIC_$SELF_TEST
IIC_$SELF_TEST:
link.w %fp,#0
movea.l (0x10,%fp),%a0
move.l #0x2c000a,(%a0)
unlk %fp
rts
.globl IIC_$SEND
IIC_$SEND:
link.w %fp,#0
movea.l (0x1c,%fp),%a0
move.l #0x2c000a,(%a0)
unlk %fp
rts
.globl IIC_$EXISTS
IIC_$EXISTS:
link.w %fp,#-4
clr.b %d0
unlk %fp
rts
.globl IIC_$RECEIVE
IIC_$RECEIVE:
link.w %fp,#0
pea (%a2)
movea.l (0x1c,%fp),%a0
move.l #0x2c000a,(%a0)
movea.l (0x18,%fp),%a1
clr.w (%a1)
movea.l (0x10,%fp),%a2
clr.w (%a2)
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl IIC_$RECEIVE_CHECK
IIC_$RECEIVE_CHECK:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
move.l #0x2c000a,(%a0)
clr.b %d0
unlk %fp
rts
.balignw 4,0x2048
