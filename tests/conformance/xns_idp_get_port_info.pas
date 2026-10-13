{ XNS_IDP_$GET_PORT_INFO, 0xE18FB8, 0x1E bytes (tools/imgdis.sh E18FB8
  1E).  Semantics: ../source/domain_os/xns/idp_misc.c: not implemented;
  stores status_$mac_port_op_not_implemented (0x3A0001; 10.2 status-code
  database: "port operation not implemented") through its third
  argument.

  A fragment of the XNS_IDP module (code segment 0xE17BF8), compiled with
  `dpc --fragment-origin 0xE18FB8'.  Its data area,
  `.bss.moddata.XNS_IDP', is placed at the map's `D E2B314 XNS_IDP'
  (manifest --section-addr).  XNS_IDP_$GET_STATS follows it
  (xns_idp_get_stats.pas).

  XNS_IDP_$GET_PORT_INFO is the first `s' routine of its segment
  (codegen-patterns 4; `tools/a5survey.py switch XNS_IDP', segment
  0xE17BF8: every routine before it is `S'), so it loads A5 without using
  the data: the manifest passes `a5-always=XNS_IDP_$GET_PORT_INFO'.

  Patterns exercised (docs/codegen-patterns.md):
  - 4    always mode: `pea (A5); lea 0xE2B314,A5' ... `movea.l
         (-4,A6),A5' (0xE18FBC, 0xE18FCE);
  - 1.2  default convention: three by-address slots, the status the last
         (0x10,A6);
  - 9    `move.l #0x3a0001,(A0)' through the status slot (0xE18FC4).

  Reconstructed (not known from the original): the names and types of
  the two parameters it never reads (`in ... integer32' for each 4-byte
  slot; ../source's names); `out' for the status; `xns_idp_$data_stand_in',
  a variable only so that the unit has a data area (this routine reads
  none of it; the XNS_IDP block's first longword is packets_sent,
  xns_idp_get_stats.pas).  The status constant is ../source's name. }
module xns_idp;
const
  status_$mac_port_op_not_implemented = 16#003A0001;
type
  status_$t = record
    all: integer32;
  end;
var
  xns_idp_$data_stand_in: integer32;   { stand-in, see above }

procedure xns_idp_$get_port_info(in channel: integer32; in port_info: integer32;
                                 out status: status_$t);
begin
  status.all := status_$mac_port_op_not_implemented;
end;
