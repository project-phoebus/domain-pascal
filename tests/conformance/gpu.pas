{ GPU_$INIT, 0xE707FC, 0x12 bytes, and GPU_$TERM, 0xE7080E, 2 bytes: the
  whole GPU module (map `I E707FC GPU size = 14'; tools/imgdis.sh E707FC
  14).  Semantics: ../source/domain_os/gpu/init.c (stores
  status_$gpu_device_not_present, 0x2D0001; 10.2 status-code database:
  "device not present in system") and gpu/term.c (does nothing).

  A whole map module (bead domain-pascal-xdh.2): tools/conform.py compiles
  it without --fragment-origin, and the link test swaps it for both C
  objects.  The module is 0x14 bytes, a multiple of 4: no module-end
  filler.

  Patterns exercised (docs/codegen-patterns.md):
  - 9    `status.all := ...' through the status slot, `movea.l (8,A6),A0;
         move.l #0x2d0001,(A0)' (0xE70800), `link #0';
  - 2    GPU_$TERM, an empty procedure: a bare `rts' (0xE7080E).

  Reconstructed (not known from the original): GPU_$TERM without
  parameters (an empty procedure's are not visible; the C has none);
  `out' as the status mode; status_$t as a record with the integer32
  field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3).  The constant is
  ../source's name (gpu/gpu.h). }
module gpu;
const
  status_$gpu_device_not_present = 16#002D0001;
type
  status_$t = record
    all: integer32;
  end;

procedure gpu_$init(out status: status_$t);
begin
  status.all := status_$gpu_device_not_present;
end;

procedure gpu_$term;
begin
end;
