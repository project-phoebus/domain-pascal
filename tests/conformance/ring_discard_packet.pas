{ The nested routine at 0xE76470, 0x2E bytes (tools/imgdis.sh E76470 2E),
  of the RING module-local routine at 0xE7649E, which follows it directly
  in the RING code segment 0xE75400.  Semantics:
  ../source/domain_os/ring/receive_packet.c (`ring_$discard_packet', nested
  in `ring_$receive_packet').  Externals (SAU2 map): NETBUF_$RTN_HDR
  0xE0EEB4, PKT_$DUMP_DATA 0xE127E6.

  The unit is the nested routine alone (bead domain-pascal-5aw.18): the
  manifest's `enclosing=RECEIVE_PACKET' has tools/conform.py check that
  dpc emits the enclosing routine right after it, at 0xE7649E, and leave
  that routine out of the comparison (README.md, "Nested routines").  Its
  body here is a stand-in: one call of the nested routine with the
  arguments the image passes (0xE7660C..0xE76618: a pad, a word, a
  longword, an address).  What the nested code depends on is the
  enclosing routine's heading, which is reconstructed: its fifth
  parameter, by address, at (0x16,A6) (unit word at 8, then four
  addresses at 0xA, 0xE, 0x12, 0x16; the C's `data_len_p').

  Patterns (docs/codegen-patterns.md):
  - 6    the static link: called only by its enclosing routine, so it
         takes the dynamic link, `movea.l (A6),A2' (0xE76476), and its
         caller passes no link (the A1 written before the `bsr.w' at
         0xE76618 is a scratch for an argument, 0xE7660E `movea.l
         (0x16,A6),A1; move.w (A1),-(SP)', which tools/nestsurvey.py
         classes `chain'); A2, callee-saved (`pea (A2)', 2 "Register save")
         because the link lives across the first call; loaded first, no
         parameter being loaded at entry (6, "Where the callee loads the
         link"; 12.2: each slot is used once);
  - 6    an uplevel by-address parameter dereferenced:
         `movea.l (0x16,A2),A0; move.w (A0),-(SP)' (0xE76486);
  - 6    the `internal' convention for a nested routine's own
         parameters: the pointer `hdr' by value at (8,A6), passed by
         address with `pea (8,A6)' (0xE76478); `data_pa' by address,
         passed on with `move.l (0xc,A6),-(SP)' (0xE7648C); the word
         `len' at (0x10,A6), pushed by the caller and never read;
  - 1.1, 1.4  `addq.w #4,SP' after the first call, the pad `subq.l
         #2,SP' before the word and longword of PKT_$DUMP_DATA, no pop
         before `unlk';
  - 2    `link.w A6,#-8' (see below), the restore `movea.l (-0xc,A6),A2'.

  Reconstructed (not known from the original): every name in the unit;
  the parameter types (`hdr' a pointer, the C's ring_$pkt_hdr_t *;
  `data_pa' an integer32 by address, the C's uint32_t *); NETBUF_$RTN_HDR
  and PKT_$DUMP_DATA as val_param routines with a `univ' by-address first
  parameter (../source/domain_os/netbuf/netbuf.h, pkt/pkt.h; PKT_$DUMP_DATA's
  word is pushed by value).

  The frame: `link #-8', never addressed.  The frame-size model of
  codegen-patterns 2 ("Frame size") explains 4 bytes, the slot at the top
  of a routine that is not a global symbol; the other 4 are not
  explained, as for 0xE00EB0 (ast_page_zero.pas), and are the stand-in
  local `unexplained'.  The enclosing routine's frame does not matter
  here: the nested routine reaches only its parameter. }
module ring;
type
  ring_$pkt_hdr_ptr_t = ^integer32;

procedure netbuf_$rtn_hdr(var va: univ ring_$pkt_hdr_ptr_t); options(val_param); extern;
procedure pkt_$dump_data(var buffers: univ integer32; len: integer); options(val_param);
  extern;

procedure receive_packet(unit: integer; var hdr_p: ring_$pkt_hdr_ptr_t;
                         var data_pa: integer32; var hdr_len: integer;
                         var data_len: integer); internal;

  procedure discard_packet(hdr: ring_$pkt_hdr_ptr_t; var data_pa: integer32;
                           len: integer);
  var
    unexplained: integer32;   { stand-in: 4 frame bytes the model leaves unexplained }
  begin
    netbuf_$rtn_hdr(hdr);
    pkt_$dump_data(data_pa, data_len);
  end;

begin
  { stand-in body (not compared): the call at 0xE7660C..0xE76618 }
  discard_packet(hdr_p, data_pa, data_len);
end;
