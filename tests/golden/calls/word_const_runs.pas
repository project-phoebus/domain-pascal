{ Golden: codegen-patterns 1.5 "Adjacent word constants merged", runs of
  three or more (bead domain-pascal-9pq8; tools/pushsurvey.py).  A run of
  adjacent constant 2-byte slots (words, and `false', a byte 0) is paired
  in push order, from its rightmost argument; an odd run's leftmost
  (last pushed) word stands alone.  `true' (`st -(SP)') is never merged
  and splits a run.
  - five words: NETWORK_$PAGE_SERVER 0xE115FA `subq.l #2,SP; move.l
    #0x80400; move.l #0x100001; move.w #1' for SOCK_$OPEN;
  - four words: MSG_$SARI 0xE59DE0 `move.l #0x10400; move.l #0x10001'
    for SOCK_$ALLOCATE_USER;
  - three zero words: 0xE135CE `subq.l #2,SP; clr.l; clr.w' for
    NETLOG_$LOG_IT's last three;
  - `false' inside a run: PACCT_$SHUTDN 0xE5A71A `clr.l; clr.l; clr.l;
    move.l #0x40000' for FILE_$PRIV_UNLOCK (4, 0, false, 0, then two
    longword zeros);
  - `false' left over: 0xE548BE `move.l #0x880000; clr.w' for
    FILE_$PRIV_LOCK's key, flags and local_only;
  - `true' splitting a run: 0xE51510 `pea (0x4).w; st; clr.l' (side,
    lock_mode | local_only | flags, key). }
module wruns;
type
  uid_t = record high, low: integer32; end;
  status_t = record all: integer32; end;
  clock_t = record high: integer32; low: integer; end;
  ids_t = array[1..2] of integer32;

var
  any_uid: extern uid_t;
  num: extern integer;
  word_v: extern integer;
  asid_v: extern integer;
  ok: extern boolean;

function sock_$open(sock_num: integer; max_queue: integer;
                    hdr_pages: integer; data_pages: integer;
                    max_data_len: integer): boolean;
  options(extern, val_param);

function sock_$allocate_user(out sock_num: integer; max_queue: integer;
                             hdr_pages: integer; data_pages: integer;
                             max_data_len: integer): boolean;
  options(extern, val_param);

procedure netlog_$log_it(kind: integer; in id: ids_t; p3: integer;
                         p4: integer; p5: integer; p6: integer;
                         p7: integer; p8: integer);
  options(extern, val_param);

function file_$priv_unlock(in file_uid: uid_t; lock_slot: integer32;
                           lock_mode: integer; asid: integer;
                           by_key: boolean; key: integer;
                           rem_key: integer32; rem_node: integer32;
                           out dtv_out: clock_t;
                           out status: status_t): boolean;
  options(extern, val_param);

procedure file_$priv_lock(in file_uid: uid_t; asid: integer;
                          side: integer; lock_mode: integer;
                          local_only: boolean; flags: integer;
                          key: integer; rem_key: integer32;
                          rem_node: integer32; rem_extra: integer32;
                          in acl_ctx: univ integer32; rem_wait: integer;
                          in out slot_io: integer32;
                          out rights_out: integer;
                          out status: status_t);
  options(extern, val_param);

procedure wruns_$sock;
begin
  ok := sock_$open(1, 16#10, 1, 8, 16#400);
  ok := sock_$allocate_user(num, 1, 1, 1, 16#400);
end;

procedure wruns_$log(var ids: ids_t);
begin
  netlog_$log_it(5, ids, word_v, asid_v, word_v, 0, 0, 0);
end;

procedure wruns_$unlock(slot: integer32);
var
  dtv: clock_t;
  st: status_t;
begin
  ok := file_$priv_unlock(any_uid, slot, 4, 0, false, 0, 0, 0, dtv, st);
end;

procedure wruns_$lock(mode: integer);
var
  slot: integer32;
  rights: integer;
  st: status_t;
begin
  file_$priv_lock(any_uid, asid_v, 1, mode, false, 16#88, 0, 0, 0, 0, 0,
                  0, slot, rights, st);
  file_$priv_lock(any_uid, asid_v, 0, 4, true, 0, 0, 0, 0, 0, 0, 0, slot,
                  rights, st);
end;
