{ Golden: codegen-patterns 5.1 ("`x in c', a constant register-sized
  set"; bead domain-pascal-bitu): a run-time element tested against a
  constant set of one word or one longword, the constant in a data
  register and `btst Dk,Dn', no range check.
  - AS_IO: `vol_idx in [1..10]', an integer constructor (one word to
    dpc-sema) loaded as the longword `move.l #0x7fe', the `0..65535'
    element zero-extended around it (AS_IO_SETUP 0xE6B75C..0xE6B768
    `clr.l D0; move.l #0x7fe,D1; move.w (A0),D0; btst D0,D1; bne.b');
  - DISMOUNT: the element a by-value parameter loaded at entry into a
    register, copied into the cleared one (DISK_$DISMOUNT 0xE6CFF8..
    0xE6D008 `move.w (8,A6),D2; clr.l D0; move.l #0x7fe,D1; move.w
    D2,D0; btst D0,D1; beq.w', then `subq.l #2,SP; move.w D2,-(SP)'
    0xE6D01A);
  - INT_IN: an `integer' element of a constant `moveq' loads, not
    extended: the element, then the constant (TTY_$K_SET_DELAY
    0xE67A18..0xE67A1C `move.w (A2),D0; moveq #0x1f,D1; btst D0,D1';
    tty_$i_wait 0xE1C264..0xE1C26A), as a value `sne' in the set's
    register (PROC1_$TST_LOCK 0xE148DE);
  - ENUM_IN: a set of an enumeration of 16 values, one word, that
    `moveq' loads (36 of the 51 constant sets of compiled code). }
module in_const_set;
type
  pinteger = 0..65535;
  status_$t = integer32;
  e16 = (e0, e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15);
var
  gw: extern integer;

procedure disk_$invalidate(vol_idx: pinteger); options(val_param, extern);

procedure as_io(var vol_idx: pinteger; out status: status_$t);
begin
  if not (vol_idx in [1..10]) then
    status := 16#8000F;
end;

procedure dismount(vol_idx: pinteger); options(val_param);
begin
  if vol_idx in [1..10] then begin
    disk_$invalidate(vol_idx);
    gw := vol_idx;
  end;
end;

function int_in(k: integer): boolean; options(val_param);
begin
  int_in := k in [1..6];
end;

procedure enum_in(e: e16); options(val_param);
begin
  gw := 0;
  if e in [e1, e3..e6] then gw := 1;
end;
