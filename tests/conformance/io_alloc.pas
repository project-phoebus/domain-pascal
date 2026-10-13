{ IO_$ALLOC, 0xE1A9C2, 0x12 bytes (tools/imgdis.sh E1A9C2 12): the last
  routine of the IO_ module (map `I E1A404 IO_ size = 5D0').  There is no
  C for it in ../source (bead source-5v9m there lists it as untranslated;
  no image code calls it): the semantics are read from the image alone.
  It clears the longword its fourth argument points to and returns 0 in
  D0 (`clr.l D0', a 4-byte result).

  A fragment that ends its module, compiled with `dpc --fragment-origin
  0xE1A9C2 --fragment-ends-module': the 0x2048 filler at 0xE1A9D2 is the
  module-end padding of aligning code (codegen-patterns 7, 11).  The
  routine does not load A5, and this unit declares no data
  (codegen-patterns 4).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  default convention: four 4-byte slots, the status slot last at
         (0x14,A6);
  - 2    `link #-4': the result variable's frame slot, never touched;
  - 9    `status.all := status_$ok' (`movea.l (0x14,A6),A0; clr.l (A0)');
  - 3    a constant result 0 as `clr.l D0', after another statement (a
         function body of two statements that loads nothing at entry).

  Reconstructed (not known from the original): everything but the frame
  shape: the parameter names, types and modes (three 4-byte slots never
  read: `in ... integer32' stands for each; `out' for the status slot,
  which is only written), the result type (4 bytes in D0, so not a
  pointer, which would come back in A0, ref 5.5.10; integer32 is taken),
  the reading of the fourth slot as a status (it is cleared as status
  out-parameters are, codegen-patterns 9; that it is a status_$t is a
  guess from the shape alone), status_$t as a record with the integer32
  field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3), status_$ok = 0 (ref 9.1,
  p. 9-1: success is 0; "Open", pp. 4-132..4-133). }
module io_;
const
  status_$ok = 0;
type
  status_$t = record
    all: integer32;
  end;

function io_$alloc(in arg1, arg2, arg3: integer32; out status: status_$t): integer32;
begin
  status.all := status_$ok;
  io_$alloc := 0;
end;
