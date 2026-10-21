.text
.globl MBUS_$UNDEF_INT
MBUS_$UNDEF_INT:
link.w %fp,#-4
pea (.Lc0,%pc)
jsr (CRASH_SYSTEM).l
clr.b %d0
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x002a
.globl USMALL_$MORE
USMALL_$MORE:
link.w %fp,#0
pea (.Lc0,%pc)
jsr (CRASH_SYSTEM).l
addq.w #4,%sp
pea (.Lc1,%pc)
jsr (CRASH_SYSTEM).l
addq.w #4,%sp
pea (.Lc2,%pc)
jsr (CRASH_SYSTEM).l
addq.w #4,%sp
pea (.Lc0,%pc)
jsr (EXT_$PUT).l
unlk %fp
rts
.balignw 4,0x2048
.Lc1:
.short 0xffff,0xffff
.Lc2:
.short 0xffff,0x8000
.balignw 4,0x2048
