.set __dpc_fragment_origin,0xe46498
.text
ACL_$IS_PROCESS_TYPE_2:
link.w %fp,#-8
move.w (8,%fp),%d0
movea.l #PROC1_$TYPE,%a0
move.w %d0,%d1
add.w %d1,%d1
cmpi.w #2,(-2,%a0,%d1.w)
sne %d1
move.b %d1,%d0
unlk %fp
rts
