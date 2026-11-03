.set __dpc_fragment_origin,0xe68864
.text
RIP_$PACKET_LENGTH:
link.w %fp,#-8
move.w (8,%fp),%d0
add.w %d0,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
addq.w #2,%d0
unlk %fp
rts
