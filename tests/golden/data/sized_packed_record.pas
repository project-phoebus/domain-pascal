{ codegen-patterns 14 ("One-byte packed records: `[byte]'");
  reference-clarifications 52; bead domain-pascal-7kpw.  A packed record
  of 8 bits is 2 bytes (ref 3.10.7); under a `[byte]' size attribute it
  is one byte, and an assignment between the two copies that byte: SAU9
  SCSI_$C90_INT 0x3C4A57A4 `move.b (16,A0),(-72,A6)', the 53C90's status
  register into a 2-byte local, tested there as a word. }
module sized_packed_record;
type
  c90_status_t = packed record
    int: boolean;
    gross_error: boolean;
    parity_error: boolean;
    term_count: boolean;
    valid_group: boolean;
    phase: 0..7;
  end;
  c90_dest_t = packed record
    unused: 0..31;
    id: 0..7;
  end;
  c90_reg10_t = record
    case integer of
      1: (status: [byte] c90_status_t);
      2: (dest: [byte] c90_dest_t);
  end;
  c90_regs_t = [device] record
    pad: array[1..16] of char;
    r10: c90_reg10_t;
    unused_12: integer;
  end;
  c90_regs_ptr_t = ^c90_regs_t;
  cb_t = record regs: c90_regs_ptr_t; end;
  cb_ptr_t = ^cb_t;

function c90_int(cb: cb_ptr_t): boolean;
var
  stat: c90_status_t;
begin
  stat := cb^.regs^.r10.status;
  c90_int := stat.int;
end;
