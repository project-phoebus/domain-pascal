.set __dpc_fragment_origin,0xe4a4ae
.text
.space 2
.globl REM_NAME_$REGISTER_SERVER
REM_NAME_$REGISTER_SERVER:
link.w %fp,#-8
lea (.Ldata).l,%a0
move.l (TIME_$CLOCKH).l,(0x28,%a0)
st (0x3c,%a0)
unlk %fp
rts
.section ".bss.moddata.REM_NAME","aw",@nobits
.balign 4
.Ldata:
.space 64
