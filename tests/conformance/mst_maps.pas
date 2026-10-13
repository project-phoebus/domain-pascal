{ MST_$MAP, MST_$MAP_TOP, MST_$MAP_GLOBAL and MST_$MAPS,
  0xE4386C..0xE439D4 (0x60 + 0x60 + 0x56 + 0x52 bytes; tools/imgdis.sh
  E4386C 168), a fragment of the MST_UNWIRED code segment (0xE42CEC;
  `dpc --fragment-origin 0xE4386C').  Semantics: ../source/domain_os/mst/
  maps.c and the declarations in mst/mst.h; each is one call of the
  module-local routine at 0xE43182 (`mst_$alloc_segs' in ../source),
  which the map does not name and which lies outside the unit: the
  manifest gives it as `module-routines=ALLOC_SEGS=0xE43182' (bead
  domain-pascal-3h9; tests/conformance/README.md, "Routines outside the
  unit").  Externals (SAU2 map): MST_$TOUCH_COUNT 0xE24448, PROC1_$AS_ID
  0xE2060A.  The data area `.bss.moddata.MST_' is placed at the MST_UNWIRED
  data segment 0xE7CF0C (`lea (0xe7cf0c).l,A5' in each).

  Patterns exercised (docs/codegen-patterns.md):
  - 3    a pointer function's result read in A0 by its caller (bead
         domain-pascal-ae9; "Callers of pointer functions"): the wrapper
         `f := g(...)' stores the A0 of the call in its own result
         variable's slot and leaves it in A0, `move.l A0,(-4,A6)' then
         the epilogue (0xE438BE, 0xE4391E, 0xE43974, 0xE439C8); the call
         not popped (1.1);
  - 1.6  `bsr.w' to a module-local routine outside the fragment;
  - 12.4 R4(a)  the by-address parameters read once inside the argument
         list each take a new address register, A0, A1, A2, A3, A4
         (0xE43886..0xE438AA), A2-A4 saved for it;
  - 4    A5 loaded although nothing uses it (stand-in variable below;
         `a5-always=' in the manifest), saved in the `movem' with A2-A4
         or alone with `pea (A5)' (MST_$MAPS 0xE43986);
  - 1.3-1.5  word and byte value slots (the byte in the high half,
         MST_$MAPS 0xE43998 `move.b (0xa,A6),-(SP)'), `clr.w -(SP)' for
         `false', `clr.l -(SP)' for 0, `move.l #0x7fffffff', the
         2-byte pad before 38 argument bytes; MST_$MAPS `val_param', its
         byte parameters at (0xa,A6) and (0x1e,A6).

  Reconstructed (not known from the original): every name, among them
  `alloc_segs' for the routine at 0xE43182 (declared `internal; extern':
  `internal' gives the by-value convention of a routine of the same
  module, ref 5.2 p. 5-4; its body is outside the fragment) and its
  parameter list read off the pushes (12 parameters, 38 bytes); the
  modes (`var' for what is pushed as the slot's address, values for
  what is dereferenced first); `standin', the unit's data area (the
  routines load A5 but do not address it). }
module mst_;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;
var
  mst_$touch_count: extern integer;
  proc1_$as_id: extern integer;
  standin: integer32;

function alloc_segs(hint: integer32; var uid: uid_$t; start_va: integer32;
                    length: integer32; area_size: integer32; asid: integer;
                    area_id: integer; touch: integer; access: boolean;
                    direction: boolean; var map_info: integer32;
                    var status: status_$t): univ_ptr; internal; extern;

function mst_$map(var uid: uid_$t; var start_va: integer32; var length: integer32;
                  var area_id: integer; var area_size: integer32;
                  var access: boolean; var map_info: integer32;
                  var status: status_$t): univ_ptr;
begin
  mst_$map := alloc_segs(16#7fffffff, uid, start_va, length, area_size, proc1_$as_id,
                         area_id, mst_$touch_count, access, false,
                         map_info, status);
end;

function mst_$map_top(var uid: uid_$t; var start_va: integer32; var length: integer32;
                      var area_id: integer; var area_size: integer32;
                      var access: boolean; var map_info: integer32;
                      var status: status_$t): univ_ptr;
begin
  mst_$map_top := alloc_segs(16#7fffffff, uid, start_va, length, area_size, proc1_$as_id,
                             area_id, mst_$touch_count, access, false,
                             map_info, status);
end;

function mst_$map_global(var uid: uid_$t; var start_va: integer32; var length: integer32;
                         var area_id: integer; var area_size: integer32;
                         var access: boolean; var map_info: integer32;
                         var status: status_$t): univ_ptr;
begin
  mst_$map_global := alloc_segs(0, uid, start_va, length, area_size, 0,
                                area_id, 1, access, false,
                                map_info, status);
end;

function mst_$maps(asid: integer; direction: boolean; var uid: uid_$t;
                   start_va: integer32; length: integer32; area_id: integer;
                   area_size: integer32; access: boolean; var map_info: integer32;
                   var status: status_$t): univ_ptr; val_param;
begin
  mst_$maps := alloc_segs(16#7fffffff, uid, start_va, length, area_size, asid,
                          area_id, mst_$touch_count, access, direction,
                          map_info, status);
end;
