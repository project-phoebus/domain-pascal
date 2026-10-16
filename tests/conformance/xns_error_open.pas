{ MAYBE_OPEN_ERROR_SOCKET, 0xE178AA, 0x66 bytes (tools/imgdis.sh E178AA 66),
  compiled with dpc --merge-constant-stores (the manifest's
  `merge-constant-stores'): XNS_ERROR 0xE17850 is a segment that merges
  constant stores (codegen-patterns 13, "Store merging"; bead
  domain-pascal-qb9q; tools/storesurvey.py units).
  Semantics and names: ../source/domain_os/xns/error_send.pas (its
  maybe_open_error_socket) and xns/xns.ins.pas xns_$os_open_opt_t, 0x24
  bytes, here with only the fields the routine stores and reads.

  Patterns exercised (docs/codegen-patterns.md):
  - 13   two word constants stored to adjacent words of a frame record at
         0 mod 4 merged into one longword, `move.l #0x30008,(-40,A6)'
         (0xE178D2: opt.socket := 3; opt.flags := 8), then the longword
         above them stored on its own, `clr.l (-36,A6)' (0xE178DA:
         opt.demux := nil);
  - 4    the unit's data off A5 (client_ref_count +0x74, the channel
         +0x76, XNS_ERROR_$STD_IDP_CHANNEL 0xE2B312 in the map);
  - 1.3  an extern variable passed by address as `move.l #sym,-(SP)'.

  Test scaffolding, not known from the original: `unused', a longword
  local declared first so that opt is at (-40,A6) as in the image (the
  image's frame has 4 bytes above the record that the code never
  addresses; ../source's translation has no such local, so dpc places
  the record at (-36,A6) there: bead domain-pascal-qstg, also APP_$STD_OPEN
  0xE00B92 and RIP_$STD_OPEN 0xE15AAE); `rest' for the record's 28 bytes
  from +0x08. }
module xns_error, wired_proc, wired_data;
type
  status_$t = record
    all: integer32;
  end;
  ml_$exclusion_t = record
    w: array[1..4] of integer32;
  end;
  open_opt_t = record
    socket: integer;                    { 0x00 }
    flags_channel: integer;             { 0x02 }
    demux: univ_ptr;                    { 0x04 }
    rest: array[1..7] of integer32;     { 0x08 }
  end;
var
  pad: array[1..58] of integer;         { +0x00 }
  client_ref_count: integer;            { +0x74 }
  xns_error_$std_idp_channel: define integer;   { +0x76 }
  xns_error_$client_mutex: extern ml_$exclusion_t;

procedure ml_$exclusion_start(var excl: ml_$exclusion_t); extern;
procedure ml_$exclusion_stop(var excl: ml_$exclusion_t); extern;
procedure xns_idp_$os_open(in out opt: open_opt_t; out status: status_$t); extern;

procedure maybe_open_error_socket(out status: status_$t);
label
  10;
var
  unused: integer32;
  opt: open_opt_t;
begin
  status.all := 0;
  ml_$exclusion_start(xns_error_$client_mutex);
  if client_ref_count = 0 then begin
    opt.socket := 3;
    opt.flags_channel := 8;
    opt.demux := nil;
    xns_idp_$os_open(opt, status);
    if status.all <> 0 then
      goto 10;
    xns_error_$std_idp_channel := opt.flags_channel;
  end;
  client_ref_count := client_ref_count + 1;
10:
  ml_$exclusion_stop(xns_error_$client_mutex);
end;
