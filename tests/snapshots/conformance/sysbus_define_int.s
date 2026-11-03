.set __dpc_fragment_origin,0xe0abbc
.text
.globl SYSBUS_$DEFINE_INT
SYSBUS_$DEFINE_INT:
link.w %fp,#0
pea (.Lc0,%pc)
jsr (CRASH_SYSTEM).l
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x003e,0x0002
.balignw 4,0x2048
