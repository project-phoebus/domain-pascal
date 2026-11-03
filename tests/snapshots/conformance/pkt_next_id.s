.set __dpc_fragment_origin,0xe1248e
.text
.space 2
.globl PKT_$NEXT_ID
PKT_$NEXT_ID:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
pea (0x50,%a5)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
move.w (0x5c,%a5),%d2
addq.w #1,(0x5c,%a5)
cmpi.w #-0x600,(0x5c,%a5)
bls.s 1f
move.w #1,(0x5c,%a5)
1:
subq.l #2,%sp
move.w %d0,-(%sp)
pea (0x50,%a5)
jsr (ML_$SPIN_UNLOCK).l
move.w %d2,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.section ".bss.moddata.PKT","aw",@nobits
.balign 4
.Ldata:
.space 96
