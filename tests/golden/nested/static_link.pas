{ Golden: codegen-patterns 6, nested routines (bead domain-pascal-5aw.3),
  in the shape of PMAP_$FLUSH 0xE1376C and its nested routines 0xE1359C
  and 0xE1360C (`tools/nestsurvey.py').

  - Emission: the nested routines first, in source order, then their
    enclosing routine (0xE1359C, 0xE1360C, 0xE1376C).
  - `upd' is called by the enclosing routine and by its sibling `batch':
    the static link is passed in A1 (0xE13928 `movea.l A6,A1; bsr.w';
    0xE13718 `movea.l A4,A1; bsr.w') and copied at entry (0xE135A4
    `movea.l A1,A2').  `batch' is called by the enclosing routine only:
    it takes the dynamic link `(A6)' as the static link (0xE13616), and
    its caller loads nothing into A1 (0xE13856).
  - Uplevel references through the link register: the enclosing
    routine's by-address parameter passed on (0xE135B8 `move.l
    (8,A2),-(SP)'), an object through it (0xE135DA..0xE135DE), its local
    read and written (0xE1362E, 0xE1375E).
  - Nested frames start with a 4-byte static-link slot (0xE1359C `link
    #-4' with no locals).
  - The enclosing routine keeps the local its nested routines reach in
    the frame (`n', PMAP_$FLUSH's `(-0x50,A6)'), while `done', which they
    do not reach, may be held in a register (codegen-patterns 12.5; bead
    domain-pascal-thc). }
module pmx;
type
  aste_t = record
    uid: integer32;
    seg: integer;
  end;
procedure ast_$invalidate_page(var aste: aste_t; var e: integer32; vpn: integer32);
  options(val_param); extern;
procedure mmap_$avail(vpn: integer32); options(val_param); extern;
procedure ml_$lock(r: integer); options(val_param); extern;
procedure netlog_$log_it(kind: integer; seg: integer); options(val_param); extern;

procedure pmap_$flush(var aste: aste_t; var arr: integer32; start: integer;
                      count: integer; flags: integer; var status: integer32);
  options(val_param);
var
  n: integer;
  done: integer;

  procedure upd(var e: integer32; vpn: integer32; idx: integer);
  begin
    ast_$invalidate_page(aste, e, vpn);
    netlog_$log_it(5, aste.seg);
    mmap_$avail(vpn);
  end;

  procedure batch;
  begin
    ml_$lock(n);
    upd(arr, 5, n);
    n := 0;
  end;

begin
  n := count;
  status := 0;
  done := flags;
  if n > 0 then
    batch;
  upd(arr, 7, start);
  status := done;
end;
