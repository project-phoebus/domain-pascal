{ TTY_$K_INQ_FUNC_ENABLED .. TTY_$K_INQ_ECHO_FLAGS, 0xE67618..0xE6781E
  (seven routines, 0x206 bytes; tools/imgdis.sh E67618 206), a fragment of
  the TTY module's default code section (.TEXT).  Reconstructed from
  ../source's Domain Pascal translation (domain_os/tty/k_func.pas and
  k_set_input_flag.pas at ../source 3898a2ce, by Claude Opus 5.5),
  reduced to what the routines touch.  Semantics: ../source
  domain_os/tty/k_func.c, k_flags.c.  TTY_$I_GET_DESC 0xE66738 (OS_TERM)
  is external.  Each routine loads A5 without using it (`a5-always=',
  codegen-patterns 4); the data area is placed at the .DATA block
  0xE8242C.

  Patterns exercised (docs/codegen-patterns.md):
  - 3, 12.4  a pointer local assigned from a pointer function, only
         dereferenced, live across no call (bead domain-pascal-dgb): held
         in D0 from the call (`move.l A0,D0', 0xE6763C) and moved to A0
         by the first statement that dereferences it (`movea.l D0,A0',
         0xE67642), D0 left alone up to the last dereference (the set
         built in D1 and D2, D2 saved, 0xE6768E..0xE676A2); in an `if'
         whose two parts dereference it, the move just before the
         condition's branch, after the condition's slot load into A1
         (0xE67684..0xE6768A);
  - 7.1, 9  `if status.all <> status_$ok then return' on the status
         cell held in A2/A3 (`tst.l (A2); bne.b' to the epilogue);
  - 5.1  `+ [flag]' / `- [flag]' on a longword set in place (`bset.l
         D2,D1; or.l D1,(d,A0)', `not.l D1; and.l D1,(d,A0)');
  - 1.2, 12.2  `in'/`out' parameters by address, slots read once into
         A1 or loaded at entry into A2/A3.

  Reconstructed (not known from the original): the names (the C tree's
  and ../source's); the descriptor record up to func_enabled (offsets
  from ../source domain_os/tty/tty.h), its flag sets as `set of 0..31'
  and the flags as `0..31' in place of ../source's enumerated flag
  types (Apollo's, tty.ins.pas); `unused_8', ../source's stand-in
  for the 4 frame bytes no instruction addresses (`link #-8');
  `standin', the unit's data area. }
module tty;
type
  status_$t = record case integer of 1: (all: integer32); end;
  flag_t = 0..31;
  flag_set_t = set of flag_t;
  tty_$desc_t = record
    line_id: integer32;                   { 0x00 }
    handler_ptr: integer32;               { 0x04 }
    state_flags: integer;                 { 0x08 }
    pending_signal: integer;              { 0x0A }
    output_flags: flag_set_t;             { 0x0C }
    raw_saved_output_flags: flag_set_t;   { 0x10 }
    input_flags: flag_set_t;              { 0x14 }
    raw_saved_input_flags: flag_set_t;    { 0x18 }
    echo_flags: flag_set_t;               { 0x1C }
    func_enabled: flag_set_t;             { 0x20 }
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
const
  status_$ok = 0;
var
  standin: integer32;

function tty_$i_get_desc(line: integer; out status: status_$t):
  tty_$desc_ptr_t; options(extern, val_param);

procedure tty_$k_inq_func_enabled(in line: integer; out enabled: flag_set_t;
                                  out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  enabled := tty^.func_enabled;
end;

procedure tty_$k_set_input_flag(in line: integer; in flag: flag_t;
                                in value: boolean; out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  if value then
    tty^.input_flags := tty^.input_flags + [flag]
  else
    tty^.input_flags := tty^.input_flags - [flag];
end;

procedure tty_$k_inq_input_flags(in line: integer; out flags: flag_set_t;
                                 out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  flags := tty^.input_flags;
end;

procedure tty_$k_set_output_flag(in line: integer; in flag: flag_t;
                                in value: boolean; out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  if value then
    tty^.output_flags := tty^.output_flags + [flag]
  else
    tty^.output_flags := tty^.output_flags - [flag];
end;

procedure tty_$k_inq_output_flags(in line: integer; out flags: flag_set_t;
                                 out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  flags := tty^.output_flags;
end;

procedure tty_$k_set_echo_flag(in line: integer; in flag: flag_t;
                                in value: boolean; out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  if value then
    tty^.echo_flags := tty^.echo_flags + [flag]
  else
    tty^.echo_flags := tty^.echo_flags - [flag];
end;

procedure tty_$k_inq_echo_flags(in line: integer; out flags: flag_set_t;
                                 out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> status_$ok then
    return;
  flags := tty^.echo_flags;
end;

