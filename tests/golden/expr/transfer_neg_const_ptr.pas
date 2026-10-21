{ Golden: a negative integer constant transferred to a pointer type
  (owner ruling 2026-10-21, "Allow, 32-bit", docs/reference-clarifications.md
  "Type transfer of a negative constant to a pointer type"; ref 3.2.3
  p. 3-5; bead domain-pascal-j1bt).  The constant is a 32-bit integer, so
  `ddf_handler_t(-1)' is 16#FFFFFFFF, the same constant as
  `ddf_handler_t(integer32(-1))'.  SAU5 PBU's DDF handler fields hold
  16#FFFFFFFF: PBU_$ACQUIRE_INT 0xE588D4 `movea.l #-1,A1' stored into
  each handler, PBU_$SETUP 0xE59842 `cmpi.l #-1,(0x40,A3)'.  This test
  covers the forms of a pointer constant (codegen-patterns 13 stores,
  `movea.l #-1,An' as at 0xE588D4, "Pointer constants", bead
  domain-pascal-gw19; 1.5 cells and `pea (n).w'); it does not reproduce
  those two routines. }
module tn;
type
  ddf_handler_t = ^integer32;
  ddf_t = record
    unit: integer;
    handler: ddf_handler_t;
  end;
  ddf_ptr_t = ^ddf_t;
const
  no_handler = -1;

procedure tn_$take(in h: ddf_handler_t); extern;
procedure tn_$take_v(in h: ddf_handler_t); val_param; extern;

procedure tn_$clear(in d: ddf_ptr_t);
begin
  d^.handler := ddf_handler_t(-1);
  tn_$take(ddf_handler_t(no_handler));
  tn_$take_v(ddf_handler_t(-1));
end;

function tn_$is_none(in d: ddf_ptr_t): boolean;
begin
  tn_$is_none := d^.handler = ddf_handler_t(integer32(-1));
end;

function tn_$is_none2(in d: ddf_ptr_t): boolean;
begin
  tn_$is_none2 := d^.handler = ddf_handler_t(-1);
end;
