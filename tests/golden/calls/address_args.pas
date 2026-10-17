{ Golden: codegen-patterns 1.5 ("`addr(x)` by value", "The target of a
  type transfer of `addr(x)`", "The target of a pointer in the unit's
  data"; bead domain-pascal-hs92).
  - `T(addr(x))^' passed by address is x passed by address (ref "Type
    Transfer Functions", pp. 4-197..4-198: no bits change):
    ASKNODE_$WHO_NOTOPO 0xE661BE `move.l #TIME_$CLOCKH,-(SP)' to EC_$READ;
    the module-local 0xE32B76 (sio/init_dtte.pas) 0xE32B86 `pea
    (0xc,A2)', 0xE32B9E `pea (A2)', its slot held in A2;
  - the target `p^' of a pointer in the unit's data: the pointer,
    `move.l (d,A5),-(SP)' (AUDIT 0xE70D58 (the module-local 0xE70D38, audit/stop_logging.pas) to EC_$ADVANCE;
    WIN 0xE19828 to its SEEK);
  - `addr(x)' by value of another unit's element at a constant index:
    `pea (sym+d).l', the first element `move.l #sym,-(SP)' (DMA_$INIT
    0xE0A366..0xE0A3A0, the pad first);
  - `addr(x[i])' by value: `pea (d,An,Dn.w)' (TIME_$WAIT 0xE165E0,
    MSG_$RCV_CONTIGI 0xE59908; here an element of the unit's data, the
    form of the element pushed by address, golden `expr/index');
  - `addr(p^.f)' by value, p a parameter held in an address register:
    `pea (d,An)' (SMD 0xE6EBD4 `pea (4,A3)');
  - `addr(p^.f)' by value, p a pointer local: `movea.l (d,A6),A0; pea
    (k,A0)', as by address (the DISK module-local 0xE3C9FE (disk/wait_io.pas) at 0xE3CA4E). }
module addrargs;
type
  ec_t = record
    value: integer32;
    head, tail: univ_ptr;
  end;
  ec_p = ^ec_t;
  ec_bytes_t = array[1..3] of integer32;
  rec_t = record
    ec: ec_t;                 { 0x00 }
    lock: integer;            { 0x0c }
    ec2: ec_t;                { 0x0e }
  end;
  rec_p = ^rec_t;
  dtte_t = record
    output_ec: ec_bytes_t;    { 0x00 }
    input_ec: ec_bytes_t;     { 0x0c }
  end;
var
  wired: rec_p;
  clockh: extern integer32;
  othr: extern rec_p;
  as_id: extern integer;
  quit_ec: array[0..57] of ec_t;
  chans: extern array[0..3] of ec_t;

procedure ec_$advance(var ec: ec_t); extern;
procedure ec_$init(out ec: ec_t); extern;
function ec_$read(in ec: ec_t): integer32; extern;
function ec_$wait(ec1, ec2: ec_p; v: integer32): integer;
  options(extern, val_param);
procedure chan_init(p: univ_ptr; n: integer); options(extern, val_param);

procedure addrargs_$stop;
begin
  ec_$advance(wired^.ec);
end;

procedure addrargs_$init_dtte(var dtte: dtte_t; v: integer32);
begin
  ec_$init(ec_p(addr(dtte.input_ec))^);
  ec_$init(ec_p(addr(dtte.output_ec))^);
  v := ec_$read(ec_p(addr(clockh))^);
end;

procedure addrargs_$chans;
begin
  chan_init(addr(chans[3]), 3);
  chan_init(addr(chans[0]), 0);
end;

function addrargs_$wait_quit: integer;
begin
  addrargs_$wait_quit := ec_$wait(addr(quit_ec[as_id]),
                                  ec_p(addr(clockh)), 0);
end;

procedure addrargs_$wait_held(var r: rec_t);
var
  x: integer;
begin
  r.lock := 0;
  x := ec_$wait(addr(r.ec2), ec_p(addr(clockh)), 1);
  r.lock := x;
end;

procedure addrargs_$wait_local;
var
  p: rec_p;
  x: integer;
begin
  p := othr;
  x := ec_$wait(addr(p^.ec2), ec_p(addr(clockh)), 1);
end;
