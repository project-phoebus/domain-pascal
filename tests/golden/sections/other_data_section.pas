{ Golden: codegen-patterns 11, "Data per section" (bead domain-pascal-a2g).
  A routine in OS_INIT_PROC reaches its module's WIRED_DATA block, which
  is not its data section's, from the block's address as an immediate:
  - `movea.l #block,An' (`207c'), then `(d,An)'; one load serves the
    statements up to a call (SUMA_$INIT 0xE33228, four stores) and the
    block is loaded again after it (DXM_$INIT 0xE2FF54, 0xE2FF66;
    KBD_$INIT 0xE3339E); the register is not held through a call whose
    arguments use it last (XNS_IDP_$INIT 0xE3026E..0xE30280);
  - a first statement that stores a constant into the block loads it
    before the parameters loaded at entry (KBD_$INIT 0xE3336A, then
    `movea.l (8,A6),A2');
  - `addr' of a variable of the block is `lea (d,An),Am' (KBD_$INIT
    0xE333A4); the block's first variable passed by address is `move.l
    #block,-(SP)' (LOG_$INIT 0xE30054);
  - no A5: the routine has no static data of its own section. }
module kbd, wired_proc, wired_data;
type
  ring_t = record
    head, tail: integer;
  end;
  desc_t = record
    state: integer;
    flags: integer;
    buf: ^ring_t;
    ec: integer32;
  end;
var
  pad: integer32;               { WIRED_DATA +0 }
  ring: ring_t;                 { +4 }
  count: integer32;             { +8 }

procedure ec_$init(var x: integer32); extern;

[section(os_init_proc, os_init_data)]
procedure kbd_$init(var d: desc_t);
begin
  ring.head := 0;
  d.state := 0;
  d.ec := 1;
  ec_$init(count);
  d.buf := addr(ring);
  ec_$init(d.ec);
  ec_$init(pad);
end;
