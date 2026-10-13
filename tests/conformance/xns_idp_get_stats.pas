{ XNS_IDP_$GET_STATS, 0xE18FD6, 0x2C bytes (tools/imgdis.sh E18FD6 2C).
  Semantics: ../source/domain_os/xns/idp_misc.c; data layout:
  xns_$idp_data_t in ../source/domain_os/xns/xns.h (packets_sent,
  packets_received, packets_dropped: three longwords at +0, +4, +8 of the
  XNS_IDP data block, map `D E2B314 XNS_IDP size = 53C'); the record
  passed back is xns_$idp_stats_t (the same three longwords).

  A fragment of the XNS_IDP module (code segment 0xE17BF8), compiled with
  `dpc --fragment-origin 0xE18FD6'.  Its data area, the NOBITS section
  `.bss.moddata.XNS_IDP', is placed at the XNS_IDP data segment by the
  manifest (`--section-addr').  No loop and no module end, so no
  loop-alignment option is needed.

  Patterns exercised (docs/codegen-patterns.md):
  - 4     an exported routine that uses its unit's data: `pea (A5)',
          `lea 0xE2B314,A5', `movea.l (-4,A6),A5' before `unlk';
  - 12.2  the stats slot, used three times, loaded at entry into A0
          (0xE18FE2);
  - 5.2, 13  three field assignments from the data area as memory-to-
          memory moves, `(A5)' for offset 0 (0xE18FE6..0xE18FEE);
  - 9, 12.4 R4(b)  `status.all := status_$ok' as the last statement:
          the status slot, used once, in A1 (0xE18FF4) although A0 is
          dead there (A0 holds a value of earlier statements), then
          `clr.l (A1)'.

  Reconstructed (not known from the original): the names (the routine's
  from the map, the others from ../source); three field assignments
  rather than one record assignment (a 12-byte record assignment would be
  an `(An)+' copy, codegen-patterns 5.2; the C assigns three fields);
  `out' as the mode of both parameters (`var' passes the same addresses,
  ref 5.2 Table 5-1); status_$t as a record with the integer32 field
  `all' at offset 0 (ref 9.1.1-9.1.2, pp. 9-1..9-3; we do not have
  /sys/ins/base.ins.pas, so no other field is declared); status_$ok = 0
  (ref 9.1, p. 9-1: success is 0; "Open", pp. 4-132..4-133: `if
  statrec.all = 0' and `if statrec.all = status_$ok' test the same
  success).  Only the three counters of the
  0x53C-byte data block are declared: they are at offset 0. }
module xns_idp;
const
  status_$ok = 0;
type
  status_$t = record
    all: integer32;
  end;
  xns_$idp_stats_t = record
    packets_sent: integer32;
    packets_received: integer32;
    packets_dropped: integer32;
  end;
var
  packets_sent: integer32;      { +0 of the XNS_IDP data block }
  packets_received: integer32;  { +4 }
  packets_dropped: integer32;   { +8 }

procedure xns_idp_$get_stats(out stats: xns_$idp_stats_t; out status: status_$t);
begin
  stats.packets_sent := packets_sent;
  stats.packets_received := packets_received;
  stats.packets_dropped := packets_dropped;
  status.all := status_$ok;
end;
