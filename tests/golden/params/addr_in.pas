{ `addr' of an `in' parameter, or of a part of one (bead
  domain-pascal-mbtx; ref "Addr", p. 4-13: any variable except a routine
  with the internal attribute; 5.3.2 p. 5-7 forbids only assignment).
  codegen-patterns 5.2, "`addr` of a parameter".

  - ai_$put: the default convention passes the record by address; its
    slot used only for its address, stored from the slot itself, `move.l
    (8,A6),(d,A6)' (PKT_$COPY_TO_PA 0xE12538; MAC_OS_$PUT_INFO 0xE0C322).
    A slot also used once otherwise is refused (bead domain-pascal-m2w3).
  - ai_$init: a slot used three times is loaded at entry and its address
    stored from that register, `move.l A0,(4,A0)' (EC_$INIT 0xE15208,
    an `out' parameter's).
  - ai_$pass: `addr' passed by value is pushed as the parameter passed on
    by address: the slot, `move.l (d,A6),-(SP)' (LOG_$READ 0xE1780C); a
    field, `movea.l (d,A6),An; pea (k,An)' (KBD_$OUTPUT_BUFFER_DRAINED
    0xE1CE9E); a field stored, `lea (k,An),Am; move.l Am,<dst>' (0xE0BE9A).
  - ai_$val: under `val_param' an `in' integer32 is passed by value; its
    address is the slot's, `pea (d,A6)' and `lea (d,A6),An; move.l
    An,<dst>' (as for a value parameter; no compiled site of the store
    with d > 0 is known). }
module ai;
type
  info_t = record version: integer32; port: integer32; end;
  ec_t = record value: integer32; head, tail: univ_ptr; end;
  bytes_t = array[1..8] of char;
  info_ptr_t = ^info_t;
var
  g: univ_ptr;
procedure ai_$copy(in src: univ bytes_t; out dst: univ bytes_t; in n: integer32);
  options(val_param); extern;
procedure ai_$take(in p: univ_ptr); options(val_param); extern;
procedure ai_$lock; extern;

procedure ai_$put(in info: info_t; out status: integer32);
var
  dst: info_ptr_t;
  src: info_ptr_t;
begin
  status := 0;
  ai_$lock;
  src := addr(info);
  ai_$copy(src^, dst^, 8);
end;

procedure ai_$init(in out ec: ec_t);
begin
  ec.value := 0;
  ec.head := addr(ec);
  ec.tail := addr(ec);
end;

procedure ai_$pass(in info: info_t; in ec: ec_t);
begin
  ai_$take(addr(info));
  ai_$take(addr(ec.tail));
  g := addr(info.port);
end;

procedure ai_$val(in x: integer32; in y: integer32);
  options(val_param);
begin
  ai_$take(addr(y));
  g := addr(x);
end;
