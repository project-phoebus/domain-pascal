{ PKT_$INIT, 0xE2F84C, 0x34 bytes: the whole init-section segment of PKT
  (map `I E2F84C PKT size = 34' in OS_INIT_PROC; tools/imgdis.sh E2F84C
  34).  Semantics: ../source/domain_os/pkt/init.c: create the ping server
  process with PROC1_$CREATE_P and crash on a bad status.

  A whole map segment (bead domain-pascal-xdh.2): tools/conform.py
  compiles it without --fragment-origin; the module-end filler 0x2048
  follows the `rts' (0xE2F87E).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.6  `addr(r)' of another unit's routine pushed by value as `move.l
         #PKT_$PING_SERVER,-(SP)' (0xE2F85A), beside a longword constant
         pushed by value, `move.l #0x0800000f,-(SP)', and the status
         local by address, `pea (-4,A6)': a `val_param' callee
         (ref 5.2, Table 5-1); 12 bytes, so no pad; popped with `lea
         (12,SP),SP';
  - 9    the status tested `tst.l (-4,A6); beq' and passed by address to
         CRASH_SYSTEM, not popped before `unlk' (1.1);
  - `discard' (ref "Discard", p. 4-62) of PROC1_$CREATE_P's result (the
    C's uint16_t, not read: no code).

  Reconstructed (not known from the original): the names and types
  (../source's: proc1/proc1.h PROC1_$CREATE_P returns uint16_t, here
  `integer'); `val_param' on PROC1_$CREATE_P (its value arguments are
  pushed by value); the entry point's type as a pointer to a procedure
  without parameters (PKT_$PING_SERVER's heading in ../source); CRASH_SYSTEM's heading as
  ../source's misc/crash_system.ins.pas has it (`in univ status_$t',
  by address); status_$t as a record with the integer32 field `all' (ref
  9.1.1-9.1.2, pp. 9-1..9-3).  The process type 0x0800000F is
  ../source's PING_SERVER_TYPE (pkt/init.c). }
module pkt;
const
  ping_server_type = 16#0800000F;
type
  status_$t = record
    all: integer32;
  end;
  pkt_$server_t = ^procedure;

procedure pkt_$ping_server; extern;
function proc1_$create_p(in entry: pkt_$server_t; in ptype: integer32;
                         out status: status_$t): integer; val_param; extern;
procedure crash_system(in status: univ status_$t); extern;

procedure pkt_$init;
var
  status: status_$t;
begin
  discard(proc1_$create_p(addr(pkt_$ping_server), ping_server_type, status));
  if status.all <> 0 then
    crash_system(status);
end;
