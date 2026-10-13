{ MST_$SET_TOUCH_AHEAD_CNT, 0xE446FC..0xE4472E (0x30 bytes of code and
  the word cell 0xE4472C; tools/imgdis.sh E446FC 32), a fragment of the
  MST_UNWIRED code segment (0xE42CEC).  Semantics:
  ../source/domain_os/mst/set_touch_ahead_cnt.c.
  MST_$PRIV_SET_TOUCH_AHEAD_CNT 0xE44514, the routine before it, lies
  outside the unit (`module-routines=', `bsr.w').  A5 is loaded and not
  used (`a5-always='); the data area is placed at 0xE7CF0C.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  the constant 0 passed by address for the flags word (bead
         domain-pascal-5aw.9), `pea (0xe,PC)' at 0xE4471C, its cell after
         the routine.  MST_$REMAP (tests/conformance/mst_maps.pas) passes
         the same 0 to MST_$REMAP_PRIVI from its own cell 0xE43A0A: the
         two cells are not shared although both hold 0000 (Uncertain:
         codegen-patterns 1.5, cell sharing by type), so this unit does
         not depend on that one.

  Reconstructed (not known from the original): the names; the parameter
  types and modes. }
module mst_;
type
  status_$t = integer32;
var
  standin: integer32;

procedure mst_$priv_set_touch_ahead_cnt(flags: integer; var va: integer32;
                                        var length: integer32; var count: integer;
                                        var result: integer32; var status: status_$t); extern;

procedure mst_$set_touch_ahead_cnt(var va: integer32; var length: integer32;
                                   var count: integer; var result: integer32;
                                   var status: status_$t);
begin
  mst_$priv_set_touch_ahead_cnt(0, va, length, count, result, status);
end;
