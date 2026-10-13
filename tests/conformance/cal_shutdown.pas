{ CAL_$SHUTDOWN, 0xE3E6E4..0xE3E744 (0x60 bytes with the module-end
  filler; tools/imgdis.sh E3E6E4 60), the last routine of the OS_CAL_WIRED
  code segment (0xE3E520, size 0x224), so a module-tail fragment (`dpc
  --fragment-origin 0xE3E6E4 --fragment-ends-module').  Semantics:
  ../source/domain_os/cal/shutdown.c (stamp the boot volume's label with
  the time and write it back).  Externals (SAU2 map): DBUF_$GET_BLOCK
  0xE3A5B0, DBUF_$SET_BUFF 0xE3A8B6, LV_LABEL_$UID 0xE17394, TIME_$CLOCKH
  0xE2B0D4.  Unit data: the OS_CAL_WIRED data segment 0xE7B030 (A5),
  CAL_$BOOT_VOLX at (0x10,A5) (map 0xE7B040); CAL_$TIMEZONE, at offset 0,
  is declared as a stand-in of its 16 bytes.

  Patterns exercised (docs/codegen-patterns.md):
  - 3, 12.5  a pointer local assigned from a pointer function and only
         dereferenced (bead domain-pascal-500): `movea.l A0,A3' after
         DBUF_$GET_BLOCK's pop (0xE3E714), the label's fields through A3
         (0xE3E71A, 0xE3E722), its object passed by address last, `pea
         (A3)' (0xE3E730); its frame slot allocated (`link #-4') and
         never addressed;
  - 12.4 R3, R4(c)  `status', a by-address parameter loaded at entry
         into A2 (used three times), and the label in A3, both held
         through the calls; the last use of each is DBUF_$SET_BUFF's
         argument list in a body with a branch: held, callee-saved;
  - 7.1  `if status = 0' on the status cell, `tst.l (A2); bne.s' to the
         epilogue;
  - 1.5  the address of an extern variable passed with `move.l #sym'
         (0xE3E6FE), two word constants merged and two long zeros, `clr.l
         -(SP)'; a pad before each argument list; DBUF_$SET_BUFF not
         popped;
  - 4    A5 loaded for the unit's data, saved in the `movem.l'.

  Reconstructed (not known from the original): the names of the types
  and of the local; the label record (only the offsets 0xB0 and 0xE6
  matter, ../source/domain_os/bat/bat.h and cal/cal_internal.h); the
  parameter lists of DBUF_$GET_BLOCK (as in disk_get_block.pas) and of
  DBUF_$SET_BUFF (`val_param': the flags word by value, the buffer and
  the status by address) read off the pushes; `var' for the status. }
module os_cal_wired;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;
  label_t = record
    pad1: array[0..43] of integer32;
    mount_time_high: integer32;    { 0xB0 }
    pad2: array[0..24] of integer16;
    last_valid_time: integer32;    { 0xE6 }
  end;
  label_ptr = ^label_t;
var
  cal_$timezone: array[0..3] of integer32;
  cal_$boot_volx: integer;
  lv_label_$uid: extern uid_$t;
  time_$clockh: extern integer32;

function dbuf_$get_block(vol: integer; daddr: integer32; var uid: uid_$t;
                         hint: integer32; block_type: integer; flags: integer;
                         var status: status_$t): label_ptr; val_param; extern;
procedure dbuf_$set_buff(var buf: label_t; flags: integer; var status: status_$t);
  val_param; extern;

procedure cal_$shutdown(var status: status_$t);
var lbl: label_ptr;
begin
  lbl := dbuf_$get_block(cal_$boot_volx, 0, lv_label_$uid, 0, 0, 0, status);
  if status = 0 then begin
    lbl^.mount_time_high := time_$clockh;
    lbl^.last_valid_time := lbl^.mount_time_high;
    dbuf_$set_buff(lbl^, 11, status);
  end;
end;
