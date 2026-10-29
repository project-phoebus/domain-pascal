.arch 68020
.text
.globl UID_$INIT
UID_$INIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (NODE_$ME).l,%d0
bfins %d0,(4,%a5){#12:#20}
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl MAC_$OPEN
MAC_$OPEN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b (0x7bc,%a5),%d0
lsr.b #7,%d0
bfins %d0,(0x7ba,%a5){#7:#1}
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DIR_$ENTRY_TYPE
DIR_$ENTRY_TYPE:
link.w %fp,#-4
movea.l (8,%fp),%a0
bfextu (%a0){#5:#3},%d0
unlk %fp
rts
.globl MAC_$SET_F7
MAC_$SET_F7:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
andi.b #0x7f,(0x7ba,%a5)
move.b (8,%fp),%d0
andi.b #0x80,%d0
or.b %d0,(0x7ba,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl DIR_$SET_KIND
DIR_$SET_KIND:
link.w %fp,#0
movea.l (8,%fp),%a0
moveq #3,%d0
bfins %d0,(%a0){#0:#5}
unlk %fp
rts
.globl SET_VALS
SET_VALS:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
moveq #2,%d0
bfins %d0,(0x28,%a0){#1:#8}
moveq #2,%d0
bfins %d0,(0x29,%a0){#1:#2}
andi.b #0xf8,(%a1)
ori.b #0xf8,(%a1)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BIT_FIELDS","aw",@nobits
.balign 4
.Ldata:
.space 1984
