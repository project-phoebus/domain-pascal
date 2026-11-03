.set __dpc_fragment_origin,0xe6c08c
.text
.globl DISK_$GET_ERROR_INFO
DISK_$GET_ERROR_INFO:
link.w %fp,#0
movea.l #DISK_$ERROR_INFO,%a0
movea.l (8,%fp),%a1
moveq #0x14,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
move.w (%a0)+,(%a1)+
unlk %fp
rts
