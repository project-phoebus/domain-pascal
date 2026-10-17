{ Golden: codegen-patterns 5.2, "6 and 8 bytes", and 11, "Data per
  section" (bead domain-pascal-zrug).  A routine in OS_INIT_PROC copies
  the first variable of its module's WIRED_DATA block, an 8-byte UID,
  into a frame record: the block's address as an immediate in the
  source's register, `movea.l #block,A0; move.l (A0)+,(d,A6); move.l
  (A0)+,(d+4,A6)' (LOG_$INIT 0xE300B2..0xE300BC, `desc.uid :=
  log_$logfile_uid' with LOG_$LOGFILE_UID at the block's start
  0xE2B280), as for another unit's object (5.2, "Block addresses"). }
module log, wired_proc, wired_data;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  desc_t = record
    pad: array[1..2] of integer32;
    uid: uid_$t;
    flags: integer32;
  end;
var
  logfile_uid: uid_$t;          { WIRED_DATA +0 }
  count: integer32;             { +8 }

procedure get_attributes(in d: desc_t); extern;

[section(os_init_proc, os_init_data)]
procedure log_$init;
var desc: desc_t;
begin
  desc.uid := logfile_uid;
  get_attributes(desc);
end;
