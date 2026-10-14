{ codegen-patterns 3 ("Pointer locals held in D0", "The integer
  reading"); bead domain-pascal-1j61.  An integer local assigned once
  from an `a0_return' function (ref 5.5.11, p. 5-20) and only
  dereferenced through a type transfer is held in D0 (`move.l A0,D0') and
  moved to an address register inside an `if':

  - get: the `if''s `then' part alone refers to it, nothing after the
    `if' does: the move is the `then' part's first instruction, after the
    status test's branch, which goes to the statement after the `if'
    (TTY_$K_GET 0xE1C3FE..0xE1C40C `move.l A0,D0; movea.l (0x18,A6),A0;
    tst.l (A0); bne.w 0xE1C614; movea.l D0,A2; pea (A2); bsr.w', the
    result assignment at 0xE1C614; there the status slot is used once
    and loaded late, here twice and loaded at entry);
  - setfc: the first `if' that refers to it does so in its condition:
    the move is that `if''s first instruction, the target of the guard's
    branch (TTY_$K_SET_FUNC_CHAR 0xE6750C..0xE6751E `bls.b 0xE67516; ...;
    movea.l D0,A2; move.l (0x20,A2),D2; btst.l D1,D2; beq.b'). }
module pparkif;
type
  status_$t = record case integer of 1: (all: integer32); end;
  flags_t = set of 0..31;
  desc_t = record
    id: integer32;
    pad: array[1..14] of integer;
    enabled: flags_t;
    count: integer;
    cls: integer;
    raw: boolean;
  end;
  desc_ptr_t = ^desc_t;

function get_desc(line: integer; out status: status_$t): integer32;
  options(extern, val_param, a0_return);
procedure lock(in out d: desc_t); extern;
procedure set_dfl(in out d: desc_t; use_dfl: boolean);
  options(extern, val_param);

function get(in line: integer; out status: status_$t): integer;
var
  d: integer32;
  n: integer;
begin
  n := 0;
  d := get_desc(line, status);
  if status.all = 0 then begin
    lock(desc_ptr_t(d)^);
    n := desc_ptr_t(d)^.count;
  end;
  get := n;
end;

procedure setfc(in line: integer; in func: integer; out status: status_$t);
var
  d: integer32;
begin
  d := get_desc(line, status);
  if status.all <> 0 then
    return;
  if func > 17 then begin
    status.all := 16#350002;
    return;
  end;
  if func in desc_ptr_t(d)^.enabled then
    desc_ptr_t(d)^.cls := 18;
  if not desc_ptr_t(d)^.raw then
    set_dfl(desc_ptr_t(d)^, true);
end;
