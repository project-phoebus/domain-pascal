.section ".text.ROUTE_WIRED.WIRED_PROC","ax",@progbits
.globl ROUTE_$NET
ROUTE_$NET:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x5c,%d1
movea.l #NET_PORT_TABLE,%a0
mulu.w %d0,%d1
move.l (0,%a0,%d1.w),%d0
add.l (INIT_SEC+4).l,%d0
unlk %fp
rts
.balignw 4,0x2048
.section ".data.overlay.INIT_SEC","aw",@progbits
.balign 4
.globl INIT_SEC
.type INIT_SEC,@object
.size INIT_SEC,12
INIT_SEC:
.byte 0x00,0x01
.space 2
.globl OVD_$K
OVD_$K:
.byte 0xff,0xff,0xff,0xff
.globl OVD_$P
OVD_$P:
.long NET_PORT_TABLE+92
.section ".bss.overlay.NET_PORT_TABLE","aw",@nobits
.balign 4
.globl NET_PORT_TABLE
.type NET_PORT_TABLE,@object
.size NET_PORT_TABLE,736
NET_PORT_TABLE:
.globl ROUTE_$PORT
ROUTE_$PORT:
.space 736
