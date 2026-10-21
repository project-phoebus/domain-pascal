{ Golden: codegen-patterns 1.5 "Address of a symbol by address"; bead
  domain-pascal-68tc.

  `addr(x)` of another unit's object or of a routine, passed by address
  to a parameter of 4 bytes or more, is a link-time constant: a 4-byte
  cell in the code section holding the address, `pea (d,PC)`, whatever
  the parameter's size.  SAU5 VME_$INT 0xE0ABBA: `move.w #6,-(SP); pea
  (0x80,PC); move.w #6,-(SP); jsr LOG_$ADD' for LOG_$ADD's 192-byte
  `univ' data (../source's reading), the cell 0xE0AC40 `00ff 7c00'
  (PBU_CTL), the next routine at 0xE0AC44.  SAU2 PEB_$LOAD_WCS 0xE32216
  for MST_$WIRE_AREA's `in univ_ptr' pair: cells 0xE322E8
  (PEB_$WIRED_PROC_END, pushed first) and 0xE322E4
  (PEB_$WIRED_PROC_START).  A routine's address the same: DXM_$ADD_SIGNAL
  0xE172B6, cell 0xE172CC (DXM_$ADD_SIGNAL_CALLBACK). }
module usym;
type
  log_$data_t = array[1..96] of integer;
  ctl_t = record
    csr: integer;
    mask: integer;
  end;

var
  pbu_ctl: extern ctl_t;
  peb_$wired_proc_start: extern integer;
  peb_$wired_proc_end: extern integer;

procedure log_$add(entry_type: integer; in data: univ log_$data_t;
                   data_len: integer);
  options(extern, val_param);
procedure mst_$wire_area(in start_va: univ_ptr; in end_va: univ_ptr); extern;
procedure ext_$callback(in r: univ_ptr; in l: integer32); extern;
procedure usym_$cb; extern;

procedure usym_$log;
begin
  log_$add(6, addr(pbu_ctl), 6);
end;

procedure usym_$wire;
begin
  mst_$wire_area(addr(peb_$wired_proc_start), addr(peb_$wired_proc_end));
  mst_$wire_area(addr(pbu_ctl.mask), addr(pbu_ctl.mask));
end;

procedure usym_$routine;
begin
  ext_$callback(addr(usym_$cb), integer32(addr(pbu_ctl)));
end;
