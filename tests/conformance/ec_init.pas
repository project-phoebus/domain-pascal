{ EC_$INIT, 0xE151FE, 0x16 bytes (tools/imgdis.sh E151FE 16): map `E151FE
  EC_$INIT', inside the PROC1_ code segment 0xE1471C (WIRED_PROC), so the
  unit is a fragment of PROC1_ compiled with `dpc --fragment-origin
  0xE151FE'.  Semantics: ../source/domain_os/ec/init.c (the value 0, the
  waiter list empty: both list pointers at the eventcount itself);
  layout: ec/ec.h ec_$eventcount_t (value at 0, the list's head at 4,
  tail at 8; ../source/domain_os/ec/ec.ins.pas).

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  `addr' of a parameter passed by address, stored: the slot's
         value; here the slot is held in A0 for the three references and
         stored from it, `move.l A0,(0x4,A0)' (0xE15208, 0xE1520C; bead
         domain-pascal-mbtx);
  - 12.2 a by-address slot used more than once is loaded at entry
         (`movea.l (8,A6),A0', 0xE15202), here for its three
         dereferences (whether the stores of its address count is split,
         bead domain-pascal-m2w3; they do not decide it here);
  - 2    `link.w A6,#0', no locals, no A5 (no data reached).

  Reconstructed (not known from the original): the names; `out' as the
  mode (`var' and `in' pass the address alike under the default
  convention, ref 5.2 Table 5-1, p. 5-5); the list pointers as univ_ptr
  assigned `addr(ec)' (ec.ins.pas declares no type for the waiters), as
  ../source/domain_os/ec/init.pas writes them. }
module proc1_, wired_proc, wired_data;
type
  ec_$eventcount_t = record
    value: integer32;
    waiter_list_head: univ_ptr;
    waiter_list_tail: univ_ptr;
  end;

procedure ec_$init(out ec: ec_$eventcount_t);
begin
  ec.value := 0;
  ec.waiter_list_head := addr(ec);
  ec.waiter_list_tail := addr(ec);
end;
