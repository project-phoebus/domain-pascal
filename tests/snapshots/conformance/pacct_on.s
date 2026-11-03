.set __dpc_fragment_origin,0xe5a9a4
.text
.globl PACCT_$ON
PACCT_$ON:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
lea (%a5),%a0
movea.l #UID_$NIL,%a1
moveq #1,%d0
cmpm.l (%a1)+,(%a0)+
bne.s 1f
cmpm.l (%a1)+,(%a0)+
1:
sne %d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.PACCT","aw",@nobits
.balign 4
.Ldata:
.space 8
