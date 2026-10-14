{ FILE_$TRUNCATE_D, 0xE73FE4, 0x60 bytes: 0xE73FE4..0xE74044
  (tools/imgdis.sh E73FE4 60).  Semantics:
  ../source/domain_os/file/truncate_d.c.  Externals (SAU2 map):
  FILE_$CHECK_PROT 0xE5D172, AST_$TRUNCATE 0xE05C40, OS_PROC_SHUTWIRED
  0xE5D050.

  The unit is a fragment of the FILE_ module, so tools/conform.py
  compiles it with `dpc --fragment-origin 0xE73FE4'.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(b) after a call (bead domain-pascal-178): `new_size', a slot
         used once in the `then' part, is loaded where it is needed,
         after the call and the branch, into A0 (0xE74022): the lowest
         free register, which no earlier statement left a value in on any
         path (the call wrote A0 after `context''s load at 0xE73FFE), and
         A1 is not written before it;
  - 12.4 R4(b) before the first call: `context' (0xE73FFE) in A0;
  - 12.2 `uid' and `status', used several times and across calls,
         loaded at entry into A2 and A3 (0xE73FEC, 0xE73FF0);
  - 1.2/1.3 val_param arguments with a pad: `move.w #2', the longword
         `context^' through its slot, `clr.l -(SP)' for `false' and the
         word 0 after it (0xE73FFC, 1.4 "Adjacent word constants merged":
         `false' is cleared as a word), `clr.w' for one word 0;
  - 1.1  the pop of the `then' part's call omitted before the `bra' to
         the epilogue, and of the `else' part's before it.

  Reconstructed (not known from the original): the names; the modes and
  types of the callees' parameters, from the pushes; status_$t as an
  integer32.  FILE_$CHECK_PROT's fourth parameter is a boolean, as in
  ../source: its slot is 2 bytes (the pad: 22 argument bytes, `lea
  (0x18,SP)' at 0xE74010), FILE_$CHECK_PROT passes it on with `pea
  (0x12,A6)' (0xE5D226) and ACL_$RIGHTS reads it as a byte, `move.b
  (A3),-(SP)' (0xE46A54).  The `clr.l' is therefore `false' merged with
  the word 0 after it, not a longword. }
module file_truncate_d;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;

procedure file_$check_prot(in uid: uid_$t; in mask: integer; in slot: integer32;
                           in ignore_super: boolean; in option_flags: integer;
                           out rights: integer; out status: status_$t);
  options(val_param, extern);
procedure ast_$truncate(in uid: uid_$t; in new_size: integer32; in flags: integer;
                        out result: boolean; out status: status_$t);
  options(val_param, extern);
procedure os_proc_shutwired(out status: status_$t); extern;

procedure file_$truncate_d(in uid: uid_$t; in new_size: integer32; in context: integer32;
                           out status: status_$t);
var
  rights: integer;
  result: boolean;
begin
  file_$check_prot(uid, 2, context, false, 0, rights, status);
  if status = 0 then
    ast_$truncate(uid, new_size, 0, result, status)
  else
    os_proc_shutwired(status);
end;
