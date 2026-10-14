{ RING_$SVC_OPEN, 0xE76DF2, 0x30 bytes: 0xE76DF2..0xE76E22
  (tools/imgdis.sh E76DF2 30).  Semantics:
  ../source/domain_os/ring/svc_open.c; the module-local routine at
  0xE76B7C (map: no name; ../source's ring_$open_internal,
  ring/ring_internal.h) lies outside the unit (`module-routines=
  RING_$OPEN_INTERNAL=0xE76B7C', README.md "Routines outside the unit").

  The unit is a fragment of the RING module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE76DF2'.  Its data area,
  `.bss.moddata.RING', is placed at 0xE86400, the address the routine
  loads into A5 without using the data (`a5-always=RING_$SVC_OPEN').

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(b) after a call (bead domain-pascal-178): `status_ret', a
         slot used once, is loaded after the call into A0 (0xE76E12), the
         lowest free register, which no earlier statement left a value in
         (nothing before the call writes an address register);
  - 1.1  no pop after the `bsr.w' (no push or branch follows);
  - 1.2/1.3 by-address parameters passed on with `move.l (d,A6),-(SP)',
         the `internal' callee's boolean by value, `clr.w -(SP)' for
         `false', a pad first; the val_param word and longword slots
         (0x10, 0x12) never read, the status slot at 0x16;
  - 4    always mode: `pea (A5); lea 0xE86400,A5' ... `movea.l
         (-8,A6),A5'.

  Reconstructed (not known from the original): the names; the type of
  `args' (a record passed on by address, its size not visible here);
  `ring_$data_stand_in', a variable only so that the unit has a data
  area (mmap_alloc_contig.pas); `in' for the unread value parameters
  (their types only from their slot sizes). }
module ring;
type
  status_$t = integer32;
  open_args_t = record
    w: array[1..8] of integer;
  end;
var
  ring_$data_stand_in: integer32;   { stand-in, see above }

procedure ring_$open_internal(in is_os: boolean; var unit: integer; var args: open_args_t;
                              out status: status_$t); internal; extern;

procedure ring_$svc_open(var unit: integer; var args: open_args_t; in unused3: integer;
                         in unused4: integer32; out status_ret: status_$t);
  options(val_param);
var
  status: status_$t;
begin
  ring_$open_internal(false, unit, args, status);
  status_ret := status;
end;
