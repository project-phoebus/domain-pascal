{ TTY_$K_ENABLE_FUNC, 0xE675AE..0xE67618 (0x6A bytes; tools/imgdis.sh
  E675AE 6A), a fragment of the TTY code segment (0xE671DC).  A
  reconstruction after ../source's Pascal translation of the TTY module
  (domain_os/tty/k_func.pas at ../source 3898a2ce, written from the image
  by another agent; its C twin tty/k_func.c gives the semantics), cut
  down to the declarations the routine uses.  TTY_$I_GET_DESC 0xE66738
  (OS_TERM) is another unit's; the module-local routine at 0xE6726E
  (`tty_$i_set_dfl_funcs' in ../source), of the same code segment, lies
  outside the unit (`module-routines=SET_DFL_FUNCS=0xE6726E', `bsr.w').
  The routine loads A5 (.DATA's TTY block 0xE8242C) without using it
  (`a5-always=', codegen-patterns 4).

  Patterns exercised (docs/codegen-patterns.md):
  - 5.1  `s + [k]' and `s - [k]' with a run-time member on a longword
         set reached through a pointer: `clr.l D1; move.w (A3),D2; bset
         D2,D1; or.l D1,(0x20,A2)' (0xE675E6..0xE675EC) and `...; not.l
         D1; and.l D1,(0x20,A2)' (0xE675F2..0xE675FA);
  - 3    a pointer function's result (TTY_$I_GET_DESC, A0) held in D0,
         then moved to A2 for the rest of the body;
  - 7.1  a boolean `in' parameter tested by its sign through its slot.

  Pending (tests/conformance/README.md): `dpc' refuses the `enable'
  slot loaded after the first branch (codegen-patterns 12.4 R4(b)); with
  that refusal lifted by hand the set statements match the image's
  instructions but not its registers, which follow from the descriptor
  held in D0 until `movea.l D0,A2' (0xE675D6, 0xE675E0; codegen-patterns
  3, "Callers of pointer functions").

  Reconstructed (not known from the original): every name; the record
  `tty_$desc_t' cut to the two fields used, at their offsets (func_enabled
  0x20, raw_mode 0x4D9); `tty_$func_t' with 32 values (a longword set);
  `unused_8' (frame bytes nothing addresses, as in ../source); `standin',
  the unit's data area. }
module tty;
type
  status_$t = record
    case integer of
      1: (all: integer32);
  end;
  tty_$func_t = (f00, f01, f02, f03, f04, f05, f06, f07, f08, f09, f10, f11,
                 f12, f13, f14, f15, f16, f17, f18, f19, f20, f21, f22, f23,
                 f24, f25, f26, f27, f28, f29, f30, f31);
  tty_$func_set_t = set of tty_$func_t;
  tty_$desc_t = record
    pad_0: array[1..16] of integer;       { 0x00 }
    func_enabled: tty_$func_set_t;        { 0x20 }
    pad_24: array[1..1205] of char;       { 0x24 }
    raw_mode: boolean;                    { 0x4D9 }
  end;
  tty_$desc_ptr_t = ^tty_$desc_t;
var
  standin: integer32;

function tty_$i_get_desc(in line: integer; out status: status_$t): tty_$desc_ptr_t;
  options(val_param); extern;
procedure set_dfl_funcs(in out tty: tty_$desc_t; use_dfl: boolean); internal; extern;

procedure tty_$k_enable_func(in line: integer; in func: tty_$func_t;
                             in enable: boolean; out status: status_$t);
var
  tty: tty_$desc_ptr_t;
  unused_8: integer32;
begin
  tty := tty_$i_get_desc(line, status);
  if status.all <> 0 then
    return;
  if enable then
    tty^.func_enabled := tty^.func_enabled + [func]
  else
    tty^.func_enabled := tty^.func_enabled - [func];
  if not tty^.raw_mode then
    set_dfl_funcs(tty^, true);
end;
