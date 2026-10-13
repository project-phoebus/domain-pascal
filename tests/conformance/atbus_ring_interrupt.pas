{ RING_INTERRUPT, 0xE0AB0C, 0x1C bytes (tools/imgdis.sh E0AB0C 1C).
  Semantics: ../source/domain_os/ring/interrupt.c; data layout:
  io_int_ctrl_t in ../source/domain_os/io/io.h (type2_do_io at +0,
  type2_dcte at +4, _Static_assert "io_int_ctrl_t.type2_dcte"), the
  ATBUS_ data block (map `D E22904 ATBUS_ size = 2C').

  The unit is a fragment of the ATBUS_ module (code segment 0xE0AABC,
  after DISK_INTERRUPT), compiled with `dpc --fragment-origin 0xE0AB0C';
  its data area is placed at the ATBUS_ data block by the manifest.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.6  a call through a routine pointer: the argument pushed, then the
         pointer loaded from the unit's data into a new address register,
         `movea.l (A5),A0; jsr (A0)' (0xE0AB18..0xE0AB1E);
  - 1.2  the pointer type is `val_param' (ref p. 4-146; Appendix B 181,
         182): the DCTE pointer is pushed by value, `move.l (4,A5),-(SP)';
  - 1.1  the argument is not popped: `unlk' follows;
  - 4    an exported routine reading its unit's data saves A5 with
         `pea (A5)', loads it, restores it from below the locals
         (`movea.l (-8,A6),A5'); the call through the pointer is not a use
         of the data (the callee sets its own A5).

  Reconstructed (not known from the original): the names; the DCTE
  record (only its pointer is used); `in' as the pointer type's
  parameter mode (by value under `val_param' either way, Table 5-1); the
  local `frame_word', which stands for the 4 bytes of `link.w A6,#-4' the
  body never addresses (the C has no local; codegen-patterns 2 and 12.5:
  frames with untouched slots).  Only the first 8 bytes of the 0x2C-byte
  ATBUS_ data block are declared. }
module atbus_;
type
  dcte_t = record
    word0: integer32;
  end;
  dcte_ptr_t = ^dcte_t;
  do_io_t = ^procedure (in dcte: dcte_ptr_t); val_param;
  int_ctrl_t = record
    type2_do_io: do_io_t;          { +0 }
    type2_dcte: dcte_ptr_t;        { +4 }
  end;
var
  io_$int_ctrl: int_ctrl_t;        { +0 of the ATBUS_ data block }

procedure ring_interrupt;
var
  frame_word: integer32;           { stand-in, see above }
begin
  io_$int_ctrl.type2_do_io^(io_$int_ctrl.type2_dcte);
end;
