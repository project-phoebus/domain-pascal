{ Module-local routine at 0xE00EB0, 0x3C bytes, and AST_$PAGE_ZERO,
  0xE00EEC, 0x2A bytes: 0xE00EB0..0xE00F16 (tools/imgdis.sh E00EB0 66).
  Semantics: ../source/domain_os/misc/zero_page.c (0xE00EB0, called
  ZERO_PAGE there) and ast/page_zero.c.  Externals (SAU2 map): ML_$LOCK
  0xE20B12, ML_$UNLOCK 0xE20B62, MMU_$INSTALL 0xE24048, MMU_$REMOVE
  0xE23D64, AST_$ZERO_BUFF 0xFF8C00 (a 1 KB window in the I/O region).

  The unit is a fragment of the AST_ module (code segment 0xE00BE8), so
  tools/conform.py compiles it with `dpc --fragment-origin 0xE00EB0'.
  AST_ up to AST_$GET_DISM_SEQN aligns its loop heads (codegen-patterns
  7), which is what dpc does.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.1  arguments pushed right to left, the caller pops: `lea (12,SP),SP'
         for 12 bytes, `addq.w #4,SP' for 4; no pop when the call is the
         last statement (unlk follows);
  - 1.2  val_param / internal: every argument here is passed by value;
  - 1.4  `subq.l #2,SP' pad before the one-word argument of ML_$LOCK and
         ML_$UNLOCK;
  - 1.5  MMU_$INSTALL's two word constants (asid 0, protection 0x16) as
         one longword `pea (0x16).w', the first parameter in the high
         half; `addr' of another unit's variable as `move.l #sym,-(SP)';
         a word constant as `move.w #n,-(SP)';
  - 1.6  `jsr (sym).l' to other units' routines, `bsr.s' to the
         module-local routine, which comes first in the source (6);
  - 2    `link.w A6,#-12' (see below) and `link.w A6,#0', no saves;
  - 7    the `for' loop as `movea.l #sym,A0; move.w #0xFF,D0; clr.l
         (A0)+; dbf', the head aligned with the 0x2048 filler;
  - 12.2 the by-value parameter is only pushed, from the frame, each time
         (0xE00EBE, 0xE00EDE), never loaded at entry.

  Reconstructed (not known from the original): the names of the
  module-local routine, the parameters and the locals; the parameter
  types (ppn integer32, the C has uint32_t; asid and prot integer, the
  hand-written MMU_$INSTALL reads their low bytes at (0x31,SP) and
  (0x33,SP), ../source/domain_os/mmu/sau2/install.s; va a univ_ptr, so
  the caller writes `addr(...)'); AST_$ZERO_BUFF as an array of 256
  integer32 (the code clears 256 longwords from it); the loop as `for i
  := 0 to 255 do ast_$zero_buff[i] := 0'.  `internal' (ref 5.5.4) for the
  module-local routine: it has no map name and takes its argument by
  value; val_param for the rest: every caller pushes values
  (NETWORK_$PAGE_SERVER 0xE115F0 `move.l D2,-(SP); jsr AST_$PAGE_ZERO').

  The frame: 0xE00EB0 has `link #-12' and never touches it.  Unused
  frames are ordinary in the image (306 compiled routines without a loop
  never touch theirs, mostly 4 or 8 bytes), so locals kept in registers
  or optimised away still get their slots; routines whose only visible
  work is one or two `dbf' loops have 12 bytes more often than any other
  size (35 of 132 with an untouched frame: NET_IO_$FREE_ASID 0xE74E84,
  FLP_$SHUTDOWN 0xE3E228, ...), but PKT_$DUMP_DATA 0xE127E6 has 4 for a
  loop whose index it keeps in D3.  What the original declared is not
  known; `i' plus `unknown' stand in for the 12 bytes. }
module ast_;
const
  pmap_lock_id = 16#14;     { the ML_ lock of the PMAP; PMAP_LOCK_ID in ../source }
type
  ast_$page_t = array[0..255] of integer32;
var
  ast_$zero_buff: extern ast_$page_t;

procedure ml_$lock(resource_id: integer); options(val_param, extern);
procedure ml_$unlock(resource_id: integer); options(val_param, extern);
procedure mmu_$install(ppn: integer32; va: univ_ptr; asid: integer;
                       prot: integer); options(val_param, extern);
procedure mmu_$remove(ppn: integer32); options(val_param, extern);

{ 0xE00EB0: map the frame at the zero window, clear it, unmap it. }
procedure zero_page(ppn: integer32); internal;
var
  i: integer;
  { Stand-in, not from the original: fills the frame to 0xE00EB0's
    `link #-12' (codegen-patterns 2, "Untouched frames"). }
  unknown: array[1..5] of integer;
begin
  mmu_$install(ppn, addr(ast_$zero_buff), 0, 16#16);
  for i := 0 to 255 do ast_$zero_buff[i] := 0;
  mmu_$remove(ppn);
end;

procedure ast_$page_zero(ppn: integer32); options(val_param);
begin
  ml_$lock(pmap_lock_id);
  zero_page(ppn);
  ml_$unlock(pmap_lock_id);
end;
