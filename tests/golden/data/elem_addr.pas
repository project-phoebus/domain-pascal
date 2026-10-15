{ Golden: codegen-patterns 5.2 ("The element's address as a value"),
  12.4 R2.  Bead domain-pascal-o1kl; survey `tools/regalloc.py elemaddr'.

  The address of an element of another unit's array as a `with' record:
  the index's value, `movea.l #sym,An', the scaling, then `lea
  (0,An,Dx),Ar', the lower bound and field offsets in each reference;
  Ar is the allocator's, the dying base's register counted free (12.4 R2
  does not apply to an indexed `lea').
  - find: read twice, no call: the base's own register (XPD_$FIND_INDEX
    0xE5AF54..0xE5AF60 `move.w D0,D1; movea.l #0xEA551C,A0; muls.w
    #228,D1; lea (0,A0,D1.l),A0', then `(-190,A0)' and `(-185,A0)').
  - avail: live across a call: a callee-saved register, the base in A0
    (MMAP_$AVAIL 0xE0CC72..0xE0CC80 `move.l (8,A6),D2; movea.l #MMAP,A0;
    move.l D2,D0; lsl.l #4,D0; lea (0,A0,D0.l),A2', the record passed
    with `pea (-0x2000,A2)' after the call; byte-identical but for the
    frame, tests/conformance/mmap_avail.pas).
  Read once and not across a call: refused (bead domain-pascal-1ibr). }
module elad;
type
  page_t = record
    wire_count: char;
    seg_offset: char;
    segment: integer;
    wsl_index: char;
    in_wsl: boolean;
    pad: array[1..5] of integer;
  end;
  slot_t = record
    debugger_idx: integer;
    flags: integer;
    pad: integer32;
  end;
var
  mmap: extern array[16#200..16#FFF] of page_t;
  slots: extern array[1..64] of slot_t;
  cur: extern integer;

procedure crash(in n: integer32); extern;
procedure add_to_wsl(var page: page_t; vpn: integer32; wsl: integer;
                     head: boolean); options(val_param); extern;

function find(index: integer): integer; options(val_param);
begin
  find := 0;
  with slots[index] do
    if debugger_idx <> cur then
      find := 1
    else if flags = 0 then
      find := 2;
end;

procedure avail(vpn: integer32); options(val_param);
begin
  with mmap[vpn]: page do begin
    if page.wsl_index > chr(16#45) then
      crash(16#00060004);
    add_to_wsl(page, vpn, ord(page.wsl_index), true);
  end;
end;
