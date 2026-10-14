{ TTY_$K_FLUSH_INPUT 0xE1C084..0xE1C0E6 and TTY_$K_FLUSH_OUTPUT
  0xE1C0E6..0xE1C148 (0xC4 bytes; tools/imgdis.sh E1C084 C4), a fragment
  of the TTY module's WIRED_PROC code section, with TTY_$I_GET_DESC
  declared the way bead domain-pascal-yzxr's reading has it: a 4-byte
  integer result in A0 (`integer32; options(a0_return)', ref 5.5.11,
  p. 5-20), held by each caller in an integer local and converted to the
  descriptor's pointer type where dereferenced (ref 4 "Type Transfer
  Functions", p. 4-197).  Two of the 14 routines that hold the result in
  D0 where a call follows (codegen-patterns 3, `tools/resultsurvey.py
  parkd0'), compiled without `dpc --park-d0'.  Reconstructed from
  ../source's Domain Pascal translation (domain_os/tty/k_flush.pas, by
  Claude Opus 5.5), reduced to what the routines touch.  Semantics:
  ../source domain_os/tty/k_flush.c.  TTY_$I_GET_DESC 0xE66738 (OS_TERM),
  ML_$SPIN_LOCK, ML_$SPIN_UNLOCK and TTY_$SPIN_LOCK (0xE2DD74) are
  external; TTY_$I_FLUSH_INPUT 0xE1B7B0 and TTY_$I_FLUSH_OUTPUT 0xE1B806
  are routines of the module outside the unit (`module-routines=',
  `bsr.w').  Each routine loads A5 without using it (`a5-always=',
  codegen-patterns 4); the data area is placed at the WIRED_DATA block
  0xE2DDB4.  Compiled with `no-park-pointer-locals'.

  Patterns exercised (docs/codegen-patterns.md):
  - 3 "Pointer locals held in D0"  the integer local held in D0 from the
         call (`move.l A0,D0', 0xE1C0A8) and moved to the lowest free
         callee-saved address register, A2 (the status's, dead after the
         test), as the first instruction after the status test, before
         the spin lock's push (`movea.l D0,A2', 0xE1C0AE); D0 then free
         for the spin lock's token (`move.w D0,(-6,A6)', 0xE1C0BE);
  - 7.1, 9  `if status.all <> status_$ok then return';
  - 1.1, 1.3  the token pushed as a word after a pad; the last call not
         popped before `unlk'.

  Reconstructed (not known from the original): the names (the C tree's
  and ../source's); the integer declarations (the reading under test);
  the descriptor record as one stand-in field; `unused_c', ../source's
  stand-in for the frame bytes no instruction addresses (`link #-0xc');
  `standin', the unit's data area. }

module tty, wired_proc, wired_data;
type
  status_$t = record case integer of 1: (all: integer32); end;
  tty_$desc_t = record
    line_id: integer32;
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
const
  status_$ok = 0;
var
  standin: integer32;
  tty_$spin_lock: extern integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  integer32; options(extern, val_param, a0_return);
function ml_$spin_lock(var lock: integer32): integer; extern;
procedure ml_$spin_unlock(var lock: integer32; token: integer);
  options(extern, val_param);
procedure tty_$i_flush_input(in out tty: tty_$desc_t); extern;
procedure tty_$i_flush_output(in out tty: tty_$desc_t); extern;

procedure tty_$k_flush_input(in line: integer; out status: status_$t);
var
  tty: integer32;                         { (-0x4,A6): D0, then A2 }
  token: integer;                         { (-0x6,A6) }
  unused_c: integer32;
begin
  tty := tty_$i_get_desc(line, status);             { 0x00E1C096 }
  if status.all <> status_$ok then                  { 0x00E1C0AA }
    return;
  token := ml_$spin_lock(tty_$spin_lock);           { 0x00E1C0B0 }
  tty_$i_flush_input(tty_$desc_ptr_t(tty)^);        { 0x00E1C0C2 }
  ml_$spin_unlock(tty_$spin_lock, token);           { 0x00E1C0CA }
end;

procedure tty_$k_flush_output(in line: integer; out status: status_$t);
var
  tty: integer32;                         { (-0x4,A6): D0, then A2 }
  token: integer;                         { (-0x6,A6) }
  unused_c: integer32;
begin
  tty := tty_$i_get_desc(line, status);             { 0x00E1C0F8 }
  if status.all <> status_$ok then                  { 0x00E1C10C }
    return;
  token := ml_$spin_lock(tty_$spin_lock);           { 0x00E1C112 }
  tty_$i_flush_output(tty_$desc_ptr_t(tty)^);       { 0x00E1C124 }
  ml_$spin_unlock(tty_$spin_lock, token);           { 0x00E1C12C }
end;
