{ IIC_$ACQUIRE .. IIC_$RECEIVE_CHECK, 0xE70A56..0xE70AF4: the IIC module
  (map `I E70A54 IIC size = A0') without IIC_$INIT (tools/imgdis.sh
  E70A54 A0).  Semantics: ../source/domain_os/iic/iic.h and iic/receive.c:
  in this image every entry point is a stub that stores
  status_$iic_device_not_in_system (0x2C000A; 10.2 status-code database,
  "OS / internet interface controller": "device not in system") through
  its status argument.

  IIC_$INIT (0xE70A54) is a bare `rts' with no `link': an empty
  procedure, which the compiler emits without a frame (codegen-patterns
  2, Reported).  dpc gives every routine a frame, so the unit starts
  after it: a fragment compiled
  with `dpc --fragment-origin 0xE70A56 --fragment-ends-module'.  The
  module ends with the 0x2048 filler at 0xE70AF2 (the IIC code aligns,
  codegen-patterns 7; it has no loop).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2   default convention: every parameter by address, one 4-byte slot
          each; the status slot is the last one;
  - 9     error stores `move.l #0x2c000a,(A0)' through the status slot
          (`status.all := ...');
  - 12.4 R4(b) by-address slots used once in a body of several statements
          each take a register no earlier statement left a value in:
          IIC_$RECEIVE 0xE70AC0..0xE70AD0 A0, A1, then A2 (callee-saved,
          saved with `pea (A2)' and restored from (-4,A6)) although A0
          and A1 are dead;
  - 3     constant boolean results `clr.b D0' (IIC_$EXISTS 0xE70AB4);
          a function body of two statements, a status store and then the
          result (IIC_$RECEIVE_CHECK 0xE70AE2..0xE70AEC), with the frame
          slot of the result variable (`link #-4') never touched;
  - 13    `clr.w' through a slot.

  Reconstructed (not known from the original): the parameters the stubs
  never read (their number and slot size are known, their types and
  modes are not: `in ... integer32' stands for each 4-byte slot, and the
  names are ../source's placeholders); `out' as the mode of the status
  and count parameters (`var' passes the same address, ref 5.2 Table
  5-1); the count parameters as `integer' (`clr.w': 16 bits; the C types
  them uint16_t, which would be `0..65535', the same store); status_$t
  as a record with the integer32 field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3:
  status_$t comes from /sys/ins/base.ins.pas and has fields `all' and
  `code'; we do not have base.ins.pas, so only `all', at offset 0, is
  declared).  The constant is ../source's name (iic/iic.h). }
module iic;
const
  status_$iic_device_not_in_system = 16#002C000A;
type
  status_$t = record
    all: integer32;
  end;

procedure iic_$acquire(in arg1: integer32; out status: status_$t);
begin
  status.all := status_$iic_device_not_in_system;
end;

procedure iic_$release(in arg1: integer32; out status: status_$t);
begin
  status.all := status_$iic_device_not_in_system;
end;

procedure iic_$statistics(in arg1, arg2: integer32; out status: status_$t);
begin
  status.all := status_$iic_device_not_in_system;
end;

procedure iic_$self_test(in arg1, arg2: integer32; out status: status_$t);
begin
  status.all := status_$iic_device_not_in_system;
end;

procedure iic_$send(in arg1, arg2, arg3, arg4, arg5: integer32;
                    out status: status_$t);
begin
  status.all := status_$iic_device_not_in_system;
end;

function iic_$exists: boolean;
begin
  iic_$exists := false;
end;

procedure iic_$receive(in arg1, arg2: integer32; out count_ret3: integer;
                       in arg4: integer32; out count_ret5: integer;
                       out status: status_$t);
begin
  status.all := status_$iic_device_not_in_system;
  count_ret5 := 0;
  count_ret3 := 0;
end;

function iic_$receive_check(in arg1: integer32; out status: status_$t): boolean;
begin
  status.all := status_$iic_device_not_in_system;
  iic_$receive_check := false;
end;
