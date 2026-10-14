{ DXM_$INIT, 0xE2FF50, 0x2A bytes and the module-end 0x2048 (0xE2FF7A):
  map `I E2FF50 DXM_WIRED_ size = 2C' in OS_INIT_PROC (tools/imgdis.sh
  E2FF50 2C).  Semantics: ../source/domain_os/dxm/init.c (the Pascal
  rendering dxm/init.pas): EC_$INIT of the eventcounts of the two DXM
  queues.

  The queues are in the module's WIRED_DATA block (`D E2A7C0 DXM_WIRED_
  size = 63C'), another data section than the routine's (codegen-patterns
  11, "Data per section"; bead domain-pascal-a2g): the block's address
  as an immediate, `movea.l #0xE2A7C0,A0' (`207c'), the eventcount passed
  by address as `pea (d,A0)', and the block loaded again after the call
  (0xE2FF54, 0xE2FF66).  No A5.

  Patterns exercised (docs/codegen-patterns.md):
  - 11   another data section's block: `movea.l #block,A0; pea (0x62c,A0)'
         and, after `jsr', `movea.l #block,A0; pea (0x610,A0)';
  - 1.1, 1.6  `jsr abs.l' to another unit; the last call not popped
         before `unlk'.

  Reconstructed (not known from the original): the layout of the block is
  reduced to what this routine reaches, a filler for the two rings and the
  overrun count (0x604 bytes, ../source dxm/dxm.pas), then the two queue
  records of 0x1C bytes, the eventcount at +0x0C of each; the
  eventcount's type (12 bytes, ../source ec/ec.ins.pas); the section names
  as identifiers (the map's, folded); the initial values of the queues'
  masks and ring pointers are left out (the data section is NOBITS here
  and not compared). }
module dxm_wired_, wired_proc, wired_data;
type
  ec_$eventcount_t = record
    value: integer32;
    waiters: array[1..2] of integer32;
  end;
  dxm_$queue_t = record
    hdr: array[1..3] of integer32;          { +0x00 }
    ec: ec_$eventcount_t;                    { +0x0C }
    tail: integer32;                         { +0x18 }
  end;
var
  dxm_rings: array[1..385] of integer32;   { +0x000..0x604 }
  dxm_$unwired_q: dxm_$queue_t;            { +0x604 }
  dxm_$wired_q: dxm_$queue_t;              { +0x620 }

procedure ec_$init(out ec: ec_$eventcount_t); extern;

[section(os_init_proc, os_init_data)]
procedure dxm_$init;
begin
  ec_$init(dxm_$wired_q.ec);
  ec_$init(dxm_$unwired_q.ec);
end;
